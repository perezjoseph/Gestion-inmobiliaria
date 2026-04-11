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
KIRO_TIMEOUT = 900
TRUSTED_TOOLS = "fs_read,fs_write,execute_bash,grep,glob,code,introspect,session,use_subagent,web_fetch,web_search"


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
    return [
        "wsl", "-u", WSL_USER, "-d", WSL_DISTRO,
        "bash", "-lc",
        f"cd {PROJECT_DIR} && {command}"
    ]


def run_kiro(prompt, label):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(f"  [{timestamp}] Triggering kiro-cli for: {label}")

    escaped = prompt.replace('"', '\\"').replace("'", "'\\''")
    cmd = f'kiro-cli chat --trust-tools={TRUSTED_TOOLS} --agent ci-fixer --no-interactive "{escaped}"'

    proc = subprocess.Popen(
        wsl_cmd(cmd),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace"
    )

    try:
        stdout, stderr = proc.communicate(timeout=KIRO_TIMEOUT)
        stdout = stdout or ""
        stderr = stderr or ""
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.communicate()
        print(f"  ⚠ kiro-cli timed out after {KIRO_TIMEOUT // 60} minutes")
        return False

    print(f"  kiro-cli exit code: {proc.returncode}")
    if stdout:
        print(f"  stdout (last 500 chars): ...{stdout[-500:]}")
    if stderr:
        print(f"  stderr (last 300 chars): ...{stderr[-300:]}")

    if "Tool approval required" in stderr or "Tool approval required" in stdout:
        print(f"  ⚠ Tool approval error detected")
        return False

    if "denied list" in stderr or "denied list" in stdout:
        print(f"  ⚠ Tool denied in non-interactive mode")
        return False

    return True


def fix_with_retry(job, step, error_log):
    for attempt in range(1, MAX_RETRIES + 1):
        print(f"\n  === Attempt {attempt}/{MAX_RETRIES} for {job}/{step} ===")

        prompt_parts = [
            f"The CI pipeline FAILED in job '{job}', step '{step}'.",
            f"Error output:\n{error_log}\n",
        ]

        job_instructions = {
            "lint": "Fix the formatting and clippy warnings.",
            "test-backend": "Fix the failing backend tests.",
            "test-frontend": "Fix the failing frontend tests.",
            "quality-gate": "Fix the quality gate failures (dependency audit, unused deps, or OWASP issues).",
            "commit-lint": "The commit message format is wrong. This cannot be auto-fixed.",
            "sonarqube": "Fix the SonarQube analysis failures.",
        }
        prompt_parts.append(job_instructions.get(job, "Analyze the error and fix the issue."))

        prompt_parts.append(
            "After fixing, verify with cargo fmt --all, cargo clippy --workspace -- -D warnings, "
            "and cargo test --workspace. Then commit and push: "
            "git add -A && git commit -m 'fix: resolve CI failures (auto-fix)' && git push origin main"
        )

        if run_kiro(" ".join(prompt_parts), f"CI fix ({job}/{step}) attempt {attempt}"):
            print(f"  ✓ Attempt {attempt} completed")
            return True

        print(f"  ✗ Attempt {attempt} failed")

    print(f"  ✗ Failed after {MAX_RETRIES} attempts. Manual intervention needed.")
    return False


class WebhookHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length)

        try:
            payload = json.loads(body)
        except json.JSONDecodeError:
            self.send_response(400)
            self.end_headers()
            self.wfile.write(b"Invalid JSON")
            return

        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"OK")

        if self.path == "/sonarqube":
            threading.Thread(target=self._handle_sonarqube, args=(payload,), daemon=True).start()
        elif self.path == "/ci-failure":
            threading.Thread(target=self._handle_ci_failure, args=(payload,), daemon=True).start()
        else:
            print(f"  Unknown endpoint: {self.path}")

    def _handle_sonarqube(self, payload):
        project = payload.get("project", {}).get("key", "unknown")
        gate_status = payload.get("qualityGate", {}).get("status", "unknown")
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

        print(f"\n[{timestamp}] SonarQube webhook: project={project}, status={gate_status}")

        if gate_status == "OK":
            print("  Quality gate passed. No action needed.")
            return

        conditions = payload.get("qualityGate", {}).get("conditions", [])
        failed = [c for c in conditions if c.get("status") == "ERROR"]
        if not failed:
            print("  No failed conditions found.")
            return

        failures_summary = ", ".join(
            f"{c.get('metric', '?')}={c.get('value', '?')} (threshold: {c.get('errorThreshold', '?')})"
            for c in failed
        )
        print(f"  Failed conditions: {failures_summary}")

        prompt = (
            f"The SonarQube quality gate FAILED for project '{project}'. "
            f"Failed conditions: {failures_summary}. "
            f"Fetch the specific issues, fix them, verify with cargo fmt, clippy, and test. "
            f"Then commit and push: git add -A && git commit -m 'fix: resolve SonarQube failures (auto-fix)' && git push origin main"
        )

        for attempt in range(1, MAX_RETRIES + 1):
            print(f"\n  === SonarQube fix attempt {attempt}/{MAX_RETRIES} ===")
            if run_kiro(prompt, f"SonarQube fix attempt {attempt}"):
                print(f"  ✓ Completed on attempt {attempt}")
                return

        print(f"  ✗ Failed after {MAX_RETRIES} attempts.")

    def _handle_ci_failure(self, payload):
        job = payload.get("job", "unknown")
        step = payload.get("step", "unknown")
        error_log = payload.get("error_log", "No error details provided")
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

        print(f"\n[{timestamp}] CI failure webhook: job={job}, step={step}")
        print(f"  Error: {error_log[:200]}")
        fix_with_retry(job, step, error_log)

    def log_message(self, format, *args):
        pass


def main():
    local_ip = get_local_ip()
    server = HTTPServer(("0.0.0.0", PORT), WebhookHandler)
    print(f"Quality webhook listener running on port {PORT}")
    print(f"Max retries: {MAX_RETRIES} | Timeout: {KIRO_TIMEOUT // 60}min")
    print(f"Endpoints:")
    print(f"  SonarQube:  http://{local_ip}:{PORT}/sonarqube")
    print(f"  CI failure: http://{local_ip}:{PORT}/ci-failure")
    print(f"Project dir: {PROJECT_DIR}")
    print(f"WSL: {WSL_DISTRO} (user: {WSL_USER})")
    print("Waiting for webhooks...\n")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down...")
        server.server_close()


if __name__ == "__main__":
    main()
