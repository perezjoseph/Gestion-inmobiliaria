#!/usr/bin/env python3
"""
Quality Check Webhook Listener
Receives webhooks from SonarQube and GitHub Actions CI pipeline,
then triggers kiro-cli to auto-fix issues.

Endpoints:
    GET  /health       — Health check
    POST /sonarqube    — SonarQube quality gate webhook
    POST /ci-failure   — GitHub Actions pipeline failure webhook
    POST /ci-improve   — CI/CD pipeline self-improvement webhook
    POST /sonar-fix    — SonarQube open issue resolution webhook

Usage:
    python scripts/quality-webhook-listener.py
"""

import hashlib
import hmac
import ipaddress
import json
import logging
import os
import pathlib
import re
import socket
import subprocess
import threading
import time
import uuid
from collections import defaultdict
from http.server import HTTPServer, BaseHTTPRequestHandler
from datetime import datetime

_env_path = pathlib.Path(__file__).resolve().parent.parent / ".env"
if _env_path.is_file():
    with open(_env_path) as _f:
        for _line in _f:
            _line = _line.strip()
            if _line and not _line.startswith("#") and "=" in _line:
                _k, _, _v = _line.partition("=")
                os.environ.setdefault(_k.strip(), _v.strip())

# =============================================================================
# NETWORK BINDING — DO NOT CHANGE
# =============================================================================
# This listener MUST bind to 0.0.0.0 to accept webhooks from GitHub Actions
# runners on the local network 192.168.88.0/24.
#
# Security is enforced by THREE layers — all required:
#   1. _ALLOWED_NETWORKS — restricts bind address to loopback or LAN
#   2. _is_client_allowed() — rejects requests from IPs outside the LAN
#   3. HMAC signature verification — validates every POST payload
#
# DO NOT revert BIND_ADDRESS to 127.0.0.1 or localhost.
# DO NOT remove 192.168.88.0/24 from _ALLOWED_NETWORKS.
# DO NOT restrict the default to localhost-only.
# If you have a security concern, raise it — do not silently change this.
# =============================================================================
PORT = 9090
BIND_ADDRESS = os.environ.get("BIND_ADDRESS", "0.0.0.0")

_ALLOWED_NETWORKS = [
    ipaddress.ip_network("127.0.0.0/8"),
    ipaddress.ip_network("::1/128"),
    ipaddress.ip_network("192.168.88.0/24"),
]

def _is_bind_allowed(addr: str) -> bool:
    """Allow loopback, 'localhost', '0.0.0.0' (all interfaces), and the local LAN."""
    if addr in ("localhost", "0.0.0.0", "::"):
        return True
    try:
        ip = ipaddress.ip_address(addr)
    except ValueError:
        return False
    return any(ip in net for net in _ALLOWED_NETWORKS)

if not _is_bind_allowed(BIND_ADDRESS):
    raise SystemExit(
        f"BIND_ADDRESS={BIND_ADDRESS!r} is not allowed. "
        f"Must be loopback, 0.0.0.0, or within {_ALLOWED_NETWORKS}."
    )

_ALLOWED_CLIENTS = _ALLOWED_NETWORKS

def _is_client_allowed(addr: str) -> bool:
    """Reject requests from outside loopback and the local LAN."""
    try:
        ip = ipaddress.ip_address(addr)
    except ValueError:
        return False
    return any(ip in net for net in _ALLOWED_CLIENTS)
_win_project_dir = pathlib.Path(__file__).resolve().parent.parent


def _to_wsl_path(win_path: pathlib.Path) -> str:
    """Convert a Windows path to a WSL-compatible /mnt/ path."""
    drive = win_path.drive  # e.g. "D:"
    if drive and len(drive) == 2 and drive[1] == ":":
        rest = win_path.as_posix()[2:]  # strip "D:" prefix, use forward slashes
        return f"/mnt/{drive[0].lower()}{rest}"
    return str(win_path)


PROJECT_DIR = _to_wsl_path(_win_project_dir)
WSL_DISTRO = "Ubuntu-22.04"
WSL_USER = "jperez"
MAX_RETRIES = 3
KIRO_TIMEOUT = 3600
MAX_PAYLOAD_BYTES = 512 * 1024  # 512 KB
MAX_FIELD_LENGTH = 50_000
WEBHOOK_SECRET = os.environ.get("WEBHOOK_SECRET", "")

KIRO_CLI = f"/home/{WSL_USER}/.local/bin/kiro-cli"
MAX_LOG_BYTES = 50 * 1024 * 1024  # 50 MB

_SAFE_NAME_RE = re.compile(r"^[a-zA-Z0-9_\-]{1,64}$")
_SAFE_URL_RE = re.compile(r"^https://github\.com/[a-zA-Z0-9._\-]+/[a-zA-Z0-9._\-]+/actions/runs/\d+$")

_fix_lock = threading.Lock()
_sonar_fix_lock = threading.Lock()
_improve_lock = threading.Lock()
_thread_semaphore = threading.Semaphore(4)

_FIX_HISTORY_FILE = _win_project_dir / ".fix-history.json"
_FIX_HISTORY_MAX_ENTRIES = 200
_fix_history_lock = threading.Lock()


def _error_hash(job, error_log):
    """Stable hash for deduplicating similar errors."""
    normalized = re.sub(r"\d+", "N", error_log[:2000]).strip().lower()
    return hashlib.sha256(f"{job}:{normalized}".encode()).hexdigest()[:16]


def _load_fix_history():
    """Load fix history from disk. Returns dict keyed by error hash."""
    try:
        if _FIX_HISTORY_FILE.is_file():
            data = json.loads(_FIX_HISTORY_FILE.read_text(encoding="utf-8"))
            if isinstance(data, dict):
                return data
    except (json.JSONDecodeError, OSError) as e:
        log.warning(f"Fix history load failed: {e}")
    return {}


def _save_fix_history(history):
    """Persist fix history to disk, pruning old entries."""
    if len(history) > _FIX_HISTORY_MAX_ENTRIES:
        sorted_keys = sorted(
            history.keys(),
            key=lambda k: history[k].get("last_seen", ""),
        )
        for k in sorted_keys[: len(history) - _FIX_HISTORY_MAX_ENTRIES]:
            del history[k]
    try:
        _FIX_HISTORY_FILE.write_text(
            json.dumps(history, indent=2, ensure_ascii=False),
            encoding="utf-8",
            newline="\n",
        )
    except OSError as e:
        log.warning(f"Fix history save failed: {e}")


def _record_fix_attempt(job, error_log, attempt, success, strategy_notes=""):
    """Record a fix attempt in the history for future reference."""
    h = _error_hash(job, error_log)
    now = datetime.now().isoformat()
    with _fix_history_lock:
        history = _load_fix_history()
        entry = history.get(h, {
            "job": job,
            "first_seen": now,
            "attempts": [],
            "total_attempts": 0,
            "successes": 0,
        })
        entry["last_seen"] = now
        entry["total_attempts"] = entry.get("total_attempts", 0) + 1
        if success:
            entry["successes"] = entry.get("successes", 0) + 1
        entry["attempts"] = entry.get("attempts", [])[-9:] + [{
            "ts": now,
            "attempt": attempt,
            "success": success,
            "notes": strategy_notes[:500],
        }]
        history[h] = entry
        _save_fix_history(history)


def _get_past_attempts(job, error_log):
    """Retrieve past fix attempts for a similar error."""
    h = _error_hash(job, error_log)
    with _fix_history_lock:
        history = _load_fix_history()
        entry = history.get(h)
    if not entry:
        return ""
    total = entry.get("total_attempts", 0)
    successes = entry.get("successes", 0)
    if total == 0:
        return ""
    recent = entry.get("attempts", [])[-3:]
    lines = [f"FIX HISTORY: {total} previous attempts for this error pattern, {successes} succeeded."]
    for a in recent:
        status = "succeeded" if a.get("success") else "failed"
        lines.append(f"  - {a.get('ts', '?')}: attempt {a.get('attempt', '?')} {status}")
        if a.get("notes"):
            lines.append(f"    Notes: {a['notes']}")
    if total > 0 and successes == 0:
        lines.append("  WARNING: No previous attempt has succeeded. Try a fundamentally different approach.")
    return "\n".join(lines)


def _is_autofix_commit():
    """Check if the most recent commit on main is an auto-fix to prevent loops."""
    try:
        result = _wsl_bash(
            "git log -1 --format='%s' HEAD 2>/dev/null",
            timeout=15,
        )
        if result.returncode == 0:
            msg = result.stdout.strip()
            autofix_markers = ["(auto-fix)", "resolve CI failures (auto-fix)",
                               "resolve SonarQube issues (auto-fix)",
                               "improve pipeline"]
            return any(marker in msg for marker in autofix_markers)
    except Exception as e:
        log.warning(f"Auto-fix commit check failed: {e}")
    return False


_SCOPE_CONSTRAINTS = (
    "\n\nSCOPE CONSTRAINTS (mandatory):\n"
    "- Modify at most 10 files and 300 lines total.\n"
    "- Do NOT refactor unrelated code. Fix only what the error requires.\n"
    "- Do NOT add new crate dependencies unless absolutely necessary for the fix.\n"
    "- Do NOT modify CI pipeline YAML files (.github/workflows/) unless the job is specifically about pipeline config.\n"
    "- Do NOT modify scripts/quality-webhook-listener.py.\n"
    "- Prefer minimal, targeted edits over broad changes.\n"
)


class RateLimiter:
    """Sliding window rate limiter per IP address."""

    def __init__(self, max_requests=30, window_seconds=60):
        self.max_requests = max_requests
        self.window = window_seconds
        self._requests = defaultdict(list)
        self._lock = threading.Lock()

    def allow(self, ip):
        now = time.monotonic()
        with self._lock:
            timestamps = self._requests[ip]
            self._requests[ip] = [t for t in timestamps if now - t < self.window]
            if len(self._requests[ip]) >= self.max_requests:
                return False
            self._requests[ip].append(now)
            return True


_rate_limiter = RateLimiter(max_requests=30, window_seconds=60)


def _sanitize_text(value, max_len=MAX_FIELD_LENGTH):
    """Strip null bytes and control chars (except newline/tab), truncate to max_len."""
    if not isinstance(value, str):
        return ""
    value = value.replace("\x00", "")
    value = re.sub(r"[\x01-\x08\x0b\x0c\x0e-\x1f\x7f]", "", value)
    return value[:max_len]


def _validate_name(value):
    """Return value if it matches safe name pattern, else 'unknown'."""
    if isinstance(value, str) and _SAFE_NAME_RE.match(value):
        return value
    return "unknown"


def _validate_url(value):
    """Return value if it matches expected GitHub Actions run URL, else empty string."""
    if isinstance(value, str) and _SAFE_URL_RE.match(value):
        return value[:256]
    return ""

_STRUCTURED_LOGGING = os.environ.get("LOG_FORMAT", "text").lower() == "json"


class _JsonFormatter(logging.Formatter):
    """Emit log records as single-line JSON for structured log ingestion."""

    def format(self, record):
        entry = {
            "ts": self.formatTime(record, self.datefmt),
            "level": record.levelname,
            "msg": record.getMessage(),
            "logger": record.name,
        }
        if record.exc_info and record.exc_info[0]:
            entry["exception"] = self.formatException(record.exc_info)
        return json.dumps(entry, ensure_ascii=False)


_handler = logging.StreamHandler()
if _STRUCTURED_LOGGING:
    _handler.setFormatter(_JsonFormatter(datefmt="%Y-%m-%dT%H:%M:%S"))
else:
    _handler.setFormatter(logging.Formatter(
        "%(asctime)s [%(levelname)s] %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    ))

logging.basicConfig(
    level=logging.DEBUG,
    handlers=[_handler],
)
log = logging.getLogger("webhook")


def get_local_ip():
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.settimeout(2)
        sock.connect(("8.8.8.8", 80))
        ip = sock.getsockname()[0]
        sock.close()
        return ip
    except Exception:
        return "127.0.0.1"


def _wsl_bash(bash_command, timeout=60):
    """Run a bash command inside WSL via a temp script file.

    Using ``bash -l <script>`` instead of ``bash -lc <string>`` because
    wsl.exe re-interprets ``$()``, backticks, and other shell metacharacters
    in the ``-c`` argument before bash sees them.  A script file is parsed
    by bash alone, so command substitutions work correctly.
    """
    uid = uuid.uuid4().hex[:8]
    script_file = _win_project_dir / f".kiro-wsl-cmd-{uid}.sh"
    script_file_wsl = _to_wsl_path(script_file)
    try:
        script_file.write_text(
            f"#!/bin/bash\ncd {PROJECT_DIR} && {bash_command}\n",
            encoding="utf-8",
            newline="\n",
        )
        cmd = [
            "wsl", "-u", WSL_USER, "-d", WSL_DISTRO,
            "bash", "-l", script_file_wsl,
        ]
        return subprocess.run(
            cmd,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
        )
    finally:
        script_file.unlink(missing_ok=True)


def run_kiro(prompt, label):
    log.info(f"Triggering kiro-cli for: {label}")

    if not prompt or not prompt.strip():
        log.error(f"Empty prompt for {label} — skipping")
        return False

    log_file = os.path.join(os.path.dirname(__file__), "..", "kiro-debug.log")

    try:
        if os.path.isfile(log_file) and os.path.getsize(log_file) > MAX_LOG_BYTES:
            rotated = log_file + ".1"
            if os.path.isfile(rotated):
                os.remove(rotated)
            os.rename(log_file, rotated)
            log.info(f"Rotated {log_file} ({MAX_LOG_BYTES // (1024*1024)}MB limit)")
    except OSError as e:
        log.warning(f"Log rotation failed: {e}")

    # Write prompt to a file in the project directory (accessible from both
    # Windows and WSL). bash reads it with cat into a variable, then passes
    # the variable as kiro-cli's positional argument. The variable expansion
    # in "$KIRO_PROMPT" is safe — bash does not re-interpret variable values.
    uid = uuid.uuid4().hex[:8]
    prompt_file_win = _win_project_dir / f".kiro-prompt-{uid}.txt"
    prompt_file_wsl = _to_wsl_path(prompt_file_win)
    try:
        prompt_file_win.write_text(prompt, encoding="utf-8", newline="\n")
    except OSError as e:
        log.error(f"Failed to write prompt file {prompt_file_win}: {e}")
        return False

    bash_cmd = (
        f"KIRO_PROMPT=$(cat '{prompt_file_wsl}') && "
        f"[ -n \"$KIRO_PROMPT\" ] && "
        f"{KIRO_CLI} chat --trust-all-tools --agent sisyphus-kiro --no-interactive -- \"$KIRO_PROMPT\""
    )

    flags = os.O_WRONLY | os.O_CREAT | os.O_APPEND
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    fd = os.open(log_file, flags, 0o600)
    with os.fdopen(fd, "a", encoding="utf-8") as f:
        f.write(f"\n{'=' * 80}\n")
        f.write(f"[{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}] {label}\n")
        f.write(f"{'=' * 80}\n\n")
        f.flush()

        timed_out = False
        try:
            result = _wsl_bash(bash_cmd, timeout=KIRO_TIMEOUT)
            full_output = (result.stdout or "") + (result.stderr or "")
            f.write(full_output)
            f.flush()
        except subprocess.TimeoutExpired as e:
            timed_out = True
            full_output = (e.stdout or "") + (e.stderr or "") if hasattr(e, "stdout") else ""
            f.write(full_output)
            f.write("\n[TIMEOUT] Process killed\n")
            f.flush()
            log.warning(f"kiro-cli timed out after {KIRO_TIMEOUT // 60} minutes")
        except Exception as e:
            full_output = ""
            f.write(f"\n[ERROR] {e}\n")
            f.flush()
            log.error(f"kiro-cli process error: {e}", exc_info=True)
        finally:
            try:
                prompt_file_win.unlink(missing_ok=True)
            except OSError:
                pass

    if timed_out:
        return False

    log.info(f"kiro-cli exit code: {result.returncode}")
    if full_output:
        log.info(f"stdout (last 500 chars): ...{full_output[-500:]}")

    if "Tool approval required" in full_output or "denied list" in full_output:
        log.error("Tool approval/denied error detected")
        return False

    if result.returncode not in (0, 1):
        log.error(f"kiro-cli exited with unexpected code {result.returncode}")
        return False

    log.info(f"kiro-cli completed successfully for: {label}")
    return True


_RUNNER_ENV_PATTERNS = [
    "command not found",
    "No such file or directory",
    "Permission denied",
    "disk space",
    "No space left on device",
    "unable to access",
    "Could not resolve host",
    "Connection refused",
    "timeout",
    "ENOSPC",
    "cannot allocate memory",
    "docker daemon is not running",
    "Cannot connect to the Docker daemon",
]

_DEPENDENCY_PATTERNS = [
    "failed to download",
    "failed to fetch",
    "could not compile",
    "unresolved import",
    "no matching package",
    "version solving failed",
    "incompatible",
    "RUSTSEC-",
    "GHSA-",
    "CVE-",
    "CVSS",
    "vulnerability",
    "yanked",
]


def _classify_error(error_log):
    """Classify an error log into a category to guide the fix prompt."""
    lower = error_log.lower()
    for pattern in _RUNNER_ENV_PATTERNS:
        if pattern.lower() in lower:
            return "runner_environment"
    for pattern in _DEPENDENCY_PATTERNS:
        if pattern.lower() in lower:
            return "dependency"
    if "test" in lower and ("failed" in lower or "FAILED" in lower):
        return "test_failure"
    if "error[E" in error_log or "clippy" in lower or "fmt" in lower:
        return "code_quality"
    return "unknown"


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

        error_class = _classify_error(error_log)
        log.info(f"Error classified as: {error_class} for {job}/{step}")

        past_attempts_section = _get_past_attempts(job, error_log)
        previous_attempt_result = ""

        for attempt in range(1, MAX_RETRIES + 1):
            log.info(f"=== Attempt {attempt}/{MAX_RETRIES} for {job}/{step}{ctx} (class: {error_class}) ===")

            job_instructions = {
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
            instruction = job_instructions.get(job, "Analyze the error and fix the issue.")

            runner_env_note = ""
            if error_class == "runner_environment":
                runner_env_note = (
                    "\n\nIMPORTANT: This error appears to be a RUNNER ENVIRONMENT issue "
                    "(missing tool, network problem, disk space, or Docker issue). "
                    "These cannot be fixed by changing application code. "
                    "If you confirm this is a runner/infra issue, do NOT make code changes. "
                    "Instead, document the issue and suggest what the runner admin should fix. "
                    "Only make code changes if the error is actually caused by the code."
                )

            dependency_note = ""
            if error_class == "dependency":
                dependency_note = (
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
                )

            flaky_note = ""
            if error_class == "test_failure":
                flaky_note = (
                    "\n\nFLAKY TEST CHECK (do this FIRST):\n"
                    "Before making any code changes, re-run the failing test 2 times. "
                    "If it passes on re-run, the test is flaky -- do NOT change application code. "
                    "Instead, note which test is flaky and move on. "
                    "Only fix the code if the test fails consistently on every re-run."
                )

            history_section = ""
            if past_attempts_section:
                history_section = f"\n\n{past_attempts_section}"

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
                f"{runner_env_note}"
                f"{dependency_note}"
                f"{flaky_note}"
                f"{history_section}"
                f"{retry_section}"
                f"{_SCOPE_CONSTRAINTS}\n\n"
                "After fixing, run only the targeted verification command for this job. "
                "Then commit and push:\n"
                "  git add -A && git commit -m 'fix: resolve CI failures (auto-fix)' && git push origin main\n"
                "  If push fails: git pull --rebase origin main && git push origin main"
            )

            success = run_kiro(prompt, f"CI fix ({job}/{step}) attempt {attempt}")
            _record_fix_attempt(job, error_log, attempt, success,
                                f"class={error_class}, job={job}, step={step}")

            if success:
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
    """Write a structured escalation report when all retries are exhausted."""
    escalation_dir = _win_project_dir / "escalations"
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
    """Generate a human-readable recommendation for the escalation."""
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
                f"{_SCOPE_CONSTRAINTS}\n\n"
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


class TimeoutHTTPServer(HTTPServer):
    """HTTPServer with per-connection socket timeout (slow-loris protection)."""

    def get_request(self):
        conn, addr = super().get_request()
        conn.settimeout(30)
        return conn, addr


class WebhookHandler(BaseHTTPRequestHandler):
    def _send(self, code, body=b"", content_type="text/plain", request_id=None):
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("X-Frame-Options", "DENY")
        self.send_header("Content-Security-Policy", "default-src 'none'")
        self.send_header("Referrer-Policy", "no-referrer")
        self.send_header("Cache-Control", "no-store")
        self.send_header("Strict-Transport-Security", "max-age=63072000; includeSubDomains")
        self.send_header("Connection", "close")
        if request_id:
            self.send_header("X-Request-Id", request_id)
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        client_ip = self.client_address[0]
        if not _is_client_allowed(client_ip):
            log.warning(f"Rejected GET from untrusted IP {client_ip}")
            self._send(403, b"Forbidden")
            return
        log.info(f"GET {self.path} from {client_ip}")

        if not _rate_limiter.allow(client_ip):
            log.warning(f"Rate limit exceeded for {client_ip}")
            self.send_response(429)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Retry-After", "60")
            self.send_header("Connection", "close")
            self.end_headers()
            self.wfile.write(b"Too many requests")
            return

        if self.path == "/health":
            self._send(200, json.dumps({
                "status": "ok",
                "timestamp": datetime.now().isoformat(),
                "locks": {
                    "fix": _fix_lock.locked(),
                    "sonar_fix": _sonar_fix_lock.locked(),
                    "improve": _improve_lock.locked(),
                },
            }).encode(), "application/json")
            return
        self._send(404, b"Not found")

    def do_POST(self):
        client_ip = self.client_address[0]
        request_id = uuid.uuid4().hex[:12]
        if not _is_client_allowed(client_ip):
            log.warning(f"[{request_id}] Rejected POST from untrusted IP {client_ip}")
            self._send(403, b"Forbidden", request_id=request_id)
            return
        content_length_raw = self.headers.get("Content-Length", "")
        if not content_length_raw.isdigit() or int(content_length_raw) < 1:
            log.warning(f"[{request_id}] Invalid Content-Length from {client_ip}: {content_length_raw!r}")
            self._send(400, b"Invalid Content-Length", request_id=request_id)
            return
        content_length = int(content_length_raw)
        log.info(f"[{request_id}] POST {self.path} from {client_ip} ({content_length} bytes)")

        if not _rate_limiter.allow(client_ip):
            log.warning(f"[{request_id}] Rate limit exceeded for {client_ip}")
            self._send(429, b"Too many requests", request_id=request_id)
            return

        if content_length > MAX_PAYLOAD_BYTES:
            log.warning(f"[{request_id}] Payload too large: {content_length} bytes (max {MAX_PAYLOAD_BYTES})")
            self._send(413, b"Payload too large", request_id=request_id)
            return

        content_type = self.headers.get("Content-Type", "")
        if "json" not in content_type:
            log.warning(f"[{request_id}] Rejected non-JSON Content-Type: {content_type}")
            self._send(400, b"Content-Type must be application/json", request_id=request_id)
            return

        body = self.rfile.read(content_length)

        if WEBHOOK_SECRET:
            if self.path == "/sonarqube":
                sig_header = self.headers.get("X-Sonar-Webhook-HMAC-SHA256", "")
                expected = hmac.new(
                    WEBHOOK_SECRET.encode(), body, hashlib.sha256
                ).hexdigest()
            else:
                sig_header = self.headers.get("X-Signature-256", "")
                expected = "sha256=" + hmac.new(
                    WEBHOOK_SECRET.encode(), body, hashlib.sha256
                ).hexdigest()
            if not hmac.compare_digest(expected, sig_header):
                log.warning(f"[{request_id}] Invalid HMAC signature on {self.path} from {client_ip}")
                self._send(401, b"Invalid signature", request_id=request_id)
                return

        try:
            payload = json.loads(body)
        except json.JSONDecodeError as e:
            log.error(f"[{request_id}] Invalid JSON: {e}")
            self._send(400, b"Invalid JSON", request_id=request_id)
            return

        handlers = {
            "/sonarqube": self._handle_sonarqube,
            "/ci-failure": self._handle_ci_failure,
            "/ci-improve": self._handle_ci_improve,
            "/sonar-fix": self._handle_sonar_fix,
        }

        handler = handlers.get(self.path)
        if handler:
            self._send(200, b"OK", request_id=request_id)

            def _guarded(fn, p, rid):
                if not _thread_semaphore.acquire(blocking=False):
                    log.warning(f"[{rid}] Thread pool exhausted -- dropping {self.path}")
                    return
                try:
                    fn(p)
                except Exception:
                    log.exception(f"[{rid}] Handler error for {self.path}")
                finally:
                    _thread_semaphore.release()

            threading.Thread(target=_guarded, args=(handler, payload, request_id), daemon=True).start()
        else:
            log.warning(f"[{request_id}] Unknown endpoint: {self.path}")
            self._send(404, b"Not found", request_id=request_id)

    def _handle_sonarqube(self, payload):
        project = _sanitize_text(payload.get("project", {}).get("key", "unknown"), 128)
        gate_status = _validate_name(payload.get("qualityGate", {}).get("status", "unknown"))

        log.info(f"SonarQube webhook: project={project}, status={gate_status}")

        if gate_status == "OK":
            log.info("Quality gate passed. No action needed.")
            return

        conditions = payload.get("qualityGate", {}).get("conditions", [])
        failed = [c for c in conditions if c.get("status") == "ERROR"]
        if not failed:
            log.info("No failed conditions found.")
            return

        failures_summary = ", ".join(
            f"{_sanitize_text(c.get('metric', '?'), 64)}={_sanitize_text(str(c.get('value', '?')), 32)} (threshold: {_sanitize_text(str(c.get('errorThreshold', '?')), 32)})"
            for c in failed
        )
        log.info(f"Failed conditions: {failures_summary}")

        prompt = (
            f"The SonarQube quality gate FAILED for project '{project}'. "
            f"Failed conditions: {failures_summary}. "
            "Fetch the specific issues, fix them, verify with cargo fmt, clippy, and test. "
            "Then commit and push: git add -A && git commit -m 'fix: resolve SonarQube failures (auto-fix)' && git push origin main"
        )

        for attempt in range(1, MAX_RETRIES + 1):
            log.info(f"=== SonarQube fix attempt {attempt}/{MAX_RETRIES} ===")
            if run_kiro(prompt, f"SonarQube fix attempt {attempt}"):
                return

        log.error(f"Failed after {MAX_RETRIES} attempts.")

    def _handle_ci_failure(self, payload):
        job = _validate_name(payload.get("job", "unknown"))
        step = _validate_name(payload.get("step", "unknown"))
        error_log = _sanitize_text(payload.get("error_log", "No error details provided"))
        context = {
            "commit": _sanitize_text(payload.get("commit", ""), 64),
            "branch": _sanitize_text(payload.get("branch", ""), 128),
            "actor": _sanitize_text(payload.get("actor", ""), 64),
        }

        log.info(f"CI failure webhook: job={job}, step={step}, commit={context['commit'][:8]}, branch={context['branch']}")
        fix_with_retry(job, step, error_log, context)

    def _handle_ci_improve(self, payload):
        raw_focus = _sanitize_text(payload.get("focus", "general"), 128)
        focus = ",".join(_validate_name(f.strip()) for f in raw_focus.split(",") if f.strip())
        if not focus:
            focus = "general"
        pipeline_report = _sanitize_text(payload.get("pipeline_report", ""))
        sonar_report = _sanitize_text(payload.get("sonar_report", ""))
        run_url = _validate_url(payload.get("run_url", ""))

        log.info(f"CI improve webhook: focus={focus}")
        improve_pipeline(focus, pipeline_report, run_url, sonar_report)

    def _handle_sonar_fix(self, payload):
        sonar_report = _sanitize_text(payload.get("sonar_report", ""))
        run_url = _validate_url(payload.get("run_url", ""))

        log.info("SonarQube fix webhook received")
        fix_sonar_issues(sonar_report, run_url)

    def _reject_method(self):
        self._send(405, b"Method not allowed")

    do_PUT = do_DELETE = do_PATCH = do_HEAD = do_OPTIONS = do_TRACE = do_CONNECT = _reject_method

    def log_message(self, format, *args):
        pass


def main():
    if not WEBHOOK_SECRET:
        log.error("WEBHOOK_SECRET environment variable is required. Set it before starting the listener.")
        raise SystemExit(1)

    if not _win_project_dir.is_dir():
        log.error(f"PROJECT_DIR does not exist or is not a directory: {_win_project_dir}")
        raise SystemExit(1)

    local_ip = get_local_ip()

    log.info("Running startup checks...")
    try:
        result = _wsl_bash(f"{KIRO_CLI} --version", timeout=30)
        if result.returncode != 0:
            log.error(f"kiro-cli check failed (exit {result.returncode}): {result.stderr.strip()}")
            raise SystemExit(1)
        log.info(f"  kiro-cli: {result.stdout.strip()}")
    except FileNotFoundError:
        log.error("'wsl' command not found — this script must run on Windows")
        raise SystemExit(1)
    except subprocess.TimeoutExpired:
        log.error("WSL timed out — is the distro running?")
        raise SystemExit(1)

    result = _wsl_bash("test -d .", timeout=10)
    if result.returncode != 0:
        log.error(f"Project dir not accessible from WSL: {PROJECT_DIR}")
        raise SystemExit(1)
    log.info(f"  Project dir: {PROJECT_DIR} ✓")

    result = _wsl_bash(f"{KIRO_CLI} chat --help", timeout=30)
    if result.returncode != 0:
        log.error(f"kiro-cli chat --help failed (exit {result.returncode}): {result.stderr.strip()}")
        raise SystemExit(1)
    log.info("  kiro-cli chat args: ✓")

    test_prompt = (
        "--- TEST ---\n"
        "Shell metacharacters: `backticks` $(command) $HOME\n"
        "Parens: (test) and dashes: --- END ---\n"
    )
    test_file = _win_project_dir / ".kiro-prompt-init-test.txt"
    test_file_wsl = _to_wsl_path(test_file)
    try:
        test_file.write_text(test_prompt, encoding="utf-8", newline="\n")
        result = _wsl_bash(
            f"KIRO_PROMPT=$(cat '{test_file_wsl}') && "
            f"[ -n \"$KIRO_PROMPT\" ] && "
            f"{KIRO_CLI} chat --no-interactive -- \"$KIRO_PROMPT\"",
            timeout=120,
        )
    finally:
        test_file.unlink(missing_ok=True)

    if result.returncode not in (0, 1):
        log.error(
            f"kiro-cli prompt delivery test failed (exit {result.returncode}).\n"
            f"  stderr: {result.stderr.strip()[:500]}\n"
            f"  This means the prompt file is not being delivered correctly."
        )
        raise SystemExit(1)
    log.info("  kiro-cli prompt delivery: ✓")

    log.info("Startup checks passed")

    server = TimeoutHTTPServer((BIND_ADDRESS, PORT), WebhookHandler)
    server.timeout = 30
    server.request_queue_size = 8
    log.info(f"Quality webhook listener running on {BIND_ADDRESS}:{PORT}")
    log.info(f"Max retries: {MAX_RETRIES} | Timeout: {KIRO_TIMEOUT // 60}min | Max payload: {MAX_PAYLOAD_BYTES // 1024}KB")
    log.info("Endpoints:")
    log.info(f"  Health:     http://{local_ip}:{PORT}/health")
    log.info(f"  SonarQube:  http://{local_ip}:{PORT}/sonarqube")
    log.info(f"  CI failure: http://{local_ip}:{PORT}/ci-failure")
    log.info(f"  CI improve: http://{local_ip}:{PORT}/ci-improve")
    log.info(f"  Sonar fix:  http://{local_ip}:{PORT}/sonar-fix")
    log.info(f"Project dir: {PROJECT_DIR} (Windows: {_win_project_dir})")
    log.info(f"WSL: {WSL_DISTRO} (user: {WSL_USER})")
    log.info("Waiting for webhooks...")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        log.info("Shutting down...")
        server.server_close()


if __name__ == "__main__":
    main()
