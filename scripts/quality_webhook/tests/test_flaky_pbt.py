"""Property-based tests for flaky test detection.

Feature: ci-pipeline-optimization, Property 10: Flaky test detection correctly classifies tests by flakiness score
"""

import json
import tempfile
import pathlib
from unittest.mock import patch

from hypothesis import given, settings, strategies as st

from scripts.quality_webhook.flaky import (
    MAX_OUTCOMES_PER_TEST,
    FLAKINESS_THRESHOLD,
    MIN_RUNS_FOR_FLAKY,
    record_test_outcomes,
    _load_flaky_data,
)


def _make_flaky_file():
    tmp = tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False, encoding="utf-8",
    )
    tmp.write("{}")
    tmp.close()
    return pathlib.Path(tmp.name)


@given(
    outcomes=st.lists(st.booleans(), min_size=1, max_size=25),
)
@settings(max_examples=200, deadline=None)
def test_flakiness_score_equals_failure_ratio(outcomes):
    """**Validates: Requirements 12.3**

    For any sequence of pass/fail outcomes, flakiness_score equals
    failures / total over the stored history (trimmed to MAX_OUTCOMES_PER_TEST).
    """
    tmp_path = _make_flaky_file()
    try:
        with patch("scripts.quality_webhook.flaky._FLAKY_TESTS_FILE", tmp_path):
            test_name = "mod::test_example"
            for passed in outcomes:
                record_test_outcomes("test-backend", {test_name: passed})

            data = _load_flaky_data()
            entry = data["tests"][test_name]

            stored_outcomes = outcomes[-MAX_OUTCOMES_PER_TEST:]
            total = len(stored_outcomes)
            failures = sum(1 for p in stored_outcomes if not p)
            expected_score = failures / total

            assert abs(entry["flakiness_score"] - expected_score) < 1e-9
            assert entry["run_count"] == total
    finally:
        tmp_path.unlink(missing_ok=True)


@given(
    outcomes=st.lists(st.booleans(), min_size=1, max_size=25),
)
@settings(max_examples=200, deadline=None)
def test_is_flaky_flag_matches_threshold_and_min_runs(outcomes):
    """**Validates: Requirements 12.4, 12.7**

    is_flaky is True iff score > FLAKINESS_THRESHOLD and run_count >= MIN_RUNS_FOR_FLAKY.
    """
    tmp_path = _make_flaky_file()
    try:
        with patch("scripts.quality_webhook.flaky._FLAKY_TESTS_FILE", tmp_path):
            test_name = "mod::test_flaky_check"
            for passed in outcomes:
                record_test_outcomes("test-backend", {test_name: passed})

            data = _load_flaky_data()
            entry = data["tests"][test_name]

            stored_outcomes = outcomes[-MAX_OUTCOMES_PER_TEST:]
            total = len(stored_outcomes)
            failures = sum(1 for p in stored_outcomes if not p)
            score = failures / total

            expected_flaky = score > FLAKINESS_THRESHOLD and total >= MIN_RUNS_FOR_FLAKY
            assert entry["is_flaky"] == expected_flaky
    finally:
        tmp_path.unlink(missing_ok=True)
