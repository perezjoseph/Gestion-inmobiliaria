"""
Post-fix gates inspired by GSD's gate taxonomy.

Four gate types:
  - preflight: checks before a fix attempt starts (dedup, loop detection)
  - revision: checks after a fix attempt (scope enforcement, diff validation)
  - verification: independent verification that the fix actually works
  - escalation: determines whether to retry or escalate

Gates return (passed: bool, reason: str).
"""

from datetime import datetime, timedelta

from .config import (
    log,
)
from .runner import wsl_bash
from .history import _error_hash, _load_fix_history, _fix_history_lock

# Per-error-class dedup windows and thresholds (Requirements 4.1-4.5)
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
    """Reject if the same error hash failed N+ times within the dedup window.

    Uses class-specific window and threshold from DEDUP_CONFIG.
    Falls back to "unknown" config if error_class is not recognized.
    """
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
]


def revision_check_diff():
    """Reject if the staged diff adds #[ignore], todo!(), or unimplemented!() to tests."""
    import re
    try:
        result = wsl_bash("git diff --cached -- '*.rs'", timeout=30)
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
    "test-backend": "cargo test --locked -p realestate-backend --all-targets",
    "test-frontend": "cargo test --locked -p realestate-frontend --all-targets",
    "build-frontend": "trunk build --release",
    "build-backend": "cargo build -p realestate-backend --release",
    "android-lint": "(cd android && ./gradlew lint detekt)",
    "android-unit-test": "(cd android && ./gradlew testDebugUnitTest)",
    "android-build": "(cd android && ./gradlew assembleDebug)",
    "quality-gate": "cargo audit && cargo deny check",
}


def verification_run(job):
    """Independently run the verification command for a job.

    Returns (passed, output_summary).
    """
    cmd = _VERIFY_COMMANDS.get(job)
    if not cmd:
        return True, f"no verification command for job '{job}'"

    try:
        result = wsl_bash(cmd, timeout=300)
        output = (result.stdout + result.stderr)[-2000:].strip()

        if result.returncode == 0:
            return True, "verification passed"

        return False, f"verification failed (exit {result.returncode}): {output[-500:]}"

    except Exception as e:
        log.warning(f"Verification command error for {job}: {e}")
        return False, f"verification error: {e}"
