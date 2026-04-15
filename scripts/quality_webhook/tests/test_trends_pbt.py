"""Property-based tests for build duration trend analysis.

Feature: ci-pipeline-optimization, Property 9: Duration trend moving average is correctly computed
"""

import json
import tempfile
import pathlib
from unittest.mock import patch

from hypothesis import given, settings, strategies as st

from scripts.quality_webhook.trends import (
    DurationSample,
    JobTrend,
    JOB_TIMEOUTS,
    MAX_SAMPLES,
    MOVING_AVG_WINDOW,
    TIMEOUT_RISK_THRESHOLD,
    MIN_SAMPLES_FOR_ANALYSIS,
    DEFAULT_TIMEOUT_S,
    record_duration,
    analyze_trend,
)


def _make_history_file():
    tmp = tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False, encoding="utf-8",
    )
    tmp.write("{}")
    tmp.close()
    return pathlib.Path(tmp.name)


duration_strategy = st.floats(min_value=0.1, max_value=10000.0, allow_nan=False, allow_infinity=False)


@given(
    durations=st.lists(
        duration_strategy,
        min_size=1,
        max_size=30,
    ),
)
@settings(max_examples=200, deadline=None)
def test_moving_average_matches_arithmetic_mean(durations):
    """**Validates: Requirements 11.3**

    For any sequence of durations, the moving average equals the arithmetic
    mean of the last min(N, MOVING_AVG_WINDOW) samples.
    """
    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            job = "test-backend"
            for d in durations:
                record_duration(job, d, True)

            trend = analyze_trend(job)

            assert trend.sample_count == min(len(durations), MAX_SAMPLES)

            stored_durations = durations[-MAX_SAMPLES:]

            if len(stored_durations) < MIN_SAMPLES_FOR_ANALYSIS:
                assert trend.moving_avg_s == 0.0
            else:
                window = stored_durations[-MOVING_AVG_WINDOW:]
                expected_avg = sum(window) / len(window)
                assert abs(trend.moving_avg_s - expected_avg) < 1e-6
    finally:
        tmp_path.unlink(missing_ok=True)


@given(
    durations=st.lists(
        duration_strategy,
        min_size=1,
        max_size=4,
    ),
)
@settings(max_examples=200, deadline=None)
def test_timeout_risk_false_when_insufficient_samples(durations):
    """**Validates: Requirements 11.6**

    When sample count < MIN_SAMPLES_FOR_ANALYSIS, timeout_risk is always False.
    """
    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            job = "lint"
            for d in durations:
                record_duration(job, d, True)

            trend = analyze_trend(job)

            assert trend.sample_count < MIN_SAMPLES_FOR_ANALYSIS
            assert trend.timeout_risk is False
    finally:
        tmp_path.unlink(missing_ok=True)


@given(
    durations=st.lists(
        duration_strategy,
        min_size=MIN_SAMPLES_FOR_ANALYSIS,
        max_size=30,
    ),
    job=st.sampled_from(list(JOB_TIMEOUTS.keys())),
)
@settings(max_examples=200, deadline=None)
def test_timeout_risk_consistent_with_threshold(durations, job):
    """**Validates: Requirements 11.3**

    When enough samples exist, timeout_risk is True iff moving_avg >= threshold * timeout.
    """
    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            for d in durations:
                record_duration(job, d, True)

            trend = analyze_trend(job)

            stored = durations[-MAX_SAMPLES:]
            window = stored[-MOVING_AVG_WINDOW:]
            expected_avg = sum(window) / len(window)
            timeout_s = JOB_TIMEOUTS.get(job, DEFAULT_TIMEOUT_S)
            expected_risk = expected_avg >= TIMEOUT_RISK_THRESHOLD * timeout_s

            assert trend.timeout_risk == expected_risk
    finally:
        tmp_path.unlink(missing_ok=True)
