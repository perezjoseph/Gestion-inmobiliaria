"""Property-based tests for cross-job anomaly correlation.

Feature: ci-pipeline-optimization, Property 12: Correlation engine groups failures by commit within the time window
"""

import string
import time
from unittest.mock import patch

from hypothesis import given, settings, strategies as st

from scripts.quality_webhook.correlation import (
    CORRELATION_WINDOW_S,
    CorrelationGroup,
    FailureEvent,
    register_failure,
    check_correlation,
    _groups,
    _group_created_at,
    _group_counter,
    _lock,
)


def _reset_state():
    global _group_counter
    with _lock:
        _groups.clear()
        _group_created_at.clear()
    import scripts.quality_webhook.correlation as mod
    mod._group_counter = 0


commit_strategy = st.text(
    alphabet=string.hexdigits[:16],
    min_size=40,
    max_size=40,
)


@given(
    commit=commit_strategy,
    n_failures=st.integers(min_value=2, max_value=6),
    time_offsets=st.lists(
        st.floats(min_value=0, max_value=CORRELATION_WINDOW_S * 0.9, allow_nan=False, allow_infinity=False),
        min_size=2,
        max_size=6,
    ),
)
@settings(max_examples=200, deadline=None)
def test_same_commit_within_window_grouped(commit, n_failures, time_offsets):
    """**Validates: Requirements 14.1, 14.7**

    Failures with the same commit within CORRELATION_WINDOW_S are grouped together.
    """
    _reset_state()
    offsets = time_offsets[:n_failures]

    base_time = time.monotonic()
    group_ids = []
    for offset in offsets:
        with patch("scripts.quality_webhook.correlation.time") as mock_time:
            mock_time.monotonic.return_value = base_time + offset
            gid = register_failure(
                job=f"job-{len(group_ids)}",
                step="step",
                error_log="error in backend/src/main.rs",
                commit=commit,
                error_class="test_failure",
            )
            group_ids.append(gid)

    non_none = [g for g in group_ids if g is not None]
    assert len(non_none) >= 1
    assert len(set(non_none)) == 1, "All failures with same commit should be in one group"

    gid = non_none[0]
    group = check_correlation(gid)
    assert group is not None
    assert len(group.failures) == len(offsets)


@given(
    commit_a=commit_strategy,
    commit_b=commit_strategy,
)
@settings(max_examples=200, deadline=None)
def test_different_commits_never_grouped(commit_a, commit_b):
    """**Validates: Requirements 14.1, 14.7**

    Failures with different commits are never grouped together.
    """
    if commit_a == commit_b:
        return

    _reset_state()

    base_time = time.monotonic()
    with patch("scripts.quality_webhook.correlation.time") as mock_time:
        mock_time.monotonic.return_value = base_time
        gid_a = register_failure(
            job="lint",
            step="fmt",
            error_log="error in backend/src/main.rs",
            commit=commit_a,
            error_class="code_quality",
        )

    with patch("scripts.quality_webhook.correlation.time") as mock_time:
        mock_time.monotonic.return_value = base_time + 5
        gid_b = register_failure(
            job="test-backend",
            step="test",
            error_log="error in backend/src/main.rs",
            commit=commit_b,
            error_class="test_failure",
        )

    assert gid_a is not None
    assert gid_b is not None
    assert gid_a != gid_b

    group_a = check_correlation(gid_a)
    group_b = check_correlation(gid_b)
    assert group_a is not None
    assert group_b is not None
    assert len(group_a.failures) == 1
    assert len(group_b.failures) == 1
