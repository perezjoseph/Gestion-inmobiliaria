"""Unit tests for build duration trend analysis edge cases.

Requirements: 11.3, 11.5, 11.6
"""

import json
import pathlib
import tempfile
from unittest.mock import patch

import pytest

from scripts.quality_webhook.trends import (
    JOB_TIMEOUTS,
    MAX_SAMPLES,
    MOVING_AVG_WINDOW,
    TIMEOUT_RISK_THRESHOLD,
    MIN_SAMPLES_FOR_ANALYSIS,
    DEFAULT_TIMEOUT_S,
    record_duration,
    analyze_trend,
    get_all_trends,
    check_and_alert_trends,
)


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


def test_empty_duration_history_returns_no_risk(tmp_history):
    trend = analyze_trend("lint")
    assert trend.sample_count == 0
    assert trend.moving_avg_s == 0.0
    assert trend.timeout_risk is False
    assert trend.samples == []


def test_exactly_five_samples_triggers_analysis(tmp_history):
    for i in range(MIN_SAMPLES_FOR_ANALYSIS):
        record_duration("lint", 100.0, True)

    trend = analyze_trend("lint")
    assert trend.sample_count == MIN_SAMPLES_FOR_ANALYSIS
    assert trend.moving_avg_s == 100.0
    assert trend.timeout_risk is False


def test_four_samples_skips_analysis(tmp_history):
    for _ in range(MIN_SAMPLES_FOR_ANALYSIS - 1):
        record_duration("lint", 9999.0, True)

    trend = analyze_trend("lint")
    assert trend.sample_count == MIN_SAMPLES_FOR_ANALYSIS - 1
    assert trend.moving_avg_s == 0.0
    assert trend.timeout_risk is False


def test_moving_average_with_exactly_window_samples(tmp_history):
    for i in range(MOVING_AVG_WINDOW):
        record_duration("test-backend", float(100 + i), True)

    trend = analyze_trend("test-backend")
    expected = sum(100 + i for i in range(MOVING_AVG_WINDOW)) / MOVING_AVG_WINDOW
    assert abs(trend.moving_avg_s - expected) < 1e-6
    assert trend.sample_count == MOVING_AVG_WINDOW


def test_unknown_job_uses_default_timeout(tmp_history):
    assert "unknown-job-xyz" not in JOB_TIMEOUTS

    high_duration = DEFAULT_TIMEOUT_S * TIMEOUT_RISK_THRESHOLD + 1.0
    for _ in range(MIN_SAMPLES_FOR_ANALYSIS):
        record_duration("unknown-job-xyz", high_duration, True)

    trend = analyze_trend("unknown-job-xyz")
    assert trend.timeout_risk is True


def test_unknown_job_below_threshold_no_risk(tmp_history):
    low_duration = DEFAULT_TIMEOUT_S * TIMEOUT_RISK_THRESHOLD - 100.0
    for _ in range(MIN_SAMPLES_FOR_ANALYSIS):
        record_duration("unknown-job-xyz", low_duration, True)

    trend = analyze_trend("unknown-job-xyz")
    assert trend.timeout_risk is False


def test_get_all_trends_returns_all_jobs(tmp_history):
    for _ in range(MIN_SAMPLES_FOR_ANALYSIS):
        record_duration("lint", 100.0, True)
        record_duration("test-backend", 200.0, True)

    trends = get_all_trends()
    assert "lint" in trends
    assert "test-backend" in trends
    assert trends["lint"].sample_count == MIN_SAMPLES_FOR_ANALYSIS
    assert trends["test-backend"].sample_count == MIN_SAMPLES_FOR_ANALYSIS


def test_check_and_alert_trends_returns_at_risk_jobs(tmp_history):
    high_duration = JOB_TIMEOUTS["lint"] * TIMEOUT_RISK_THRESHOLD + 1.0
    for _ in range(MIN_SAMPLES_FOR_ANALYSIS):
        record_duration("lint", high_duration, True)
        record_duration("test-backend", 100.0, True)

    at_risk = check_and_alert_trends()
    assert "lint" in at_risk
    assert "test-backend" not in at_risk


def test_samples_trimmed_to_max(tmp_history):
    for i in range(MAX_SAMPLES + 10):
        record_duration("lint", float(i), True)

    trend = analyze_trend("lint")
    assert trend.sample_count == MAX_SAMPLES


def test_record_duration_stores_success_flag(tmp_history):
    record_duration("lint", 100.0, False)
    trend = analyze_trend("lint")
    assert trend.sample_count == 1
    assert trend.samples[0].success is False
