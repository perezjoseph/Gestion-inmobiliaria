"""Unit tests for flaky test detection edge cases.

Requirements: 12.1, 12.4, 12.5, 12.7
"""

import json
import pathlib
import tempfile
from unittest.mock import patch

import pytest

from scripts.quality_webhook.flaky import (
    FLAKINESS_THRESHOLD,
    MIN_RUNS_FOR_FLAKY,
    MAX_OUTCOMES_PER_TEST,
    parse_test_results,
    record_test_outcomes,
    are_all_failures_flaky,
    get_flaky_tests,
    get_flaky_test_summary,
    _load_flaky_data,
)


@pytest.fixture()
def tmp_flaky():
    tmp = tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False, encoding="utf-8",
    )
    tmp.write("{}")
    tmp.close()
    path = pathlib.Path(tmp.name)
    with patch("scripts.quality_webhook.flaky._FLAKY_TESTS_FILE", path):
        yield path
    path.unlink(missing_ok=True)


def test_parse_mixed_pass_fail_output(tmp_flaky):
    error_log = (
        "running 4 tests\n"
        "test auth::test_login ... ok\n"
        "test auth::test_logout ... FAILED\n"
        "test db::test_connect ... ok\n"
        "test db::test_query ... FAILED\n"
    )
    results = parse_test_results(error_log)
    assert results == {
        "auth::test_login": True,
        "auth::test_logout": False,
        "db::test_connect": True,
        "db::test_query": False,
    }


def test_flakiness_score_at_exactly_threshold_with_min_runs(tmp_flaky):
    test_name = "mod::test_boundary"
    failures_needed = int(FLAKINESS_THRESHOLD * MIN_RUNS_FOR_FLAKY)
    passes_needed = MIN_RUNS_FOR_FLAKY - failures_needed

    outcomes = {}
    for _ in range(passes_needed):
        outcomes = {test_name: True}
        record_test_outcomes("test-backend", outcomes)
    for _ in range(failures_needed):
        outcomes = {test_name: False}
        record_test_outcomes("test-backend", outcomes)

    data = _load_flaky_data()
    entry = data["tests"][test_name]
    score = entry["flakiness_score"]

    assert entry["run_count"] == MIN_RUNS_FOR_FLAKY
    expected_score = failures_needed / MIN_RUNS_FOR_FLAKY
    assert abs(score - expected_score) < 1e-9

    if expected_score > FLAKINESS_THRESHOLD:
        assert entry["is_flaky"] is True
    else:
        assert entry["is_flaky"] is False


def test_are_all_failures_flaky_returns_false_when_one_not_flaky(tmp_flaky):
    flaky_test = "mod::test_flaky_one"
    for i in range(MIN_RUNS_FOR_FLAKY):
        passed = i % 2 == 0
        record_test_outcomes("test-backend", {flaky_test: passed})

    data = _load_flaky_data()
    assert data["tests"][flaky_test]["is_flaky"] is True

    error_log = (
        "test mod::test_flaky_one ... FAILED\n"
        "test mod::test_unknown_new ... FAILED\n"
    )
    all_flaky, names = are_all_failures_flaky(error_log)
    assert all_flaky is False


def test_empty_error_log_returns_no_results(tmp_flaky):
    results = parse_test_results("")
    assert results == {}


def test_empty_error_log_are_all_failures_flaky(tmp_flaky):
    all_flaky, names = are_all_failures_flaky("")
    assert all_flaky is False
    assert names == []


def test_get_flaky_tests_returns_only_flaky(tmp_flaky):
    flaky_test = "mod::test_intermittent"
    stable_test = "mod::test_stable"

    for i in range(MIN_RUNS_FOR_FLAKY):
        record_test_outcomes("test-backend", {
            flaky_test: i % 2 == 0,
            stable_test: True,
        })

    flaky_list = get_flaky_tests()
    flaky_names = [r.test_name for r in flaky_list]
    assert flaky_test in flaky_names
    assert stable_test not in flaky_names


def test_get_flaky_test_summary_structure(tmp_flaky):
    for i in range(MIN_RUNS_FOR_FLAKY):
        record_test_outcomes("test-backend", {"mod::test_a": i % 2 == 0})

    summary = get_flaky_test_summary()
    assert "total_tracked" in summary
    assert "flaky_count" in summary
    assert "tests" in summary
    assert summary["total_tracked"] >= 1
    assert summary["flaky_count"] >= 1


def test_outcomes_trimmed_to_max(tmp_flaky):
    test_name = "mod::test_trim"
    for i in range(MAX_OUTCOMES_PER_TEST + 10):
        record_test_outcomes("test-backend", {test_name: True})

    data = _load_flaky_data()
    entry = data["tests"][test_name]
    assert entry["run_count"] == MAX_OUTCOMES_PER_TEST


def test_insufficient_runs_not_flagged_flaky(tmp_flaky):
    test_name = "mod::test_few_runs"
    for _ in range(MIN_RUNS_FOR_FLAKY - 1):
        record_test_outcomes("test-backend", {test_name: False})

    data = _load_flaky_data()
    entry = data["tests"][test_name]
    assert entry["is_flaky"] is False
    assert entry["flakiness_score"] == 1.0
    assert entry["run_count"] < MIN_RUNS_FOR_FLAKY


def test_are_all_failures_flaky_returns_true_when_all_flaky(tmp_flaky):
    flaky_a = "mod::test_flaky_a"
    flaky_b = "mod::test_flaky_b"

    for i in range(MIN_RUNS_FOR_FLAKY):
        record_test_outcomes("test-backend", {
            flaky_a: i % 2 == 0,
            flaky_b: i % 3 == 0,
        })

    data = _load_flaky_data()
    assert data["tests"][flaky_a]["is_flaky"] is True
    assert data["tests"][flaky_b]["is_flaky"] is True

    error_log = (
        "test mod::test_flaky_a ... FAILED\n"
        "test mod::test_flaky_b ... FAILED\n"
    )
    all_flaky, names = are_all_failures_flaky(error_log)
    assert all_flaky is True
    assert set(names) == {flaky_a, flaky_b}
