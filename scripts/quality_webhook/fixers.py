

import dataclasses
import json
import re
import threading
import time
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from datetime import datetime

from .config import (
    WIN_PROJECT_DIR, MAX_RETRIES, MAX_CONCURRENT_FIXES, SCOPE_CONSTRAINTS,
    SONAR_PARALLEL_GROUPS, log,
)
from .runner import wsl_bash, run_kiro
from .classifier import classify_error
from .history import record_fix_attempt, get_past_attempts, _error_hash
from .memory import (
    store_fix_attempt, store_fix_outcome,
    search_relevant_context, extract_outcome_from_output,
)
from .gates import (
    preflight_dedup, revision_scope_check, verification_run,
)
from .reproducer import reproduce_locally, format_errors_for_prompt, ReproCache


@dataclass
class PhaseTiming:
    reproduction_s: float = 0.0
    diagnosis_s: float = 0.0
    fix_attempt_s: float = 0.0
    scope_gate_s: float = 0.0
    verification_s: float = 0.0
    total_s: float = 0.0


_fix_lock = threading.Lock()  # backward compat: kept for server.py health endpoint
_sonar_fix_lock = threading.Lock()
_improve_lock = threading.Lock()

# Per-job lock registry: allows parallel fixes for different jobs
_job_locks: dict[str, threading.Lock] = {}
_job_locks_guard = threading.Lock()
_git_lock = threading.Lock()
_max_concurrent_fixes = threading.Semaphore(MAX_CONCURRENT_FIXES)


def _get_job_lock(job: str) -> threading.Lock:
    """Lazily create and return a per-job lock."""
    with _job_locks_guard:
        if job not in _job_locks:
            _job_locks[job] = threading.Lock()
        return _job_locks[job]


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
    "deploy": "Fix deployment failure. Check docker-compose.prod.yml. May need infra changes.",
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
    }
    return notes.get(error_class, "")


def _diagnose(job, step, error_log, error_class, context, repro=None):
    """Stage 1: Diagnose using local reproduction + gh CLI for full logs.

    Returns a short strategy string the executor can use.
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

    prompt = (
        f"DIAGNOSE ONLY -- do NOT edit files or run git.\n\n"
        f"CI FAILED: job='{job}', step='{step}'{ctx}, class={effective_class}\n"
        f"{repro_section}"
        f"{_gh_cli_block(run_url)}"
        f"Error preview:\n```\n{error_log[:2000]}\n```\n\n"
        f"Job context: {_JOB_INSTRUCTIONS.get(job, 'Analyze and fix.')}"
        f"{_build_error_notes(effective_class)}\n\n"
        f"1. Identify root cause from {source}.\n"
        f"2. List specific files/lines that need changes.\n"
        f"3. Describe minimal fix strategy in 2-3 sentences."
    )

    result = run_kiro(prompt, f"Diagnose ({job}/{step})")
    kiro_output = result[1] if isinstance(result, tuple) else ""
    return kiro_output[-2000:] if kiro_output else "diagnosis unavailable"


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


def fix_with_retry(job, step, error_log, context=None):
    """GSD-inspired fix pipeline: preflight → diagnose → execute → verify → gate."""
    job_lock = _get_job_lock(job)
    if not job_lock.acquire(blocking=False):
        log.warning(f"Fix already running for job '{job}' -- skipping {job}/{step}")
        return False

    _max_concurrent_fixes.acquire()
    cycle_start = time.monotonic()
    all_timings: list[PhaseTiming] = []
    try:
        # Gate 1: Time-based dedup
        passed, reason = preflight_dedup(job, error_log)
        if not passed:
            log.warning(f"Preflight dedup rejected {job}/{step}: {reason}")
            _write_escalation(job, step, error_log, "dedup_rejected", context,
                              extra_reason=reason)
            return False

        ctx_str = ""
        if context:
            ctx_str = (
                f" (commit: {context.get('commit', '?')[:8]}, "
                f"branch: {context.get('branch', '?')}, "
                f"actor: {context.get('actor', '?')})"
            )

        error_class = classify_error(error_log)
        log.info(f"Error classified as: {error_class} for {job}/{step}")

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

        for attempt in range(1, MAX_RETRIES + 1):
            attempt_start = time.monotonic()
            timing = PhaseTiming()

            log.info(f"=== Attempt {attempt}/{MAX_RETRIES} for {job}/{step}{ctx_str} (class: {error_class}) ===")

            instruction = _JOB_INSTRUCTIONS.get(job, "Analyze and fix.")
            verify_cmd = _JOB_VERIFY.get(job, "")
            error_note = _build_error_notes(error_class)

            # On retry, send a minimal "try different approach" directive
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
                # First attempt uses the initial reproduction timing
                timing.reproduction_s = initial_repro_s

            run_url = context.get("run_url", "") if context else ""

            repro_section = ""
            if repro and repro.reproduced:
                repro_section = format_errors_for_prompt(repro)
            elif repro:
                repro_section = "\nDid NOT reproduce locally -- likely CI-specific (env/cache/Docker).\n"

            reproduced = repro and repro.reproduced
            source = "local reproduction output" if reproduced else "gh CLI full logs"

            # Combined prompt: diagnose + fix + verify in a single kiro-cli call
            prompt = (
                f"CI FAILED: job='{job}', step='{step}'{ctx_str}, "
                f"class={error_class}, attempt {attempt}/{MAX_RETRIES}\n\n"
                f"{repro_section}"
                f"{_gh_cli_block(run_url)}"
                f"Error preview:\n```\n{error_log[:3000]}\n```\n\n"
                f"Job context: {instruction}{error_note}\n\n"
                f"STEP 1 - DIAGNOSE: Identify root cause from {source}. "
                f"List specific files/lines that need changes.\n"
                f"STEP 2 - FIX: Apply the minimal fix to resolve the issue.\n"
            )

            verify_line = f"STEP 3 - VERIFY: Run: {verify_cmd}\n" if verify_cmd else ""
            prompt += verify_line
            prompt += "STEP 4 - STAGE: git add -A\n"

            # Append history/semantic only on first attempt to save tokens
            extras = ""
            if attempt == 1:
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
            result = run_kiro(prompt, f"CI fix ({job}/{step}) attempt {attempt}")
            timing.fix_attempt_s = time.monotonic() - t0

            success = result[0] if isinstance(result, tuple) else result
            kiro_output = result[1] if isinstance(result, tuple) else ""

            outcome = extract_outcome_from_output(kiro_output) if kiro_output else {}
            strategy_note = outcome.get("strategy", "")

            if not success:
                timing.total_s = time.monotonic() - attempt_start
                all_timings.append(timing)
                last_strategy_summary = strategy_note or "unknown"
                record_fix_attempt(job, error_log, attempt, False,
                                   f"class={error_class}, job={job}, step={step}. {strategy_note}",
                                   timing=dataclasses.asdict(timing))
                store_fix_attempt(job, step, error_class, error_log, attempt, False)
                log.warning(f"Attempt {attempt} kiro-cli returned failure for {job}/{step}")
                continue

            # Gate 2: Scope enforcement
            t0 = time.monotonic()
            scope_passed, scope_reason = revision_scope_check()
            timing.scope_gate_s = time.monotonic() - t0

            if not scope_passed:
                timing.total_s = time.monotonic() - attempt_start
                all_timings.append(timing)
                log.warning(f"Scope gate rejected attempt {attempt} for {job}/{step}: {scope_reason}")
                wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
                last_strategy_summary = f"SCOPE VIOLATION ({scope_reason}). {strategy_note}"
                record_fix_attempt(job, error_log, attempt, False,
                                   f"scope_rejected: {scope_reason}. {strategy_note}",
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
                record_fix_attempt(job, error_log, attempt, False,
                                   f"verify_failed: {verify_reason[:200]}. {strategy_note}",
                                   timing=dataclasses.asdict(timing))
                store_fix_attempt(job, step, error_class, error_log, attempt, False)
                continue

            # All gates passed -- commit and push
            timing.total_s = time.monotonic() - attempt_start
            all_timings.append(timing)
            log.info(f"All gates passed for attempt {attempt} ({job}/{step}). Committing.")
            with _git_lock:
                commit_result = wsl_bash(
                    "git add -A && "
                    "git commit -m 'fix: resolve CI failures (auto-fix)' && "
                    "git push origin main || "
                    "(git pull --rebase origin main && git push origin main)",
                    timeout=120,
                )

            commit_ok = commit_result.returncode == 0
            record_fix_attempt(job, error_log, attempt, commit_ok,
                               f"class={error_class}, job={job}, step={step}. {strategy_note}",
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
                return True

            log.warning(f"Commit/push failed for attempt {attempt} ({job}/{step})")
            last_strategy_summary = f"commit/push failed. {strategy_note}"

        _write_escalation(job, step, error_log, error_class, context, repro=repro)
        log.error(f"Failed after {MAX_RETRIES} attempts for {job}/{step}. Escalation written.")
        _log_timing_summary(job, step, cycle_start, all_timings)
        return False
    finally:
        _max_concurrent_fixes.release()
        job_lock.release()


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
            "max_retries_exhausted": MAX_RETRIES,
            "error_log_preview": error_log[:3000],
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
    }
    return recs.get(error_class, f"Auto-fix exhausted {MAX_RETRIES} attempts for {job}. Manual investigation required.")


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


def _fix_sonar_file_group(file_group_report, group_label):
    """Fix sonar issues for a single file group. Returns (success, label)."""
    is_kotlin = any(f.endswith((".kt", ".kts")) for f in group_label.split(","))
    verify_cmd = (
        "(cd android && ./gradlew spotlessApply detekt testDebugUnitTest)"
        if is_kotlin else
        "cargo fmt && cargo clippy --locked -- -D warnings && cargo test --locked"
    )

    prompt = (
        "Fix ONLY the SonarQube issues listed below.\n\n"
        f"Issues:\n```\n{file_group_report}\n```\n\n"
        "For each: open file at line, read context, apply minimal fix.\n"
        "Security hotspots: evaluate if actually vulnerable before changing.\n"
        f"Verify: {verify_cmd}\n"
        "Stage: git add -A. Do NOT commit.\n"
        f"{SCOPE_CONSTRAINTS}"
    )

    result = run_kiro(prompt, f"Sonar fix ({group_label})")
    success = result[0] if isinstance(result, tuple) else result
    return success, group_label


def fix_sonar_issues(sonar_report, run_url):
    """Wave-based sonar fix: parse by file, execute groups in parallel."""
    if not _sonar_fix_lock.acquire(blocking=False):
        log.warning("Another SonarQube fix is already running -- skipping")
        return False

    try:
        file_groups = _parse_sonar_report_by_file(sonar_report)

        if not file_groups:
            log.warning("No parseable issues in sonar report, falling back to single prompt")
            file_groups = {"all": sonar_report.splitlines()}

        # Build waves: group files into batches for parallel execution
        group_items = list(file_groups.items())
        waves = []
        for i in range(0, len(group_items), SONAR_PARALLEL_GROUPS):
            waves.append(group_items[i:i + SONAR_PARALLEL_GROUPS])

        log.info(f"Sonar fix: {len(file_groups)} file groups in {len(waves)} waves")

        for attempt in range(1, MAX_RETRIES + 1):
            log.info(f"=== SonarQube fix attempt {attempt}/{MAX_RETRIES} ({len(waves)} waves) ===")

            all_succeeded = True
            failed_groups = []

            for wave_idx, wave in enumerate(waves):
                log.info(f"  Wave {wave_idx + 1}/{len(waves)}: {len(wave)} groups")

                if len(wave) == 1:
                    file_name, issues = wave[0]
                    group_report = "\n".join(issues)
                    success, label = _fix_sonar_file_group(group_report, file_name)
                    if not success:
                        all_succeeded = False
                        failed_groups.append(label)
                else:
                    with ThreadPoolExecutor(max_workers=SONAR_PARALLEL_GROUPS) as executor:
                        futures = {}
                        for file_name, issues in wave:
                            group_report = "\n".join(issues)
                            future = executor.submit(
                                _fix_sonar_file_group, group_report, file_name,
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

            # After all waves, verify scope and commit
            scope_passed, scope_reason = revision_scope_check()
            if not scope_passed:
                log.warning(f"Sonar fix scope gate rejected: {scope_reason}")
                wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
                if attempt < MAX_RETRIES:
                    continue
                log.error("Sonar fix exceeded scope limits on all attempts")
                return False

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
                    log.warning(f"Sonar fix partial success. Failed groups: {failed_groups}")
                else:
                    log.info(f"SonarQube fix attempt {attempt} completed (all groups)")
                return True

            log.warning(f"SonarQube fix attempt {attempt} commit/push failed")

        log.error(f"SonarQube fix failed after {MAX_RETRIES} attempts.")
        return False
    finally:
        _sonar_fix_lock.release()


def improve_pipeline(focus, pipeline_report, run_url, sonar_report=""):
    if not _improve_lock.acquire(blocking=False):
        log.warning("Another pipeline improvement is already running -- skipping")
        return False

    try:
        for attempt in range(1, MAX_RETRIES + 1):
            log.info(f"=== Pipeline improvement attempt {attempt}/{MAX_RETRIES} (focus: {focus}) ===")

            sonar_section = ""
            if sonar_report and "unavailable" not in sonar_report:
                sonar_section = f"\n\n--- SONARQUBE ISSUES ---\n{sonar_report}\n--- END SONARQUBE ISSUES ---\n"

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

            focus_specific = ""
            for f in focus.split(","):
                f = f.strip()
                if f in focus_guidance:
                    focus_specific += focus_guidance[f]

            prompt = (
                f"CI/CD pipeline engineer. Rust/WASM + Android project.\n"
                f"Focus: {focus}\n"
                f"{_gh_cli_block(run_url)}\n"
                f"--- PIPELINE REPORT ---\n{pipeline_report}\n--- END ---\n"
                f"{sonar_section}\n"
                "Fetch full logs via gh CLI first.\n\n"
                "CLASSIFY each failure:\n"
                "  A) RUNNER ENV → document, don't change code\n"
                "  B) PIPELINE CONFIG → fix in .github/workflows/\n"
                "  C) DEPENDENCY VULN → fix in Cargo.toml/deny.toml/libs.versions.toml\n"
                "  D) APP CODE → skip (handled by /ci-failure)\n\n"
                f"{focus_specific}"
                "Look for: cache misses, slow jobs, failed jobs, redundant steps.\n"
                "Read workflow YAML fully before editing. Verify YAML validity after.\n"
                "Stage: git add -A. Do NOT commit.\n"
                f"Run URL: {run_url}"
            )

            result = run_kiro(prompt, f"Pipeline improve ({focus}) attempt {attempt}")
            success = result[0] if isinstance(result, tuple) else result

            if not success:
                log.warning(f"Pipeline improvement attempt {attempt} failed")
                continue

            # Scope gate
            scope_passed, scope_reason = revision_scope_check()
            if not scope_passed:
                log.warning(f"Pipeline improve scope gate rejected: {scope_reason}")
                wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
                continue

            commit_result = wsl_bash(
                "git add -A && "
                "git diff --cached --quiet && echo 'nothing to commit' || "
                f"(git commit -m 'ci: improve pipeline ({focus})' && "
                "git push origin main || "
                "(git pull --rebase origin main && git push origin main))",
                timeout=120,
            )

            if commit_result.returncode == 0:
                log.info(f"Pipeline improvement attempt {attempt} completed")
                return True

            log.warning(f"Pipeline improvement attempt {attempt} commit/push failed")

        log.error(f"Pipeline improvement failed after {MAX_RETRIES} attempts.")
        return False
    finally:
        _improve_lock.release()
