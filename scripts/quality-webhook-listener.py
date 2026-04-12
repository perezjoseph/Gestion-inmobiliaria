#!/usr/bin/env python3
"""
Quality Check Webhook Listener
Receives webhooks from SonarQube and GitHub Actions CI pipeline,
then triggers kiro-cli to auto-fix issues.

Endpoints:
    POST /sonarqube    — SonarQube quality gate webhook
    POST /ci-failure   — GitHub Actions pipeline failure webhook
    POST /ci-improve   — CI/CD pipeline self-improvement webhook
    POST /sonar-fix    — SonarQube open issue resolution webhook

Usage:
    python scripts/quality-webhook-listener.py
"""

import json
import logging
import os
import socket
import subprocess
import threading
from http.server import HTTPServer, BaseHTTPRequestHandler
from datetime import datetime

PORT = 9090
PROJECT_DIR = "/mnt/d/realestate"
WSL_DISTRO = "Ubuntu-22.04"
WSL_USER = "jperez"
MAX_RETRIES = 3
KIRO_TIMEOUT = 3600
TRUST_ALL_TOOLS = True

_fix_lock = threading.Lock()

logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S"
)
log = logging.getLogger("webhook")


def get_local_ip():
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.connect(("192.168.88.25", 80))
        ip = sock.getsockname()[0]
        sock.close()
        return ip
    except Exception:
        return "127.0.0.1"


def wsl_cmd(command):
    cmd = [
        "wsl", "-u", WSL_USER, "-d", WSL_DISTRO,
        "bash", "-lc",
        f"cd {PROJECT_DIR} && {command}"
    ]
    log.debug(f"WSL command: {' '.join(cmd[:6])} ...")
    return cmd


def run_kiro(prompt, label):
    log.info(f"Triggering kiro-cli for: {label}")

    escaped = prompt.replace('"', '\\"').replace("'", "'\\''")
    cmd = f'kiro-cli chat --trust-all-tools --agent sisyphus-kiro --no-interactive "{escaped}"'
    log.debug(f"Full kiro command: {cmd[:200]}...")

    log_file = os.path.join(os.path.dirname(__file__), "..", "kiro-debug.log")
    log.info(f"Streaming kiro output to: {os.path.abspath(log_file)}")

    with open(log_file, "a", encoding="utf-8") as f:
        f.write(f"\n{'='*80}\n")
        f.write(f"[{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}] {label}\n")
        f.write(f"Command: {cmd[:300]}...\n")
        f.write(f"{'='*80}\n\n")
        f.flush()

        proc = subprocess.Popen(
            wsl_cmd(cmd),
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace"
        )
        log.debug(f"Process started with PID: {proc.pid}")

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

    if "Tool approval required" in full_output:
        log.error("Tool approval error detected")
        return False

    if "denied list" in full_output:
        log.error("Tool denied in non-interactive mode")
        return False

    if proc.returncode != 0 and proc.returncode != 1:
        log.error(f"kiro-cli exited with unexpected code {proc.returncode}")
        return False

    log.info(f"kiro-cli completed successfully for: {label}")
    return True


def fix_with_retry(job, step, error_log):
    if job == "commit-lint":
        log.info("Skipping commit-lint — cannot auto-fix commit messages")
        return False

    acquired = _fix_lock.acquire(blocking=False)
    log.debug(f"Lock acquired: {acquired}")
    if not acquired:
        log.warning(f"Another fix is already running — skipping {job}/{step}")
        return False

    try:
        for attempt in range(1, MAX_RETRIES + 1):
            log.info(f"=== Attempt {attempt}/{MAX_RETRIES} for {job}/{step} ===")

            prompt_parts = [
                f"The CI pipeline FAILED in job '{job}', step '{step}'.",
                f"Error output:\n{error_log}\n",
            ]

            job_instructions = {
                "lint": "Fix the formatting and clippy warnings.",
                "test-backend": "Fix the failing backend tests.",
                "test-frontend": "Fix the failing frontend tests.",
                "quality-gate": "Fix the quality gate failures (dependency audit, unused deps, or OWASP issues).",
                "sonarqube": "Fix the SonarQube analysis failures.",
            }
            instruction = job_instructions.get(job, "Analyze the error and fix the issue.")
            prompt_parts.append(instruction)
            log.debug(f"Job instruction: {instruction}")

            prompt_parts.append(
                "The error output above contains the actual CI failure details. "
                "Fix the specific issues reported — do NOT re-run the full test suite to discover failures. "
                "After fixing, only run targeted verification for what you changed. "
                "Then commit and push: git add -A && git commit -m 'fix: resolve CI failures (auto-fix)' && git push origin main. "
                "If push fails due to remote changes, run git pull --rebase origin main first then push again."
            )

            prompt = " ".join(prompt_parts)
            log.debug(f"Prompt length: {len(prompt)} chars")

            if run_kiro(prompt, f"CI fix ({job}/{step}) attempt {attempt}"):
                log.info(f"✓ Attempt {attempt} completed for {job}/{step}")
                return True

            log.warning(f"✗ Attempt {attempt} failed for {job}/{step}")

        log.error(f"Failed after {MAX_RETRIES} attempts for {job}/{step}. Manual intervention needed.")
        return False
    finally:
        _fix_lock.release()
        log.debug("Lock released")


_sonar_fix_lock = threading.Lock()


def fix_sonar_issues(sonar_report, run_url):
    acquired = _sonar_fix_lock.acquire(blocking=False)
    log.debug(f"Sonar fix lock acquired: {acquired}")
    if not acquired:
        log.warning("Another SonarQube fix is already running — skipping")
        return False

    try:
        for attempt in range(1, MAX_RETRIES + 1):
            log.info(f"=== SonarQube issue fix attempt {attempt}/{MAX_RETRIES} ===")

            prompt = (
                "You are a code quality specialist. Your job is to resolve open SonarQube issues "
                "in this Rust/WASM real estate management project.\n\n"
                f"SonarQube issue report:\n{sonar_report}\n\n"
                "INSTRUCTIONS — work autonomously through this self-improvement loop:\n"
                "1. The report above contains open SonarQube issues with severity, file, line, rule, and message. "
                "For each issue, open the file at the specified line, understand the rule violation, and fix it.\n"
                "2. Prioritize BLOCKER and HIGH severity first, then MEDIUM.\n"
                "3. Also review and fix any security hotspots listed — check the file and line, "
                "understand the risk, and apply the appropriate fix.\n"
                "4. After fixing issues, run cargo fmt, cargo clippy, and cargo test to verify nothing is broken.\n"
                "5. If tests fail after your changes, fix the test failures before proceeding.\n"
                "6. Stage, commit, and push:\n"
                "   git add -A && git commit -m 'fix: resolve SonarQube issues (auto-fix)' && git push origin main\n"
                "   If push fails due to remote changes: git pull --rebase origin main && git push origin main\n"
                "7. Review your changes. If you missed issues or introduced new ones, repeat from step 1.\n"
                "8. Do NOT stop until all reported issues are addressed.\n\n"
                f"Run URL for context: {run_url}"
            )

            if run_kiro(prompt, f"SonarQube issue fix attempt {attempt}"):
                log.info(f"✓ SonarQube fix attempt {attempt} completed")
                return True

            log.warning(f"✗ SonarQube fix attempt {attempt} failed")

        log.error(f"SonarQube fix failed after {MAX_RETRIES} attempts.")
        return False
    finally:
        _sonar_fix_lock.release()
        log.debug("Sonar fix lock released")


_improve_lock = threading.Lock()


def improve_pipeline(focus, pipeline_report, run_url):
    acquired = _improve_lock.acquire(blocking=False)
    log.debug(f"Improve lock acquired: {acquired}")
    if not acquired:
        log.warning("Another pipeline improvement is already running — skipping")
        return False

    try:
        for attempt in range(1, MAX_RETRIES + 1):
            log.info(f"=== Pipeline improvement attempt {attempt}/{MAX_RETRIES} (focus: {focus}) ===")

            prompt = (
                "You are a CI/CD pipeline engineer. Your job is to improve the GitHub Actions pipeline "
                "at .github/workflows/ci.yml for this Rust/WASM project. "
                f"Focus area: {focus}. "
                f"Pipeline analysis report:\n{pipeline_report}\n\n"
                "INSTRUCTIONS — work autonomously through this self-improvement loop:\n"
                "1. Read .github/workflows/ci.yml and scripts/quality-webhook-listener.py fully.\n"
                "2. Based on the focus area and report, identify concrete improvements:\n"
                "   - SECURITY: Add secret scanning (gitleaks/trufflehog), pin action versions to SHA, "
                "add permissions least-privilege per job, add SAST scanning, harden artifact handling, "
                "add supply chain security (SLSA provenance, Sigstore signing), check for hardcoded secrets in env blocks.\n"
                "   - BUGS: Fix race conditions, missing error handling, fragile grep/sed parsing, "
                "missing 'if: always()' guards, output truncation issues, cache key collisions, "
                "missing timeout-minutes on jobs.\n"
                "   - PIPELINE: Optimize cache strategy, reduce redundant checkouts, add job-level timeout-minutes, "
                "add retry logic for flaky network steps, improve artifact retention policies, "
                "add workflow_dispatch for manual runs, add path filters to skip irrelevant jobs.\n"
                "   - REPORTING: Improve error collection fidelity, add structured annotations, "
                "add job summary markdown output, improve webhook payloads with richer context.\n"
                "3. Make ALL the improvements you identify. Do not stop at one.\n"
                "4. After making changes, verify the YAML is valid (check indentation, syntax).\n"
                "5. Stage, commit, and push:\n"
                "   git add -A && git commit -m 'ci: improve pipeline ({focus})' && git push origin main\n"
                "   If push fails due to remote changes: git pull --rebase origin main && git push origin main\n"
                "6. Review your own changes. If you spot more improvements, repeat from step 2.\n"
                "7. Do NOT stop until you are confident the pipeline is meaningfully better.\n\n"
                f"Run URL for context: {run_url}"
            )

            if run_kiro(prompt, f"Pipeline improve ({focus}) attempt {attempt}"):
                log.info(f"✓ Pipeline improvement attempt {attempt} completed")
                return True

            log.warning(f"✗ Pipeline improvement attempt {attempt} failed")

        log.error(f"Pipeline improvement failed after {MAX_RETRIES} attempts.")
        return False
    finally:
        _improve_lock.release()
        log.debug("Improve lock released")


class WebhookHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length)
        log.debug(f"Received POST {self.path} ({content_length} bytes)")

        try:
            payload = json.loads(body)
        except json.JSONDecodeError as e:
            log.error(f"Invalid JSON: {e}")
            self.send_response(400)
            self.end_headers()
            self.wfile.write(b"Invalid JSON")
            return

        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"OK")

        if self.path == "/sonarqube":
            log.debug("Dispatching to SonarQube handler")
            threading.Thread(target=self._handle_sonarqube, args=(payload,), daemon=True).start()
        elif self.path == "/ci-failure":
            log.debug("Dispatching to CI failure handler")
            threading.Thread(target=self._handle_ci_failure, args=(payload,), daemon=True).start()
        elif self.path == "/ci-improve":
            log.debug("Dispatching to CI pipeline improvement handler")
            threading.Thread(target=self._handle_ci_improve, args=(payload,), daemon=True).start()
        elif self.path == "/sonar-fix":
            log.debug("Dispatching to SonarQube issue fix handler")
            threading.Thread(target=self._handle_sonar_fix, args=(payload,), daemon=True).start()
        else:
            log.warning(f"Unknown endpoint: {self.path}")

    def _handle_sonarqube(self, payload):
        project = payload.get("project", {}).get("key", "unknown")
        gate_status = payload.get("qualityGate", {}).get("status", "unknown")

        log.info(f"SonarQube webhook: project={project}, status={gate_status}")
        log.debug(f"Full SonarQube payload: {json.dumps(payload, indent=2)[:500]}")

        if gate_status == "OK":
            log.info("Quality gate passed. No action needed.")
            return

        conditions = payload.get("qualityGate", {}).get("conditions", [])
        failed = [c for c in conditions if c.get("status") == "ERROR"]
        if not failed:
            log.info("No failed conditions found.")
            return

        failures_summary = ", ".join(
            f"{c.get('metric', '?')}={c.get('value', '?')} (threshold: {c.get('errorThreshold', '?')})"
            for c in failed
        )
        log.info(f"Failed conditions: {failures_summary}")

        prompt = (
            f"The SonarQube quality gate FAILED for project '{project}'. "
            f"Failed conditions: {failures_summary}. "
            f"Fetch the specific issues, fix them, verify with cargo fmt, clippy, and test. "
            f"Then commit and push: git add -A && git commit -m 'fix: resolve SonarQube failures (auto-fix)' && git push origin main"
        )

        for attempt in range(1, MAX_RETRIES + 1):
            log.info(f"=== SonarQube fix attempt {attempt}/{MAX_RETRIES} ===")
            if run_kiro(prompt, f"SonarQube fix attempt {attempt}"):
                log.info(f"✓ Completed on attempt {attempt}")
                return

        log.error(f"Failed after {MAX_RETRIES} attempts.")

    def _handle_ci_failure(self, payload):
        job = payload.get("job", "unknown")
        step = payload.get("step", "unknown")
        error_log = payload.get("error_log", "No error details provided")

        log.info(f"CI failure webhook: job={job}, step={step}")
        log.debug(f"Full CI payload: {json.dumps(payload, indent=2)}")
        log.info(f"Error: {error_log[:200]}")
        fix_with_retry(job, step, error_log)

    def _handle_ci_improve(self, payload):
        focus = payload.get("focus", "general")
        pipeline_report = payload.get("pipeline_report", "")
        run_url = payload.get("run_url", "")

        log.info(f"CI improve webhook: focus={focus}")
        log.debug(f"Full CI improve payload: {json.dumps(payload, indent=2)[:1000]}")
        improve_pipeline(focus, pipeline_report, run_url)

    def _handle_sonar_fix(self, payload):
        sonar_report = payload.get("sonar_report", "")
        run_url = payload.get("run_url", "")

        log.info("SonarQube fix webhook received")
        log.debug(f"Sonar fix payload: {json.dumps(payload, indent=2)[:1000]}")
        fix_sonar_issues(sonar_report, run_url)

    def log_message(self, format, *args):
        pass


def main():
    local_ip = get_local_ip()
    server = HTTPServer(("0.0.0.0", PORT), WebhookHandler)
    log.info(f"Quality webhook listener running on port {PORT}")
    log.info(f"Max retries: {MAX_RETRIES} | Timeout: {KIRO_TIMEOUT // 60}min")
    log.info(f"Trust all tools: {TRUST_ALL_TOOLS}")
    log.info(f"Endpoints:")
    log.info(f"  SonarQube:  http://{local_ip}:{PORT}/sonarqube")
    log.info(f"  CI failure: http://{local_ip}:{PORT}/ci-failure")
    log.info(f"  CI improve: http://{local_ip}:{PORT}/ci-improve")
    log.info(f"  Sonar fix:  http://{local_ip}:{PORT}/sonar-fix")
    log.info(f"Project dir: {PROJECT_DIR}")
    log.info(f"WSL: {WSL_DISTRO} (user: {WSL_USER})")
    log.info("Waiting for webhooks...")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        log.info("Shutting down...")
        server.server_close()


if __name__ == "__main__":
    main()
