

import dataclasses
import json
import random
import re
import threading
import time
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from datetime import datetime

from .config import (
    WIN_PROJECT_DIR, MAX_CONCURRENT_FIXES, SCOPE_CONSTRAINTS,
    SONAR_PARALLEL_GROUPS, log,
)
from .runner import wsl_bash, run_kiro
from .classifier import classify_error
from .history import (
    record_fix_attempt, get_past_attempts, _error_hash,
    _fix_history_lock, _load_fix_history,
)
from .memory import (
    store_fix_attempt, store_fix_outcome,
    search_relevant_context, extract_outcome_from_output,
)
from .gates import (
    preflight_dedup, verification_run, revision_check_diff,
    revision_security_check, revision_maintainability_check,
    revision_correctness_check, baseline_verification_incremental,
    verification_run_pbt,
)
from .reproducer import reproduce_locally, format_errors_for_prompt, ReproCache
from .trends import (
    record_duration, check_and_alert_trends, get_all_trends,
    record_optimization, get_optimization_history,
)
from .vulns import extract_vuln_info, record_vuln_failure
from .decisions import record_strategy_outcome, get_ranked_strategies
from .flaky import parse_test_results, record_test_outcomes, are_all_failures_flaky
from .security import sanitize_for_prompt
from .tracker import get_tracker, JobState


_DOMAIN_INVARIANTS = {
    "contratos": [
        "No overlapping active contracts: a propiedad cannot have two contratos "
        "with estado='activo' whose date ranges overlap.",
        "Contrato integrity: every contrato references exactly one propiedad and one inquilino.",
        "Propiedad estado cascade: creating an active contrato sets propiedad.estado to 'ocupada'. "
        "Cancelling the last active contrato sets it back to 'disponible'.",
    ],
    "inquilinos": [
        "Cedula uniqueness: inquilino.cedula is unique across the system.",
    ],
    "usuarios": [
        "Email uniqueness: usuario.email is unique across the system.",
    ],
    "pagos": [
        "Payment lateness: a pago is late when fecha_pago > fecha_vencimiento (paid after due) "
        "OR fecha_pago IS NULL AND fecha_vencimiento < today (unpaid past due).",
        "A pago always belongs to a contrato.",
    ],
    "gastos": [
        "Gasto scope: a gasto belongs to a propiedad and optionally to a unidad. "
        "If unidad_id is set, it must belong to the referenced propiedad.",
    ],
    "propiedades": [
        "Currency consistency: every monetary entity carries its own moneda field (DOP or USD).",
        "Propiedad estado cascade: creating an active contrato sets estado to 'ocupada'.",
    ],
}


def _get_domain_from_files(files: list[str]) -> set[str]:
    domains = set()
    for f in files:
        lower = f.lower()
        for domain in _DOMAIN_INVARIANTS:
            if domain in lower:
                domains.add(domain)
    return domains


def build_invariant_section(affected_files: list[str]) -> str:
    domains = _get_domain_from_files(affected_files)
    if not domains:
        return ""
    lines = ["BUSINESS INVARIANTS (these MUST hold — violations are bugs):"]
    for domain in sorted(domains):
        for inv in _DOMAIN_INVARIANTS[domain]:
            lines.append(f"  - {inv}")
    return "\n".join(lines) + "\n"


@dataclass
class PhaseTiming:
    reproduction_s: float = 0.0
    diagnosis_s: float = 0.0
    fix_attempt_s: float = 0.0
    scope_gate_s: float = 0.0
    verification_s: float = 0.0
    total_s: float = 0.0


_sonar_fix_lock = threading.Lock()
_improve_lock = threading.Lock()

# Per-job lock registry: allows parallel fixes for different jobs
_job_locks: dict[str, threading.Lock] = {}
_job_locks_guard = threading.Lock()
_git_lock = threading.Lock()
_max_concurrent_fixes = threading.Semaphore(MAX_CONCURRENT_FIXES)

_pending_fixes_lock = threading.Lock()
_pending_fixes: dict[str, tuple[str, str, str, dict | None]] = {}


def _get_job_lock(job: str) -> threading.Lock:
    with _job_locks_guard:
        if job not in _job_locks:
            _job_locks[job] = threading.Lock()
        return _job_locks[job]


def _queue_pending_fix(job, step, error_log, context):
    with _pending_fixes_lock:
        prev = _pending_fixes.get(job)
        _pending_fixes[job] = (job, step, error_log, context)
        if prev is not None:
            log.info(f"Replaced stale pending fix for job '{job}' with newer failure")
        else:
            log.info(f"Queued pending fix for job '{job}' (fix already in progress)")


def _pop_pending_fix(job):
    with _pending_fixes_lock:
        return _pending_fixes.pop(job, None)


_AUTO_FIX_PATTERN = re.compile(r"^[0-9a-f]+ fix: resolve CI failures")
_MAX_AUTO_FIX_CHAIN = 2


def _check_auto_fix_chain(job):
    with _git_lock:
        result = wsl_bash("git log --oneline -5 main", timeout=30)
    if result.returncode != 0:
        return False
    lines = [l.strip() for l in result.stdout.strip().splitlines() if l.strip()]
    if len(lines) < _MAX_AUTO_FIX_CHAIN:
        return False
    consecutive = 0
    for line in lines:
        if _AUTO_FIX_PATTERN.match(line):
            consecutive += 1
        else:
            break
    if consecutive < _MAX_AUTO_FIX_CHAIN:
        return False
    with _fix_history_lock:
        history = _load_fix_history()
    recent_failures = 0
    for key, entry in history.items():
        if not isinstance(entry, dict) or entry.get("job") != job:
            continue
        for attempt in entry.get("attempts", [])[-consecutive:]:
            if not attempt.get("success"):
                recent_failures += 1
    return recent_failures >= _MAX_AUTO_FIX_CHAIN


def _rollback_last_auto_fix(job):
    log.warning(f"Auto-fix loop detected for {job}. Reverting last auto-fix commit.")
    with _git_lock:
        revert_result = wsl_bash(
            "git revert HEAD --no-edit && "
            "git push origin main || "
            "(git pull --rebase origin main && git push origin main)",
            timeout=120,
        )
    revert_ok = revert_result.returncode == 0
    if revert_ok:
        log.info(f"Successfully reverted last auto-fix for {job}")
    else:
        log.error(f"Failed to revert auto-fix for {job}: {revert_result.stderr[:500]}")
    record_fix_attempt(job, "auto-fix-loop-revert", 0, revert_ok,
                       f"rollback: reverted last auto-fix commit for {job}")
    return revert_ok


def _parse_strategies(diagnosis_output):
    strategies = []
    m1 = re.search(r"STRATEGY_1:\s*(.+?)\s*\((\d+)%\)", diagnosis_output)
    m2 = re.search(r"STRATEGY_2:\s*(.+?)\s*\((\d+)%\)", diagnosis_output)
    if m1:
        strategies.append((m1.group(1).strip(), int(m1.group(2))))
    if m2:
        strategies.append((m2.group(1).strip(), int(m2.group(2))))
    return strategies


def _parse_run_id(run_url):
    """Extract the numeric run ID from a GitHub Actions run URL."""
    if not run_url:
        return "", ""
    match = re.search(r"github\.com/([^/]+/[^/]+)/actions/runs/(\d+)", run_url)
    if match:
        return match.group(2), match.group(1)
    match = re.search(r"/actions/runs/(\d+)", run_url)
    return (match.group(1), "") if match else ("", "")


def _gh_cli_block(run_url):
    """Compact gh CLI reference for fetching full pipeline logs."""
    run_id, repo = _parse_run_id(run_url)
    if not run_id:
        return ""
    r = f" -R {repo}" if repo else ""
    return (
        f"\nGH CLI (error_log may be truncated -- fetch full logs first):\n"
        f"  gh run view {run_id}{r} --log-failed\n"
        f"  gh run view {run_id}{r} --json jobs "
        f"--jq '.jobs[]|\"\\(.name) (\\(.conclusion)) id=\\(.databaseId)\"'\n"
        f"  gh api repos/{repo}/check-runs/<check_run_id>/annotations\n"
        f"Run: {run_url}\n"
    )


_JOB_VERIFY = {
    "lint": "cargo fmt --all && cargo clippy --locked -p realestate-backend -- -D warnings && cargo clippy --locked -p realestate-frontend --target wasm32-unknown-unknown -- -D warnings",
    "test-backend": "cargo test --locked -p realestate-backend --all-targets",
    "test-frontend": "cargo test --locked -p realestate-frontend --all-targets",
    "quality-gate": "cargo audit && cargo deny check",
    "android-lint": "(cd android && ./gradlew lint detekt)",
    "android-unit-test": "(cd android && ./gradlew testDebugUnitTest)",
    "android-build": "(cd android && ./gradlew assembleDebug)",
    "android-quality-gate": "(cd android && ./gradlew dependencies)",
    "build-frontend": "(cd frontend && trunk build --release)",
    "build-backend": "cargo build -p realestate-backend --release",
    "deploy": "docker compose -f docker-compose.prod.yml config --quiet",
}

_JOB_INSTRUCTIONS = {
    "lint": "Fix formatting and clippy warnings.",
    "test-backend": "Fix failing backend tests. Read test output to identify which tests failed and why.",
    "test-frontend": "Fix failing frontend tests.",
    "quality-gate": (
        "Fix quality gate failures (cargo-audit/cargo-deny/OWASP). "
        "Update crate versions in Cargo.toml or suppress in deny.toml. Do NOT modify CI YAML."
    ),
    "sonarqube": "Fix SonarQube analysis failures. Check server config and scan settings.",
    "android-lint": (
        "Fix Android lint/detekt warnings in android/. "
        "For ktfmt: run './gradlew spotlessApply' to auto-fix."
    ),
    "android-unit-test": "Fix failing Android unit tests in android/.",
    "android-build": "Fix Android build errors in android/.",
    "android-quality-gate": (
        "Fix Android dependency vulnerabilities (CVSS >= 7). "
        "Update versions in android/gradle/libs.versions.toml or add suppression."
    ),
    "android-sonarqube": "Fix Android SonarQube failures. Check scan config and connectivity.",
    "build-frontend": "Fix frontend WASM build errors (target wasm32-unknown-unknown).",
    "build-backend": "Fix backend release build errors.",
    "container-image-backend": "Fix backend container build or Trivy scan failure. Check Dockerfile.backend.",
    "container-image-frontend": "Fix frontend container build or Trivy scan failure. Check Dockerfile.frontend.",
    "deploy": (
        "Deploy failure. Distinguish between:\n"
        "- [health-check] failures: the app deployed but /health is not responding. "
        "This is likely an application bug — check startup code, database connections, "
        "and health endpoint implementation.\n"
        "- [deploy] failures: docker compose up failed. Check docker-compose.prod.yml "
        "syntax, image references, and environment variables.\n"
        "- [attestation] failures: image attestation verification failed. Check the "
        "container build and signing process."
    ),
    "secret-scan": (
        "WARNING: Gitleaks found secrets. Remove the credential, "
        "add to .gitleaksignore if false positive, rotate any exposed secrets."
    ),
}


def _build_error_notes(error_class):
    notes = {
        "runner_environment": (
            "\nRUNNER ENVIRONMENT issue (missing tool, network, disk, Docker). "
            "Do NOT change application code. Document what the runner admin should fix."
        ),
        "dependency": (
            "\nDEPENDENCY FIX: parse CVE/RUSTSEC IDs → find patched version → "
            "update Cargo.toml → cargo update -p <crate> → cargo audit. "
            "If no patch: suppress in .cargo/audit.toml or find alternative. "
            "Verify: cargo test --locked -p realestate-backend --all-targets."
        ),
        "test_failure": (
            "\nFLAKY CHECK: re-run the failing test 2x first. "
            "If it passes on re-run, it's flaky -- do NOT change code. "
            "Only fix if it fails consistently."
        ),
        "flaky": (
            "\nINTERMITTENT FAILURE (network/timing/resources). "
            "Do NOT change code. Re-run CI instead. "
            "Escalate if same error persists across 3+ runs."
        ),
        "build_failure": (
            "\nBUILD FAILURE: read the full compiler output. "
            "Check for missing imports, type errors, linker failures, "
            "or WASM target incompatibilities. Fix the source code."
        ),
        "pbt_failure": (
            "\nPROPERTY-BASED TEST FAILURE: This is a proptest edge case.\n"
            "1. Read the minimal failing input — it's the exact boundary condition.\n"
            "2. Re-run with the seed to reproduce: PROPTEST_SEED=<seed> cargo test <name>\n"
            "3. Fix the business logic, not the test. The test found a real bug.\n"
            "4. NEVER reduce PROPTEST_CASES or skip PBT tests.\n"
            "5. After fixing, run with 500 cases to verify: PROPTEST_CASES=500 cargo test <name>\n"
        ),
    }
    return notes.get(error_class, "")


def _diagnose(job, step, error_log, error_class, context, repro=None, deep=False):
    """Diagnose a CI failure using local reproduction + gh CLI for full logs.

    When deep=True, includes past attempt history, semantic memory, and
    instructions to research libraries/changelogs. Returns up to 4000 chars.
    When deep=False (default), returns a short strategy string (~2000 chars).
    """
    run_url = context.get("run_url", "") if context else ""
    ctx = ""
    if context:
        ctx = f" (commit: {context.get('commit', '?')[:8]}, branch: {context.get('branch', '?')})"

    effective_class = repro.error_class if (repro and repro.error_class) else error_class

    repro_section = ""
    if repro and repro.reproduced:
        repro_section = format_errors_for_prompt(repro)
    elif repro:
        repro_section = "\nDid NOT reproduce locally -- likely CI-specific (env/cache/Docker).\n"

    reproduced = repro and repro.reproduced
    source = "local reproduction output" if reproduced else "gh CLI full logs"
    error_preview_len = 3000 if deep else 2000

    preamble = (
        "DEEP RESEARCH MODE -- previous quick-fix attempts have ALL failed.\n"
        if deep else ""
    )

    prompt = (
        f"{preamble}"
        f"DIAGNOSE ONLY -- do NOT edit files or run git.\n\n"
        f"CI FAILED: job='{job}', step='{step}'{ctx}, class={effective_class}\n"
        f"{repro_section}"
        f"{_gh_cli_block(run_url)}"
        f"Error preview:\n```\n{sanitize_for_prompt(error_log, error_preview_len)}\n```\n\n"
        f"Job context: {_JOB_INSTRUCTIONS.get(job, 'Analyze and fix.')}"
        f"{_build_error_notes(effective_class)}\n\n"
    )

    if deep:
        past = get_past_attempts(job, error_log)
        semantic = search_relevant_context(job, error_log)
        if past:
            prompt += f"{past}\n\n"
        if semantic:
            prompt += f"{semantic}\n\n"
        prompt += (
            f"INSTRUCTIONS:\n"
            f"1. Fetch the FULL CI logs using: gh run view <id> --log-failed\n"
            f"2. Read every file mentioned in the errors. Understand the code, not just the error message.\n"
            f"3. Check git log for recent changes to those files.\n"
            f"4. Identify the ROOT CAUSE -- not the symptom. Why did previous fixes fail?\n"
            f"5. Research: if this involves a library or framework, look up docs or changelogs.\n"
            f"6. Write a detailed analysis to stdout:\n"
            f"   - Root cause (1-2 sentences)\n"
            f"   - Why previous approaches failed\n"
            f"   - Exact fix plan: which files, which lines, what changes\n"
            f"   - Any risks or side effects\n"
        )
    else:
        prompt += (
            f"1. Identify root cause from {source}.\n"
            f"2. List specific files/lines that need changes.\n"
            f"3. Describe minimal fix strategy in 2-3 sentences.\n"
            f"4. Output your top 2 fix strategies with confidence (0-100). "
            f"Format: STRATEGY_1: <name> (<confidence>%) / STRATEGY_2: <name> (<confidence>%)"
        )

    label = f"Deep research ({job}/{step})" if deep else f"Diagnose ({job}/{step})"
    result = run_kiro(prompt, label)
    kiro_output = result[1] if isinstance(result, tuple) else ""
    limit = 4000 if deep else 2000
    return kiro_output[-limit:] if kiro_output else "diagnosis unavailable"


def _log_timing_summary(job, step, cycle_start, all_timings):
    """Log a summary of phase timing for the entire fix cycle."""
    total_cycle_s = time.monotonic() - cycle_start
    n = len(all_timings)
    if n == 0:
        log.info(f"Fix cycle timing for {job}/{step}: total={total_cycle_s:.1f}s (no attempts)")
        return

    sum_repro = sum(t.reproduction_s for t in all_timings)
    sum_fix = sum(t.fix_attempt_s for t in all_timings)
    sum_scope = sum(t.scope_gate_s for t in all_timings)
    sum_verify = sum(t.verification_s for t in all_timings)

    log.info(
        f"Fix cycle timing for {job}/{step}: "
        f"total={total_cycle_s:.1f}s, attempts={n}, "
        f"reproduction={sum_repro:.1f}s, "
        f"diagnosis+fix={sum_fix:.1f}s, "
        f"scope_gate={sum_scope:.1f}s, "
        f"verification={sum_verify:.1f}s"
    )


def fix_once_and_push(job, step, error_log, context=None):
    tracker = get_tracker()
    commit = context.get("commit", "") if context else ""
    pipeline = tracker.get_or_create(commit) if commit else None

    if pipeline and pipeline.deployed:
        log.info(f"Pipeline {pipeline.lineage_id} already deployed, skipping fix for {job}/{step}")
        return False
    if commit and tracker.is_budget_exhausted(commit):
        _write_pipeline_escalation(pipeline)
        log.error(
            f"Pipeline budget exhausted for lineage "
            f"{pipeline.lineage_id if pipeline else 'unknown'} "
            f"(round {pipeline.round_number if pipeline else '?'}/"
            f"{pipeline.budget_total if pipeline else '?'}). "
            f"No more fixes will be dispatched."
        )
        return False
    if commit:
        should_wait, wait_s = tracker.should_backoff(commit, job)
        if should_wait:
            log.info(f"Backoff {wait_s}s before fixing {job}/{step}")
            time.sleep(wait_s)
            pipeline = tracker.get_or_create(commit)
            if pipeline.deployed:
                log.info(f"Pipeline deployed during backoff, skipping fix for {job}/{step}")
                return False
    if pipeline and pipeline.job_states.get(job) == JobState.FIXING:
        log.info(f"Job {job} already being fixed for lineage {pipeline.lineage_id}, skipping")
        return False

    job_lock = _get_job_lock(job)
    if not job_lock.acquire(blocking=False):
        _queue_pending_fix(job, step, error_log, context)
        return False

    _max_concurrent_fixes.acquire()
    cycle_start = time.monotonic()
    timing = PhaseTiming()
    _fix_count_changed = False
    try:
        try:
            from .server import _active_fix_count_lock
            import scripts.quality_webhook.server as _srv
            with _active_fix_count_lock:
                _srv._active_fix_count += 1
                _fix_count_changed = True
        except ImportError:
            pass

        if commit:
            tracker.mark_job_fixing(commit, job)

        if _check_auto_fix_chain(job):
            _rollback_last_auto_fix(job)
            if commit:
                tracker.mark_job_fixing(commit, job)
                pipeline_now = tracker.get_or_create(commit)
                pipeline_now.job_states[job] = JobState.FAILED
            return False

        passed, reason = preflight_dedup(job, error_log)
        if not passed:
            log.warning(f"Preflight dedup rejected {job}/{step}: {reason}")
            log.info(f"Entering deep research phase for {job}/{step}")
            research_fix = _deep_research_fix(job, step, error_log, context)
            if not research_fix:
                _write_escalation(job, step, error_log, "dedup_rejected", context,
                                  extra_reason=reason)
                if commit:
                    pipeline_now = tracker.get_or_create(commit)
                    pipeline_now.job_states[job] = JobState.FAILED
            return research_fix

        ctx_str = ""
        if context:
            ctx_str = (
                f" (commit: {context.get('commit', '?')[:8]}, "
                f"branch: {context.get('branch', '?')}, "
                f"actor: {context.get('actor', '?')})"
            )

        error_class = classify_error(error_log)
        log.info(f"Error classified as: {error_class} for {job}/{step}")

        test_results = parse_test_results(error_log)
        if test_results:
            record_test_outcomes(job, test_results)

        if error_class == "test_failure":
            all_flaky, flaky_names = are_all_failures_flaky(error_log)
            if all_flaky:
                log.info(
                    f"All failing tests are known-flaky for {job}/{step}: "
                    f"{flaky_names}. Skipping fix attempt. "
                    f"Recommendation: quarantine these tests."
                )
                if commit:
                    pipeline_now = tracker.get_or_create(commit)
                    pipeline_now.job_states[job] = JobState.FAILED
                return False

        if error_class == "dependency":
            vuln_pairs = extract_vuln_info(error_log)
            for crate, advisories in vuln_pairs:
                record_vuln_failure(crate, advisories)

        repro_cache = ReproCache(error_hash=_error_hash(job, error_log))

        log.info(f"=== Reproducing {job}/{step} locally ===")
        t0 = time.monotonic()
        repro = reproduce_locally(job, error_class, cache=repro_cache)
        timing.reproduction_s = time.monotonic() - t0
        if repro.reproduced:
            log.info(f"Failure REPRODUCED locally: {len(repro.errors)} errors, "
                     f"class refined to '{repro.error_class}'")
            if repro.error_class:
                error_class = repro.error_class
        elif repro.raw_output:
            log.info(f"Failure did NOT reproduce locally for {job}/{step}")
        else:
            log.info(f"No local reproduction commands for {job}/{step}")

        past_attempts_section = get_past_attempts(job, error_log)
        semantic_context = search_relevant_context(job, error_log)

        attempt_start = time.monotonic()

        attempt = pipeline.round_number if pipeline else 1
        _PHASE_QUICK_MAX = 3
        _PHASE_DEEP_MAX = 6
        if attempt <= _PHASE_QUICK_MAX:
            phase = "quick"
        elif attempt <= _PHASE_DEEP_MAX:
            phase = "deep"
        else:
            phase = "investigate"

        log.info(f"=== Fix attempt (round {attempt}) [{phase}] for {job}/{step}{ctx_str} (class: {error_class}) ===")

        instruction = _JOB_INSTRUCTIONS.get(job, "Analyze and fix.")
        verify_cmd = _JOB_VERIFY.get(job, "")
        error_note = _build_error_notes(error_class)

        run_url = context.get("run_url", "") if context else ""

        repro_section = ""
        if repro and repro.reproduced:
            repro_section = format_errors_for_prompt(repro)
        elif repro:
            repro_section = "\nDid NOT reproduce locally -- likely CI-specific (env/cache/Docker).\n"

        reproduced = repro and repro.reproduced
        source = "local reproduction output" if reproduced else "gh CLI full logs"

        ranked_strategies = get_ranked_strategies(job, error_class)
        strategies_section = ""
        if ranked_strategies:
            lines = [f"  - {s}: {r:.0%} success rate" for s, r in ranked_strategies]
            strategies_section = "\nPREFERRED STRATEGIES (by historical success rate):\n" + "\n".join(lines) + "\n"

        exploring = False
        parsed_strategies = []

        if phase == "deep":
            log.info(f"Phase escalation: running deep diagnosis before round {attempt}")
            t0 = time.monotonic()
            diagnosis = _diagnose(job, step, error_log, error_class, context,
                                  repro=repro, deep=True)
            timing.diagnosis_s = time.monotonic() - t0

            prompt = (
                f"RESEARCH-INFORMED FIX for {job}/{step}{ctx_str}, "
                f"class={error_class}, round {attempt}\n\n"
                f"Previous quick-fix attempts (1-{_PHASE_QUICK_MAX}) all failed. "
                f"A deep analysis was performed. Here are the findings:\n"
                f"```\n{diagnosis}\n```\n\n"
                f"{repro_section}"
                f"{_gh_cli_block(run_url)}"
                f"Error preview:\n```\n{sanitize_for_prompt(error_log, 3000)}\n```\n\n"
                f"Job context: {instruction}{error_note}\n\n"
                f"{strategies_section}"
                f"EXECUTE the fix plan from the analysis above.\n"
                f"STEP 1 - FIX: Apply the changes described in the analysis.\n"
            )

        elif phase == "investigate":
            log.info(f"Phase escalation: investigation mode for round {attempt}")
            t0 = time.monotonic()
            diagnosis = _diagnose(job, step, error_log, error_class, context,
                                  repro=repro, deep=True)
            timing.diagnosis_s = time.monotonic() - t0

            prompt = (
                f"INVESTIGATION MODE for {job}/{step}{ctx_str}, "
                f"class={error_class}, round {attempt}\n\n"
                f"ALL previous attempts ({attempt - 1}) have failed. "
                f"Standard fixes and deep research have not worked.\n\n"
                f"Previous analysis:\n```\n{diagnosis}\n```\n\n"
                f"{repro_section}"
                f"{_gh_cli_block(run_url)}"
                f"Error preview:\n```\n{sanitize_for_prompt(error_log, 3000)}\n```\n\n"
                f"Job context: {instruction}{error_note}\n\n"
                f"MANDATORY INVESTIGATION STEPS before writing any fix:\n"
                f"1. Fetch FULL CI logs: gh run view <id> --log-failed\n"
                f"2. Read EVERY file mentioned in errors + their imports and callers\n"
                f"3. Check git log -10 for recent changes to affected files\n"
                f"4. Diff Cargo.lock or gradle lockfiles for dependency changes\n"
                f"5. Search the web for the exact error message if it involves a library\n"
                f"6. Read project steering files in .kiro/steering/ for conventions\n"
                f"7. Check if the error exists in files NOT mentioned in the error output\n"
                f"8. Write a 3-sentence plan BEFORE editing any file\n\n"
                f"THEN apply the fix.\n"
                f"STEP 1 - INVESTIGATE: Complete all 8 steps above.\n"
                f"STEP 2 - FIX: Apply the minimal fix based on your investigation.\n"
            )

        else:
            prompt = (
                f"CI FAILED: job='{job}', step='{step}'{ctx_str}, "
                f"class={error_class}, round {attempt}\n\n"
                f"{repro_section}"
                f"{_gh_cli_block(run_url)}"
                f"Error preview:\n```\n{sanitize_for_prompt(error_log, 3000)}\n```\n\n"
                f"Job context: {instruction}{error_note}\n\n"
                f"{strategies_section}"
                f"STEP 1 - DIAGNOSE: Identify root cause from {source}. "
                f"List specific files/lines that need changes.\n"
                f"STEP 2 - FIX: Apply the minimal fix to resolve the issue.\n"
            )

            if phase == "quick" and attempt == 1:
                diag_for_strats = _diagnose(
                    job, step, error_log, error_class, context,
                    repro=repro, deep=False,
                )
                parsed_strategies = _parse_strategies(diag_for_strats)
                if len(parsed_strategies) >= 2 and random.random() < 0.10:
                    exploring = True
                    alt_strategy = parsed_strategies[1][0]
                    prompt += f"\nEXPLORATION: Prefer this alternative approach: {alt_strategy}\n"
                    log.info(f"Exploration mode: trying alternative strategy '{alt_strategy}' for {job}/{step}")

        verify_line = f"STEP 3 - VERIFY: Run: {verify_cmd}\n" if verify_cmd else ""
        prompt += verify_line
        prompt += "STEP 4 - STAGE: git add -A\n"

        extras = ""
        if attempt == 1 or phase != "quick":
            if past_attempts_section:
                extras += f"\n{past_attempts_section}"
            if semantic_context:
                extras += f"\n{semantic_context}"

        affected_files = sorted({e.file for e in repro.errors if e.file}) if repro else []
        invariant_section = build_invariant_section(affected_files)
        if invariant_section:
            extras += f"\n{invariant_section}"

        if error_class == "pbt_failure" and repro and repro.raw_output:
            from .reproducer import _parse_pbt_failures
            pbt_errors = _parse_pbt_failures(repro.raw_output)
            if pbt_errors:
                seed_notes = "\n".join(str(e) for e in pbt_errors[:5])
                extras += f"\nPBT SEED INFO:\n{seed_notes}\n"

        prompt += (
            f"{extras}"
            f"{SCOPE_CONSTRAINTS}\n"
            "Do NOT commit."
        )

        t0 = time.monotonic()
        result = run_kiro(prompt, f"CI fix ({job}/{step}) round {attempt} [{phase}]")
        timing.fix_attempt_s = time.monotonic() - t0

        success = result[0] if isinstance(result, tuple) else result
        kiro_output = result[1] if isinstance(result, tuple) else ""

        outcome = extract_outcome_from_output(kiro_output) if kiro_output else {}
        strategy_note = outcome.get("strategy", "")

        pbt_seed_note = ""
        if error_class == "pbt_failure" and repro and repro.raw_output:
            from .reproducer import _parse_pbt_failures as _ppf
            pbt_errs = _ppf(repro.raw_output)
            if pbt_errs:
                pbt_seed_note = "; ".join(str(e) for e in pbt_errs[:3])

        if not success:
            timing.total_s = time.monotonic() - attempt_start
            log.warning(f"Round {attempt} kiro-cli returned failure for {job}/{step}")
            record_strategy_outcome(job, error_class, strategy_note or error_class, False)
            record_fix_attempt(job, error_log, attempt, False,
                               f"class={error_class}, job={job}, step={step}. {strategy_note}",
                               timing=dataclasses.asdict(timing))
            store_fix_attempt(job, step, error_class, error_log, attempt, False)
            if commit:
                pipeline_now = tracker.get_or_create(commit)
                pipeline_now.job_states[job] = JobState.FAILED
            return False

        rev_passed, rev_reason = revision_check_diff()
        if not rev_passed:
            timing.total_s = time.monotonic() - attempt_start
            log.warning(f"Gate 3a rejected round {attempt} ({job}/{step}): {rev_reason}")
            wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
            record_strategy_outcome(job, error_class, strategy_note or error_class, False)
            record_fix_attempt(job, error_log, attempt, False,
                               f"revision_rejected: {rev_reason}. {strategy_note}",
                               timing=dataclasses.asdict(timing))
            store_fix_attempt(job, step, error_class, error_log, attempt, False)
            if commit:
                pipeline_now = tracker.get_or_create(commit)
                pipeline_now.job_states[job] = JobState.FAILED
            return False

        sec_passed, sec_reason = revision_security_check()
        if not sec_passed:
            timing.total_s = time.monotonic() - attempt_start
            log.warning(f"Gate 3b rejected round {attempt} ({job}/{step}): {sec_reason}")
            wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
            record_strategy_outcome(job, error_class, strategy_note or error_class, False)
            record_fix_attempt(job, error_log, attempt, False,
                               f"security_rejected: {sec_reason}. {strategy_note}",
                               timing=dataclasses.asdict(timing))
            store_fix_attempt(job, step, error_class, error_log, attempt, False)
            if commit:
                pipeline_now = tracker.get_or_create(commit)
                pipeline_now.job_states[job] = JobState.FAILED
            return False

        maint_passed, maint_reason = revision_maintainability_check()
        if not maint_passed:
            timing.total_s = time.monotonic() - attempt_start
            log.warning(f"Gate 3c rejected round {attempt} ({job}/{step}): {maint_reason}")
            wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
            record_strategy_outcome(job, error_class, strategy_note or error_class, False)
            record_fix_attempt(job, error_log, attempt, False,
                               f"maintainability_rejected: {maint_reason}. {strategy_note}",
                               timing=dataclasses.asdict(timing))
            store_fix_attempt(job, step, error_class, error_log, attempt, False)
            if commit:
                pipeline_now = tracker.get_or_create(commit)
                pipeline_now.job_states[job] = JobState.FAILED
            return False

        correct_passed, correct_reason = revision_correctness_check()
        if not correct_passed:
            timing.total_s = time.monotonic() - attempt_start
            log.warning(f"Gate 3e rejected round {attempt} ({job}/{step}): {correct_reason}")
            wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
            record_strategy_outcome(job, error_class, strategy_note or error_class, False)
            record_fix_attempt(job, error_log, attempt, False,
                               f"correctness_rejected: {correct_reason}. {strategy_note}",
                               timing=dataclasses.asdict(timing))
            store_fix_attempt(job, step, error_class, error_log, attempt, False)
            if commit:
                pipeline_now = tracker.get_or_create(commit)
                pipeline_now.job_states[job] = JobState.FAILED
            return False

        t0 = time.monotonic()
        if error_class == "pbt_failure":
            verify_passed, verify_reason = verification_run_pbt(job)
        else:
            verify_passed, verify_reason = verification_run(job)
        timing.verification_s = time.monotonic() - t0

        if not verify_passed:
            timing.total_s = time.monotonic() - attempt_start
            log.warning(f"Gate 4 verification failed for round {attempt} ({job}/{step}): {verify_reason}")
            wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
            record_strategy_outcome(job, error_class, strategy_note or error_class, False)
            record_fix_attempt(job, error_log, attempt, False,
                               f"verify_failed: {verify_reason[:200]}. {strategy_note}",
                               timing=dataclasses.asdict(timing))
            store_fix_attempt(job, step, error_class, error_log, attempt, False)
            if commit:
                pipeline_now = tracker.get_or_create(commit)
                pipeline_now.job_states[job] = JobState.FAILED
            return False

        baseline_passed, baseline_reason = baseline_verification_incremental(job)
        if not baseline_passed:
            timing.total_s = time.monotonic() - attempt_start
            log.warning(f"Gate 5 baseline failed for round {attempt} ({job}/{step}): {baseline_reason}")
            wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
            record_strategy_outcome(job, error_class, strategy_note or error_class, False)
            record_fix_attempt(job, error_log, attempt, False,
                               f"baseline_failed: {baseline_reason[:200]}. {strategy_note}",
                               timing=dataclasses.asdict(timing))
            store_fix_attempt(job, step, error_class, error_log, attempt, False)
            if commit:
                pipeline_now = tracker.get_or_create(commit)
                pipeline_now.job_states[job] = JobState.FAILED
            return False

        timing.total_s = time.monotonic() - attempt_start
        log.info(f"All gates passed for round {attempt} ({job}/{step}). Committing.")

        lineage_tag = ""
        if pipeline:
            lineage_tag = f" [lineage:{pipeline.lineage_id[:12]}]"
        commit_msg = f"fix: resolve CI failures (auto-fix){lineage_tag}"

        pre_fix_sha_result = wsl_bash("git rev-parse HEAD", timeout=10)
        pre_fix_sha = pre_fix_sha_result.stdout.strip()[:12] if pre_fix_sha_result.returncode == 0 else "unknown"
        with _git_lock:
            commit_result = wsl_bash(
                "git add -A && "
                f"git commit -m '{commit_msg}' && "
                "git push origin main || "
                "(git pull --rebase origin main && git push origin main)",
                timeout=120,
            )

        commit_ok = commit_result.returncode == 0
        record_strategy_outcome(job, error_class, strategy_note or error_class, commit_ok)
        if exploring and parsed_strategies:
            unchosen_name = parsed_strategies[0][0]
            record_strategy_outcome(job, error_class, unchosen_name, None)
        record_fix_attempt(job, error_log, attempt, commit_ok,
                           f"class={error_class}, job={job}, step={step}. pre_fix_sha={pre_fix_sha}. {strategy_note}"
                           + (f" PBT: {pbt_seed_note}" if pbt_seed_note else ""),
                           timing=dataclasses.asdict(timing))
        store_fix_attempt(job, step, error_class, error_log, attempt, commit_ok)

        if commit_ok:
            new_sha_result = wsl_bash("git rev-parse HEAD", timeout=10)
            new_sha = new_sha_result.stdout.strip() if new_sha_result.returncode == 0 else ""
            files_changed = [f.strip() for f in outcome.get("files_changed", "").split(",") if f.strip()]

            if commit:
                tracker.mark_job_fixed(commit, job, new_sha, files_changed, strategy_note or f"round_{attempt}_{phase}")

            store_fix_outcome(
                job, error_class,
                files_changed=outcome.get("files_changed", ""),
                strategy=strategy_note or f"Fixed via round {attempt}",
            )
            log.info(f"Round {attempt} completed for {job}/{step}")
            total_cycle_s = time.monotonic() - cycle_start
            record_duration(job, total_cycle_s, True)
            at_risk = check_and_alert_trends()
            for rj in at_risk:
                log.warning(f"Trend alert: job '{rj}' approaching timeout threshold")
            return True

        log.warning(f"Commit/push failed for round {attempt} ({job}/{step})")
        if commit:
            pipeline_now = tracker.get_or_create(commit)
            pipeline_now.job_states[job] = JobState.FAILED
        return False

    finally:
        wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
        if _fix_count_changed:
            try:
                from .server import _active_fix_count_lock
                import scripts.quality_webhook.server as _srv
                with _active_fix_count_lock:
                    _srv._active_fix_count -= 1
            except ImportError:
                pass
        _max_concurrent_fixes.release()
        job_lock.release()

        pending = _pop_pending_fix(job)
        if pending is not None:
            p_job, p_step, p_error_log, p_context = pending
            log.info(f"Draining queued fix for job '{p_job}/{p_step}'")
            threading.Thread(
                target=fix_once_and_push,
                args=(p_job, p_step, p_error_log, p_context),
                daemon=True,
            ).start()


def _fix_with_retry_legacy(job, step, error_log, context=None):
    job_lock = _get_job_lock(job)
    if not job_lock.acquire(blocking=False):
        _queue_pending_fix(job, step, error_log, context)
        return False

    _max_concurrent_fixes.acquire()
    cycle_start = time.monotonic()
    all_timings: list[PhaseTiming] = []
    _fix_count_changed = False
    try:
        try:
            from .server import _active_fix_count_lock
            import scripts.quality_webhook.server as _srv
            with _active_fix_count_lock:
                _srv._active_fix_count += 1
                _fix_count_changed = True
        except ImportError:
            pass
        # Gate 0: Auto-fix loop detection
        if _check_auto_fix_chain(job):
            _rollback_last_auto_fix(job)
            return False

        # Gate 1: Time-based dedup
        passed, reason = preflight_dedup(job, error_log)
        if not passed:
            log.warning(f"Preflight dedup rejected {job}/{step}: {reason}")
            log.info(f"Entering deep research phase for {job}/{step}")
            research_fix = _deep_research_fix(job, step, error_log, context)
            if not research_fix:
                _write_escalation(job, step, error_log, "dedup_rejected", context,
                                  extra_reason=reason)
            return research_fix

        ctx_str = ""
        if context:
            ctx_str = (
                f" (commit: {context.get('commit', '?')[:8]}, "
                f"branch: {context.get('branch', '?')}, "
                f"actor: {context.get('actor', '?')})"
            )

        error_class = classify_error(error_log)
        log.info(f"Error classified as: {error_class} for {job}/{step}")

        test_results = parse_test_results(error_log)
        if test_results:
            record_test_outcomes(job, test_results)

        if error_class == "test_failure":
            all_flaky, flaky_names = are_all_failures_flaky(error_log)
            if all_flaky:
                log.info(
                    f"All failing tests are known-flaky for {job}/{step}: "
                    f"{flaky_names}. Skipping fix attempt. "
                    f"Recommendation: quarantine these tests."
                )
                return False

        if error_class == "dependency":
            vuln_pairs = extract_vuln_info(error_log)
            for crate, advisories in vuln_pairs:
                record_vuln_failure(crate, advisories)

        # Create reproduction cache for this fix cycle
        repro_cache = ReproCache(error_hash=_error_hash(job, error_log))

        # Stage 0: Local reproduction (timed)
        log.info(f"=== Reproducing {job}/{step} locally ===")
        t0 = time.monotonic()
        repro = reproduce_locally(job, error_class, cache=repro_cache)
        initial_repro_s = time.monotonic() - t0
        if repro.reproduced:
            log.info(f"Failure REPRODUCED locally: {len(repro.errors)} errors, "
                     f"class refined to '{repro.error_class}'")
            if repro.error_class:
                error_class = repro.error_class
        elif repro.raw_output:
            log.info(f"Failure did NOT reproduce locally for {job}/{step}")
        else:
            log.info(f"No local reproduction commands for {job}/{step}")

        past_attempts_section = get_past_attempts(job, error_log)
        semantic_context = search_relevant_context(job, error_log)
        last_strategy_summary = ""

        attempt = 0
        _PHASE_QUICK_MAX = 3
        _PHASE_DEEP_MAX = 6
        while True:
            attempt += 1
            attempt_start = time.monotonic()
            timing = PhaseTiming()

            if attempt <= _PHASE_QUICK_MAX:
                phase = "quick"
            elif attempt <= _PHASE_DEEP_MAX:
                phase = "deep"
            else:
                phase = "investigate"

            log.info(f"=== Attempt {attempt} [{phase}] for {job}/{step}{ctx_str} (class: {error_class}) ===")

            instruction = _JOB_INSTRUCTIONS.get(job, "Analyze and fix.")
            verify_cmd = _JOB_VERIFY.get(job, "")
            error_note = _build_error_notes(error_class)

            retry_section = ""
            if last_strategy_summary:
                retry_section = (
                    f"\nPREVIOUS ATTEMPT FAILED: {last_strategy_summary}\n"
                    "You MUST try a DIFFERENT approach.\n"
                )

                log.info(f"Re-reproducing {job}/{step} locally after failed attempt")
                t0 = time.monotonic()
                repro = reproduce_locally(job, error_class, cache=repro_cache)
                timing.reproduction_s = time.monotonic() - t0
                if repro.reproduced and repro.error_class:
                    error_class = repro.error_class
            else:
                timing.reproduction_s = initial_repro_s

            run_url = context.get("run_url", "") if context else ""

            repro_section = ""
            if repro and repro.reproduced:
                repro_section = format_errors_for_prompt(repro)
            elif repro:
                repro_section = "\nDid NOT reproduce locally -- likely CI-specific (env/cache/Docker).\n"

            reproduced = repro and repro.reproduced
            source = "local reproduction output" if reproduced else "gh CLI full logs"

            ranked_strategies = get_ranked_strategies(job, error_class)
            strategies_section = ""
            if ranked_strategies:
                lines = [f"  - {s}: {r:.0%} success rate" for s, r in ranked_strategies]
                strategies_section = "\nPREFERRED STRATEGIES (by historical success rate):\n" + "\n".join(lines) + "\n"

            exploring = False
            parsed_strategies = []

            # Phase-dependent prompt construction
            if phase == "deep":
                log.info(f"Phase escalation: running deep diagnosis before attempt {attempt}")
                t0 = time.monotonic()
                diagnosis = _diagnose(job, step, error_log, error_class, context,
                                      repro=repro, deep=True)
                timing.diagnosis_s = time.monotonic() - t0

                prompt = (
                    f"RESEARCH-INFORMED FIX for {job}/{step}{ctx_str}, "
                    f"class={error_class}, attempt {attempt}\n\n"
                    f"Previous quick-fix attempts (1-{_PHASE_QUICK_MAX}) all failed. "
                    f"A deep analysis was performed. Here are the findings:\n"
                    f"```\n{diagnosis}\n```\n\n"
                    f"{repro_section}"
                    f"{_gh_cli_block(run_url)}"
                    f"Error preview:\n```\n{sanitize_for_prompt(error_log, 3000)}\n```\n\n"
                    f"Job context: {instruction}{error_note}\n\n"
                    f"{strategies_section}"
                    f"EXECUTE the fix plan from the analysis above.\n"
                    f"STEP 1 - FIX: Apply the changes described in the analysis.\n"
                )

            elif phase == "investigate":
                log.info(f"Phase escalation: investigation mode for attempt {attempt}")
                t0 = time.monotonic()
                diagnosis = _diagnose(job, step, error_log, error_class, context,
                                      repro=repro, deep=True)
                timing.diagnosis_s = time.monotonic() - t0

                prompt = (
                    f"INVESTIGATION MODE for {job}/{step}{ctx_str}, "
                    f"class={error_class}, attempt {attempt}\n\n"
                    f"ALL previous attempts ({attempt - 1}) have failed. "
                    f"Standard fixes and deep research have not worked.\n\n"
                    f"Previous analysis:\n```\n{diagnosis}\n```\n\n"
                    f"{repro_section}"
                    f"{_gh_cli_block(run_url)}"
                    f"Error preview:\n```\n{sanitize_for_prompt(error_log, 3000)}\n```\n\n"
                    f"Job context: {instruction}{error_note}\n\n"
                    f"MANDATORY INVESTIGATION STEPS before writing any fix:\n"
                    f"1. Fetch FULL CI logs: gh run view <id> --log-failed\n"
                    f"2. Read EVERY file mentioned in errors + their imports and callers\n"
                    f"3. Check git log -10 for recent changes to affected files\n"
                    f"4. Diff Cargo.lock or gradle lockfiles for dependency changes\n"
                    f"5. Search the web for the exact error message if it involves a library\n"
                    f"6. Read project steering files in .kiro/steering/ for conventions\n"
                    f"7. Check if the error exists in files NOT mentioned in the error output\n"
                    f"8. Write a 3-sentence plan BEFORE editing any file\n\n"
                    f"THEN apply the fix.\n"
                    f"STEP 1 - INVESTIGATE: Complete all 8 steps above.\n"
                    f"STEP 2 - FIX: Apply the minimal fix based on your investigation.\n"
                )

            else:
                prompt = (
                    f"CI FAILED: job='{job}', step='{step}'{ctx_str}, "
                    f"class={error_class}, attempt {attempt}\n\n"
                    f"{repro_section}"
                    f"{_gh_cli_block(run_url)}"
                    f"Error preview:\n```\n{sanitize_for_prompt(error_log, 3000)}\n```\n\n"
                    f"Job context: {instruction}{error_note}\n\n"
                    f"{strategies_section}"
                    f"STEP 1 - DIAGNOSE: Identify root cause from {source}. "
                    f"List specific files/lines that need changes.\n"
                    f"STEP 2 - FIX: Apply the minimal fix to resolve the issue.\n"
                )

                if phase == "quick" and attempt == 1:
                    diag_for_strats = _diagnose(
                        job, step, error_log, error_class, context,
                        repro=repro, deep=False,
                    )
                    parsed_strategies = _parse_strategies(diag_for_strats)
                    if len(parsed_strategies) >= 2 and random.random() < 0.10:
                        exploring = True
                        alt_strategy = parsed_strategies[1][0]
                        prompt += f"\nEXPLORATION: Prefer this alternative approach: {alt_strategy}\n"
                        log.info(f"Exploration mode: trying alternative strategy '{alt_strategy}' for {job}/{step}")

            verify_line = f"STEP 3 - VERIFY: Run: {verify_cmd}\n" if verify_cmd else ""
            prompt += verify_line
            prompt += "STEP 4 - STAGE: git add -A\n"

            extras = ""
            if attempt == 1:
                if past_attempts_section:
                    extras += f"\n{past_attempts_section}"
                if semantic_context:
                    extras += f"\n{semantic_context}"
            elif phase != "quick":
                if past_attempts_section:
                    extras += f"\n{past_attempts_section}"
                if semantic_context:
                    extras += f"\n{semantic_context}"

            prompt += (
                f"{extras}"
                f"{retry_section}"
                f"{SCOPE_CONSTRAINTS}\n"
                "Do NOT commit."
            )

            t0 = time.monotonic()
            result = run_kiro(prompt, f"CI fix ({job}/{step}) attempt {attempt} [{phase}]")
            timing.fix_attempt_s = time.monotonic() - t0

            success = result[0] if isinstance(result, tuple) else result
            kiro_output = result[1] if isinstance(result, tuple) else ""

            outcome = extract_outcome_from_output(kiro_output) if kiro_output else {}
            strategy_note = outcome.get("strategy", "")

            if not success:
                timing.total_s = time.monotonic() - attempt_start
                all_timings.append(timing)
                last_strategy_summary = strategy_note or "unknown"
                record_strategy_outcome(job, error_class, strategy_note or error_class, False)
                record_fix_attempt(job, error_log, attempt, False,
                                   f"class={error_class}, job={job}, step={step}. {strategy_note}",
                                   timing=dataclasses.asdict(timing))
                store_fix_attempt(job, step, error_class, error_log, attempt, False)
                log.warning(f"Attempt {attempt} kiro-cli returned failure for {job}/{step}")
                continue

            # Gate 2: Revision check (reject forbidden patterns like #[ignore])
            rev_passed, rev_reason = revision_check_diff()
            if not rev_passed:
                timing.total_s = time.monotonic() - attempt_start
                all_timings.append(timing)
                log.warning(f"Revision gate rejected attempt {attempt} ({job}/{step}): {rev_reason}")
                wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
                last_strategy_summary = f"REVISION REJECTED ({rev_reason}). {strategy_note}"
                record_strategy_outcome(job, error_class, strategy_note or error_class, False)
                record_fix_attempt(job, error_log, attempt, False,
                                   f"revision_rejected: {rev_reason}. {strategy_note}",
                                   timing=dataclasses.asdict(timing))
                store_fix_attempt(job, step, error_class, error_log, attempt, False)
                continue

            # Gate 3: Independent verification
            t0 = time.monotonic()
            verify_passed, verify_reason = verification_run(job)
            timing.verification_s = time.monotonic() - t0

            if not verify_passed:
                timing.total_s = time.monotonic() - attempt_start
                all_timings.append(timing)
                log.warning(f"Verification gate failed for attempt {attempt} ({job}/{step}): {verify_reason}")
                wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
                last_strategy_summary = f"VERIFY FAILED ({verify_reason[:200]}). {strategy_note}"
                record_strategy_outcome(job, error_class, strategy_note or error_class, False)
                record_fix_attempt(job, error_log, attempt, False,
                                   f"verify_failed: {verify_reason[:200]}. {strategy_note}",
                                   timing=dataclasses.asdict(timing))
                store_fix_attempt(job, step, error_class, error_log, attempt, False)
                continue

            # All gates passed -- commit and push
            timing.total_s = time.monotonic() - attempt_start
            all_timings.append(timing)
            log.info(f"All gates passed for attempt {attempt} ({job}/{step}). Committing.")
            pre_fix_sha_result = wsl_bash("git rev-parse HEAD", timeout=10)
            pre_fix_sha = pre_fix_sha_result.stdout.strip()[:12] if pre_fix_sha_result.returncode == 0 else "unknown"
            with _git_lock:
                commit_result = wsl_bash(
                    "git add -A && "
                    "git commit -m 'fix: resolve CI failures (auto-fix)' && "
                    "git push origin main || "
                    "(git pull --rebase origin main && git push origin main)",
                    timeout=120,
                )

            commit_ok = commit_result.returncode == 0
            record_strategy_outcome(job, error_class, strategy_note or error_class, commit_ok)
            if exploring and parsed_strategies:
                unchosen_name = parsed_strategies[0][0]
                record_strategy_outcome(job, error_class, unchosen_name, None)
            record_fix_attempt(job, error_log, attempt, commit_ok,
                               f"class={error_class}, job={job}, step={step}. pre_fix_sha={pre_fix_sha}. {strategy_note}",
                               timing=dataclasses.asdict(timing))
            store_fix_attempt(job, step, error_class, error_log, attempt, commit_ok)

            if commit_ok:
                store_fix_outcome(
                    job, error_class,
                    files_changed=outcome.get("files_changed", ""),
                    strategy=strategy_note or f"Fixed via attempt {attempt}",
                )
                log.info(f"Attempt {attempt} completed for {job}/{step}")
                _log_timing_summary(job, step, cycle_start, all_timings)
                total_cycle_s = time.monotonic() - cycle_start
                record_duration(job, total_cycle_s, True)
                at_risk = check_and_alert_trends()
                for rj in at_risk:
                    log.warning(f"Trend alert: job '{rj}' approaching timeout threshold")
                return True

            log.warning(f"Commit/push failed for attempt {attempt} ({job}/{step})")
            last_strategy_summary = f"commit/push failed. {strategy_note}"

    finally:
        wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
        if _fix_count_changed:
            try:
                from .server import _active_fix_count_lock
                import scripts.quality_webhook.server as _srv
                with _active_fix_count_lock:
                    _srv._active_fix_count -= 1
            except ImportError:
                pass
        _max_concurrent_fixes.release()
        job_lock.release()

        pending = _pop_pending_fix(job)
        if pending is not None:
            p_job, p_step, p_error_log, p_context = pending
            log.info(f"Draining queued fix for job '{p_job}/{p_step}'")
            threading.Thread(
                target=_fix_with_retry_legacy,
                args=(p_job, p_step, p_error_log, p_context),
                daemon=True,
            ).start()


def _deep_research_fix(job, step, error_log, context=None):
    """Last-resort research phase when quick fixes keep failing.

    Instead of giving up, this:
    1. Runs _diagnose(deep=True) to deeply analyze the root cause
    2. Attempts one final fix using the research-informed diagnosis
    3. Gates the result through revision + verification as usual
    """
    ctx_str = ""
    if context:
        ctx_str = (
            f" (commit: {context.get('commit', '?')[:8]}, "
            f"branch: {context.get('branch', '?')})"
        )

    error_class = classify_error(error_log)

    repro_cache = ReproCache(error_hash=_error_hash(job, error_log))
    repro = reproduce_locally(job, error_class, cache=repro_cache)
    if repro.reproduced and repro.error_class:
        error_class = repro.error_class

    # Phase 1: Deep diagnosis via _diagnose(deep=True)
    log.info(f"=== Deep research: diagnosing {job}/{step}{ctx_str} ===")
    diagnosis = _diagnose(job, step, error_log, error_class, context,
                          repro=repro, deep=True)

    if not diagnosis or diagnosis == "diagnosis unavailable":
        log.warning(f"Deep research produced no output for {job}/{step}")
        return False

    log.info(f"Deep research complete for {job}/{step}, proceeding to planned fix")

    # Phase 2: Execute the researched fix
    log.info(f"=== Deep research fix attempt for {job}/{step}{ctx_str} ===")
    verify_cmd = _JOB_VERIFY.get(job, "")

    repro_section = ""
    if repro and repro.reproduced:
        repro_section = format_errors_for_prompt(repro)

    fix_prompt = (
        f"RESEARCH-INFORMED FIX for {job}/{step}{ctx_str}\n\n"
        f"You previously analyzed this failure in depth. "
        f"Here is your analysis:\n"
        f"```\n{diagnosis}\n```\n\n"
        f"{repro_section}"
        f"EXECUTE the fix plan from your analysis above.\n"
        f"STEP 1: Apply the changes described in your analysis.\n"
    )
    if verify_cmd:
        fix_prompt += f"STEP 2: Verify with: {verify_cmd}\n"
    fix_prompt += (
        f"STEP 3: git add -A\n"
        f"{SCOPE_CONSTRAINTS}\n"
        f"Do NOT commit."
    )

    result = run_kiro(fix_prompt, f"Deep research fix ({job}/{step})")
    success = result[0] if isinstance(result, tuple) else result
    kiro_output = result[1] if isinstance(result, tuple) else ""

    if not success:
        log.warning(f"Deep research fix attempt failed for {job}/{step}")
        record_fix_attempt(job, error_log, 0, False,
                           f"deep_research_fix failed. class={error_class}")
        store_fix_attempt(job, step, error_class, error_log, 0, False)
        return False

    # Gate: revision check
    rev_passed, rev_reason = revision_check_diff()
    if not rev_passed:
        log.warning(f"Deep research fix rejected by revision gate ({job}/{step}): {rev_reason}")
        wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
        record_fix_attempt(job, error_log, 0, False,
                           f"deep_research revision_rejected: {rev_reason}")
        store_fix_attempt(job, step, error_class, error_log, 0, False)
        return False

    # Gate: verification
    verify_passed, verify_reason = verification_run(job)
    if not verify_passed:
        log.warning(f"Deep research fix failed verification ({job}/{step}): {verify_reason}")
        wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
        record_fix_attempt(job, error_log, 0, False,
                           f"deep_research verify_failed: {verify_reason[:200]}")
        store_fix_attempt(job, step, error_class, error_log, 0, False)
        return False

    # All gates passed — commit
    log.info(f"Deep research fix passed all gates for {job}/{step}. Committing.")
    pre_fix_sha_result = wsl_bash("git rev-parse HEAD", timeout=10)
    pre_fix_sha = pre_fix_sha_result.stdout.strip()[:12] if pre_fix_sha_result.returncode == 0 else "unknown"
    with _git_lock:
        commit_result = wsl_bash(
            "git add -A && "
            "git commit -m 'fix: resolve CI failures (deep-research auto-fix)' && "
            "git push origin main || "
            "(git pull --rebase origin main && git push origin main)",
            timeout=120,
        )

    commit_ok = commit_result.returncode == 0
    outcome = extract_outcome_from_output(kiro_output) if kiro_output else {}
    strategy_note = outcome.get("strategy", "deep_research")

    record_strategy_outcome(job, error_class, strategy_note, commit_ok)
    record_fix_attempt(job, error_log, 0, commit_ok,
                       f"deep_research class={error_class}. pre_fix_sha={pre_fix_sha}. {strategy_note}")
    store_fix_attempt(job, step, error_class, error_log, 0, commit_ok)

    if commit_ok:
        store_fix_outcome(
            job, error_class,
            files_changed=outcome.get("files_changed", ""),
            strategy=f"deep_research: {strategy_note}",
        )
        log.info(f"Deep research fix succeeded for {job}/{step}")
        return True

    log.warning(f"Deep research fix commit/push failed for {job}/{step}")
    return False


def _write_escalation(job, step, error_log, error_class, context, extra_reason="", repro=None):
    escalation_dir = WIN_PROJECT_DIR / "escalations"
    try:
        escalation_dir.mkdir(exist_ok=True)
        ts = datetime.now().strftime("%Y%m%d_%H%M%S")

        # Include actual strategies attempted from history
        past = get_past_attempts(job, error_log)
        semantic = search_relevant_context(job, error_log)

        repro_info = {}
        if repro:
            repro_info = {
                "reproduced_locally": repro.reproduced,
                "local_error_count": len(repro.errors),
                "local_error_class": repro.error_class,
                "local_errors": [str(e) for e in repro.errors[:10]],
                "files_affected": sorted({e.file for e in repro.errors if e.file}),
            }

        report = {
            "timestamp": datetime.now().isoformat(),
            "job": job,
            "step": step,
            "error_class": error_class,
            "context": context or {},
            "max_retries_exhausted": "unlimited",
            "error_log_preview": sanitize_for_prompt(error_log, 3000),
            "local_reproduction": repro_info,
            "recommendation": _escalation_recommendation(error_class, job),
            "strategies_attempted": past or "no history",
            "similar_past_fixes": semantic or "no semantic matches",
            "extra_reason": extra_reason,
        }
        path = escalation_dir / f"{ts}_{job}.json"
        path.write_text(json.dumps(report, indent=2, ensure_ascii=False),
                        encoding="utf-8", newline="\n")
        log.info(f"Escalation report written to {path}")
    except OSError as e:
        log.warning(f"Failed to write escalation report: {e}")


def _escalation_recommendation(error_class, job):
    recs = {
        "runner_environment": (
            "This is a runner/infrastructure issue. Check: disk space, Docker daemon, "
            "network connectivity, installed tools on the self-hosted runner."
        ),
        "dependency": (
            f"Dependency vulnerability in {job}. Manual review needed: "
            "check if a patched version exists, evaluate if suppression is appropriate, "
            "or find an alternative library."
        ),
        "test_failure": (
            f"Persistent test failure in {job} that auto-fix could not resolve. "
            "The test may require understanding of business logic changes. "
            "Check if the test expectations need updating or if there's a real regression."
        ),
        "code_quality": (
            f"Code quality issue in {job} that auto-fix could not resolve. "
            "May require architectural changes or understanding of the broader context."
        ),
        "dedup_rejected": (
            f"Fix for {job} was rejected by dedup gate. The same error pattern "
            "has failed multiple times recently. This likely requires human investigation."
        ),
        "build_failure": (
            f"Persistent build failure in {job} that auto-fix could not resolve. "
            "Check compiler output for type errors, missing dependencies, or "
            "WASM target incompatibilities."
        ),
    }
    return recs.get(error_class, f"Auto-fix could not resolve {job}. Manual investigation required.")


def _write_pipeline_escalation(pipeline) -> None:
    from .decisions import compute_feasibility
    escalation_dir = WIN_PROJECT_DIR / "escalations"
    try:
        escalation_dir.mkdir(exist_ok=True)
        ts = datetime.now().strftime("%Y%m%d_%H%M%S")
        lineage_id = pipeline.lineage_id

        round_history = []
        for r in range(1, pipeline.round_number + 1):
            round_entry = {
                "round": r,
                "files_changed": pipeline.files_changed.get(r, []),
            }
            snapshot = pipeline.quality_snapshots.get(r)
            if snapshot:
                round_entry["quality_snapshot"] = {
                    "clippy_warning_count": snapshot.clippy_warning_count,
                    "test_count": snapshot.test_count,
                    "sonar_issue_count": snapshot.sonar_issue_count,
                    "vuln_count": snapshot.vuln_count,
                    "timestamp": snapshot.timestamp,
                }
            round_history.append(round_entry)

        feasibility_scores = {}
        for job, state in pipeline.job_states.items():
            if state.value == "failed":
                result = compute_feasibility(job, "unknown")
                feasibility_scores[job] = {
                    "score": result.score,
                    "sample_count": result.sample_count,
                    "recommendation": result.recommendation,
                }

        regression_events = []
        for r in range(2, pipeline.round_number + 1):
            prev_files = set(pipeline.files_changed.get(r - 1, []))
            curr_files = set(pipeline.files_changed.get(r, []))
            overlap = prev_files & curr_files
            if overlap:
                regression_events.append({
                    "round": r,
                    "overlapping_files": sorted(overlap),
                })

        report = {
            "timestamp": datetime.now().isoformat(),
            "lineage_id": lineage_id,
            "current_commit": pipeline.current_commit,
            "total_rounds": pipeline.round_number,
            "budget_total": pipeline.budget_total,
            "budget_remaining": pipeline.budget_remaining,
            "deployed": pipeline.deployed,
            "sonarqube_passed": pipeline.sonarqube_passed,
            "job_states": {k: v.value for k, v in pipeline.job_states.items()},
            "pending_strategies": pipeline.pending_strategies,
            "round_history": round_history,
            "regression_events": regression_events,
            "feasibility_scores": feasibility_scores,
            "duration": {
                "created_at": pipeline.created_at.isoformat(),
                "last_updated": pipeline.last_updated.isoformat(),
            },
        }
        path = escalation_dir / f"pipeline_{ts}_{lineage_id}.json"
        path.write_text(
            json.dumps(report, indent=2, ensure_ascii=False),
            encoding="utf-8", newline="\n",
        )
        log.error(
            f"Pipeline escalation: lineage {lineage_id} exhausted budget "
            f"after {pipeline.round_number} rounds. "
            f"Report written to {path.name}"
        )
    except OSError as e:
        log.warning(f"Failed to write pipeline escalation report: {e}")


def _parse_sonar_report_by_file(sonar_report):
    """Group sonar issues by file for wave-based parallel execution.

    Expected format per line: severity | file:line | rule | message
    """
    file_groups = defaultdict(list)
    for line in sonar_report.strip().splitlines():
        line = line.strip()
        if not line or line.startswith("#") or line.startswith("---"):
            continue
        parts = [p.strip() for p in line.split("|")]
        if len(parts) >= 3:
            file_ref = parts[1] if len(parts) > 1 else ""
            file_name = file_ref.split(":")[0].strip() if ":" in file_ref else file_ref.strip()
            if file_name:
                file_groups[file_name].append(line)
            else:
                file_groups["_ungrouped"].append(line)
        else:
            file_groups["_ungrouped"].append(line)

    return dict(file_groups)


def _fix_sonar_file_group(file_group_report, group_label, attempt=1):
    """Fix sonar issues for a single file group. Returns (success, label)."""
    is_kotlin = any(f.endswith((".kt", ".kts")) for f in group_label.split(","))
    verify_cmd = (
        "(cd android && ./gradlew spotlessApply detekt testDebugUnitTest)"
        if is_kotlin else
        "cargo fmt && cargo clippy --locked -- -D warnings && cargo test --locked"
    )

    if attempt <= 3:
        phase = "quick"
    elif attempt <= 6:
        phase = "deep"
    else:
        phase = "investigate"

    if phase == "investigate":
        prompt = (
            f"INVESTIGATION MODE: SonarQube issues in {group_label} after {attempt - 1} failed attempts.\n\n"
            f"Issues:\n```\n{sanitize_for_prompt(file_group_report, 3000)}\n```\n\n"
            "MANDATORY STEPS:\n"
            "1. Read the ENTIRE file, not just the flagged lines\n"
            "2. Read imports and callers of the affected functions\n"
            "3. Check git log for recent changes to this file\n"
            "4. Search the web for the SonarQube rule ID if unfamiliar\n"
            "5. Write a plan BEFORE editing\n\n"
            "THEN apply the fix.\n"
            f"Verify: {verify_cmd}\n"
            "Stage: git add -A. Do NOT commit.\n"
            f"{SCOPE_CONSTRAINTS}"
        )
    elif phase == "deep":
        prompt = (
            f"DEEP RESEARCH: SonarQube issues in {group_label}. Quick fixes failed.\n\n"
            f"Issues:\n```\n{sanitize_for_prompt(file_group_report, 3000)}\n```\n\n"
            "Read the full file and understand the context before fixing.\n"
            "Security hotspots: evaluate if actually vulnerable before changing.\n"
            f"Verify: {verify_cmd}\n"
            "Stage: git add -A. Do NOT commit.\n"
            f"{SCOPE_CONSTRAINTS}"
        )
    else:
        prompt = (
            "Fix ONLY the SonarQube issues listed below.\n\n"
            f"Issues:\n```\n{sanitize_for_prompt(file_group_report, 3000)}\n```\n\n"
            "For each: open file at line, read context, apply minimal fix.\n"
            "Security hotspots: evaluate if actually vulnerable before changing.\n"
            f"Verify: {verify_cmd}\n"
            "Stage: git add -A. Do NOT commit.\n"
            f"{SCOPE_CONSTRAINTS}"
        )

    result = run_kiro(prompt, f"Sonar fix ({group_label}) [{phase}]")
    success = result[0] if isinstance(result, tuple) else result
    return success, group_label


def fix_sonar_issues(sonar_report, run_url):
    tracker = get_tracker()
    commit = ""
    pipeline = None
    result = wsl_bash("git rev-parse HEAD", timeout=10)
    if result.returncode == 0:
        commit = result.stdout.strip()
    if commit:
        pipeline = tracker.get_or_create(commit)
        if pipeline.deployed:
            log.info(f"Pipeline {pipeline.lineage_id} already deployed, skipping SonarQube fix")
            return False
        if tracker.is_budget_exhausted(commit):
            log.error(
                f"Pipeline budget exhausted for lineage {pipeline.lineage_id} "
                f"(round {pipeline.round_number}/{pipeline.budget_total}). "
                f"Skipping SonarQube fix."
            )
            return False

    if not _sonar_fix_lock.acquire(blocking=False):
        log.warning("Another SonarQube fix is already running -- skipping")
        return False

    try:
        file_groups = _parse_sonar_report_by_file(sonar_report)

        if not file_groups:
            log.warning("No parseable issues in sonar report, falling back to single prompt")
            file_groups = {"all": sonar_report.splitlines()}

        group_items = list(file_groups.items())
        waves = []
        for i in range(0, len(group_items), SONAR_PARALLEL_GROUPS):
            waves.append(group_items[i:i + SONAR_PARALLEL_GROUPS])

        attempt = pipeline.round_number if pipeline else 1

        if attempt <= 3:
            phase = "quick"
        elif attempt <= 6:
            phase = "deep"
        else:
            phase = "investigate"

        log.info(f"Sonar fix: {len(file_groups)} file groups in {len(waves)} waves (round {attempt}) [{phase}]")

        all_succeeded = True
        failed_groups = []

        for wave_idx, wave in enumerate(waves):
            log.info(f"  Wave {wave_idx + 1}/{len(waves)}: {len(wave)} groups")

            if len(wave) == 1:
                file_name, issues = wave[0]
                group_report = "\n".join(issues)
                success, label = _fix_sonar_file_group(group_report, file_name, attempt)
                if not success:
                    all_succeeded = False
                    failed_groups.append(label)
            else:
                with ThreadPoolExecutor(max_workers=SONAR_PARALLEL_GROUPS) as executor:
                    futures = {}
                    for file_name, issues in wave:
                        group_report = "\n".join(issues)
                        future = executor.submit(
                            _fix_sonar_file_group, group_report, file_name, attempt,
                        )
                        futures[future] = file_name

                    for future in as_completed(futures):
                        try:
                            success, label = future.result(timeout=600)
                            if not success:
                                all_succeeded = False
                                failed_groups.append(label)
                        except Exception as e:
                            all_succeeded = False
                            failed_groups.append(futures[future])
                            log.warning(f"Sonar fix group failed: {e}")

        commit_result = wsl_bash(
            "git add -A && "
            "git diff --cached --quiet && echo 'nothing to commit' || "
            "(git commit -m 'fix: resolve SonarQube issues (auto-fix)' && "
            "git push origin main || "
            "(git pull --rebase origin main && git push origin main))",
            timeout=120,
        )

        if commit_result.returncode == 0:
            if failed_groups:
                log.warning(f"Sonar fix partial success (round {attempt}). Failed groups: {failed_groups}")
            else:
                log.info(f"SonarQube fix (round {attempt}) [{phase}] completed (all groups)")
            return True

        log.warning(f"SonarQube fix (round {attempt}) [{phase}] commit/push failed")
        return False
    finally:
        _sonar_fix_lock.release()


def _parse_durations(pipeline_report):
    """Extract job durations from the pipeline report table."""
    durations = {}
    for line in pipeline_report.splitlines():
        match = re.match(r"\s*(\S+)\s*\|\s*\S+\s*\|\s*(\d+)s?\s*\|", line)
        if match:
            job, dur = match.group(1), match.group(2)
            try:
                durations[job] = int(dur)
            except ValueError:
                pass
    return durations


def _has_failures(focus):
    """Check if the focus indicates actual job failures vs general optimization."""
    return focus in ("security", "bugs", "pipeline")


_PIPELINE_CACHE_NOTE = (
    "NOTE: Rust compilation cache uses persistent runner volumes, NOT actions/cache or Swatinem/rust-cache. "
    "Cache column showing 'volume' is correct. Do NOT add rust-cache or actions/cache for Rust targets.\n"
)

_MAX_IMPROVE_ATTEMPTS = 8


def _build_failure_prompt(focus, pipeline_report, run_url, sonar_section, focus_specific, retry_section, phase):
    """Build a prompt focused on fixing pipeline failures."""
    if phase == "investigate":
        return (
            f"INVESTIGATION MODE: CI/CD pipeline failure fix after failed attempts.\n"
            f"Focus: {focus}\n"
            f"{_gh_cli_block(run_url)}\n"
            f"--- PIPELINE REPORT ---\n{sanitize_for_prompt(pipeline_report, 5000)}\n--- END ---\n"
            f"{sonar_section}\n"
            "MANDATORY INVESTIGATION STEPS:\n"
            "1. Fetch full logs via gh CLI\n"
            "2. Read ALL workflow YAML files in .github/workflows/\n"
            "3. Check git log -10 for recent CI changes\n"
            "4. Search the web for any error messages from CI tools\n"
            "5. Read .kiro/steering/ for project conventions\n"
            "6. Write a plan BEFORE editing any file\n\n"
            "CLASSIFY each failure:\n"
            "  A) RUNNER ENV → document, don't change code\n"
            "  B) PIPELINE CONFIG → fix in .github/workflows/\n"
            "  C) DEPENDENCY VULN → fix in Cargo.toml/deny.toml/libs.versions.toml\n"
            "  D) APP CODE → skip (handled by /ci-failure)\n\n"
            f"{focus_specific}"
            f"{_PIPELINE_CACHE_NOTE}"
            f"{retry_section}"
            "Stage: git add -A. Do NOT commit.\n"
            f"Run URL: {run_url}"
        )
    elif phase == "deep":
        return (
            f"DEEP RESEARCH: CI/CD pipeline failure fix. Quick fixes failed.\n"
            f"Focus: {focus}\n"
            f"{_gh_cli_block(run_url)}\n"
            f"--- PIPELINE REPORT ---\n{sanitize_for_prompt(pipeline_report, 5000)}\n--- END ---\n"
            f"{sonar_section}\n"
            "Fetch full logs via gh CLI first. Read workflow YAML fully before editing.\n\n"
            "CLASSIFY each failure:\n"
            "  A) RUNNER ENV → document, don't change code\n"
            "  B) PIPELINE CONFIG → fix in .github/workflows/\n"
            "  C) DEPENDENCY VULN → fix in Cargo.toml/deny.toml/libs.versions.toml\n"
            "  D) APP CODE → skip (handled by /ci-failure)\n\n"
            f"{focus_specific}"
            f"{_PIPELINE_CACHE_NOTE}"
            f"{retry_section}"
            "Verify YAML validity after editing.\n"
            "Stage: git add -A. Do NOT commit.\n"
            f"Run URL: {run_url}"
        )
    else:
        return (
            f"CI/CD pipeline engineer. Fix pipeline failures.\n"
            f"Focus: {focus}\n"
            f"{_gh_cli_block(run_url)}\n"
            f"--- PIPELINE REPORT ---\n{sanitize_for_prompt(pipeline_report, 5000)}\n--- END ---\n"
            f"{sonar_section}\n"
            "Fetch full logs via gh CLI first.\n\n"
            "CLASSIFY each failure:\n"
            "  A) RUNNER ENV → document, don't change code\n"
            "  B) PIPELINE CONFIG → fix in .github/workflows/\n"
            "  C) DEPENDENCY VULN → fix in Cargo.toml/deny.toml/libs.versions.toml\n"
            "  D) APP CODE → skip (handled by /ci-failure)\n\n"
            f"{focus_specific}"
            f"{_PIPELINE_CACHE_NOTE}"
            "Read workflow YAML fully before editing. Verify YAML validity after.\n"
            f"{retry_section}"
            "Stage: git add -A. Do NOT commit.\n"
            f"Run URL: {run_url}"
        )


def _build_optimization_prompt(pipeline_report, run_url, sonar_section, trend_section, optimization_history_section=""):
    return (
        "CI/CD pipeline optimization. Rust/WASM + Android project on self-hosted runners with persistent volumes.\n"
        f"{_gh_cli_block(run_url)}\n"
        f"--- PIPELINE REPORT ---\n{sanitize_for_prompt(pipeline_report, 5000)}\n--- END ---\n"
        f"{trend_section}"
        f"{optimization_history_section}"
        f"{sonar_section}\n"
        "RULES:\n"
        "- If the pipeline is healthy and durations are stable, make NO changes. "
        "Report 'Pipeline healthy, no optimization needed' and stage nothing.\n"
        "- Only optimize if you find a concrete, measurable improvement.\n"
        "- Target: reduce wall-clock time or eliminate redundant work.\n"
        "- Do NOT add actions/cache or Swatinem/rust-cache. Compilation cache uses persistent runner volumes.\n"
        "- Do NOT restructure jobs unless you can justify the time savings.\n\n"
        "LOOK FOR:\n"
        "- Jobs that could run in parallel but have unnecessary `needs:` dependencies\n"
        "- Duplicate tool installations across jobs\n"
        "- Steps that could be skipped with path filters\n"
        "- Jobs approaching their timeout (see trend data)\n\n"
        "Read workflow YAML fully before editing. Verify YAML validity after.\n"
        "Stage: git add -A. Do NOT commit.\n"
        f"Run URL: {run_url}"
    )


def _build_trend_section():
    """Build a trend summary from historical duration data."""
    trends = get_all_trends()
    if not trends:
        return ""

    lines = ["--- DURATION TRENDS ---"]
    for job, trend in sorted(trends.items()):
        if trend.sample_count < 3:
            continue
        risk = " ⚠️ TIMEOUT RISK" if trend.timeout_risk else ""
        lines.append(
            f"  {job}: avg={trend.moving_avg_s:.0f}s "
            f"(samples={trend.sample_count}){risk}"
        )
    lines.append("--- END TRENDS ---\n")
    return "\n".join(lines) if len(lines) > 2 else ""


def improve_pipeline(focus, pipeline_report, run_url, sonar_report=""):
    if not _improve_lock.acquire(blocking=False):
        log.warning("Another pipeline improvement is already running -- skipping")
        return False

    try:
        is_failure = _has_failures(focus)

        if not is_failure:
            durations = _parse_durations(pipeline_report)
            total_s = sum(durations.values())
            if total_s > 0 and total_s < 1200:
                log.info(
                    f"Pipeline healthy (total {total_s}s < 1200s threshold, "
                    f"focus={focus}) -- skipping optimization"
                )
                return True
            if not durations:
                log.info(
                    "No duration data in report and no failures -- "
                    "skipping optimization"
                )
                return True

        sonar_section = ""
        if sonar_report and "unavailable" not in sonar_report:
            sonar_section = (
                f"\n\n--- SONARQUBE ISSUES ---\n"
                f"{sanitize_for_prompt(sonar_report, 5000)}\n"
                f"--- END SONARQUBE ISSUES ---\n"
            )

        focus_guidance = {
            "security": (
                "SECURITY: Fix in Cargo.toml/deny.toml/libs.versions.toml, NOT pipeline YAML. "
                "Only modify YAML if the security tool itself is misconfigured.\n"
            ),
            "bugs": (
                "BUGS: /ci-failure webhook handles code fixes. "
                "Only fix pipeline if failure is from misconfiguration (env, tools, caching).\n"
            ),
            "pipeline": (
                "PIPELINE: Fix build/deploy config (Dockerfile, docker-compose, deploy scripts).\n"
            ),
        }

        _PHASE_QUICK_MAX = 3
        _PHASE_DEEP_MAX = 6
        last_strategy = ""

        for attempt in range(1, _MAX_IMPROVE_ATTEMPTS + 1):
            if attempt <= _PHASE_QUICK_MAX:
                phase = "quick"
            elif attempt <= _PHASE_DEEP_MAX:
                phase = "deep"
            else:
                phase = "investigate"

            log.info(
                f"=== Pipeline {'fix' if is_failure else 'optimization'} "
                f"attempt {attempt}/{_MAX_IMPROVE_ATTEMPTS} [{phase}] "
                f"(focus: {focus}) ==="
            )

            retry_section = ""
            if last_strategy:
                retry_section = (
                    f"\nPREVIOUS ATTEMPT FAILED: {last_strategy}\n"
                    "You MUST try a DIFFERENT approach.\n"
                )

            if is_failure:
                focus_specific = ""
                for f in focus.split(","):
                    f = f.strip()
                    if f in focus_guidance:
                        focus_specific += focus_guidance[f]

                prompt = _build_failure_prompt(
                    focus, pipeline_report, run_url,
                    sonar_section, focus_specific, retry_section, phase,
                )
                label = f"Pipeline fix ({focus}) attempt {attempt} [{phase}]"
            else:
                trend_section = _build_trend_section()
                opt_history = get_optimization_history()
                optimization_history_section = ""
                if opt_history:
                    lines = ["--- PAST OPTIMIZATIONS ---"]
                    for rec in opt_history:
                        ts = rec.get("timestamp", "?")[:19]
                        summary = rec.get("changes_summary", "?")
                        focus_val = rec.get("focus", "?")
                        pre_dur = rec.get("pre_durations", {})
                        dur_str = ", ".join(f"{k}={v:.0f}s" for k, v in pre_dur.items()) if pre_dur else "n/a"
                        lines.append(f"  [{ts}] focus={focus_val} durations=[{dur_str}] changes: {summary}")
                    lines.append("--- END PAST OPTIMIZATIONS ---\n")
                    optimization_history_section = "\n".join(lines) + "\n"
                prompt = _build_optimization_prompt(
                    pipeline_report, run_url, sonar_section, trend_section,
                    optimization_history_section,
                )
                label = f"Pipeline optimize attempt {attempt} [{phase}]"

            result = run_kiro(prompt, label)
            success = result[0] if isinstance(result, tuple) else result

            if not success:
                log.warning(f"Pipeline improvement attempt {attempt} [{phase}] failed")
                last_strategy = f"attempt {attempt} [{phase}] failed"
                if not is_failure:
                    log.info("Optimization attempt failed -- not retrying for green builds")
                    return False
                continue

            commit_result = wsl_bash(
                "git add -A && "
                "git diff --cached --quiet && echo 'nothing to commit' || "
                f"(git commit -m 'ci: {'fix' if is_failure else 'optimize'} pipeline ({focus})' && "
                "git push origin main || "
                "(git pull --rebase origin main && git push origin main))",
                timeout=120,
            )

            if commit_result.returncode == 0:
                log.info(f"Pipeline improvement attempt {attempt} [{phase}] completed")
                if not is_failure:
                    durations = _parse_durations(pipeline_report)
                    record_optimization(
                        f"attempt {attempt} [{phase}] focus={focus}",
                        durations,
                        focus,
                    )
                return True

            log.warning(f"Pipeline improvement attempt {attempt} [{phase}] commit/push failed")
            last_strategy = f"commit/push failed on attempt {attempt}"

        log.error(
            f"Pipeline improvement exhausted all {_MAX_IMPROVE_ATTEMPTS} attempts "
            f"(focus: {focus})"
        )
        return False
    finally:
        _improve_lock.release()
