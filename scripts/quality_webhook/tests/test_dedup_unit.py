"""
Unit tests for DEDUP_CONFIG completeness and preflight_dedup edge cases.

Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5
"""

import threading
from datetime import datetime, timedelta
from unittest.mock import patch

import pytest

from scripts.quality_webhook.gates import DEDUP_CONFIG, preflight_dedup

EXPECTED_ERROR_CLASSES = [
    "flaky",
    "runner_environment",
    "test_failure",
    "code_quality",
    "dependency",
    "unknown",
]

EXPECTED_CONFIG = {
    "flaky":              {"window_minutes": 15,  "threshold": 5},
    "runner_environment": {"window_minutes": 120, "threshold": 2},
    "test_failure":       {"window_minutes": 60,  "threshold": 3},
    "code_quality":       {"window_minutes": 60,  "threshold": 3},
    "dependency":         {"window_minutes": 180, "threshold": 2},
    "unknown":            {"window_minutes": 60,  "threshold": 3},
}


# --- DEDUP_CONFIG completeness ---

class TestDedupConfigCompleteness:
    """All 6 error classes must have entries in DEDUP_CONFIG."""

    @pytest.mark.parametrize("error_class", EXPECTED_ERROR_CLASSES)
    def test_error_class_present(self, error_class):
        assert error_class in DEDUP_CONFIG

    def test_no_extra_classes(self):
        assert set(DEDUP_CONFIG.keys()) == set(EXPECTED_ERROR_CLASSES)

    @pytest.mark.parametrize("error_class", EXPECTED_ERROR_CLASSES)
    def test_config_has_required_keys(self, error_class):
        cfg = DEDUP_CONFIG[error_class]
        assert "window_minutes" in cfg
        assert "threshold" in cfg

    @pytest.mark.parametrize("error_class", EXPECTED_ERROR_CLASSES)
    def test_config_values_match_spec(self, error_class):
        """Each class's window and threshold match the design spec."""
        assert DEDUP_CONFIG[error_class] == EXPECTED_CONFIG[error_class]


# --- Fallback to "unknown" for unrecognized class ---

class TestUnrecognizedClassFallback:
    """preflight_dedup falls back to 'unknown' config for unrecognized classes."""

    def _run_dedup(self, error_class, failure_count, window_minutes):
        fake_hash = "deadbeef12345678"
        now = datetime.now()
        attempts = [
            {
                "ts": (now - timedelta(minutes=window_minutes * 0.5, seconds=i)).isoformat(),
                "attempt": i + 1,
                "success": False,
                "notes": "",
            }
            for i in range(failure_count)
        ]
        history = {
            fake_hash: {
                "job": "test-job",
                "first_seen": now.isoformat(),
                "last_seen": now.isoformat(),
                "attempts": attempts,
                "total_attempts": failure_count,
                "successes": 0,
            }
        }
        with (
            patch("scripts.quality_webhook.gates._error_hash", return_value=fake_hash),
            patch("scripts.quality_webhook.gates._load_fix_history", return_value=history),
            patch("scripts.quality_webhook.gates._fix_history_lock", threading.Lock()),
        ):
            return preflight_dedup("test-job", "some error", error_class=error_class)

    def test_unrecognized_class_uses_unknown_threshold(self):
        """An unrecognized class should behave identically to 'unknown'."""
        unknown_cfg = DEDUP_CONFIG["unknown"]
        # At threshold -> reject (same as "unknown")
        passed, _ = self._run_dedup(
            "totally_made_up_class",
            unknown_cfg["threshold"],
            unknown_cfg["window_minutes"],
        )
        assert not passed

    def test_unrecognized_class_allows_below_unknown_threshold(self):
        unknown_cfg = DEDUP_CONFIG["unknown"]
        passed, _ = self._run_dedup(
            "totally_made_up_class",
            unknown_cfg["threshold"] - 1,
            unknown_cfg["window_minutes"],
        )
        assert passed


# --- Boundary: exactly at threshold vs one below ---

class TestThresholdBoundary:
    """Exactly at threshold -> reject; one below -> allow."""

    @staticmethod
    def _build_history(fake_hash, failure_count, window_minutes):
        now = datetime.now()
        attempts = [
            {
                "ts": (now - timedelta(minutes=window_minutes * 0.5, seconds=i)).isoformat(),
                "attempt": i + 1,
                "success": False,
                "notes": "",
            }
            for i in range(failure_count)
        ]
        return {
            fake_hash: {
                "job": "test-job",
                "first_seen": now.isoformat(),
                "last_seen": now.isoformat(),
                "attempts": attempts,
                "total_attempts": failure_count,
                "successes": 0,
            }
        }

    @pytest.mark.parametrize("error_class", EXPECTED_ERROR_CLASSES)
    def test_exactly_at_threshold_rejects(self, error_class):
        cfg = DEDUP_CONFIG[error_class]
        fake_hash = "abcdef1234567890"
        history = self._build_history(fake_hash, cfg["threshold"], cfg["window_minutes"])

        with (
            patch("scripts.quality_webhook.gates._error_hash", return_value=fake_hash),
            patch("scripts.quality_webhook.gates._load_fix_history", return_value=history),
            patch("scripts.quality_webhook.gates._fix_history_lock", threading.Lock()),
        ):
            passed, reason = preflight_dedup("test-job", "error log", error_class=error_class)

        assert not passed, (
            f"{error_class}: {cfg['threshold']} failures (== threshold) should reject"
        )
        assert "failed" in reason

    @pytest.mark.parametrize("error_class", EXPECTED_ERROR_CLASSES)
    def test_one_below_threshold_allows(self, error_class):
        cfg = DEDUP_CONFIG[error_class]
        count = cfg["threshold"] - 1
        fake_hash = "abcdef1234567890"
        history = self._build_history(fake_hash, count, cfg["window_minutes"])

        with (
            patch("scripts.quality_webhook.gates._error_hash", return_value=fake_hash),
            patch("scripts.quality_webhook.gates._load_fix_history", return_value=history),
            patch("scripts.quality_webhook.gates._fix_history_lock", threading.Lock()),
        ):
            passed, reason = preflight_dedup("test-job", "error log", error_class=error_class)

        assert passed, (
            f"{error_class}: {count} failures (< threshold {cfg['threshold']}) should allow"
        )

    def test_zero_failures_always_allows(self):
        """No prior attempts -> always pass."""
        fake_hash = "abcdef1234567890"
        with (
            patch("scripts.quality_webhook.gates._error_hash", return_value=fake_hash),
            patch("scripts.quality_webhook.gates._load_fix_history", return_value={}),
            patch("scripts.quality_webhook.gates._fix_history_lock", threading.Lock()),
        ):
            passed, reason = preflight_dedup("test-job", "error log", error_class="flaky")

        assert passed
        assert "no prior attempts" in reason
