import json
import re
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
    fix_once_and_push, fix_sonar_issues, improve_pipeline,
    _job_locks, _git_lock, _max_concurrent_fixes,
    _sonar_fix_lock, _improve_lock,
    _pending_fixes, _pending_fixes_lock,
    _write_pipeline_escalation,
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
from .tracker import get_tracker, JobState
from .queue import enqueue, dequeue, mark_done, mark_failed, requeue_stale, pending_count, cleanup_old, get_stats as get_queue_stats

_thread_semaphore = threading.Semaphore(THREAD_POOL_SIZE)
_holds_lock = threading.Lock()
_pending_holds: dict[str, dict] = {}

_active_fix_count = 0
_active_fix_count_lock = threading.Lock()
_active_thread_count = 0
_active_thread_count_lock = threading.Lock()

_JOB_RESULT_ROW_RE = re.compile(
    r"^\|\s*(?P<job>[^|]+?)\s*\|\s*(?P<status>success|failure|skipped|cancelled)\s*\|",
    re.IGNORECASE,
)
_COMMIT_SHA_RE = re.compile(r"\b([0-9a-f]{7,40})\b")
_COMMIT_LABEL_RE = re.compile(
    r"(?:commit|sha|ref)[:\s]+([0-9a-f]{7,40})", re.IGNORECASE,
)


# ---------------------------------------------------------------------------
# Helpers extracted from handler methods to reduce cognitive complexity
# ---------------------------------------------------------------------------

def _check_pipeline_budget(tracker, commit, label=""):
    """Return (pipeline, should_skip). If should_skip is True, caller should return."""
    pipeline = tracker.get_or_create(commit)
    if pipeline.deployed:
        log.info(f"Pipeline {pipeline.lineage_id} already deployed, skipping {label}")
        return pipeline, True
    if tracker.is_budget_exhausted(commit):
        log.error(
            f"Pipeline budget exhausted for lineage {pipeline.lineage_id} "
            f"(round {pipeline.round_number}/{pipeline.budget_total}). "
            f"Skipping {label}."
        )
        return pipeline, True
    return pipeline, False


def _dispatch_payloads(payloads, commit):
    """Register failures, attempt correlated dispatch, then dispatch all."""
    group_id = None
    for j, s, el, ec, _ in payloads:
        gid = register_failure(j, s, el, commit, ec)
        if gid is not None:
            group_id = gid

    if group_id is not None:
        group = check_correlation(group_id)
        if group is not None and should_dispatch_correlated_fix(group_id):
            if not group.dispatched:
                group.dispatched = True
                all_jobs = [p[0] for p in payloads]
                log.info(
                    f"Dispatching correlated fix for commit {commit[:8]} "
                    f"({len(all_jobs)} failures)"
                )
                record_co_failure_event(commit, all_jobs)
                for j, s, el, _ec, ctx in payloads:
                    fix_once_and_push(j, s, el, ctx)
                return

    record_co_failure_event(commit, [p[0] for p in payloads])
    for j, s, el, _ec, ctx in payloads:
        fix_once_and_push(j, s, el, ctx)


def _handle_hold_existing_entry(commit, job, step, error_log, error_class, context):
    """Handle a correlated failure arriving while a hold is already active."""
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

    all_payloads = list(entry["payloads"])

    def _dispatch_correlated():
        tracker = get_tracker()
        _pipeline, skip = _check_pipeline_budget(
            tracker, commit, f"correlated dispatch for {commit[:8]}"
        )
        if skip:
            return
        _dispatch_payloads(all_payloads, commit)

    threading.Thread(target=_dispatch_correlated, daemon=True).start()


def _handle_hold_new_entry(commit, job, step, error_log, error_class, context,
                           hold_seconds, correlated_jobs):
    """Create a new hold entry and start the expiry timer."""
    def _hold_expired():
        with _holds_lock:
            entry = _pending_holds.pop(commit, None)
        if entry is None:
            return

        tracker = get_tracker()
        _pipeline, skip = _check_pipeline_budget(
            tracker, commit, f"expired hold dispatch for {commit[:8]}"
        )
        if skip:
            return

        log.info(f"Hold expired for commit {commit[:8]}, dispatching independent fix")
        failed_jobs = entry["failed_jobs"]
        record_co_failure_event(commit, failed_jobs)
        for j, s, el, ec, ctx in entry["payloads"]:
            fix_once_and_push(j, s, el, ctx)

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


def _try_correlated_dispatch(tracker, group_id, commit, job, step, error_log, context):
    """Attempt correlated fix dispatch. Return True if handled."""
    group = check_correlation(group_id)
    if group is None or not should_dispatch_correlated_fix(group_id):
        return False
    if group.dispatched:
        log.info(f"Correlated fix already dispatched for group {group_id}")
        return True
    if commit:
        _pipeline, skip = _check_pipeline_budget(
            tracker, commit, f"correlated fix for group {group_id}"
        )
        if skip:
            return True
    group.dispatched = True
    log.info(
        f"Dispatching correlated fix for group {group_id} "
        f"({len(group.failures)} failures, commit {group.commit[:8]})"
    )
    fix_once_and_push(job, step, error_log, context)
    return True


def _determine_sonar_phase(attempt):
    """Return the phase name based on the attempt number."""
    _PHASE_QUICK_MAX = 3
    _PHASE_DEEP_MAX = 6
    if attempt <= _PHASE_QUICK_MAX:
        return "quick"
    if attempt <= _PHASE_DEEP_MAX:
        return "deep"
    return "investigate"


def _build_sonar_prompt(phase, project, failures_summary, attempt):
    """Build the Kiro prompt for a SonarQube fix based on the phase."""
    from .security import sanitize_for_prompt as _sfp
    if phase == "investigate":
        return (
            f"INVESTIGATION MODE: The SonarQube quality gate has FAILED for project '{_sfp(project, 128)}' "
            f"after multiple failed fix attempts (round {attempt}).\n\n"
            f"Failed conditions: {_sfp(failures_summary, 1000)}\n\n"
            "MANDATORY INVESTIGATION STEPS:\n"
            "1. Fetch the specific SonarQube issues via the API or web UI\n"
            "2. Read EVERY file mentioned + their imports and callers\n"
            "3. Check git log -10 for recent changes to affected files\n"
            "4. Search the web for the SonarQube rule IDs if unfamiliar\n"
            "5. Read .kiro/steering/ for project conventions\n"
            "6. Write a 3-sentence plan BEFORE editing any file\n\n"
            "THEN fix, verify with cargo fmt, clippy, and test.\n"
            "Stage: git add -A. Do NOT commit."
        )
    if phase == "deep":
        return (
            f"DEEP RESEARCH: The SonarQube quality gate FAILED for project '{_sfp(project, 128)}'. "
            f"Quick fixes did not work (round {attempt}).\n\n"
            f"Failed conditions: {_sfp(failures_summary, 1000)}\n\n"
            "Fetch the specific issues, understand the root cause deeply. "
            "Read the affected files and their context before making changes. "
            "Fix them, verify with cargo fmt, clippy, and test.\n"
            "Stage: git add -A. Do NOT commit."
        )
    return (
        f"The SonarQube quality gate FAILED for project '{_sfp(project, 128)}'. "
        f"Failed conditions: {_sfp(failures_summary, 1000)}. "
        "Fetch the specific issues, fix them, verify with cargo fmt, clippy, and test. "
        "Stage: git add -A. Do NOT commit."
    )


def _confirm_strategies(pipeline, job_results, tracker, commit):
    """Confirm pending strategies based on job results."""
    for job, result in job_results.items():
        if job in pipeline.pending_strategies:
            if result == "success":
                tracker.confirm_strategy(commit, job, True)
            elif result == "failure":
                tracker.confirm_strategy(commit, job, False)


def _detect_regressions(failed_jobs, pipeline, tracker, commit):
    """Detect regression jobs and record events. Return the set of regression job names."""
    regression_jobs = set()
    prev_round = pipeline.round_number - 1
    prev_files = set(pipeline.files_changed.get(prev_round, []))
    if not prev_files:
        return regression_jobs
    for job in failed_jobs:
        regressed_from = tracker.detect_regression(commit, job, prev_files)
        if regressed_from:
            regression_jobs.add(job)
            _record_regression_event(
                commit, pipeline.lineage_id, pipeline.round_number,
                job, regressed_from, sorted(prev_files),
            )
    return regression_jobs


def _dispatch_failed_job_fixes(failed_jobs, regression_jobs, commit, run_url,
                               pipeline, sonar_report, sonar_result, branch="main"):
    """Dispatch fix threads for failed jobs and optionally a sonar fix."""
    log.info(f"Dispatching fixes for {len(failed_jobs)} failed jobs: {failed_jobs}")
    for job in failed_jobs:
        error_context = ""
        if job in regression_jobs:
            error_context = (
                "REGRESSION DETECTED: This failure may be caused by fixes "
                "applied in the previous round. Files changed in the prior "
                "round overlap with this job's scope. Review the previous "
                "fix carefully and address both the original issue and the "
                "regression it introduced."
            )
        threading.Thread(
            target=fix_once_and_push,
            args=(job, "ci-improve", error_context, {"commit": commit, "run_url": run_url, "branch": branch}),
            daemon=True,
        ).start()

    if not pipeline.sonarqube_passed and sonar_result == "failure":
        if not _sonar_fix_lock.locked():
            log.info(
                f"Lineage {pipeline.lineage_id}: dispatching sonar fix alongside job fixes"
            )
            threading.Thread(
                target=fix_sonar_issues,
                args=(sonar_report, run_url),
                kwargs={"commit": commit, "branch": branch},
                daemon=True,
            ).start()


def _record_regression_event(
    commit: str, lineage_id: str, round_number: int,
    failed_job: str, regressed_from: list[str], overlapping_files: list[str],
) -> None:
    from .history import _fix_history_lock, _load_fix_history, _save_fix_history
    event = {
        "ts": datetime.now().isoformat(),
        "commit": commit[:12],
        "lineage_id": lineage_id,
        "round": round_number,
        "failed_job": failed_job,
        "regressed_from_jobs": regressed_from,
        "overlapping_files": overlapping_files,
    }
    with _fix_history_lock:
        history = _load_fix_history()
        regressions = history.setdefault("regression_events", [])
        regressions.append(event)
        regressions[:] = regressions[-50:]
        _save_fix_history(history)
    log.warning(
        f"Regression event recorded: {failed_job} regressed from "
        f"{regressed_from} in round {round_number} (lineage {lineage_id})"
    )


def _parse_job_results(report: str) -> dict[str, str]:
    results: dict[str, str] = {}
    if not report:
        return results
    for line in report.splitlines():
        m = _JOB_RESULT_ROW_RE.match(line.strip())
        if m:
            job = m.group("job").strip().lower().replace(" ", "-")
            status = m.group("status").strip().lower()
            if job and job not in ("job", "---", ""):
                results[job] = status
    return results


def _extract_commit_from_report(report: str) -> str:
    if not report:
        return ""
    m = _COMMIT_LABEL_RE.search(report)
    if m:
        return m.group(1)
    for line in report.splitlines():
        line = line.strip()
        if line.startswith("|") or line.startswith("---"):
            continue
        sha_match = _COMMIT_SHA_RE.search(line)
        if sha_match:
            candidate = sha_match.group(1)
            if len(candidate) >= 7:
                return candidate
    return ""


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


def _process_deprecation_report(deprecation_report: str, run_url: str, commit: str, branch: str):
    """Write a deprecation report to a JSON file for later review."""
    try:
        dest_dir = WIN_PROJECT_DIR / "deprecations"
        dest_dir.mkdir(parents=True, exist_ok=True)
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        filename = f"{timestamp}_{commit[:8] or 'unknown'}.json"
        filepath = dest_dir / filename
        filepath.write_text(json.dumps({
            "timestamp": datetime.now().isoformat(),
            "commit": commit,
            "branch": branch,
            "run_url": run_url,
            "deprecation_report": deprecation_report,
            "status": "pending_review",
        }, indent=2, ensure_ascii=False), encoding="utf-8")
        log.info(f"Deprecation report written to {filepath}")
    except OSError as exc:
        log.warning(f"Failed to write deprecation report: {exc}")


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
            with _pending_fixes_lock:
                pending_jobs = list(_pending_fixes.keys())

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
                    "pending_queue": pending_jobs,
                },
                "thread_pool": {
                    "active": max(0, active_threads),
                    "max": THREAD_POOL_SIZE,
                },
                "duration_trends": duration_trends,
                "flaky_tests": get_flaky_test_summary(),
                "vuln_patterns": get_vuln_summary(),
                "correlations": get_correlation_summary(),
                "pipeline_tracker": get_tracker().get_summary(),
                "webhook_queue": get_queue_stats(),
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
            "/ci-deprecations": self._handle_ci_deprecations,
        }

        handler = handlers.get(self.path)
        if handler:
            queue_id = enqueue(self.path, payload)
            self._send(200, b"OK", request_id=request_id)
            self._dispatch_handler(handler, payload, request_id, queue_id)
        else:
            log.warning(f"[{request_id}] Unknown endpoint: {self.path}")
            self._send(404, b"Not found", request_id=request_id)

    def _dispatch_handler(self, handler, payload, request_id, queue_id):
        def _guarded():
            if not _thread_semaphore.acquire(blocking=True, timeout=30):
                log.warning(f"[{request_id}] Thread pool timed out -- deferring {self.path}")
                mark_failed(queue_id, "thread pool timeout")
                return
            with _active_thread_count_lock:
                global _active_thread_count
                _active_thread_count += 1
            try:
                handler(payload)
                mark_done(queue_id)
            except Exception:
                log.exception(f"[{request_id}] Handler error for {self.path}")
                mark_failed(queue_id, "handler exception")
            finally:
                with _active_thread_count_lock:
                    _active_thread_count -= 1
                _thread_semaphore.release()

        threading.Thread(target=_guarded, daemon=True).start()

    def _handle_sonarqube(self, payload):
        project = sanitize_text(payload.get("project", {}).get("key", "unknown"), 128)
        gate_status = validate_name(payload.get("qualityGate", {}).get("status", "unknown"))

        log.info(f"SonarQube webhook: project={project}, status={gate_status}")

        if gate_status == "OK":
            log.info("Quality gate passed. No action needed.")
            return

        commit = sanitize_text(payload.get("revision", ""), 64)
        branch = sanitize_text(payload.get("branch", {}).get("name", ""), 128) or "main"
        pipeline = None
        if commit:
            tracker = get_tracker()
            pipeline, skip = _check_pipeline_budget(tracker, commit, "SonarQube fix")
            if skip:
                return

        conditions = payload.get("qualityGate", {}).get("conditions", [])
        failed = [c for c in conditions if c.get("status") == "ERROR"]
        if not failed:
            log.info("No failed conditions found.")
            return

        failures_summary = ", ".join(
            f"{sanitize_text(c.get('metric', '?'), 64)}={sanitize_text(str(c.get('value', '?')), 32)} "
            f"(threshold: {sanitize_text(str(c.get('errorThreshold', '?')), 32)})"
            for c in failed
        )
        log.info(f"Failed conditions: {failures_summary}")

        attempt = pipeline.round_number if commit and pipeline else 1
        phase = _determine_sonar_phase(attempt)
        log.info(f"=== SonarQube fix attempt (round {attempt}) [{phase}] ===")

        prompt = _build_sonar_prompt(phase, project, failures_summary, attempt)
        self._run_sonar_worktree_fix(prompt, commit, attempt, phase, branch)

    def _run_sonar_worktree_fix(self, prompt, commit, attempt, phase, branch="main"):
        from .worktrees import setup_worktree, commit_and_push, cleanup_worktree
        wt_path, wt_name = setup_worktree(branch, commit or "HEAD")
        if not wt_path:
            log.error("Failed to create worktree for SonarQube fix")
            return

        try:
            result = run_kiro(prompt, f"SonarQube fix (round {attempt}) [{phase}]", cwd=wt_path)
            success = result[0] if isinstance(result, tuple) else result
            if success:
                commit_ok, commit_err = commit_and_push(wt_path, branch, "fix: resolve SonarQube failures (auto-fix)")
                if commit_ok and commit_err == "nothing to commit":
                    log.warning(f"SonarQube fix (round {attempt}) [{phase}] produced no changes")
                elif commit_ok:
                    log.info(f"SonarQube fix (round {attempt}) [{phase}] succeeded")
                else:
                    log.warning(f"SonarQube fix (round {attempt}) [{phase}] commit/push failed")
            else:
                log.warning(f"SonarQube fix (round {attempt}) [{phase}] failed")
        finally:
            if wt_name:
                cleanup_worktree(wt_name)

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

        commit = context["commit"]
        tracker = get_tracker()

        if commit:
            skip = self._ci_failure_pipeline_guard(tracker, commit, job, step)
            if skip:
                return

        error_class = classify_error(error_log)
        should_hold, correlated_jobs, hold_seconds = should_hold_for_correlation(job)

        if should_hold and commit:
            self._ci_failure_hold_path(
                commit, job, step, error_log, error_class, context,
                hold_seconds, correlated_jobs,
            )
            return

        group_id = register_failure(job, step, error_log, commit, error_class)
        if group_id is not None:
            if _try_correlated_dispatch(tracker, group_id, commit, job, step, error_log, context):
                return

        fix_once_and_push(job, step, error_log, context)

    def _ci_failure_pipeline_guard(self, tracker, commit, job, step):
        """Check pipeline state and return True if the failure should be skipped."""
        pipeline = tracker.get_or_create(commit)

        prev_state = pipeline.job_states.get(job)
        if prev_state == JobState.PASSED:
            log.warning(
                f"FIX REGRESSION: job {job} was marked PASSED but failed again "
                f"(commit {commit[:8]}, lineage {pipeline.lineage_id}). "
                f"Previous auto-fix did not resolve the issue."
            )
            round_files = pipeline.files_changed.get(pipeline.round_number, [])
            _record_regression_event(
                commit, pipeline.lineage_id, pipeline.round_number,
                job, [job], round_files,
            )

        if pipeline.deployed:
            log.info(
                f"Pipeline {pipeline.lineage_id} already deployed, "
                f"skipping ci-failure for {job}/{step}"
            )
            return True
        if tracker.is_budget_exhausted(commit):
            log.error(
                f"Pipeline budget exhausted for lineage {pipeline.lineage_id} "
                f"(round {pipeline.round_number}/{pipeline.budget_total}). "
                f"Skipping ci-failure for {job}/{step}."
            )
            return True
        if pipeline.job_states.get(job) == JobState.FIXING:
            log.info(
                f"Job {job} already being fixed for lineage {pipeline.lineage_id}, "
                f"skipping duplicate ci-failure dispatch"
            )
            return True
        return False

    def _ci_failure_hold_path(self, commit, job, step, error_log, error_class,
                              context, hold_seconds, correlated_jobs):
        """Handle the correlation hold path. Always consumes the failure."""
        with _holds_lock:
            if commit in _pending_holds:
                _handle_hold_existing_entry(
                    commit, job, step, error_log, error_class, context,
                )
            else:
                _handle_hold_new_entry(
                    commit, job, step, error_log, error_class, context,
                    hold_seconds, correlated_jobs,
                )

    def _handle_ci_improve(self, payload):
        raw_focus = sanitize_text(payload.get("focus", "general"), 128)
        focus = ",".join(validate_name(f.strip()) for f in raw_focus.split(",") if f.strip())
        if not focus:
            focus = "general"
        pipeline_report = sanitize_text(payload.get("pipeline_report", ""))
        sonar_report = sanitize_text(payload.get("sonar_report", ""))
        run_url = validate_url(payload.get("run_url", ""))
        branch = sanitize_text(payload.get("branch", ""), 128) or "main"

        job_results = _parse_job_results(pipeline_report)
        commit = sanitize_text(payload.get("commit", ""), 64) or _extract_commit_from_report(pipeline_report)

        log.info(
            f"CI improve webhook: focus={focus}, "
            f"jobs={len(job_results)}, commit={commit[:8] if commit else 'unknown'}"
        )

        tracker = get_tracker()

        if commit and job_results:
            pipeline = tracker.update_from_report(commit, job_results)
        elif commit:
            pipeline = tracker.get_or_create(commit)
        else:
            improve_pipeline(focus, pipeline_report, run_url, sonar_report, branch=branch)
            return

        if self._ci_improve_deploy_success(pipeline, job_results, tracker, commit):
            return

        if tracker.is_budget_exhausted(commit):
            _write_pipeline_escalation(pipeline)
            log.error(
                f"Pipeline budget exhausted for lineage {pipeline.lineage_id} "
                f"(round {pipeline.round_number}/{pipeline.budget_total}). "
                f"No more fixes will be dispatched."
            )
            return

        _confirm_strategies(pipeline, job_results, tracker, commit)

        sonar_result = job_results.get("sonarqube", job_results.get("sonar", "unknown"))
        if sonar_result == "success":
            pipeline.sonarqube_passed = True
        elif sonar_result == "failure":
            pipeline.sonarqube_passed = False

        failed_jobs = [
            j for j, r in job_results.items()
            if r == "failure" and pipeline.job_states.get(j) != JobState.FIXING
        ]

        if failed_jobs:
            regression_jobs = _detect_regressions(failed_jobs, pipeline, tracker, commit)
            _dispatch_failed_job_fixes(
                failed_jobs, regression_jobs, commit, run_url,
                pipeline, sonar_report, sonar_result, branch,
            )
        else:
            improve_pipeline(focus, pipeline_report, run_url, sonar_report, commit=commit, branch=branch)

    @staticmethod
    def _ci_improve_deploy_success(pipeline, job_results, tracker, commit):
        """Handle deploy success. Return True if no further processing needed."""
        if job_results.get("deploy") != "success":
            return False
        tracker.mark_deployed(commit)
        for job in pipeline.pending_strategies.keys():
            tracker.confirm_strategy(commit, job, True)
        log.info(
            f"Deploy SUCCESS for lineage {pipeline.lineage_id}. "
            f"Pipeline complete — no further fixes needed."
        )
        return True

    def _handle_sonar_fix(self, payload):
        sonar_report = sanitize_text(payload.get("sonar_report", ""))
        run_url = validate_url(payload.get("run_url", ""))
        commit = sanitize_text(payload.get("commit", ""), 64)
        branch = sanitize_text(payload.get("branch", ""), 128) or "main"

        log.info(f"SonarQube fix webhook received (commit={commit[:8] if commit else 'unknown'})")
        fix_sonar_issues(sonar_report, run_url, branch=branch, commit=commit)

    def _handle_ci_deprecations(self, payload):
        deprecation_report = sanitize_text(payload.get("deprecation_report", ""))
        run_url = validate_url(payload.get("run_url", ""))
        commit = sanitize_text(payload.get("commit", ""), 64)
        branch = sanitize_text(payload.get("branch", ""), 128)

        log.info(f"CI deprecations webhook: commit={commit[:8]}, branch={branch}")

        if not deprecation_report or "unavailable" in deprecation_report or "No deprecations" in deprecation_report:
            log.info("No actionable deprecations found, skipping")
            return

        _process_deprecation_report(deprecation_report, run_url, commit, branch)

    def _reject_method(self):
        self._send(405, b"Method not allowed")

    do_PUT = do_DELETE = do_PATCH = do_HEAD = do_OPTIONS = do_TRACE = do_CONNECT = _reject_method

    def log_message(self, format, *args):
        # Suppress default stderr logging from BaseHTTPRequestHandler;
        # all logging goes through the project's ``log`` logger instead.
        pass


_REPLAY_HANDLERS = {
    "/ci-failure": "_handle_ci_failure",
    "/ci-improve": "_handle_ci_improve",
    "/sonarqube": "_handle_sonarqube",
    "/sonar-fix": "_handle_sonar_fix",
    "/ci-deprecations": "_handle_ci_deprecations",
}


def _replay_pending_queue():
    handler_instance = WebhookHandler.__new__(WebhookHandler)

    while True:
        item = dequeue()
        if item is None:
            break
        row_id, endpoint, payload = item
        method_name = _REPLAY_HANDLERS.get(endpoint)
        if not method_name:
            mark_failed(row_id, f"unknown endpoint: {endpoint}")
            continue

        log.info(f"Replaying queued webhook: {endpoint} (queue_id={row_id})")

        def _run_replay(mid, p, rid):
            try:
                getattr(handler_instance, mid)(p)
                mark_done(rid)
            except Exception as exc:
                log.exception(f"Replay failed for queue_id={rid}: {exc}")
                mark_failed(rid, str(exc))

        threading.Thread(
            target=_run_replay,
            args=(method_name, payload, row_id),
            daemon=True,
        ).start()


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

    try:
        result = wsl_bash("test -d .", timeout=10)
    except subprocess.TimeoutExpired:
        log.error(f"WSL timed out checking project dir: {PROJECT_DIR}")
        raise SystemExit(1)
    if result.returncode != 0:
        log.error(f"Project dir not accessible from WSL: {PROJECT_DIR}")
        raise SystemExit(1)
    log.info(f"  Project dir: {PROJECT_DIR} ✓")

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
            f"echo \"$KIRO_PROMPT\"",
            timeout=15,
        )
    except subprocess.TimeoutExpired:
        log.error("Prompt file delivery test timed out")
        raise SystemExit(1)
    finally:
        test_file.unlink(missing_ok=True)

    if result.returncode != 0:
        log.error(
            f"Prompt file delivery test failed (exit {result.returncode}).\n"
            f"  stderr: {result.stderr.strip()[:500]}\n"
            f"  The cat-to-variable pipeline is broken."
        )
        raise SystemExit(1)
    delivered = result.stdout.strip()
    if "--- TEST ---" not in delivered or "--- END ---" not in delivered:
        log.error(
            f"Prompt content was mangled during delivery.\n"
            f"  Got: {delivered[:300]}"
        )
        raise SystemExit(1)
    log.info("  Prompt file delivery: ✓")

    log.info("Startup checks passed")

    wsl_bash("git worktree prune", timeout=15)
    requeue_stale()
    cleanup_old()
    pending = pending_count()
    if pending > 0:
        log.info(f"Replaying {pending} pending webhook(s) from queue")
        _replay_pending_queue()

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
