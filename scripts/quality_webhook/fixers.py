import json
import threading
from datetime import datetime

from .config import WIN_PROJECT_DIR, MAX_RETRIES, SCOPE_CONSTRAINTS, log
from .runner import wsl_bash, run_kiro
from .classifier import classify_error
from .history import record_fix_attempt, get_past_attempts
from .memory import store_fix_attempt, store_fix_outcome, search_relevant_context

_fix_lock = threading.Lock()
_sonar_fix_lock = threading.Lock()
_improve_lock = threading.Lock()


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


def fix_with_retry(job, step, error_log, context=None):
    if not _fix_lock.acquire(blocking=False):
        log.warning(f"Another fix is already running -- skipping {job}/{step}")
        return False

    try:
        if _is_autofix_commit():
            log.warning(f"Last commit is an auto-fix -- skipping {job}/{step} to prevent loop")
            return False

        ctx = ""
        if context:
            ctx = f" (commit: {context.get('commit', '?')[:8]}, branch: {context.get('branch', '?')}, actor: {context.get('actor', '?')})"

        error_class = classify_error(error_log)
        log.info(f"Error classified as: {error_class} for {job}/{step}")

        past_attempts_section = get_past_attempts(job, error_log)
        semantic_context = search_relevant_context(job, error_log)
        previous_attempt_result = ""

        for attempt in range(1, MAX_RETRIES + 1):
            log.info(f"=== Attempt {attempt}/{MAX_RETRIES} for {job}/{step}{ctx} (class: {error_class}) ===")

            instruction = _JOB_INSTRUCTIONS.get(job, "Analyze the error and fix the issue.")
            error_note = _build_error_notes(error_class)

            history_section = f"\n\n{past_attempts_section}" if past_attempts_section else ""
            semantic_section = f"\n\n{semantic_context}" if semantic_context else ""
            retry_section = ""
            if previous_attempt_result:
                retry_section = (
                    f"\n\nPREVIOUS ATTEMPT FAILED (attempt {attempt - 1}):\n"
                    f"{previous_attempt_result}\n"
                    "You MUST try a DIFFERENT approach this time. Do not repeat the same fix."
                )

            prompt = (
                f"The CI pipeline FAILED in job '{job}', step '{step}'.{ctx}\n"
                f"Error classification: {error_class}\n"
                f"Fix attempt: {attempt} of {MAX_RETRIES}\n\n"
                f"Error output:\n```\n{error_log}\n```\n\n"
                f"TASK: {instruction}"
                f"{error_note}"
                f"{history_section}"
                f"{semantic_section}"
                f"{retry_section}"
                f"{SCOPE_CONSTRAINTS}\n\n"
                "After fixing, run only the targeted verification command for this job. "
                "Then commit and push:\n"
                "  git add -A && git commit -m 'fix: resolve CI failures (auto-fix)' && git push origin main\n"
                "  If push fails: git pull --rebase origin main && git push origin main"
            )

            success = run_kiro(prompt, f"CI fix ({job}/{step}) attempt {attempt}")
            record_fix_attempt(job, error_log, attempt, success,
                               f"class={error_class}, job={job}, step={step}")
            store_fix_attempt(job, step, error_class, error_log, attempt, success)

            if success:
                store_fix_outcome(job, error_class, strategy=f"Fixed via attempt {attempt}")
                log.info(f"Attempt {attempt} completed for {job}/{step}")
                return True

            previous_attempt_result = (
                f"kiro-cli returned failure for attempt {attempt}. "
                f"The fix either did not compile, tests still failed, or the push was rejected. "
                f"Error class: {error_class}. Job: {job}, Step: {step}."
            )
            log.warning(f"Attempt {attempt} failed for {job}/{step}")

        _write_escalation(job, step, error_log, error_class, context)
        log.error(f"Failed after {MAX_RETRIES} attempts for {job}/{step}. Escalation written.")
        return False
    finally:
        _fix_lock.release()


def _write_escalation(job, step, error_log, error_class, context):
    escalation_dir = WIN_PROJECT_DIR / "escalations"
    try:
        escalation_dir.mkdir(exist_ok=True)
        ts = datetime.now().strftime("%Y%m%d_%H%M%S")
        report = {
            "timestamp": datetime.now().isoformat(),
            "job": job,
            "step": step,
            "error_class": error_class,
            "context": context or {},
            "max_retries_exhausted": MAX_RETRIES,
            "error_log_preview": error_log[:3000],
            "recommendation": _escalation_recommendation(error_class, job),
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
    }
    return recs.get(error_class, f"Auto-fix exhausted {MAX_RETRIES} attempts for {job}. Manual investigation required.")


def fix_sonar_issues(sonar_report, run_url):
    if not _sonar_fix_lock.acquire(blocking=False):
        log.warning("Another SonarQube fix is already running -- skipping")
        return False

    try:
        if _is_autofix_commit():
            log.warning("Last commit is an auto-fix -- skipping SonarQube fix to prevent loop")
            return False

        previous_attempt_result = ""

        for attempt in range(1, MAX_RETRIES + 1):
            log.info(f"=== SonarQube issue fix attempt {attempt}/{MAX_RETRIES} ===")

            retry_section = ""
            if previous_attempt_result:
                retry_section = (
                    f"\n\nPREVIOUS ATTEMPT FAILED (attempt {attempt - 1}):\n"
                    f"{previous_attempt_result}\n"
                    "Try a different approach. Focus on the issues that were NOT fixed in the previous attempt."
                )

            prompt = (
                "You are a code quality specialist working on a Rust/WASM real estate management project "
                "with a Kotlin Android module.\n\n"
                f"SonarQube issue report:\n```\n{sonar_report}\n```\n\n"
                "METHODOLOGY:\n"
                "1. Parse the report: each line has severity | file:line | rule | message.\n"
                "2. Group issues by file to minimize context switches.\n"
                "3. Fix BLOCKER and HIGH severity first, then MEDIUM.\n"
                "4. For each issue:\n"
                "   a. Open the file at the specified line.\n"
                "   b. Read the surrounding context (10 lines above and below).\n"
                "   c. Understand the rule violation (the rule ID tells you what's wrong).\n"
                "   d. Apply the minimal fix that resolves the violation.\n"
                "5. For security hotspots: evaluate if the code is actually vulnerable. "
                "If safe, note it. If vulnerable, fix it.\n"
                "6. After all fixes:\n"
                "   - For Rust files: run 'cargo fmt', 'cargo clippy --locked -- -D warnings', "
                "and 'cargo test --locked' to verify.\n"
                "   - For Kotlin files: run './gradlew spotlessApply detekt testDebugUnitTest' in android/.\n"
                "7. If tests fail after your changes, fix the test failures before proceeding.\n"
                "8. Stage, commit, and push:\n"
                "   git add -A && git commit -m 'fix: resolve SonarQube issues (auto-fix)' && git push origin main\n"
                "   If push fails: git pull --rebase origin main && git push origin main\n"
                "9. Do NOT stop until all reported issues are addressed.\n"
                f"{retry_section}"
                f"{SCOPE_CONSTRAINTS}\n\n"
                f"Run URL for context: {run_url}"
            )

            success = run_kiro(prompt, f"SonarQube issue fix attempt {attempt}")

            if success:
                log.info(f"SonarQube fix attempt {attempt} completed")
                return True

            previous_attempt_result = (
                f"kiro-cli returned failure for SonarQube fix attempt {attempt}. "
                "Some issues may remain unfixed or the verification step failed."
            )
            log.warning(f"SonarQube fix attempt {attempt} failed")

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

            prompt = (
                "You are a CI/CD pipeline engineer for a Rust/WASM real estate management project "
                "with an Android module.\n\n"
                f"Focus area: {focus}\n\n"
                f"--- PIPELINE REPORT ---\n{pipeline_report}\n--- END PIPELINE REPORT ---\n"
                f"{sonar_section}\n"
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
                "4. Stage, commit, and push:\n"
                f"   git add -A && git commit -m 'ci: improve pipeline ({focus})' && git push origin main\n"
                "   If push fails: git pull --rebase origin main && git push origin main\n"
                "5. Do NOT stop until you are confident the pipeline is meaningfully better.\n\n"
                f"Run URL: {run_url}"
            )

            if run_kiro(prompt, f"Pipeline improve ({focus}) attempt {attempt}"):
                log.info(f"Pipeline improvement attempt {attempt} completed")
                return True

            log.warning(f"Pipeline improvement attempt {attempt} failed")

        log.error(f"Pipeline improvement failed after {MAX_RETRIES} attempts.")
        return False
    finally:
        _improve_lock.release()
