"""Property-based tests for predictive decision cache.

Feature: ci-pipeline-optimization, Property 14: Strategy ranking preserves success rate ordering
"""

import json
import tempfile
import pathlib
from unittest.mock import patch

from hypothesis import given, settings, assume, strategies as st

from scripts.quality_webhook.decisions import (
    MAX_STRATEGIES_IN_PROMPT,
    get_ranked_strategies,
    record_strategy_outcome,
)


def _make_history_file():
    tmp = tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False, encoding="utf-8",
    )
    tmp.write("{}")
    tmp.close()
    return pathlib.Path(tmp.name)


strategy_records = st.dictionaries(
    keys=st.text(
        alphabet="abcdefghijklmnopqrstuvwxyz_- ",
        min_size=1,
        max_size=30,
    ),
    values=st.tuples(
        st.integers(min_value=0, max_value=50),
        st.integers(min_value=0, max_value=50),
    ),
    min_size=1,
    max_size=15,
)


@given(records=strategy_records)
@settings(max_examples=50, deadline=None)
def test_strategy_ranking_descending_order(records):
    """**Validates: Requirements 15.3, 15.4**

    For any set of strategies with success/failure counts, get_ranked_strategies
    returns them in strictly descending order of success rate, with at most
    MAX_STRATEGIES_IN_PROMPT entries.
    """
    # Feature: ci-pipeline-optimization, Property 14: Strategy ranking preserves success rate ordering
    filtered = {k: v for k, v in records.items() if v[0] + v[1] > 0}
    assume(len(filtered) > 0)

    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            job = "lint"
            error_class = "code_quality"

            for strategy, (successes, failures) in filtered.items():
                for _ in range(successes):
                    record_strategy_outcome(job, error_class, strategy, True)
                for _ in range(failures):
                    record_strategy_outcome(job, error_class, strategy, False)

            result = get_ranked_strategies(job, error_class)

            assert len(result) <= MAX_STRATEGIES_IN_PROMPT

            assert len(result) == min(len(filtered), MAX_STRATEGIES_IN_PROMPT)

            for i in range(len(result) - 1):
                assert result[i][1] >= result[i + 1][1], (
                    f"Not descending: {result[i]} should have rate >= {result[i + 1]}"
                )

            for strategy_name, rate in result:
                successes, failures = filtered[strategy_name]
                expected_rate = successes / (successes + failures)
                assert abs(rate - expected_rate) < 1e-9
    finally:
        tmp_path.unlink(missing_ok=True)


@given(
    records=st.dictionaries(
        keys=st.text(
            alphabet="abcdefghijklmnopqrstuvwxyz_- ",
            min_size=1,
            max_size=30,
        ),
        values=st.tuples(
            st.integers(min_value=0, max_value=50),
            st.integers(min_value=0, max_value=50),
        ),
        min_size=6,
        max_size=15,
    ),
)
@settings(max_examples=50, deadline=None)
def test_strategy_ranking_capped_at_max(records):
    """**Validates: Requirements 15.3, 15.4**

    When more strategies exist than MAX_STRATEGIES_IN_PROMPT, only the top-rated
    ones are returned.
    """
    # Feature: ci-pipeline-optimization, Property 14: Strategy ranking preserves success rate ordering
    filtered = {k: v for k, v in records.items() if v[0] + v[1] > 0}
    assume(len(filtered) > MAX_STRATEGIES_IN_PROMPT)

    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            job = "test-backend"
            error_class = "test_failure"

            for strategy, (successes, failures) in filtered.items():
                for _ in range(successes):
                    record_strategy_outcome(job, error_class, strategy, True)
                for _ in range(failures):
                    record_strategy_outcome(job, error_class, strategy, False)

            result = get_ranked_strategies(job, error_class)

            assert len(result) == MAX_STRATEGIES_IN_PROMPT

            all_rates = sorted(
                [s / (s + f) for s, f in filtered.values()],
                reverse=True,
            )
            returned_rates = [r for _, r in result]
            for rate in returned_rates:
                assert rate >= all_rates[MAX_STRATEGIES_IN_PROMPT - 1] - 1e-9
    finally:
        tmp_path.unlink(missing_ok=True)
