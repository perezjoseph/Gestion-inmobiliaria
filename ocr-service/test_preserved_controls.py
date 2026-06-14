"""Regression test for preserved existing controls.

**Validates: Requirements 14.1**

Property 6: No regression — every pre-existing deniedPaths entry, the git-block
guard, the auto-format behavior, and the max-2-block escape hatch are preserved
unchanged.
"""

import json
from pathlib import Path

AUTOFIX_JSON = Path(__file__).resolve().parent.parent / ".kiro" / "agents" / "autofix.json"


def load_config():
    return json.loads(AUTOFIX_JSON.read_text())


def test_pre_existing_denied_paths_preserved():
    config = load_config()
    denied = config["toolsSettings"]["write"]["deniedPaths"]
    required = [
        ".github/workflows/**",
        ".github/actions/**",
        ".kiro/agents/**",
        ".kiro/steering/**",
        ".kiro/skills/**",
        "infra/**",
        "Cargo.toml",
        "backend/Cargo.toml",
        "frontend/Cargo.toml",
        "Cargo.lock",
    ]
    for p in required:
        assert p in denied, f"Pre-existing deniedPath '{p}' is missing"


def test_git_block_guard_present():
    config = load_config()
    pre = config["hooks"]["preToolUse"]
    git_guards = [
        h for h in pre
        if h.get("matcher") == "execute_bash" and "git" in h["command"]
    ]
    assert len(git_guards) >= 1, "No git-block guard found in preToolUse hooks"
    guard = git_guards[0]
    assert "commit|push|checkout|reset|rebase|merge" in guard["command"]
    assert "exit 2" in guard["command"]


def test_auto_format_hooks_present():
    config = load_config()
    post = config["hooks"]["postToolUse"]
    formatters = [
        h for h in post
        if h.get("matcher") in ("fs_write", "str_replace")
        and "cargo fmt" in h["command"]
    ]
    assert len(formatters) >= 2, (
        f"Expected at least 2 auto-format hooks (fs_write + str_replace), got {len(formatters)}"
    )
    for fmt in formatters:
        assert "cargo fmt" in fmt["command"]
        assert "eslint --fix" in fmt["command"]
        assert "spotlessApply" in fmt["command"]
        assert "ruff format" in fmt["command"]


def test_max_two_block_escape_hatch():
    config = load_config()
    stop = config["hooks"]["stop"]
    assert len(stop) >= 1, "No stop hook found"
    gate = stop[0]["command"]
    assert "-ge 2" in gate, "Max-2-block escape hatch condition not found"
    assert "cleanup" in gate, "Cleanup function not found in stop gate"
