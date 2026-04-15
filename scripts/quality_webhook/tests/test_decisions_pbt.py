"""Property-based tests for predictive decision cache.

Feature: ci-pipeline-optimization, Properties 13, 14, 15
"""

import json
import tempfile
import pathlib
from datetime import datetime
from unittest.mock import patch

from hypothesis import given, settings, assume, strategies as st

from scripts.quality_webhook.decisions import (
    CO_FAILURE_MIN_EVENTS,
    CO_FAILURE_THRESHOLD,
    FEASIBILITY_THRESHOLD,
    FEASIBILITY_WARN_THRESHOLD,
    MAX_STRATEGIES_IN_PROMPT,
    MIN_ATTEMPTS_FOR_FEASIBILITY,
    REPRO_PROFILE_HISTORY,
    compute_feasibility,
    get_correlated_jobs,
    get_ranked_strategies,
    get_steps_to_skip,
    record_co_failure_event,
    record_repro_outcome,
    record_strategy_outcome,
)


def _make_history_file():
    tmp = tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False, encoding="utf-8",
    )
    tmp.write("{}")
    tmp.close()
    return pathlib.Path(tmp.name)


def _seed_strategy_stats(tmp_path, job, error_class, filtered):
    key = f"{job}|{error_class}"
    now = datetime.now().isoformat()
    entries = []
    for strategy, (successes, failures) in filtered.items():
        entries.extend(
            {"strategy": strategy, "ts": now, "success": True}
            for _ in range(successes)
        )
        entries.extend(
            {"strategy": strategy, "ts": now, "success": False}
            for _ in range(failures)
        )
    history = {"strategy_stats": {key: entries}}
    tmp_path.write_text(json.dumps(history), encoding="utf-8")


def _seed_feasibility_outcomes(tmp_path, job, error_class, outcomes):
    key = f"{job}|{error_class}"
    now = datetime.now().isoformat()
    entries = [
        {"strategy": "default", "ts": now, "success": outcome}
        for outcome in outcomes
    ]
    history = {"strategy_stats": {key: entries}}
    tmp_path.write_text(json.dumps(history), encoding="utf-8")


strategy_records = st.dictionaries(
    keys=st.text(
        alphabet="abcdefghijklmnopqrstuvwxyz_- ",
        min_size=1,
        max_size=30,
    ),
    values=st.tuples(
        st.integers(min_value=0, max_value=20),
        st.integers(min_value=0, max_value=20),
    ),
    min_size=1,
    max_size=10,
)


@given(records=strategy_records)
@settings(max_examples=50, deadline=None)
def test_strategy_ranking_descending_order(records):
    """**Validates: Requirements 15.3, 15.4**"""
    filtered = {k: v for k, v in records.items() if v[0] + v[1] > 0}
    assume(len(filtered) > 0)

    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            job = "lint"
            error_class = "code_quality"
            _seed_strategy_stats(tmp_path, job, error_class, filtered)

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
            st.integers(min_value=0, max_value=20),
            st.integers(min_value=0, max_value=20),
        ),
        min_size=6,
        max_size=10,
    ),
)
@settings(max_examples=50, deadline=None)
def test_strategy_ranking_capped_at_max(records):
    """**Validates: Requirements 15.3, 15.4**"""
    filtered = {k: v for k, v in records.items() if v[0] + v[1] > 0}
    assume(len(filtered) > MAX_STRATEGIES_IN_PROMPT)

    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            job = "test-backend"
            error_class = "test_failure"
            _seed_strategy_stats(tmp_path, job, error_class, filtered)

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


@given(records=strategy_records)
@settings(max_examples=50, deadline=None)
def test_record_strategy_outcome_round_trips(records):
    """**Validates: Requirements 15.1**"""
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

            returned = dict(result)
            for strategy_name in returned:
                successes, failures = filtered[strategy_name]
                expected_rate = successes / (successes + failures)
                assert abs(returned[strategy_name] - expected_rate) < 1e-9
    finally:
        tmp_path.unlink(missing_ok=True)


@given(outcomes=st.lists(st.booleans(), min_size=5, max_size=25))
@settings(max_examples=50, deadline=None)
def test_feasibility_score_reflects_success_rate(outcomes):
    """**Validates: Requirements 16.1, 16.4**"""
    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            job = "test-backend"
            error_class = "test_failure"
            _seed_feasibility_outcomes(tmp_path, job, error_class, outcomes)

            result = compute_feasibility(job, error_class)

            recent = outcomes[-20:]
            expected_score = sum(recent) / len(recent)
            assert abs(result.score - expected_score) < 1e-9
            assert result.sample_count == len(recent)

            if expected_score < FEASIBILITY_THRESHOLD:
                assert result.recommendation == "skip"
            elif expected_score < FEASIBILITY_WARN_THRESHOLD:
                assert result.recommendation == "warn"
            else:
                assert result.recommendation == "proceed"
    finally:
        tmp_path.unlink(missing_ok=True)


@given(outcomes=st.lists(st.booleans(), min_size=0, max_size=4))
@settings(max_examples=50, deadline=None)
def test_feasibility_defaults_when_insufficient_data(outcomes):
    """**Validates: Requirements 16.1, 16.4**"""
    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            job = "lint"
            error_class = "code_quality"
            _seed_feasibility_outcomes(tmp_path, job, error_class, outcomes)

            result = compute_feasibility(job, error_class)

            assert result.score == 1.0
            assert result.sample_count == len(outcomes)
            assert result.recommendation == "proceed"
    finally:
        tmp_path.unlink(missing_ok=True)


@given(outcomes=st.lists(st.booleans(), min_size=0, max_size=15))
@settings(max_examples=50, deadline=None)
def test_repro_profile_skip_logic(outcomes):
    """**Validates: Requirements 17.3, 17.4, 17.5**"""
    job = "lint"
    step = "fmt-check"
    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            stored = outcomes[-REPRO_PROFILE_HISTORY:]
            data = {"repro_profiles": {job: {step: stored}}}
            tmp_path.write_text(json.dumps(data), encoding="utf-8")

            skips = get_steps_to_skip(job)

            should_skip = (
                len(outcomes) >= REPRO_PROFILE_HISTORY
                and all(outcomes[-REPRO_PROFILE_HISTORY:])
            )
            if should_skip:
                assert step in skips
            else:
                assert step not in skips
    finally:
        tmp_path.unlink(missing_ok=True)


@given(outcomes=st.lists(st.booleans(), min_size=1, max_size=15))
@settings(max_examples=50, deadline=None)
def test_repro_profile_record_and_read_roundtrip(outcomes):
    """**Validates: Requirements 17.3, 17.4, 17.5**"""
    job = "test-backend"
    step = "cargo-test"
    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            for outcome in outcomes:
                record_repro_outcome(job, step, outcome)

            skips = get_steps_to_skip(job)

            expected_stored = outcomes[-REPRO_PROFILE_HISTORY:]
            should_skip = (
                len(outcomes) >= REPRO_PROFILE_HISTORY
                and all(outcomes[-REPRO_PROFILE_HISTORY:])
            )
            if should_skip:
                assert step in skips
            else:
                assert step not in skips

            raw = json.loads(tmp_path.read_text(encoding="utf-8"))
            stored = raw["repro_profiles"][job][step]
            assert stored == expected_stored
    finally:
        tmp_path.unlink(missing_ok=True)


@given(
    co_failures=st.integers(min_value=0, max_value=10),
    total_a=st.integers(min_value=1, max_value=15),
    total_b=st.integers(min_value=1, max_value=15),
)
@settings(max_examples=50, deadline=None)
def test_co_failure_pair_detection(co_failures, total_a, total_b):
    """**Validates: Requirements 18.1, 18.2**"""
    assume(total_a >= co_failures)
    assume(total_b >= co_failures)

    job_a = "lint"
    job_b = "build-backend"
    pair_key = f"{job_a}|{job_b}"

    tmp_path = _make_history_file()
    try:
        with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
            data = {
                "co_failure_matrix": {
                    pair_key: {
                        "co_failures": co_failures,
                        "total_a": total_a,
                        "total_b": total_b,
                    }
                }
            }
            tmp_path.write_text(json.dumps(data), encoding="utf-8")

            correlated_for_a = get_correlated_jobs(job_a)
            correlated_for_b = get_correlated_jobs(job_b)

            denominator = min(total_a, total_b)
            expected_correlated = (
                co_failures >= CO_FAILURE_MIN_EVENTS
                and co_failures / denominator > CO_FAILURE_THRESHOLD
            )

            if expected_correlated:
                assert job_b in correlated_for_a, (
                    f"Expected {job_b} in correlated jobs for {job_a}: "
                    f"co={co_failures}, min_total={denominator}, "
                    f"rate={co_failures / denominator:.2f}"
                )
                assert job_a in correlated_for_b, (
                    f"Expected {job_a} in correlated jobs for {job_b}: "
                    f"co={co_failures}, min_total={denominator}, "
                    f"rate={co_failures / denominator:.2f}"
                )
            else:
                assert job_b not in correlated_for_a, (
                    f"Did not expect {job_b} in correlated jobs for {job_a}: "
                    f"co={co_failures}, min_total={denominator}, "
                    f"rate={co_failures / denominator:.2f}"
                )
                assert job_a not in correlated_for_b, (
                    f"Did not expect {job_a} in correlated jobs for {job_b}: "
                    f"co={co_failures}, min_total={denominator}, "
                    f"rate={co_failures / denominator:.2f}"
                )
    finally:
        tmp_path.unlink(missing_ok=True)
