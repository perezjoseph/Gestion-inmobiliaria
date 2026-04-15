"""Unit tests for cross-job anomaly correlation edge cases.

Requirements: 14.1, 14.2, 14.5, 14.7
"""

import json
import pathlib
import tempfile
import time
from unittest.mock import patch

import pytest

from scripts.quality_webhook.correlation import (
    CORRELATION_WINDOW_S,
    MIN_FAILURES_FOR_CORRELATION,
    register_failure,
    check_correlation,
    should_dispatch_correlated_fix,
    build_correlated_fix_prompt,
    track_cache_health,
    get_correlation_summary,
    _groups,
    _group_created_at,
    _lock,
)
import scripts.quality_webhook.correlation as correlation_mod


@pytest.fixture(autouse=True)
def reset_correlation_state():
    with _lock:
        _groups.clear()
        _group_created_at.clear()
    correlation_mod._group_counter = 0
    yield
    with _lock:
        _groups.clear()
        _group_created_at.clear()
    correlation_mod._group_counter = 0


@pytest.fixture()
def tmp_history():
    tmp = tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False, encoding="utf-8",
    )
    tmp.write("{}")
    tmp.close()
    path = pathlib.Path(tmp.name)
    with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", path):
        yield path
    path.unlink(missing_ok=True)


COMMIT_A = "a" * 40
COMMIT_B = "b" * 40


def test_two_failures_same_commit_within_60s():
    base = time.monotonic()
    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base
        gid1 = register_failure("lint", "fmt", "error in backend/src/main.rs", COMMIT_A, "code_quality")

    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base + 30
        gid2 = register_failure("test-backend", "test", "error in backend/src/main.rs", COMMIT_A, "test_failure")

    assert gid1 == gid2
    group = check_correlation(gid1)
    assert group is not None
    assert len(group.failures) == 2


def test_no_grouping_different_commits():
    base = time.monotonic()
    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base
        gid1 = register_failure("lint", "fmt", "error", COMMIT_A, "code_quality")

    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base + 5
        gid2 = register_failure("test-backend", "test", "error", COMMIT_B, "test_failure")

    assert gid1 != gid2


def test_no_grouping_time_gap_exceeds_60s():
    base = time.monotonic()
    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base
        gid1 = register_failure("lint", "fmt", "error", COMMIT_A, "code_quality")

    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base + CORRELATION_WINDOW_S + 1
        gid2 = register_failure("test-backend", "test", "error", COMMIT_A, "test_failure")

    assert gid1 != gid2
    group1 = check_correlation(gid1)
    group2 = check_correlation(gid2)
    assert len(group1.failures) == 1
    assert len(group2.failures) == 1


def test_shared_root_cause_common_file_paths():
    base = time.monotonic()
    shared_file = "backend/src/handlers/auth.rs"
    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base
        gid = register_failure(
            "lint", "clippy",
            f"error in {shared_file}: unused variable",
            COMMIT_A, "code_quality",
        )

    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base + 10
        register_failure(
            "test-backend", "test",
            f"test failed at {shared_file}: assertion error",
            COMMIT_A, "test_failure",
        )

    group = check_correlation(gid)
    assert group is not None
    assert shared_file in group.shared_files
    assert group.shared_root_cause is not None
    assert "shared files" in group.shared_root_cause


def test_fallback_independent_fixes_no_shared_root_cause():
    base = time.monotonic()
    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base
        gid = register_failure(
            "lint", "fmt",
            "formatting error in some code",
            COMMIT_A, "code_quality",
        )

    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base + 10
        register_failure(
            "test-backend", "test",
            "network timeout during test",
            COMMIT_A, "test_failure",
        )

    group = check_correlation(gid)
    assert group is not None
    assert not group.shared_files
    assert group.shared_root_cause is None
    assert should_dispatch_correlated_fix(gid) is False


def test_should_dispatch_requires_min_failures_and_root_cause():
    base = time.monotonic()
    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base
        gid = register_failure("lint", "fmt", "error", COMMIT_A, "code_quality")

    assert should_dispatch_correlated_fix(gid) is False

    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base + 5
        register_failure(
            "test-backend", "test",
            "error in backend/src/main.rs",
            COMMIT_A, "code_quality",
        )

    check_correlation(gid)
    has_root = should_dispatch_correlated_fix(gid)
    group = _groups.get(gid)
    if group and group.shared_root_cause:
        assert has_root is True
    else:
        assert has_root is False


def test_build_correlated_fix_prompt_content():
    base = time.monotonic()
    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base
        gid = register_failure(
            "lint", "clippy",
            "error in backend/src/main.rs: unused import",
            COMMIT_A, "code_quality",
        )

    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base + 5
        register_failure(
            "test-backend", "test",
            "error in backend/src/main.rs: test failed",
            COMMIT_A, "test_failure",
        )

    group = check_correlation(gid)
    prompt = build_correlated_fix_prompt(group)
    assert "lint" in prompt
    assert "test-backend" in prompt
    assert COMMIT_A[:8] in prompt
    assert "CORRELATED CI FAILURES" in prompt


def test_cache_health_degradation_detection(tmp_history):
    for _ in range(3):
        track_cache_health("lint", False)

    summary = get_correlation_summary()
    cache = summary["cache_health"]
    assert "lint" in cache
    assert cache["lint"]["consecutive_misses"] == 3

    data = json.loads(tmp_history.read_text(encoding="utf-8"))
    entry = data["cache_health"]["lint"]
    assert entry["degraded"] is True
    assert entry["hit_rate"] == 0.0


def test_cache_health_no_degradation_with_hits(tmp_history):
    for _ in range(5):
        track_cache_health("test-backend", True)

    data = json.loads(tmp_history.read_text(encoding="utf-8"))
    entry = data["cache_health"]["test-backend"]
    assert entry["degraded"] is False
    assert entry["hit_rate"] == 1.0
    assert entry["consecutive_misses"] == 0


def test_cache_health_mixed_runs(tmp_history):
    track_cache_health("lint", True)
    track_cache_health("lint", False)
    track_cache_health("lint", True)

    data = json.loads(tmp_history.read_text(encoding="utf-8"))
    entry = data["cache_health"]["lint"]
    assert entry["hit_rate"] == pytest.approx(0.67, abs=0.01)
    assert entry["degraded"] is False
    assert entry["consecutive_misses"] == 0


def test_correlation_summary_structure():
    summary = get_correlation_summary()
    assert "active_groups" in summary
    assert "cache_health" in summary
    assert isinstance(summary["active_groups"], int)
    assert isinstance(summary["cache_health"], dict)


def test_single_failure_not_dispatched():
    base = time.monotonic()
    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base
        gid = register_failure("lint", "fmt", "error", COMMIT_A, "code_quality")

    assert should_dispatch_correlated_fix(gid) is False


def test_prune_stale_groups():
    base = time.monotonic()
    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base
        gid = register_failure("lint", "fmt", "error", COMMIT_A, "code_quality")

    assert gid is not None
    assert gid in _groups

    with patch("scripts.quality_webhook.correlation.time") as mt:
        mt.monotonic.return_value = base + 700
        register_failure("test-backend", "test", "error", COMMIT_B, "test_failure")

    assert gid not in _groups
