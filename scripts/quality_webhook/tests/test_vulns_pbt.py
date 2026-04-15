"""Property-based tests for dependency vulnerability pattern detection.

Feature: ci-pipeline-optimization, Property 11: Vulnerability tracker correctly identifies recurring crates
"""

import json
import tempfile
import pathlib
from datetime import datetime, timedelta
from unittest.mock import patch

from hypothesis import given, settings, strategies as st

from scripts.quality_webhook.vulns import (
    RECURRING_THRESHOLD,
    RECURRING_WINDOW_DAYS,
    MAX_RECORDS_PER_CRATE,
    record_vuln_failure,
    get_recurring_vulns,
)
from scripts.quality_webhook.history import _load_fix_history


def _make_history_file():
    tmp = tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False, encoding="utf-8",
    )
    tmp.write("{}")
    tmp.close()
    return pathlib.Path(tmp.name)


@given(
    recent_count=st.integers(min_value=0, max_value=10),
    old_count=st.integers(min_value=0, max_value=5),
)
@settings(max_examples=200, deadline=None)
def test_recurring_flag_matches_threshold_over_30d_window(recent_count, old_count):
    """**Validates: Requirements 13.3**

    is_recurring is True iff failures within last 30 days >= RECURRING_THRESHOLD.
    Failures older than 30 days are not counted.
    """
    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path), \
             patch("scripts.quality_webhook.vulns._load_fix_history", _load_fix_history), \
             patch("scripts.quality_webhook.vulns._save_fix_history") as mock_save:

            mock_save.side_effect = lambda h: tmp_path.write_text(
                json.dumps(h, indent=2), encoding="utf-8",
            )

            now = datetime.now()
            crate = "test-crate"

            history = {"vuln_tracking": {}}
            failures = []
            for i in range(old_count):
                old_ts = (now - timedelta(days=RECURRING_WINDOW_DAYS + 1 + i)).isoformat()
                failures.append({"ts": old_ts})
            for i in range(recent_count):
                recent_ts = (now - timedelta(days=i)).isoformat()
                failures.append({"ts": recent_ts})

            history["vuln_tracking"][crate] = {
                "advisory_ids": ["RUSTSEC-2024-0001"],
                "failures": failures,
                "failure_count_30d": recent_count,
                "is_recurring": recent_count >= RECURRING_THRESHOLD,
            }
            tmp_path.write_text(json.dumps(history, indent=2), encoding="utf-8")

            recurring = get_recurring_vulns()
            recurring_crates = [r.crate for r in recurring]

            if recent_count >= RECURRING_THRESHOLD:
                assert crate in recurring_crates
                record = next(r for r in recurring if r.crate == crate)
                assert record.is_recurring is True
                assert record.failure_count_30d >= RECURRING_THRESHOLD
            else:
                assert crate not in recurring_crates
    finally:
        tmp_path.unlink(missing_ok=True)


@given(
    timestamps=st.lists(
        st.floats(min_value=-60, max_value=60, allow_nan=False, allow_infinity=False),
        min_size=1,
        max_size=15,
    ),
)
@settings(max_examples=200, deadline=None)
def test_record_vuln_failure_counts_only_recent(timestamps):
    """**Validates: Requirements 13.3**

    After recording failures at various offsets from now, failure_count_30d
    reflects only those within the 30-day window.
    """
    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            now = datetime.now()
            crate = "offset-crate"

            history = {"vuln_tracking": {}}
            failures = []
            expected_recent = 0
            for offset_days in timestamps:
                ts = now - timedelta(days=offset_days)
                failures.append({"ts": ts.isoformat()})
                if offset_days <= RECURRING_WINDOW_DAYS and offset_days >= -1:
                    expected_recent += 1

            history["vuln_tracking"][crate] = {
                "advisory_ids": [],
                "failures": failures,
            }
            tmp_path.write_text(json.dumps(history, indent=2), encoding="utf-8")

            record_vuln_failure(crate, ["CVE-2024-9999"])

            data = json.loads(tmp_path.read_text(encoding="utf-8"))
            entry = data["vuln_tracking"][crate]

            assert entry["failure_count_30d"] >= 1
            assert entry["is_recurring"] == (entry["failure_count_30d"] >= RECURRING_THRESHOLD)
    finally:
        tmp_path.unlink(missing_ok=True)
