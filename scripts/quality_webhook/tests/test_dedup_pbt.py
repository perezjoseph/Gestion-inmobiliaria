# Feature: ci-pipeline-optimization, Property 4: Adaptive deduplication uses class-specific configuration
"""
Property-based test for adaptive deduplication in preflight_dedup.

**Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5**

Property 4: For any error class and any fix history with N recent failures
within the configured window, preflight_dedup rejects the request if and only
if N >= the threshold defined in DEDUP_CONFIG for that error class.
"""

import json
import threading
from datetime import datetime, timedelta
from unittest.mock import patch

from hypothesis import given, settings, assume
from hypothesis import strategies as st

from scripts.quality_webhook.gates import preflight_dedup, DEDUP_CONFIG

ERROR_CLASSES = ["flaky", "runner_environment", "test_failure",
                 "code_quality", "dependency", "unknown"]


def _build_fix_history(error_hash, failure_count, window_minutes, inside_window=True):
    """Build a fake fix history dict with the given number of recent failures.

    If inside_window is True, all failure timestamps are placed within the
    dedup window. If False, all are placed outside the window.
    """
    now = datetime.now()
    attempts = []
    for i in range(failure_count):
        if inside_window:
            ts = now - timedelta(minutes=window_minutes * 0.5, seconds=i)
        else:
            ts = now - timedelta(minutes=window_minutes + 10, seconds=i)
        attempts.append({
            "ts": ts.isoformat(),
            "attempt": i + 1,
            "success": False,
            "notes": "",
        })
    return {
        error_hash: {
            "job": "test-job",
            "first_seen": now.isoformat(),
            "last_seen": now.isoformat(),
            "attempts": attempts,
            "total_attempts": failure_count,
            "successes": 0,
        }
    }


@given(
    error_class=st.sampled_from(ERROR_CLASSES),
    failure_count=st.integers(min_value=0, max_value=10),
)
@settings(max_examples=200, deadline=None)
def test_dedup_rejects_iff_failures_at_or_above_threshold(error_class, failure_count):
    """Property 4: preflight_dedup rejects iff recent failures >= threshold.

    **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5**

    For any error class and failure count, when all failures are within the
    configured window, the gate rejects if and only if the count meets or
    exceeds the class-specific threshold.
    """
    cfg = DEDUP_CONFIG[error_class]
    threshold = cfg["threshold"]
    window_minutes = cfg["window_minutes"]

    fake_hash = "abcdef1234567890"
    history = _build_fix_history(fake_hash, failure_count, window_minutes, inside_window=True)

    with patch("scripts.quality_webhook.gates._error_hash", return_value=fake_hash), \
         patch("scripts.quality_webhook.gates._load_fix_history", return_value=history), \
         patch("scripts.quality_webhook.gates._fix_history_lock", threading.Lock()):

        passed, reason = preflight_dedup("test-job", "some error log", error_class=error_class)

    if failure_count >= threshold:
        assert not passed, (
            f"Expected rejection for {error_class}: "
            f"{failure_count} failures >= threshold {threshold}, got passed=True"
        )
    else:
        assert passed, (
            f"Expected pass for {error_class}: "
            f"{failure_count} failures < threshold {threshold}, got passed=False"
        )


@given(
    error_class=st.sampled_from(ERROR_CLASSES),
    failure_count=st.integers(min_value=1, max_value=10),
)
@settings(max_examples=200, deadline=None)
def test_dedup_allows_when_failures_outside_window(error_class, failure_count):
    """Property 4 (window boundary): failures outside the window are not counted.

    **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5**

    When all failures are outside the configured window, preflight_dedup
    always allows the request regardless of failure count.
    """
    cfg = DEDUP_CONFIG[error_class]
    window_minutes = cfg["window_minutes"]

    fake_hash = "abcdef1234567890"
    history = _build_fix_history(fake_hash, failure_count, window_minutes, inside_window=False)

    with patch("scripts.quality_webhook.gates._error_hash", return_value=fake_hash), \
         patch("scripts.quality_webhook.gates._load_fix_history", return_value=history), \
         patch("scripts.quality_webhook.gates._fix_history_lock", threading.Lock()):

        passed, reason = preflight_dedup("test-job", "some error log", error_class=error_class)

    assert passed, (
        f"Expected pass for {error_class}: all {failure_count} failures are "
        f"outside the {window_minutes}-minute window, got passed=False"
    )


@given(
    error_class=st.sampled_from(ERROR_CLASSES),
    inside_count=st.integers(min_value=0, max_value=5),
    outside_count=st.integers(min_value=0, max_value=5),
)
@settings(max_examples=200, deadline=None)
def test_dedup_counts_only_within_window_failures(error_class, inside_count, outside_count):
    """Property 4 (mixed timestamps): only failures within the window count.

    **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5**

    When the history contains a mix of failures inside and outside the window,
    only the inside-window failures are counted against the threshold.
    """
    cfg = DEDUP_CONFIG[error_class]
    threshold = cfg["threshold"]
    window_minutes = cfg["window_minutes"]

    now = datetime.now()
    attempts = []

    for i in range(inside_count):
        ts = now - timedelta(minutes=window_minutes * 0.5, seconds=i)
        attempts.append({"ts": ts.isoformat(), "attempt": i + 1, "success": False, "notes": ""})

    for i in range(outside_count):
        ts = now - timedelta(minutes=window_minutes + 10, seconds=i)
        attempts.append({"ts": ts.isoformat(), "attempt": inside_count + i + 1, "success": False, "notes": ""})

    fake_hash = "abcdef1234567890"
    history = {
        fake_hash: {
            "job": "test-job",
            "first_seen": now.isoformat(),
            "last_seen": now.isoformat(),
            "attempts": attempts,
            "total_attempts": inside_count + outside_count,
            "successes": 0,
        }
    }

    with patch("scripts.quality_webhook.gates._error_hash", return_value=fake_hash), \
         patch("scripts.quality_webhook.gates._load_fix_history", return_value=history), \
         patch("scripts.quality_webhook.gates._fix_history_lock", threading.Lock()):

        passed, reason = preflight_dedup("test-job", "some error log", error_class=error_class)

    if inside_count >= threshold:
        assert not passed, (
            f"Expected rejection for {error_class}: "
            f"{inside_count} in-window failures >= threshold {threshold}"
        )
    else:
        assert passed, (
            f"Expected pass for {error_class}: "
            f"{inside_count} in-window failures < threshold {threshold}"
        )
