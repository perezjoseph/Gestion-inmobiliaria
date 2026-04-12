#!/usr/bin/env python3
"""
Quality Check Webhook Listener
Receives webhooks from SonarQube and GitHub Actions CI pipeline,
then triggers kiro-cli to auto-fix issues.

Endpoints:
    POST /sonarqube    — SonarQube quality gate webhook
    POST /ci-failure   — GitHub Actions pipeline failure webhook

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
