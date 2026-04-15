# Feature: ci-pipeline-optimization, Property 8: Phase timing data round-trips through fix history
"""
Property-based test for phase timing round-trip through fix history.

**Validates: Requirements 10.1, 10.2**

Property 8: For any PhaseTiming record with non-negative float values for each
phase (reproduction, diagnosis, fix_attempt, scope_gate, verification), storing
the timing via record_fix_attempt and retrieving it via the fix history preserves
all phase duration values exactly.
"""

import dataclasses
import json
import tempfile
from pathlib import Path
from unittest.mock import patch

from hypothesis import given, settings
from hypothesis import strategies as st

from scripts.quality_webhook.fixers import PhaseTiming
from scripts.quality_webhook.history import (
    _error_hash,
    _load_fix_history,
    record_fix_attempt,
)

phase_float = st.floats(min_value=0, max_value=10000, allow_nan=False)


@given(
    reproduction_s=phase_float,
    diagnosis_s=phase_float,
    fix_attempt_s=phase_float,
    scope_gate_s=phase_float,
    verification_s=phase_float,
    total_s=phase_float,
    job=st.text(min_size=1, max_size=30, alphabet="abcdefghijklmnopqrstuvwxyz-"),
    error_log=st.text(min_size=1, max_size=200),
)
@settings(max_examples=100, deadline=None)
def test_phase_timing_round_trips_through_fix_history(
    reproduction_s,
    diagnosis_s,
    fix_attempt_s,
    scope_gate_s,
    verification_s,
    total_s,
    job,
    error_log,
):
    """Property 8: all 6 phase timing values survive a store-then-load cycle.

    **Validates: Requirements 10.1, 10.2**

    Generate a PhaseTiming, convert to dict, store via record_fix_attempt,
    load via _load_fix_history, find the attempt record, and verify all 6
    timing values match exactly.
    """
    timing = PhaseTiming(
        reproduction_s=reproduction_s,
        diagnosis_s=diagnosis_s,
        fix_attempt_s=fix_attempt_s,
        scope_gate_s=scope_gate_s,
        verification_s=verification_s,
        total_s=total_s,
    )
    timing_dict = dataclasses.asdict(timing)

    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False
    ) as tmp:
        tmp.write("{}")
        tmp_path = Path(tmp.name)

    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            record_fix_attempt(
                job=job,
                error_log=error_log,
                attempt=1,
                success=True,
                timing=timing_dict,
            )

            history = _load_fix_history()

        h = _error_hash(job, error_log)
        assert h in history, f"Error hash '{h}' not found in history after recording"

        entry = history[h]
        attempts = entry.get("attempts", [])
        assert len(attempts) >= 1, "No attempts found in history entry"

        last_attempt = attempts[-1]
        assert "timing" in last_attempt, "Timing data missing from attempt record"

        stored_timing = last_attempt["timing"]

        assert stored_timing["reproduction_s"] == reproduction_s
        assert stored_timing["diagnosis_s"] == diagnosis_s
        assert stored_timing["fix_attempt_s"] == fix_attempt_s
        assert stored_timing["scope_gate_s"] == scope_gate_s
        assert stored_timing["verification_s"] == verification_s
        assert stored_timing["total_s"] == total_s
    finally:
        tmp_path.unlink(missing_ok=True)
