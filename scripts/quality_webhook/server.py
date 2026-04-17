import json
import subprocess
import threading
import uuid
from datetime import datetime
from http.server import HTTPServer, BaseHTTPRequestHandler

from .config import (
    PORT, BIND_ADDRESS, WEBHOOK_SECRET, WIN_PROJECT_DIR, PROJECT_DIR,
    MAX_PAYLOAD_BYTES, WSL_DISTRO, WSL_USER,
    THREAD_POOL_SIZE, REQUEST_QUEUE_SIZE, MAX_CONCURRENT_FIXES,
    log, get_local_ip, to_wsl_path,
)
from .runner import wsl_bash, run_kiro
from .security import (
    is_client_allowed, verify_hmac, rate_limiter,
    sanitize_text, validate_name, validate_url,
)
from .fixers import (
    fix_with_retry, fix_sonar_issues, improve_pipeline,
    _job_locks, _git_lock, _max_concurrent_fixes,
    _sonar_fix_lock, _improve_lock,
)
from .memory import get_memory_stats
from .history import _load_fix_history
from .trends import get_all_trends
from .flaky import get_flaky_test_summary
from .vulns import get_vuln_summary
from .correlation import (
    register_failure, check_correlation,
    should_dispatch_correlated_fix, build_correlated_fix_prompt,
    get_correlation_summary,
)
from .decisions import should_hold_for_correlation, record_co_failure_event, CO_FAILURE_HOLD_S, get_decision_cache_summary
from .classifier import classify_error

_thread_semaphore = threading.Semaphore(THREAD_POOL_SIZE)
_holds_lock = threading.Lock()
_pending_holds: dict[str, dict] = {}

_active_fix_count = 0
_active_fix_count_lock = threading.Lock()
_active_thread_count = 0
_active_thread_count_lock = threading.Lock()


def _build_timing_stats():
    history = _load_fix_history()
    last_fix = None
    total_s_sum = 0.0
    fix_count = 0

    for entry in history.values():
        if not isinstance(entry, dict):
            continue
        for attempt in entry.get("attempts", []):
            timing = attempt.get("timing")
            if not timing or not isinstance(timing, dict):
                continue
            fix_count += 1
            total_s = timing.get("total_s", 0.0)
            total_s_sum += total_s
            if last_fix is None or attempt.get("ts", "") > last_fix.get("ts", ""):
                last_fix = {
                    "job": entry.get("job", "unknown"),
                    "total_s": total_s,
                    "phases": {
                        k: v for k, v in timing.items() if k != "total_s"
                    },
                    "ts": attempt.get("ts", ""),
                }

    result = {
        "fix_count": fix_count,
        "avg_total_s": round(total_s_sum / fix_count, 1) if fix_count > 0 else 0.0,
    }
    if last_fix:
        result["last_fix"] = {
            "job": last_fix["job"],
            "total_s": last_fix["total_s"],
            "phases": last_fix["phases"],
        }
    return result


class TimeoutHTTPServer(HTTPServer):
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
        if not is_client_allowed(client_ip):
            log.warning(f"Rejected GET from untrusted IP {client_ip}")
            self._send(403, b"Forbidden")
            return
        log.info(f"GET {self.path} from {client_ip}")

        if not rate_limiter.allow(client_ip):
            log.warning(f"Rate limit exceeded for {client_ip}")
            self.send_response(429)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Retry-After", "60")
            self.send_header("Connection", "close")
            self.end_headers()
            self.wfile.write(b"Too many requests")
            return

        if self.path == "/health":
            job_lock_status = {
                name: lock.locked() for name, lock in _job_locks.items()
            }
            job_lock_status["sonar_fix"] = _sonar_fix_lock.locked()
            job_lock_status["improve"] = _improve_lock.locked()

            timing_info = _build_timing_stats()

            with _active_fix_count_lock:
                active_fixes = _active_fix_count
            with _active_thread_count_lock:
                active_threads = _active_thread_count

            trends = get_all_trends()
            duration_trends = {}
            for job_name, trend in trends.items():
                duration_trends[job_name] = {
                    "moving_avg_s": round(trend.moving_avg_s, 1),
                    "sample_count": trend.sample_count,
                    "timeout_risk": trend.timeout_risk,
                }

            self._send(200, json.dumps({
                "status": "ok",
                "timestamp": datetime.now().isoformat(),
                "locks": job_lock_status,
                "memory": get_memory_stats(),
                "timing": timing_info,
                "concurrent_fixes": {
                    "active": max(0, active_fixes),
                    "max": MAX_CONCURRENT_FIXES,
                },
                "thread_pool": {
                    "active": max(0, active_threads),
                    "max": THREAD_POOL_SIZE,
                },
                "duration_trends": duration_trends,
                "flaky_tests": get_flaky_test_summary(),
                "vuln_patterns": get_vuln_summary(),
                "correlations": get_correlation_summary(),
                **get_decision_cache_summary(),
            }).encode(), "application/json")
            return
        self._send(404, b"Not found")

    def do_POST(self):
        client_ip = self.client_address[0]
        request_id = uuid.uuid4().hex[:12]
        if not is_client_allowed(client_ip):
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

        if not rate_limiter.allow(client_ip):
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

        if not verify_hmac(self.path, body, self.headers, WEBHOOK_SECRET):
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
                if not _thread_semaphore.acquire(blocking=True, timeout=30):
                    log.warning(f"[{rid}] Thread pool timed out -- deferring {self.path}")
                    return
                with _active_thread_count_lock:
                    global _active_thread_count
                    _active_thread_count += 1
                try:
                    fn(p)
                except Exception:
                    log.exception(f"[{rid}] Handler error for {self.path}")
                finally:
                    with _active_thread_count_lock:
                        _active_thread_count -= 1
                    _thread_semaphore.release()

            threading.Thread(target=_guarded, args=(handler, payload, request_id), daemon=True).start()
        else:
            log.warning(f"[{request_id}] Unknown endpoint: {self.path}")
            self._send(404, b"Not found", request_id=request_id)

    def _handle_sonarqube(self, payload):
        project = sanitize_text(payload.get("project", {}).get("key", "unknown"), 128)
        gate_status = validate_name(payload.get("qualityGate", {}).get("status", "unknown"))

        log.info(f"SonarQube webhook: project={project}, status={gate_status}")

        if gate_status == "OK":
            log.info("Quality gate passed. No action needed.")
            return

        conditions = payload.get("qualityGate", {}).get("conditions", [])
        failed = [c for c in conditions if c.get("status") == "ERROR"]
        if not failed:
            log.info("No failed conditions found.")
            return

        from .security import sanitize_for_prompt as _sfp
        failures_summary = ", ".join(
            f"{sanitize_text(c.get('metric', '?'), 64)}={sanitize_text(str(c.get('value', '?')), 32)} "
            f"(threshold: {sanitize_text(str(c.get('errorThreshold', '?')), 32)})"
            for c in failed
        )
        log.info(f"Failed conditions: {failures_summary}")

        _PHASE_QUICK_MAX = 3
        _PHASE_DEEP_MAX = 6
        last_strategy = ""
        attempt = 0
        while True:
            attempt += 1

            if attempt <= _PHASE_QUICK_MAX:
                phase = "quick"
            elif attempt <= _PHASE_DEEP_MAX:
                phase = "deep"
            else:
                phase = "investigate"

            log.info(f"=== SonarQube fix attempt {attempt} [{phase}] ===")

            retry_section = ""
            if last_strategy:
                retry_section = (
                    f"\nPREVIOUS ATTEMPT FAILED: {last_strategy}\n"
                    "You MUST try a DIFFERENT approach.\n"
                )

            if phase == "investigate":
                prompt = (
                    f"INVESTIGATION MODE: The SonarQube quality gate has FAILED for project '{_sfp(project, 128)}' "
                    f"after {attempt - 1} failed fix attempts.\n\n"
                    f"Failed conditions: {_sfp(failures_summary, 1000)}\n\n"
                    "MANDATORY INVESTIGATION STEPS:\n"
                    "1. Fetch the specific SonarQube issues via the API or web UI\n"
                    "2. Read EVERY file mentioned + their imports and callers\n"
                    "3. Check git log -10 for recent changes to affected files\n"
                    "4. Search the web for the SonarQube rule IDs if unfamiliar\n"
                    "5. Read .kiro/steering/ for project conventions\n"
                    "6. Write a 3-sentence plan BEFORE editing any file\n\n"
                    "THEN fix, verify with cargo fmt, clippy, and test.\n"
                    f"{retry_section}"
                    "Stage: git add -A && git commit -m 'fix: resolve SonarQube failures (auto-fix)' && git push origin main"
                )
            elif phase == "deep":
                prompt = (
                    f"DEEP RESEARCH: The SonarQube quality gate FAILED for project '{_sfp(project, 128)}'. "
                    f"Quick fixes (attempts 1-{_PHASE_QUICK_MAX}) did not work.\n\n"
                    f"Failed conditions: {_sfp(failures_summary, 1000)}\n\n"
                    "Fetch the specific issues, understand the root cause deeply. "
                    "Read the affected files and their context before making changes. "
                    "Fix them, verify with cargo fmt, clippy, and test.\n"
                    f"{retry_section}"
                    "Then commit and push: git add -A && git commit -m 'fix: resolve SonarQube failures (auto-fix)' && git push origin main"
                )
            else:
                prompt = (
                    f"The SonarQube quality gate FAILED for project '{_sfp(project, 128)}'. "
                    f"Failed conditions: {_sfp(failures_summary, 1000)}. "
                    "Fetch the specific issues, fix them, verify with cargo fmt, clippy, and test. "
                    f"{retry_section}"
                    "Then commit and push: git add -A && git commit -m 'fix: resolve SonarQube failures (auto-fix)' && git push origin main"
                )

            result = run_kiro(prompt, f"SonarQube fix attempt {attempt} [{phase}]")
            success = result[0] if isinstance(result, tuple) else result
            if success:
                return
            last_strategy = f"attempt {attempt} [{phase}] failed"

    def _handle_ci_failure(self, payload):
        job = validate_name(payload.get("job", "unknown"))
        step = validate_name(payload.get("step", "unknown"))
        error_log = sanitize_text(payload.get("error_log", "No error details provided"))
        run_url = validate_url(payload.get("run_url", ""))
        context = {
            "commit": sanitize_text(payload.get("commit", ""), 64),
            "branch": sanitize_text(payload.get("branch", ""), 128),
            "actor": sanitize_text(payload.get("actor", ""), 64),
            "run_url": run_url,
        }

        log.info(f"CI failure webhook: job={job}, step={step}, commit={context['commit'][:8]}, branch={context['branch']}")

        error_class = classify_error(error_log)

        commit = context["commit"]
        should_hold, correlated_jobs, hold_seconds = should_hold_for_correlation(job)

        if should_hold and commit:
            with _holds_lock:
                if commit in _pending_holds:
                    entry = _pending_holds.pop(commit)
                    entry["failed_jobs"].append(job)
                    entry["payloads"].append((job, step, error_log, error_class, context))
                    timer = entry.get("timer")
                    if timer is not None:
                        timer.cancel()

                    log.info(
                        f"Correlated failure arrived during hold for commit {commit[:8]}: "
                        f"jobs={entry['failed_jobs']}"
                    )

                    all_failed_jobs = list(entry["failed_jobs"])
                    all_payloads = list(entry["payloads"])

                    def _dispatch_correlated():
                        group_id = None
                        for j, s, el, ec, _ in all_payloads:
                            gid = register_failure(j, s, el, commit, ec)
                            if gid is not None:
                                group_id = gid

                        if group_id is not None:
                            group = check_correlation(group_id)
                            if group is not None and should_dispatch_correlated_fix(group_id):
                                if not group.dispatched:
                                    group.dispatched = True
                                    prompt = build_correlated_fix_prompt(group)
                                    log.info(
                                        f"Dispatching correlated fix for commit {commit[:8]} "
                                        f"({len(all_failed_jobs)} failures)"
                                    )
                                    from .runner import run_kiro as _run_kiro
                                    _run_kiro(prompt, f"Correlated fix (hold-batch, {commit[:8]})")
                                    record_co_failure_event(commit, all_failed_jobs)
                                    return

                        record_co_failure_event(commit, all_failed_jobs)
                        for j, s, el, ec, ctx in all_payloads:
                            fix_with_retry(j, s, el, ctx)

                    threading.Thread(target=_dispatch_correlated, daemon=True).start()
                    return

                def _hold_expired():
                    with _holds_lock:
                        entry = _pending_holds.pop(commit, None)
                    if entry is None:
                        return
                    log.info(f"Hold expired for commit {commit[:8]}, dispatching independent fix")
                    failed_jobs = entry["failed_jobs"]
                    record_co_failure_event(commit, failed_jobs)
                    for j, s, el, ec, ctx in entry["payloads"]:
                        fix_with_retry(j, s, el, ctx)

                timer = threading.Timer(hold_seconds, _hold_expired)
                _pending_holds[commit] = {
                    "failed_jobs": [job],
                    "payloads": [(job, step, error_log, error_class, context)],
                    "timer": timer,
                }
                timer.start()
                log.info(
                    f"Holding dispatch for commit {commit[:8]} ({hold_seconds}s), "
                    f"waiting for correlated jobs: {correlated_jobs}"
                )
                return

        group_id = register_failure(job, step, error_log, commit, error_class)

        if group_id is not None:
            group = check_correlation(group_id)
            if group is not None and should_dispatch_correlated_fix(group_id):
                if not group.dispatched:
                    group.dispatched = True
                    prompt = build_correlated_fix_prompt(group)
                    log.info(
                        f"Dispatching correlated fix for group {group_id} "
                        f"({len(group.failures)} failures, commit {group.commit[:8]})"
                    )
                    from .runner import run_kiro as _run_kiro
                    _run_kiro(prompt, f"Correlated fix ({group_id})")
                    return
                log.info(f"Correlated fix already dispatched for group {group_id}")
                return

        fix_with_retry(job, step, error_log, context)

    def _handle_ci_improve(self, payload):
        raw_focus = sanitize_text(payload.get("focus", "general"), 128)
        focus = ",".join(validate_name(f.strip()) for f in raw_focus.split(",") if f.strip())
        if not focus:
            focus = "general"
        pipeline_report = sanitize_text(payload.get("pipeline_report", ""))
        sonar_report = sanitize_text(payload.get("sonar_report", ""))
        run_url = validate_url(payload.get("run_url", ""))

        log.info(f"CI improve webhook: focus={focus}")
        improve_pipeline(focus, pipeline_report, run_url, sonar_report)

    def _handle_sonar_fix(self, payload):
        sonar_report = sanitize_text(payload.get("sonar_report", ""))
        run_url = validate_url(payload.get("run_url", ""))

        log.info("SonarQube fix webhook received")
        fix_sonar_issues(sonar_report, run_url)

    def _reject_method(self):
        self._send(405, b"Method not allowed")

    do_PUT = do_DELETE = do_PATCH = do_HEAD = do_OPTIONS = do_TRACE = do_CONNECT = _reject_method

    def log_message(self, format, *args):
        pass


def main():
    from .config import KIRO_CLI

    if not WEBHOOK_SECRET:
        log.error("WEBHOOK_SECRET environment variable is required. Set it before starting the listener.")
        raise SystemExit(1)

    if not WIN_PROJECT_DIR.is_dir():
        log.error(f"PROJECT_DIR does not exist or is not a directory: {WIN_PROJECT_DIR}")
        raise SystemExit(1)

    local_ip = get_local_ip()

    log.info("Running startup checks...")
    try:
        result = wsl_bash(f"{KIRO_CLI} --version", timeout=30)
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

    result = wsl_bash("test -d .", timeout=10)
    if result.returncode != 0:
        log.error(f"Project dir not accessible from WSL: {PROJECT_DIR}")
        raise SystemExit(1)
    log.info(f"  Project dir: {PROJECT_DIR} ✓")

    result = wsl_bash(f"{KIRO_CLI} chat --help", timeout=30)
    if result.returncode != 0:
        log.error(f"kiro-cli chat --help failed (exit {result.returncode}): {result.stderr.strip()}")
        raise SystemExit(1)
    log.info("  kiro-cli chat args: ✓")

    test_prompt = (
        "--- TEST ---\n"
        "Shell metacharacters: `backticks` $(command) $HOME\n"
        "Parens: (test) and dashes: --- END ---\n"
    )
    test_file = WIN_PROJECT_DIR / ".kiro-prompt-init-test.txt"
    test_file_wsl = to_wsl_path(test_file)
    try:
        test_file.write_text(test_prompt, encoding="utf-8", newline="\n")
        result = wsl_bash(
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
    server.request_queue_size = REQUEST_QUEUE_SIZE
    log.info(f"Quality webhook listener running on {BIND_ADDRESS}:{PORT}")
    log.info(f"Max concurrent fixes: {MAX_CONCURRENT_FIXES} | Max payload: {MAX_PAYLOAD_BYTES // 1024}KB")
    log.info("Endpoints:")
    log.info(f"  Health:     http://{local_ip}:{PORT}/health")
    log.info(f"  SonarQube:  http://{local_ip}:{PORT}/sonarqube")
    log.info(f"  CI failure: http://{local_ip}:{PORT}/ci-failure")
    log.info(f"  CI improve: http://{local_ip}:{PORT}/ci-improve")
    log.info(f"  Sonar fix:  http://{local_ip}:{PORT}/sonar-fix")
    log.info(f"Project dir: {PROJECT_DIR} (Windows: {WIN_PROJECT_DIR})")
    log.info(f"WSL: {WSL_DISTRO} (user: {WSL_USER})")
    log.info("Waiting for webhooks...")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        log.info("Shutting down...")
        server.server_close()
