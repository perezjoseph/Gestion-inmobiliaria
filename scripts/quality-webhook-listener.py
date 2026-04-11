#!/usr/bin/env python3
"""
Quality Check Webhook Listener
Receives webhooks from SonarQube and GitHub Actions CI pipeline,
then triggers kiro-cli to auto-fix issues in a retry loop until
all checks pass.

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


def run_shell(command, timeout=120):
    try:
        result = subprocess.run(
            wsl_cmd(command), capture_output=True, text=True, timeout=timeout
        )
        return result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return -1, "", "Timed out"
    except Exception as e:
        return -1, "", str(e)


def run_kiro(prompt, label):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(f"  [{timestamp}] Triggering kiro-cli for: {label}")

    escaped = prompt.replace('"', '\\"').replace("'", "'\\''")
    
    cmd_variants = [
        f'kiro-cli chat --no-interactive -a "{escaped}"',
        f'kiro-cli chat --no-interactive --agent ci-fixer -a "{escaped}"',
    ]

    for i, cmd in enumerate(cmd_variants):
        print(f"  Trying command variant {i + 1}/{len(cmd_variants)}")
        code, stdout, stderr = run_shell(cmd, timeout=600)
        
        print(f"  kiro-cli exit code: {code}")
        if stdout:
            print(f"  stdout (last 500 chars): ...{stdout[-500:]}")
        if stderr:
            print(f"  stderr (last 300 chars): ...{stderr[-300:]}")

        if "Tool approval required" in stderr or "Tool approval required" in stdout:
            print(f"  ⚠ Tool approval error detected, trying next variant...")
            continue
        
        return code == 0
    
    print(f"  ✗ All command variants failed with tool approval errors")
    return False


def verify_and_push(job):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

    checks = {
        "lint": [
            ("cargo fmt --all -- --check", "Formatting check"),
            ("cargo clippy --workspace -- -D warnings", "Clippy check"),
        ],
        "test-backend": [
            ("cargo test -p realestate-backend --all-targets", "Backend tests"),
        ],
        "test-frontend": [
            ("cargo test -p realestate-frontend --all-targets", "Frontend tests"),
        ],
        "quality-gate": [
            ("cargo audit", "Dependency audit"),
        ],
    }

    verification_cmds = checks.get(job, [("cargo check --workspace", "Compile check")])

    for cmd, label in verification_cmds:
        print(f"  [{timestamp}] Verifying: {label}")
        code, stdout, stderr = run_shell(cmd)
        if code != 0:
            print(f"  ✗ {label} still failing")
            return False
        print(f"  ✓ {label} passed")

    print(f"  [{timestamp}] All checks passed. Committing and pushing...")
    run_shell("git add -A")
    code, _, _ = run_shell(
        'git diff --cached --quiet || git commit -m "fix: resolve CI pipeline failures (auto-fix)"'
    )
    code, stdout, stderr = run_shell("git push origin main")
    if code == 0:
        print(f"  ✓ Pushed successfully")
        return True
    else:
        print(f"  ✗ Push failed: {stderr[:200]}")
        return False


def fix_with_retry(job, step, error_log):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

    for attempt in range(1, MAX_RETRIES + 1):
        print(f"\n  === Attempt {attempt}/{MAX_RETRIES} for {job}/{step} ===")

        prompt_parts = [
            f"The CI pipeline FAILED in job '{job}', step '{step}'.",
            f"Error output:\n{error_log}\n",
        ]

        if job == "lint":
            prompt_parts.append(
                "Fix the formatting and clippy warnings. "
                "Run cargo fmt --all and cargo clippy --workspace -- -D warnings to verify."
            )
        elif job == "test-backend":
            prompt_parts.append(
                "Fix the failing backend tests. "
                "Run cargo test -p realestate-backend --all-targets to verify."
            )
        elif job == "test-frontend":
            prompt_parts.append(
                "Fix the failing frontend tests. "
                "Run cargo test -p realestate-frontend --all-targets to verify."
            )
        elif job == "quality-gate":
            prompt_parts.append(
                "Fix the quality gate failures (dependency audit, unused deps, or OWASP issues). "
                "Run cargo audit and check Cargo.toml dependencies."
            )
        else:
            prompt_parts.append(
                "Analyze the error and fix the issue. "
                "Run cargo check --workspace to verify the fix compiles."
            )

        prompt_parts.append(
            "After fixing, run cargo fmt, cargo clippy, and cargo test to verify everything passes. "
            "Do NOT commit or push — just fix the code."
        )

        prompt = " ".join(prompt_parts)
        kiro_ok = run_kiro(prompt, f"CI fix ({job}/{step}) attempt {attempt}")

        if verify_and_push(job):
            print(f"  ✓ Fixed and pushed on attempt {attempt}")
            return True

        print(f"  ✗ Attempt {attempt} did not fully resolve the issue")

    print(f"  ✗ Failed to fix {job}/{step} after {MAX_RETRIES} attempts. Manual intervention needed.")
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
            threading.Thread(
                target=self._handle_sonarqube, args=(payload,), daemon=True
            ).start()
        elif self.path == "/ci-failure":
            threading.Thread(
                target=self._handle_ci_failure, args=(payload,), daemon=True
            ).start()
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
            f"Use the SonarQube MCP tools to fetch the specific issues, "
            f"then fix them in the codebase. After fixing, run cargo fmt, "
            f"cargo clippy, and cargo test to verify. "
            f"Do NOT commit or push — just fix the code."
        )

        for attempt in range(1, MAX_RETRIES + 1):
            print(f"\n  === SonarQube fix attempt {attempt}/{MAX_RETRIES} ===")
            run_kiro(prompt, f"SonarQube fix attempt {attempt}")

            if verify_and_push("quality-gate"):
                print(f"  ✓ SonarQube issues fixed and pushed on attempt {attempt}")
                return

        print(f"  ✗ Failed to fix SonarQube issues after {MAX_RETRIES} attempts.")

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
    print(f"Max retries per failure: {MAX_RETRIES}")
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
