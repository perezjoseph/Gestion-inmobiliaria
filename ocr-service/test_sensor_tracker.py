"""Unit tests for the per-stack sensor-ran tracker hook.

Validates: Requirements 3.1
"""

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

pytestmark = pytest.mark.skipif(sys.platform == "win32", reason="bash required")

TRACKER_SCRIPT = r"""
CMD=$(cat | jq -r .tool_input.command 2>/dev/null)
DIR="${SENSOR_DIR}"
mkdir -p "$DIR"
case "$CMD" in
  *"cargo clippy"*|*"cargo test"*|*"cargo fmt"*) touch "$DIR/rust" ;;
  *"npm run build"*|*"npm test"*|*"npx eslint"*) touch "$DIR/ts" ;;
  *"gradlew build"*|*"gradlew test"*) touch "$DIR/kotlin" ;;
  *"pytest"*|*"ruff check"*|*"ruff format"*) touch "$DIR/python" ;;
  *hadolint*) touch "$DIR/docker" ;;
  *kubeconform*) touch "$DIR/k8s" ;;
  *shellcheck*) touch "$DIR/shell" ;;
esac
"""


def run_tracker(command: str, sensor_dir: str) -> subprocess.CompletedProcess:
    payload = json.dumps({"tool_input": {"command": command}})
    env = {**os.environ, "SENSOR_DIR": sensor_dir}
    return subprocess.run(
        ["bash", "-c", TRACKER_SCRIPT],
        input=payload,
        capture_output=True,
        text=True,
        timeout=10,
        env=env,
    )


def markers_in(sensor_dir: str) -> set[str]:
    return {f.name for f in Path(sensor_dir).iterdir() if f.is_file()}


class TestRustMarker:
    def test_cargo_clippy(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("cargo clippy --locked -p realestate-backend -- -D warnings", d)
        assert "rust" in markers_in(d)

    def test_cargo_test(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("cargo test --workspace", d)
        assert "rust" in markers_in(d)

    def test_cargo_fmt(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("cargo fmt -- --check", d)
        assert "rust" in markers_in(d)


class TestTsMarker:
    def test_npm_run_build(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("npm run build", d)
        assert "ts" in markers_in(d)

    def test_npm_test(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("npm test", d)
        assert "ts" in markers_in(d)

    def test_npx_eslint(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("npx eslint src/ --fix", d)
        assert "ts" in markers_in(d)


class TestKotlinMarker:
    def test_gradlew_build(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("./gradlew build", d)
        assert "kotlin" in markers_in(d)

    def test_gradlew_test(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("./gradlew test", d)
        assert "kotlin" in markers_in(d)


class TestPythonMarker:
    def test_pytest(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("python -m pytest -q", d)
        assert "python" in markers_in(d)

    def test_ruff_check(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("ruff check .", d)
        assert "python" in markers_in(d)

    def test_ruff_format(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("ruff format --check .", d)
        assert "python" in markers_in(d)


class TestDockerMarker:
    def test_hadolint(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("hadolint Dockerfile", d)
        assert "docker" in markers_in(d)


class TestK8sMarker:
    def test_kubeconform(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("kubeconform -strict -ignore-missing-schemas infra/k8s/app/", d)
        assert "k8s" in markers_in(d)


class TestShellMarker:
    def test_shellcheck(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("shellcheck script.sh", d)
        assert "shell" in markers_in(d)


class TestUnrecognizedCommand:
    def test_echo_creates_no_marker(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("echo hello", d)
        assert markers_in(d) == set()

    def test_ls_creates_no_marker(self, tmp_path):
        d = str(tmp_path / "sensors")
        run_tracker("ls -la", d)
        assert markers_in(d) == set()
