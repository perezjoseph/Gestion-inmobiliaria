#!/usr/bin/env python3
"""
Quality Check Webhook Listener
Receives webhooks from SonarQube and GitHub Actions CI pipeline,
then triggers kiro-cli to auto-fix issues when quality checks fail.

Endpoints:
    POST /sonarqube   — SonarQube quality gate webhook
    POST /ci-failure   — GitHub Actions pipeline failure webhook

Usage:
    python scripts/quality-webhook-listener.py

SonarQube webhook URL:  http://<this-machine-ip>:9090/sonarqube
CI failure webhook URL: http://<this-machine-ip>:9090/ci-failure
"""

import json
import subprocess
import threading
from http.server import HTTPServer, BaseHTTPRequestHandler
from datetime import datetime

PORT = 9090
PROJECT_DIR = "/mnt/d/realestate"
WSL_DISTRO = "Ubuntu-22.04"
WSL_USER = "jperez"


def run_kiro(prompt, label):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(f"  [{timestamp}] Triggering kiro-cli for: {label}")

    escaped = prompt.replace('"', '\\"')
    cmd = [
        "wsl", "-u", WSL_USER, "-d", WSL_DISTRO,
        "bash", "-lc",
        f'cd {PROJECT_DIR} && kiro-cli chat --no-interactive --trust-all-tools "{escaped}"'
    ]

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
        print(f"  kiro-cli exit code: {result.returncode}")
        if result.stdout:
            print(f"  stdout (last 500 chars): ...{result.stdout[-500:]}")
        if result.returncode != 0 and result.stderr:
            print(f"  stderr: {result.stderr[-300:]}")
    except subprocess.TimeoutExpired:
        print("  kiro-cli timed out after 10 minutes")
    except Exception as e:
        print(f"  Error running kiro-cli: {e}")


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
            f"Commit the fixes with message 'fix: resolve SonarQube quality gate failures'."
        )
        run_kiro(prompt, "SonarQube quality gate fix")

    def _handle_ci_failure(self, payload):
        job = payload.get("job", "unknown")
        step = payload.get("step", "unknown")
        error_log = payload.get("error_log", "No error details provided")
        run_url = payload.get("run_url", "")
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

        print(f"\n[{timestamp}] CI failure webhook: job={job}, step={step}")
        print(f"  Error: {error_log[:200]}")

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
            "Commit the fixes with message 'fix: resolve CI pipeline failures'."
        )

        run_kiro(" ".join(prompt_parts), f"CI fix ({job}/{step})")

    def log_message(self, format, *args):
        pass


def main():
    server = HTTPServer(("0.0.0.0", PORT), WebhookHandler)
    print(f"Quality webhook listener running on port {PORT}")
    print(f"Endpoints:")
    print(f"  SonarQube:  http://<this-machine-ip>:{PORT}/sonarqube")
    print(f"  CI failure: http://<this-machine-ip>:{PORT}/ci-failure")
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
