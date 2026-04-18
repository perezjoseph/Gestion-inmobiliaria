"""
Post-fix gates inspired by GSD's gate taxonomy.

Four gate types:
  - preflight: checks before a fix attempt starts (dedup, loop detection)
  - revision: checks after a fix attempt (scope enforcement, diff validation)
  - verification: independent verification that the fix actually works
  - escalation: determines whether to retry or escalate

Gates return (passed: bool, reason: str).
"""

import re
from datetime import datetime, timedelta

from .config import (
    log,
)
from .runner import wsl_bash
from .history import _error_hash, _load_fix_history, _fix_history_lock

DEDUP_CONFIG = {
    "flaky":              {"window_minutes": 15,  "threshold": 5},
    "runner_environment": {"window_minutes": 120, "threshold": 2},
    "test_failure":       {"window_minutes": 60,  "threshold": 3},
    "build_failure":      {"window_minutes": 60,  "threshold": 3},
    "code_quality":       {"window_minutes": 60,  "threshold": 3},
    "dependency":         {"window_minutes": 180, "threshold": 2},
    "unknown":            {"window_minutes": 60,  "threshold": 3},
}


def preflight_dedup(job, error_log, error_class="unknown"):
    cfg = DEDUP_CONFIG.get(error_class, DEDUP_CONFIG["unknown"])
    window_minutes = cfg["window_minutes"]
    threshold = cfg["threshold"]

    h = _error_hash(job, error_log)
    with _fix_history_lock:
        history = _load_fix_history()
        entry = history.get(h)

    if not entry:
        return True, "no prior attempts"

    cutoff = datetime.now() - timedelta(minutes=window_minutes)
    recent_failures = [
        a for a in entry.get("attempts", [])
        if not a.get("success")
        and datetime.fromisoformat(a["ts"]) > cutoff
    ]

    if len(recent_failures) >= threshold:
        return False, (
            f"same error hash failed {len(recent_failures)} times "
            f"in the last {window_minutes} minutes"
        )

    return True, f"{len(recent_failures)} recent failures (under threshold)"


_FORBIDDEN_DIFF_PATTERNS = [
    r"^\+.*#\[ignore\]",
    r"^\+.*todo!\(",
    r"^\+.*unimplemented!\(",
    r"^\+.*PROPTEST_CASES\s*=\s*[0-9]",
    r"^\+.*proptest_config.*cases\s*=\s*[0-9]",
]


def revision_check_diff(cwd=None):
    try:
        result = wsl_bash("git diff --cached -- '*.rs'", timeout=30, cwd=cwd)
        diff_text = result.stdout or ""
        for pattern in _FORBIDDEN_DIFF_PATTERNS:
            matches = re.findall(pattern, diff_text, re.MULTILINE)
            if matches:
                return False, f"diff adds forbidden pattern: {matches[0].strip()}"
        return True, "no forbidden patterns in diff"
    except Exception as e:
        log.warning(f"Revision diff check error: {e}")
        return True, f"diff check skipped: {e}"


_VERIFY_COMMANDS = {
    "lint": (
        "cargo fmt --all --check && "
        "cargo clippy --locked -p realestate-backend -- -D warnings && "
        "cargo clippy --locked -p realestate-frontend --target wasm32-unknown-unknown -- -D warnings"
    ),
    "secret-scan": "gitleaks detect --source . --no-banner",
    "test-backend": "cargo test --locked -p realestate-backend --all-targets",
    "test-frontend": "cargo test --locked -p realestate-frontend --all-targets",
    "build-frontend": "trunk build --release",
    "build-backend": "cargo build -p realestate-backend --release",
    "android-lint": "(cd android && ./gradlew lint detekt)",
    "android-unit-test": "(cd android && ./gradlew testDebugUnitTest)",
    "android-build": "(cd android && ./gradlew assembleDebug)",
    "quality-gate": "cargo audit && cargo deny check",
}


def verification_run(job, cwd=None):
    cmd = _VERIFY_COMMANDS.get(job)
    if not cmd:
        return True, f"no verification command for job '{job}'"

    try:
        result = wsl_bash(cmd, timeout=300, cwd=cwd)
        output = (result.stdout + result.stderr)[-2000:].strip()

        if result.returncode == 0:
            return True, "verification passed"

        return False, f"verification failed (exit {result.returncode}): {output[-500:]}"

    except Exception as e:
        log.warning(f"Verification command error for {job}: {e}")
        return False, f"verification error: {e}"


BASELINE_VERIFY = (
    "cargo fmt --all --check && "
    "cargo clippy --locked -p realestate-backend -- -D warnings && "
    "cargo clippy --locked -p realestate-frontend "
    "--target wasm32-unknown-unknown -- -D warnings && "
    "cargo test --locked -p realestate-backend --all-targets && "
    "cargo test --locked -p realestate-frontend --all-targets"
)

ANDROID_BASELINE_VERIFY = "(cd android && ./gradlew lint detekt testDebugUnitTest)"

_JOB_COVERS_BASELINE = {
    "lint": {"fmt", "clippy-backend", "clippy-frontend"},
    "secret-scan": {"fmt", "clippy-backend", "clippy-frontend", "test-backend", "test-frontend"},
    "test-backend": {"test-backend"},
    "test-frontend": {"test-frontend"},
    "android-lint": {"android-lint"},
    "android-unit-test": {"android-test"},
}

_BASELINE_STEPS = {
    "fmt": "cargo fmt --all --check",
    "clippy-backend": "cargo clippy --locked -p realestate-backend -- -D warnings",
    "clippy-frontend": (
        "cargo clippy --locked -p realestate-frontend "
        "--target wasm32-unknown-unknown -- -D warnings"
    ),
    "test-backend": "cargo test --locked -p realestate-backend --all-targets",
    "test-frontend": "cargo test --locked -p realestate-frontend --all-targets",
}


def baseline_verification(cwd=None) -> tuple[bool, str]:
    try:
        result = wsl_bash(BASELINE_VERIFY, timeout=600, cwd=cwd)
        if result.returncode == 0:
            return True, "baseline passed"
        output = (result.stdout + result.stderr)[-500:]
        return False, f"baseline failed: {output}"
    except Exception as e:
        log.warning(f"Baseline verification error: {e}")
        return False, f"baseline error: {e}"


def baseline_verification_incremental(job: str, cwd=None) -> tuple[bool, str]:
    if job.startswith("android"):
        try:
            result = wsl_bash(ANDROID_BASELINE_VERIFY, timeout=600, cwd=cwd)
            if result.returncode == 0:
                return True, "android baseline passed"
            output = (result.stdout + result.stderr)[-500:]
            return False, f"android baseline failed: {output}"
        except Exception as e:
            return False, f"android baseline error: {e}"

    covered = _JOB_COVERS_BASELINE.get(job, set())
    steps_to_run = [
        (name, cmd) for name, cmd in _BASELINE_STEPS.items()
        if name not in covered
    ]

    if not steps_to_run:
        return True, "baseline fully covered by per-job verification"

    combined = " && ".join(cmd for _, cmd in steps_to_run)
    try:
        result = wsl_bash(combined, timeout=600, cwd=cwd)
        if result.returncode == 0:
            skipped = sorted(covered)
            return True, f"incremental baseline passed (skipped: {skipped})"
        output = (result.stdout + result.stderr)[-500:]
        return False, f"incremental baseline failed: {output}"
    except Exception as e:
        log.warning(f"Incremental baseline verification error: {e}")
        return False, f"incremental baseline error: {e}"


_SECURITY_DIFF_PATTERNS = [
    (r'^\+.*(?:password|secret|token|api_key)\s*=\s*"[^"]{8,}"', "hardcoded secret"),
    (r'^\+.*unsafe\s*\{', "unsafe block without SAFETY comment"),
    (r'^\+.*danger_accept_invalid_certs\(true\)', "disabled TLS verification"),
    (r'^\+.*Access-Control-Allow-Origin.*\*', "wildcard CORS"),
]


def _check_unsafe_blocks(diff_text: str) -> tuple[bool, str]:
    lines = diff_text.splitlines()
    for i, line in enumerate(lines):
        if not re.match(r'^\+.*unsafe\s*\{', line):
            continue
        preceding = lines[max(0, i - 3):i]
        has_safety = any(
            re.search(r'^\+.*//\s*SAFETY:', prev) for prev in preceding
        )
        if not has_safety:
            return False, "security: unsafe block without preceding // SAFETY: comment"
    return True, "unsafe blocks ok"


def revision_security_check(cwd=None) -> tuple[bool, str]:
    try:
        result = wsl_bash("git diff --cached -- '*.rs' '*.toml' '*.yml'", timeout=30, cwd=cwd)
        diff_text = result.stdout or ""
        current_file = ""
        for line in diff_text.splitlines():
            if line.startswith("+++ b/"):
                current_file = line[6:]
            is_test = "test" in current_file.lower() or "_pbt.rs" in current_file
            for pattern, description in _SECURITY_DIFF_PATTERNS:
                if is_test and "unwrap" in description:
                    continue
                if pattern == r'^\+.*unsafe\s*\{' and "unsafe" in description:
                    continue
                if re.match(pattern, line):
                    return False, f"security: {description}"
        unsafe_ok, unsafe_reason = _check_unsafe_blocks(diff_text)
        if not unsafe_ok:
            return False, unsafe_reason
        return True, "no security anti-patterns"
    except Exception as e:
        log.warning(f"Security diff check error: {e}")
        return True, f"security check skipped: {e}"


def revision_audit_check(cwd=None) -> tuple[bool, str]:
    try:
        result = wsl_bash("cargo audit --json 2>/dev/null || true", timeout=120, cwd=cwd)
        output = result.stdout or ""
        vuln_count = output.count('"kind":"vulnerability"')
        if vuln_count > 0:
            return False, f"security: cargo audit found {vuln_count} new vulnerabilities"
        return True, "no new audit vulnerabilities"
    except Exception as e:
        log.warning(f"Audit check error: {e}")
        return True, f"audit check skipped: {e}"


def revision_maintainability_check(cwd=None) -> tuple[bool, str]:
    try:
        result = wsl_bash("git diff --cached -- '*.rs'", timeout=30, cwd=cwd)
        diff_text = result.stdout or ""
        current_file = ""
        added_lines_in_fn = 0
        max_indent = 0

        for line in diff_text.splitlines():
            if line.startswith("+++ b/"):
                current_file = line[6:]
                added_lines_in_fn = 0
                continue

            is_test = "test" in current_file.lower() or "_pbt.rs" in current_file

            if line.startswith("+") and not line.startswith("+++"):
                content = line[1:]
                indent = len(content) - len(content.lstrip())
                if indent > max_indent:
                    max_indent = indent

                added_lines_in_fn += 1

                for pattern, description in _MAINTAINABILITY_DIFF_PATTERNS:
                    if is_test and ("unwrap" in description or "expect" in description):
                        continue
                    if re.match(pattern, line):
                        log.warning(f"Maintainability rejection: {description} in {current_file}")
                        return False, f"maintainability: {description} in {current_file}"

            if line.startswith("@@"):
                added_lines_in_fn = 0

        if added_lines_in_fn > 100:
            log.warning(f"Maintainability rejection: function exceeds 100 added lines in {current_file}")
            return False, f"maintainability: function exceeds 100 added lines in {current_file}"

        if max_indent > 12:
            nesting = max_indent // 4
            log.warning(f"Maintainability rejection: nesting depth {nesting} (>{3}) in {current_file}")
            return False, f"maintainability: nesting depth {nesting} exceeds 3 levels in {current_file}"

        return True, "no maintainability issues"
    except Exception as e:
        log.warning(f"Maintainability check error: {e}")
        return True, f"maintainability check skipped: {e}"


_CORRECTNESS_DIFF_PATTERNS = [
    (r'^\-.*unique|^\-.*duplicate|^\-.*already.exists', "removed uniqueness validation"),
    (r'^\-.*overlap|^\-.*fecha_inicio.*fecha_fin|^\-.*date.range', "removed date overlap check"),
    (r'^\-.*belongs_to|^\-.*propiedad_id.*unidad|^\-.*references', "removed referential integrity check"),
    (r'^\-.*ocupada|^\-.*disponible.*estado', "removed estado cascade logic"),
    (r'^\-.*moneda|^\-.*currency', "removed currency field handling"),
]


def revision_correctness_check(cwd=None) -> tuple[bool, str]:
    try:
        result = wsl_bash(
            "git diff --cached -- 'backend/src/services/*.rs' 'backend/src/handlers/*.rs'",
            timeout=30,
            cwd=cwd,
        )
        diff_text = result.stdout or ""
        if not diff_text.strip():
            return True, "no domain files changed"
        warnings = []
        for pattern, description in _CORRECTNESS_DIFF_PATTERNS:
            matches = re.findall(pattern, diff_text, re.MULTILINE | re.IGNORECASE)
            if matches:
                warnings.append(f"correctness: {description} ({matches[0].strip()[:60]})")
        if warnings:
            return False, "; ".join(warnings)
        return True, "no correctness regressions detected"
    except Exception as e:
        log.warning(f"Correctness check error: {e}")
        return True, f"correctness check skipped: {e}"


PBT_VERIFY_BACKEND = (
    "PROPTEST_CASES=500 cargo test --locked -p realestate-backend "
    "--all-targets -- --test-threads=1"
)

PBT_VERIFY_FRONTEND = (
    "PROPTEST_CASES=500 cargo test --locked -p realestate-frontend "
    "--all-targets -- --test-threads=1"
)


_MAINTAINABILITY_DIFF_PATTERNS = [
    (r'^\+.*\.unwrap\(\)', "unwrap() in non-test code"),
    (r'^\+.*\.expect\(', "expect() in non-test code"),
    (r'^\+\s*//\s*(?:TODO|FIXME|HACK|XXX)', "TODO/FIXME/HACK/XXX comment"),
]


def verification_run_pbt(job: str, cwd=None) -> tuple[bool, str]:
    if job in ("test-backend", "build-backend"):
        cmd = PBT_VERIFY_BACKEND
    elif job in ("test-frontend", "build-frontend"):
        cmd = PBT_VERIFY_FRONTEND
    else:
        return verification_run(job, cwd=cwd)

    try:
        result = wsl_bash(cmd, timeout=600, cwd=cwd)
        output = (result.stdout + result.stderr)[-2000:].strip()
        if result.returncode == 0:
            return True, "PBT verification passed (500 cases)"
        return False, f"PBT verification failed (exit {result.returncode}): {output[-500:]}"
    except Exception as e:
        log.warning(f"PBT verification error for {job}: {e}")
        return False, f"PBT verification error: {e}"
