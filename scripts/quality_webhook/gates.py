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
    MAX_DIFF_FILES, MAX_DIFF_LINES, DEDUP_WINDOW_MINUTES,
    log,
)
from .runner import wsl_bash
from .history import _error_hash, _load_fix_history, _fix_history_lock


def preflight_dedup(job, error_log):
    """Reject if the same error hash failed N+ times within the dedup window."""
    h = _error_hash(job, error_log)
    with _fix_history_lock:
        history = _load_fix_history()
        entry = history.get(h)

    if not entry:
        return True, "no prior attempts"

    cutoff = datetime.now() - timedelta(minutes=DEDUP_WINDOW_MINUTES)
    recent_failures = [
        a for a in entry.get("attempts", [])
        if not a.get("success")
        and datetime.fromisoformat(a["ts"]) > cutoff
    ]

    if len(recent_failures) >= 3:
        return False, (
            f"same error hash failed {len(recent_failures)} times "
            f"in the last {DEDUP_WINDOW_MINUTES} minutes"
        )

    return True, f"{len(recent_failures)} recent failures (under threshold)"


def revision_scope_check():
    """Check that staged changes don't exceed scope limits.

    Runs git diff --stat on staged changes and enforces file/line limits.
    """
    try:
        result = wsl_bash("git diff --cached --stat 2>/dev/null || git diff HEAD --stat 2>/dev/null", timeout=15)
        if result.returncode != 0:
            return True, "git diff unavailable, skipping scope check"

        output = result.stdout.strip()
        if not output:
            return True, "no changes detected"

        files_match = re.search(r"(\d+) files? changed", output)
        insertions = re.search(r"(\d+) insertions?", output)
        deletions = re.search(r"(\d+) deletions?", output)

        file_count = int(files_match.group(1)) if files_match else 0
        ins = int(insertions.group(1)) if insertions else 0
        dels = int(deletions.group(1)) if deletions else 0
        total_lines = ins + dels

        if file_count > MAX_DIFF_FILES:
            return False, f"touched {file_count} files (max {MAX_DIFF_FILES})"

        if total_lines > MAX_DIFF_LINES:
            return False, f"changed {total_lines} lines (max {MAX_DIFF_LINES})"

        return True, f"{file_count} files, {total_lines} lines"

    except Exception as e:
        log.warning(f"Scope check failed: {e}")
        return True, f"scope check error: {e}"


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
