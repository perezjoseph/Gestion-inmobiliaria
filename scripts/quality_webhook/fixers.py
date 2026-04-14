"""
CI fixer workflows with GSD-inspired patterns:

1. Post-fix verification gate — independent verify after kiro returns
2. Scope enforcement gate — reject diffs exceeding file/line limits
3. Fresh context on retry — trim prompts instead of accumulating context
4. Time-based dedup — skip to escalation if same error hash failed recently
5. Parallel sonar file groups — wave-based execution for independent files
6. Diagnose-before-fix — separate "read and plan" from "execute the fix"
"""

import json
import re
import threading
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime

from .config import (
    WIN_PROJECT_DIR, MAX_RETRIES, SCOPE_CONSTRAINTS,
    SONAR_PARALLEL_GROUPS, log,
)
from .runner import wsl_bash, run_kiro
from .classifier import classify_error
from .history import record_fix_attempt, get_past_attempts
from .memory import (
    store_fix_attempt, store_fix_outcome,
    search_relevant_context, extract_outcome_from_output,
)
from .gates import (
    preflight_dedup, revision_scope_check, verification_run,
)

_fix_lock = threading.Lock()
_sonar_fix_lock = threading.Lock()
_improve_lock = threading.Lock()


def _parse_run_id(run_url):
    """Extract the numeric run ID from a GitHub Actions run URL."""
    if not run_url:
        return "", ""
    match = re.search(r"github\.com/([^/]+/[^/]+)/actions/runs/(\d+)", run_url)
    if match:
        return match.group(2), match.group(1)
    match = re.search(r"/actions/runs/(\d+)", run_url)
    return (match.group(1), "") if match else ("", "")


def _gh_troubleshoot_instructions(run_url, job):
    """Build gh CLI instructions for fetching full pipeline logs and annotations."""
    run_id, repo = _parse_run_id(run_url)
    if not run_id:
        return ""
    repo_flag = f" -R {repo}" if repo else ""
    return (
        f"\n\nTROUBLESHOOTING WITH GH CLI (do this FIRST):\n"
        f"The error_log below may be truncated. Use gh cli to get the full picture:\n"
        f"  1. View the failed run summary:  gh run view {run_id}{repo_flag}\n"
        f"  2. View full logs of failed jobs: gh run view {run_id}{repo_flag} --log-failed\n"
        f"  3. If you need a specific job:    gh run view {run_id}{repo_flag} --job <job-id> --log\n"
        f"  4. List jobs with their IDs:      gh run view {run_id}{repo_flag} --json jobs --jq '.jobs[] | \"\\(.name) (\\(.conclusion)) id=\\(.databaseId)\"'\n"
        f"  5. View annotations (::error:: and ::warning:: messages from the pipeline):\n"
        f"     gh api repos/{repo}/check-runs/<check_run_id>/annotations\n"
        f"     Get check_run_ids from step 4 (databaseId values), then query annotations for failed jobs.\n"
        f"     Annotations often contain the most precise error location (file, line, message).\n"
        f"The gh output gives you the COMPLETE untruncated logs. "
        f"Use them to understand the real root cause before making any changes.\n"
        f"Run URL: {run_url}\n"
    )


def _is_autofix_commit():
    try:
        result = wsl_bash("git log -1 --format='%s' HEAD 2>/dev/null", timeout=15)
        if result.returncode == 0:
            msg = result.stdout.strip()
            autofix_markers = [
                "(auto-fix)", "resolve CI failures (auto-fix)",
                "resolve SonarQube issues (auto-fix)", "improve pipeline",
            ]
            return any(marker in msg for marker in autofix_markers)
    except Exception as e:
        log.warning(f"Auto-fix commit check failed: {e}")
    return False


_JOB_INSTRUCTIONS = {
    "lint": (
        "Fix the formatting and clippy warnings. "
        "Run 'cargo fmt --all' for formatting, then 'cargo clippy --locked -p realestate-backend -- -D warnings' "
        "and 'cargo clippy --locked -p realestate-frontend --target wasm32-unknown-unknown -- -D warnings' to verify."
    ),
    "test-backend": (
        "Fix the failing backend tests. Read the test output carefully to identify which tests failed and why. "
        "Run 'cargo test --locked -p realestate-backend --all-targets' to verify."
    ),
    "test-frontend": (
        "Fix the failing frontend tests. "
        "Run 'cargo test --locked -p realestate-frontend --all-targets' to verify."
    ),
    "quality-gate": (
        "Fix the quality gate failures. The error log contains details from cargo-audit, cargo-deny, or OWASP scans. "
        "For RUSTSEC/CVE advisories: update the affected crate version in Cargo.toml, run 'cargo update' for the specific crate, then verify with 'cargo audit'. "
        "For OWASP findings: check the vulnerability details and update or suppress as appropriate. "
        "For license violations: check deny.toml and adjust exceptions if the license is acceptable. "
        "Do NOT modify the CI pipeline YAML -- fix the dependency versions in Cargo.toml or deny.toml."
    ),
    "sonarqube": "Fix the SonarQube analysis failures. Check the SonarQube server configuration and scan settings.",
    "android-lint": (
        "Fix the Android lint warnings in the android/ directory. "
        "Read the specific lint errors in the output. "
        "Run './gradlew lint' in android/ to verify. "
        "For detekt issues, run './gradlew detekt' in android/. "
        "For ktfmt issues, run './gradlew spotlessApply' in android/ to auto-fix formatting."
    ),
    "android-unit-test": (
        "Fix the failing Android unit tests in the android/ directory. "
        "Read the test output to identify which tests failed. "
        "Run './gradlew testDebugUnitTest' in android/ to verify."
    ),
    "android-build": (
        "Fix the Android build errors in the android/ directory. "
        "Read the Gradle build output for compilation errors. "
        "Run './gradlew assembleDebug' in android/ to verify."
    ),
    "android-quality-gate": (
        "Fix the Android dependency security issues. "
        "The OWASP report found vulnerabilities with CVSS >= 7. "
        "Update vulnerable dependencies in android/gradle/libs.versions.toml to patched versions. "
        "If no patch exists, add a suppression to android-owasp-suppressions.xml with justification. "
        "Run './gradlew dependencies' in android/ to verify resolution."
    ),
    "android-sonarqube": "Fix the Android SonarQube analysis failures. Check scan configuration and server connectivity.",
    "build-frontend": (
        "Fix the frontend WASM build errors. "
        "Run 'trunk build --release' in frontend/ to reproduce. "
        "Check for compilation errors targeting wasm32-unknown-unknown."
    ),
    "build-backend": (
        "Fix the backend release build errors. "
        "Run 'cargo build -p realestate-backend --release' to reproduce."
    ),
    "container-image-backend": (
        "Fix the backend container image build or Trivy scan failure. "
        "Check Dockerfile.backend for build errors. "
        "For Trivy vulnerabilities, update base image or fix vulnerable packages."
    ),
    "container-image-frontend": (
        "Fix the frontend container image build or Trivy scan failure. "
        "Check Dockerfile.frontend for build errors."
    ),
    "deploy": (
        "Fix the production deployment failure. "
        "Check docker-compose.prod.yml and service health. "
        "This may require infrastructure changes that cannot be auto-fixed."
    ),
    "secret-scan": (
        "WARNING: Gitleaks detected secrets in the repository. "
        "Do NOT commit the secret again. Remove the leaked credential from the codebase, "
        "add it to .gitleaksignore if it's a false positive, and rotate any exposed credentials."
    ),
}


def _build_error_notes(error_class):
    notes = {
        "runner_environment": (
            "\n\nIMPORTANT: This error appears to be a RUNNER ENVIRONMENT issue "
            "(missing tool, network problem, disk space, or Docker issue). "
            "These cannot be fixed by changing application code. "
            "If you confirm this is a runner/infra issue, do NOT make code changes. "
            "Instead, document the issue and suggest what the runner admin should fix. "
            "Only make code changes if the error is actually caused by the code."
        ),
        "dependency": (
            "\n\nDEPENDENCY ISSUE METHODOLOGY:\n"
            "1. Parse the CVE/RUSTSEC/GHSA IDs from the error log.\n"
            "2. For each advisory, identify the affected crate and its current version.\n"
            "3. Check if a patched version exists (use 'cargo audit' output or search crates.io).\n"
            "4. If a patch exists: update the version constraint in Cargo.toml, run 'cargo update -p <crate>'.\n"
            "5. If no patch exists: evaluate if the vulnerability applies to our usage. "
            "If not applicable, add to .cargo/audit.toml [advisories.ignore]. "
            "If applicable, find an alternative crate or implement a workaround.\n"
            "6. Verify: run 'cargo audit' and 'cargo deny check' to confirm resolution.\n"
            "7. Run 'cargo test --locked -p realestate-backend --all-targets' to ensure nothing broke."
        ),
        "test_failure": (
            "\n\nFLAKY TEST CHECK (do this FIRST):\n"
            "Before making any code changes, re-run the failing test 2 times. "
            "If it passes on re-run, the test is flaky -- do NOT change application code. "
            "Instead, note which test is flaky and move on. "
            "Only fix the code if the test fails consistently on every re-run."
        ),
    }
    return notes.get(error_class, "")


def _diagnose(job, step, error_log, error_class, context):
    """Stage 1: Diagnose the error using gh CLI for full logs, then produce a fix plan.

    Returns a short strategy string the executor can use.
    """
    ctx = ""
    run_url = ""
    if context:
        ctx = f" (commit: {context.get('commit', '?')[:8]}, branch: {context.get('branch', '?')})"
        run_url = context.get("run_url", "")

    instruction = _JOB_INSTRUCTIONS.get(job, "Analyze the error and fix the issue.")
    error_note = _build_error_notes(error_class)
    gh_instructions = _gh_troubleshoot_instructions(run_url, job)

    prompt = (
        f"DIAGNOSE ONLY -- do NOT make changes yet.\n\n"
        f"The CI pipeline FAILED in job '{job}', step '{step}'.{ctx}\n"
        f"Error classification: {error_class}\n"
        f"{gh_instructions}\n"
        f"Webhook error preview (may be truncated):\n```\n{error_log[:2000]}\n```\n\n"
        f"Context: {instruction}\n"
        f"{error_note}\n\n"
        "YOUR TASK:\n"
        "1. Use the gh CLI commands above to fetch the FULL error logs from the pipeline run.\n"
        "2. Read the complete output carefully.\n"
        "3. Identify the root cause.\n"
        "4. List the specific files that need changes.\n"
        "5. Describe the minimal fix strategy in 2-3 sentences.\n"
        "6. Do NOT edit any files. Do NOT run git commands.\n"
        "7. Output your diagnosis as plain text."
    )

    result = run_kiro(prompt, f"Diagnose ({job}/{step})")
    kiro_output = result[1] if isinstance(result, tuple) else ""
    return kiro_output[-2000:] if kiro_output else "diagnosis unavailable"


def fix_with_retry(job, step, error_log, context=None):
    """GSD-inspired fix pipeline: preflight → diagnose → execute → verify → gate."""
    if not _fix_lock.acquire(blocking=False):
        log.warning(f"Another fix is already running -- skipping {job}/{step}")
        return False

    try:
        if _is_autofix_commit():
            log.warning(f"Last commit is an auto-fix -- skipping {job}/{step} to prevent loop")
            return False

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

        # Stage 1: Diagnose before fixing
        log.info(f"=== Diagnosing {job}/{step} ===")
        diagnosis = _diagnose(job, step, error_log, error_class, context)
        log.info(f"Diagnosis complete for {job}/{step}")

        past_attempts_section = get_past_attempts(job, error_log)
        semantic_context = search_relevant_context(job, error_log)
        last_strategy_summary = ""

        for attempt in range(1, MAX_RETRIES + 1):
            log.info(f"=== Attempt {attempt}/{MAX_RETRIES} for {job}/{step}{ctx_str} (class: {error_class}) ===")

            instruction = _JOB_INSTRUCTIONS.get(job, "Analyze the error and fix the issue.")
            error_note = _build_error_notes(error_class)

            # Fresh context per retry: only include what's needed
            history_section = f"\n\n{past_attempts_section}" if past_attempts_section else ""
            semantic_section = f"\n\n{semantic_context}" if semantic_context else ""

            # On retry, send a minimal "try different approach" directive
            retry_section = ""
            if last_strategy_summary:
                retry_section = (
                    f"\n\nPREVIOUS ATTEMPT FAILED. Strategy tried: {last_strategy_summary}\n"
                    "You MUST try a DIFFERENT approach. Do not repeat the same fix."
                )

            run_url = context.get("run_url", "") if context else ""
            gh_instructions = _gh_troubleshoot_instructions(run_url, job)

            prompt = (
                f"The CI pipeline FAILED in job '{job}', step '{step}'.{ctx_str}\n"
                f"Error classification: {error_class}\n"
                f"Fix attempt: {attempt} of {MAX_RETRIES}\n\n"
                f"DIAGNOSIS:\n{diagnosis[:1500]}\n"
                f"{gh_instructions}\n"
                f"Webhook error preview (may be truncated):\n```\n{error_log[:3000]}\n```\n\n"
                f"TASK: First use the gh CLI commands above to fetch the FULL error logs, "
                f"then apply the fix.\n{instruction}"
                f"{error_note}"
                f"{history_section}"
                f"{semantic_section}"
                f"{retry_section}"
                f"{SCOPE_CONSTRAINTS}\n\n"
                "After fixing, run only the targeted verification command for this job. "
                "Then stage your changes:\n"
                "  git add -A\n"
                "Do NOT commit yet -- the system will verify and commit."
            )

            result = run_kiro(prompt, f"CI fix ({job}/{step}) attempt {attempt}")
            success = result[0] if isinstance(result, tuple) else result
            kiro_output = result[1] if isinstance(result, tuple) else ""

            outcome = extract_outcome_from_output(kiro_output) if kiro_output else {}
            strategy_note = outcome.get("strategy", "")

            if not success:
                last_strategy_summary = strategy_note or "unknown"
                record_fix_attempt(job, error_log, attempt, False,
                                   f"class={error_class}, job={job}, step={step}. {strategy_note}")
                store_fix_attempt(job, step, error_class, error_log, attempt, False)
                log.warning(f"Attempt {attempt} kiro-cli returned failure for {job}/{step}")
                continue

            # Gate 2: Scope enforcement
            scope_passed, scope_reason = revision_scope_check()
            if not scope_passed:
                log.warning(f"Scope gate rejected attempt {attempt} for {job}/{step}: {scope_reason}")
                wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
                last_strategy_summary = f"SCOPE VIOLATION ({scope_reason}). {strategy_note}"
                record_fix_attempt(job, error_log, attempt, False,
                                   f"scope_rejected: {scope_reason}. {strategy_note}")
                store_fix_attempt(job, step, error_class, error_log, attempt, False)
                continue

            # Gate 3: Independent verification
            verify_passed, verify_reason = verification_run(job)
            if not verify_passed:
                log.warning(f"Verification gate failed for attempt {attempt} ({job}/{step}): {verify_reason}")
                wsl_bash("git checkout -- . 2>/dev/null; git clean -fd 2>/dev/null", timeout=15)
                last_strategy_summary = f"VERIFY FAILED ({verify_reason[:200]}). {strategy_note}"
                record_fix_attempt(job, error_log, attempt, False,
                                   f"verify_failed: {verify_reason[:200]}. {strategy_note}")
                store_fix_attempt(job, step, error_class, error_log, attempt, False)
                continue

            # All gates passed -- commit and push
            log.info(f"All gates passed for attempt {attempt} ({job}/{step}). Committing.")
            commit_result = wsl_bash(
                "git add -A && "
                "git commit -m 'fix: resolve CI failures (auto-fix)' && "
                "git push origin main || "
                "(git pull --rebase origin main && git push origin main)",
                timeout=120,
            )

            commit_ok = commit_result.returncode == 0
            record_fix_attempt(job, error_log, attempt, commit_ok,
                               f"class={error_class}, job={job}, step={step}. {strategy_note}")
            store_fix_attempt(job, step, error_class, error_log, attempt, commit_ok)

            if commit_ok:
                store_fix_outcome(
                    job, error_class,
                    files_changed=outcome.get("files_changed", ""),
                    strategy=strategy_note or f"Fixed via attempt {attempt}",
                )
                log.info(f"Attempt {attempt} completed for {job}/{step}")
                return True

            log.warning(f"Commit/push failed for attempt {attempt} ({job}/{step})")
            last_strategy_summary = f"commit/push failed. {strategy_note}"

        _write_escalation(job, step, error_log, error_class, context)
        log.error(f"Failed after {MAX_RETRIES} attempts for {job}/{step}. Escalation written.")
        return False
    finally:
        _fix_lock.release()


def _write_escalation(job, step, error_log, error_class, context, extra_reason=""):
    escalation_dir = WIN_PROJECT_DIR / "escalations"
    try:
        escalation_dir.mkdir(exist_ok=True)
        ts = datetime.now().strftime("%Y%m%d_%H%M%S")

        # Include actual strategies attempted from history
        past = get_past_attempts(job, error_log)
        semantic = search_relevant_context(job, error_log)

        report = {
            "timestamp": datetime.now().isoformat(),
            "job": job,
            "step": step,
            "error_class": error_class,
            "context": context or {},
            "max_retries_exhausted": MAX_RETRIES,
            "error_log_preview": error_log[:3000],
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


def _fix_sonar_file_group(file_group_report, run_url, group_label):
    """Fix sonar issues for a single file group. Returns (success, label)."""
    is_kotlin = any(f.endswith((".kt", ".kts")) for f in group_label.split(","))
    verify_cmd = (
        "(cd android && ./gradlew spotlessApply detekt testDebugUnitTest)"
        if is_kotlin else
        "cargo fmt && cargo clippy --locked -- -D warnings && cargo test --locked"
    )

    prompt = (
        "You are a code quality specialist. Fix ONLY the issues listed below.\n\n"
        f"SonarQube issues for this file group:\n```\n{file_group_report}\n```\n\n"
        "METHODOLOGY:\n"
        "1. Open each file at the specified line.\n"
        "2. Read 10 lines of context above and below.\n"
        "3. Apply the minimal fix for each violation.\n"
        "4. For security hotspots: evaluate if actually vulnerable before changing.\n"
        f"5. Verify: {verify_cmd}\n"
        "6. Stage changes: git add -A\n"
        "Do NOT commit -- the orchestrator handles commits.\n"
        f"{SCOPE_CONSTRAINTS}\n\n"
        f"Run URL: {run_url}"
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
        if _is_autofix_commit():
            log.warning("Last commit is an auto-fix -- skipping SonarQube fix to prevent loop")
            return False

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
                    success, label = _fix_sonar_file_group(group_report, run_url, file_name)
                    if not success:
                        all_succeeded = False
                        failed_groups.append(label)
                else:
                    with ThreadPoolExecutor(max_workers=SONAR_PARALLEL_GROUPS) as executor:
                        futures = {}
                        for file_name, issues in wave:
                            group_report = "\n".join(issues)
                            future = executor.submit(
                                _fix_sonar_file_group, group_report, run_url, file_name,
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
                    "SECURITY FOCUS:\n"
                    "- If the error is from OWASP/cargo-audit/cargo-deny, the fix belongs in Cargo.toml, "
                    "deny.toml, or android/gradle/libs.versions.toml -- NOT in the pipeline YAML.\n"
                    "- Only modify the pipeline YAML if the security tool itself is misconfigured.\n"
                    "- For dependency vulnerabilities: update crate versions, add suppressions for false positives.\n"
                ),
                "bugs": (
                    "BUG FIX FOCUS:\n"
                    "- The /ci-failure webhook handles application code fixes.\n"
                    "- Only improve the pipeline if the failure is caused by pipeline misconfiguration "
                    "(wrong environment, missing tools, bad caching).\n"
                    "- If error logs show code compilation or test failures, do NOT change the pipeline.\n"
                ),
                "pipeline": (
                    "PIPELINE FOCUS:\n"
                    "- Fix build/deploy pipeline configuration issues.\n"
                    "- Check Dockerfile syntax, docker-compose config, deployment scripts.\n"
                ),
            }

            focus_specific = ""
            for f in focus.split(","):
                f = f.strip()
                if f in focus_guidance:
                    focus_specific += focus_guidance[f] + "\n"

            gh_instructions = _gh_troubleshoot_instructions(run_url, "pipeline")

            prompt = (
                "You are a CI/CD pipeline engineer for a Rust/WASM real estate management project "
                "with an Android module.\n\n"
                f"Focus area: {focus}\n"
                f"{gh_instructions}\n"
                f"--- PIPELINE REPORT ---\n{pipeline_report}\n--- END PIPELINE REPORT ---\n"
                f"{sonar_section}\n"
                "FIRST: Use the gh CLI commands above to fetch the FULL logs from the pipeline run. "
                "The pipeline report below may be truncated or incomplete.\n\n"
                "DECISION FRAMEWORK -- read the error logs and classify each failure:\n"
                "  A) RUNNER ENVIRONMENT (missing tool, network, disk, Docker): "
                "Document the issue but do NOT change code. These need admin intervention.\n"
                "  B) PIPELINE CONFIG (wrong step order, bad caching, missing env var): "
                "Fix in .github/workflows/ci.yml or android-ci.yml.\n"
                "  C) DEPENDENCY VULNERABILITY (CVE, RUSTSEC, OWASP): "
                "Fix in Cargo.toml, deny.toml, or android/gradle/libs.versions.toml. NOT the pipeline YAML.\n"
                "  D) APPLICATION CODE (compilation, test failure): "
                "Skip -- the /ci-failure webhook handles these.\n\n"
                f"{focus_specific}"
                "IMPROVEMENTS TO LOOK FOR:\n"
                "- Jobs with cache MISS: investigate cache key strategy, consider broader restore-keys.\n"
                "- Slow jobs (high duration): look for parallelization, unnecessary steps, or missing caching.\n"
                "- Failed jobs: read the error logs and apply the decision framework above.\n"
                "- All jobs passed: look for security hardening, redundant steps, or reporting gaps.\n\n"
                "RULES:\n"
                "1. Read the relevant workflow YAML files fully before making changes.\n"
                "2. Make ALL improvements you identify. Do not stop at one.\n"
                "3. After making changes, verify the YAML is valid.\n"
                "4. Stage changes: git add -A\n"
                "Do NOT commit -- the system will verify and commit.\n\n"
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
