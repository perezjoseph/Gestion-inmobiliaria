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

PORT = 9090
BIND_ADDRESS = os.environ.get("BIND_ADDRESS", "127.0.0.1")
_ALLOWED_BIND = {"127.0.0.1", "::1", "localhost", "0.0.0.0"}
if BIND_ADDRESS not in _ALLOWED_BIND:
    raise SystemExit(
        f"BIND_ADDRESS={BIND_ADDRESS!r} is not allowed. "
        f"Must be one of {_ALLOWED_BIND} to prevent network exposure."
    )
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

MAX_LOG_BYTES = 50 * 1024 * 1024  # 50 MB

_SAFE_NAME_RE = re.compile(r"^[a-zA-Z0-9_\-]{1,64}$")
_SAFE_URL_RE = re.compile(r"^https://github\.com/[a-zA-Z0-9._\-]+/[a-zA-Z0-9._\-]+/actions/runs/\d+$")

_fix_lock = threading.Lock()
_sonar_fix_lock = threading.Lock()
_improve_lock = threading.Lock()
_thread_semaphore = threading.Semaphore(4)


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

logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
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


def wsl_cmd(command):
    return [
        "wsl", "-u", WSL_USER, "-d", WSL_DISTRO,
        "bash", "-lc",
        f"cd {PROJECT_DIR} && {command}",
    ]


def run_kiro(prompt, label):
    log.info(f"Triggering kiro-cli for: {label}")

    cmd = "kiro-cli chat --trust-all-tools --agent sisyphus-kiro --no-interactive -"

    log_file = os.path.join(os.path.dirname(__file__), "..", "kiro-debug.log")

    # Rotate log if it exceeds MAX_LOG_BYTES to prevent disk exhaustion
    try:
        if os.path.isfile(log_file) and os.path.getsize(log_file) > MAX_LOG_BYTES:
            rotated = log_file + ".1"
            if os.path.isfile(rotated):
                os.remove(rotated)
            os.rename(log_file, rotated)
            log.info(f"Rotated {log_file} ({MAX_LOG_BYTES // (1024*1024)}MB limit)")
    except OSError as e:
        log.warning(f"Log rotation failed: {e}")

    flags = os.O_WRONLY | os.O_CREAT | os.O_APPEND
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    fd = os.open(log_file, flags, 0o600)
    with os.fdopen(fd, "a", encoding="utf-8") as f:
        f.write(f"\n{'=' * 80}\n")
        f.write(f"[{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}] {label}\n")
        f.write(f"{'=' * 80}\n\n")
        f.flush()

        proc = subprocess.Popen(
            wsl_cmd(cmd),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
        )

        try:
            proc.stdin.write(prompt)
            proc.stdin.close()
        except BrokenPipeError:
            log.warning("Broken pipe writing prompt to kiro-cli stdin")

        output_lines = []
        try:
            for line in proc.stdout:
                f.write(line)
                f.flush()
                output_lines.append(line)
            proc.wait(timeout=KIRO_TIMEOUT)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
            f.write("\n[TIMEOUT] Process killed\n")
            log.warning(f"kiro-cli timed out after {KIRO_TIMEOUT // 60} minutes")
            return False
        except Exception as e:
            proc.kill()
            f.write(f"\n[ERROR] {e}\n")
            log.error(f"kiro-cli process error: {e}", exc_info=True)
            return False

    full_output = "".join(output_lines)
    log.info(f"kiro-cli exit code: {proc.returncode}")
    if full_output:
        log.info(f"stdout (last 500 chars): ...{full_output[-500:]}")

    if "Tool approval required" in full_output or "denied list" in full_output:
        log.error("Tool approval/denied error detected")
        return False

    if proc.returncode not in (0, 1):
        log.error(f"kiro-cli exited with unexpected code {proc.returncode}")
        return False

    log.info(f"kiro-cli completed successfully for: {label}")
    return True


def fix_with_retry(job, step, error_log, context=None):
    if not _fix_lock.acquire(blocking=False):
        log.warning(f"Another fix is already running -- skipping {job}/{step}")
        return False

    try:
        ctx = ""
        if context:
            ctx = f" (commit: {context.get('commit', '?')[:8]}, branch: {context.get('branch', '?')}, actor: {context.get('actor', '?')})"

        for attempt in range(1, MAX_RETRIES + 1):
            log.info(f"=== Attempt {attempt}/{MAX_RETRIES} for {job}/{step}{ctx} ===")

            job_instructions = {
                "lint": "Fix the formatting and clippy warnings.",
                "test-backend": "Fix the failing backend tests.",
                "test-frontend": "Fix the failing frontend tests.",
                "quality-gate": "Fix the quality gate failures (dependency audit or OWASP issues).",
                "sonarqube": "Fix the SonarQube analysis failures.",
                "android-lint": "Fix the Android lint warnings in the android/ directory. Run ./gradlew lint in android/ to verify.",
                "android-unit-test": "Fix the failing Android unit tests in the android/ directory. Run ./gradlew testDebugUnitTest in android/ to verify.",
                "android-build": "Fix the Android build errors in the android/ directory. Run ./gradlew assembleDebug in android/ to verify.",
                "android-quality-gate": "Fix the Android dependency security issues. Check the OWASP report and update vulnerable dependencies in android/gradle/libs.versions.toml.",
            }
            instruction = job_instructions.get(job, "Analyze the error and fix the issue.")

            prompt = (
                f"The CI pipeline FAILED in job '{job}', step '{step}'.{ctx} "
                f"Error output:\n{error_log}\n\n"
                f"{instruction} "
                "Fix the specific issues reported. After fixing, only run targeted verification. "
                "Then commit and push: git add -A && git commit -m 'fix: resolve CI failures (auto-fix)' && git push origin main. "
                "If push fails: git pull --rebase origin main && git push origin main."
            )

            if run_kiro(prompt, f"CI fix ({job}/{step}) attempt {attempt}"):
                log.info(f"Attempt {attempt} completed for {job}/{step}")
                return True

            log.warning(f"Attempt {attempt} failed for {job}/{step}")

        log.error(f"Failed after {MAX_RETRIES} attempts for {job}/{step}. Manual intervention needed.")
        return False
    finally:
        _fix_lock.release()


def fix_sonar_issues(sonar_report, run_url):
    if not _sonar_fix_lock.acquire(blocking=False):
        log.warning("Another SonarQube fix is already running -- skipping")
        return False

    try:
        for attempt in range(1, MAX_RETRIES + 1):
            log.info(f"=== SonarQube issue fix attempt {attempt}/{MAX_RETRIES} ===")

            prompt = (
                "You are a code quality specialist. Your job is to resolve open SonarQube issues "
                "in this Rust/WASM real estate management project.\n\n"
                f"SonarQube issue report:\n{sonar_report}\n\n"
                "INSTRUCTIONS -- work autonomously through this self-improvement loop:\n"
                "1. The report above contains open SonarQube issues with severity, file, line, rule, and message. "
                "For each issue, open the file at the specified line, understand the rule violation, and fix it.\n"
                "2. Prioritize BLOCKER and HIGH severity first, then MEDIUM.\n"
                "3. Also review and fix any security hotspots listed.\n"
                "4. After fixing issues, run cargo fmt, cargo clippy, and cargo test to verify nothing is broken.\n"
                "5. If tests fail after your changes, fix the test failures before proceeding.\n"
                "6. Stage, commit, and push:\n"
                "   git add -A && git commit -m 'fix: resolve SonarQube issues (auto-fix)' && git push origin main\n"
                "   If push fails: git pull --rebase origin main && git push origin main\n"
                "7. Do NOT stop until all reported issues are addressed.\n\n"
                f"Run URL for context: {run_url}"
            )

            if run_kiro(prompt, f"SonarQube issue fix attempt {attempt}"):
                log.info(f"SonarQube fix attempt {attempt} completed")
                return True

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

            prompt = (
                "You are a CI/CD pipeline engineer. Your job is to improve the GitHub Actions pipeline "
                "at .github/workflows/ci.yml for this Rust/WASM project.\n\n"
                f"Focus area: {focus}\n\n"
                f"--- PIPELINE REPORT ---\n{pipeline_report}\n--- END PIPELINE REPORT ---\n"
                f"{sonar_section}\n"
                "INSTRUCTIONS:\n"
                "1. Read .github/workflows/ci.yml and scripts/quality-webhook-listener.py fully.\n"
                "2. The report above contains job results, durations, cache hit/miss stats, and error logs "
                "from any failed jobs. Use this data to identify concrete, targeted improvements:\n"
                "   - Jobs with cache MISS: investigate cache key strategy, consider broader restore-keys.\n"
                "   - Slow jobs (high duration): look for parallelization, unnecessary steps, or missing caching.\n"
                "   - Failed jobs: read the error logs in the report and fix the root cause in the pipeline config "
                "     (not the application code -- that's handled by /ci-failure).\n"
                "   - All jobs passed: look for security hardening, redundant steps, or reporting gaps.\n"
                "3. Make ALL improvements you identify. Do not stop at one.\n"
                "4. After making changes, verify the YAML is valid.\n"
                "5. Stage, commit, and push:\n"
                f"   git add -A && git commit -m 'ci: improve pipeline ({focus})' && git push origin main\n"
                "   If push fails: git pull --rebase origin main && git push origin main\n"
                "6. Do NOT stop until you are confident the pipeline is meaningfully better.\n\n"
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
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("X-Frame-Options", "DENY")
        self.send_header("Content-Security-Policy", "default-src 'none'")
        self.send_header("Referrer-Policy", "no-referrer")
        self.send_header("Cache-Control", "no-store")
        self.send_header("Connection", "close")
        if request_id:
            self.send_header("X-Request-Id", request_id)
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        client_ip = self.client_address[0]
        log.info(f"GET {self.path} from {client_ip}")

        if not _rate_limiter.allow(client_ip):
            log.warning(f"Rate limit exceeded for {client_ip}")
            self._send(429, b"Too many requests")
            self.send_header("Retry-After", "60")
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
