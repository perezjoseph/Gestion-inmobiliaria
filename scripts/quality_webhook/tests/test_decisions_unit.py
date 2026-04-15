import json
import tempfile
import pathlib
from datetime import datetime
from unittest.mock import patch

from scripts.quality_webhook.decisions import (
    MAX_STRATEGIES_IN_PROMPT,
    compute_feasibility,
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


def _seed_strategy_stats(tmp_path, job, error_class, entries):
    key = f"{job}|{error_class}"
    now = datetime.now().isoformat()
    records = []
    for strategy, (successes, failures) in entries.items():
        records.extend(
            {"strategy": strategy, "ts": now, "success": True}
            for _ in range(successes)
        )
        records.extend(
            {"strategy": strategy, "ts": now, "success": False}
            for _ in range(failures)
        )
    history = {"strategy_stats": {key: records}}
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


class TestGetRankedStrategiesEmpty:
    def test_empty_history_file_returns_empty_list(self):
        tmp_path = _make_history_file()
        try:
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = get_ranked_strategies("build", "compile_error")
                assert result == []
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_no_matching_key_returns_empty_list(self):
        tmp_path = _make_history_file()
        try:
            _seed_strategy_stats(tmp_path, "lint", "style", {"fix_imports": (3, 1)})
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = get_ranked_strategies("build", "compile_error")
                assert result == []
        finally:
            tmp_path.unlink(missing_ok=True)


class TestGetRankedStrategiesSingle:
    def test_single_strategy_returns_list_of_one(self):
        tmp_path = _make_history_file()
        try:
            _seed_strategy_stats(tmp_path, "lint", "style", {"fix_imports": (7, 3)})
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = get_ranked_strategies("lint", "style")
                assert len(result) == 1
                assert result[0][0] == "fix_imports"
                assert abs(result[0][1] - 0.7) < 1e-9
        finally:
            tmp_path.unlink(missing_ok=True)


class TestGetRankedStrategiesOrdering:
    def test_equal_success_rates_have_stable_ordering(self):
        tmp_path = _make_history_file()
        try:
            _seed_strategy_stats(tmp_path, "test", "failure", {
                "retry": (5, 5),
                "isolate": (3, 3),
                "reset": (1, 1),
            })
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = get_ranked_strategies("test", "failure")
                assert len(result) == 3
                rates = [r for _, r in result]
                assert all(abs(r - 0.5) < 1e-9 for r in rates)
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_descending_order_by_success_rate(self):
        tmp_path = _make_history_file()
        try:
            _seed_strategy_stats(tmp_path, "build", "compile", {
                "patch_dep": (9, 1),
                "rollback": (1, 9),
                "clean_cache": (5, 5),
            })
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = get_ranked_strategies("build", "compile")
                assert len(result) == 3
                assert result[0][0] == "patch_dep"
                assert abs(result[0][1] - 0.9) < 1e-9
                assert result[1][0] == "clean_cache"
                assert abs(result[1][1] - 0.5) < 1e-9
                assert result[2][0] == "rollback"
                assert abs(result[2][1] - 0.1) < 1e-9
        finally:
            tmp_path.unlink(missing_ok=True)


class TestGetRankedStrategiesCap:
    def test_max_strategies_returned_when_more_exist(self):
        tmp_path = _make_history_file()
        try:
            strategies = {
                f"strategy_{i}": (10 - i, i)
                for i in range(8)
            }
            _seed_strategy_stats(tmp_path, "deploy", "timeout", strategies)
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = get_ranked_strategies("deploy", "timeout")
                assert len(result) == 5
                assert len(result) == MAX_STRATEGIES_IN_PROMPT
        finally:
            tmp_path.unlink(missing_ok=True)


class TestRecordStrategyOutcome:
    def test_recording_increments_correct_counters(self):
        tmp_path = _make_history_file()
        try:
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                record_strategy_outcome("lint", "style", "fix_imports", True)
                record_strategy_outcome("lint", "style", "fix_imports", True)
                record_strategy_outcome("lint", "style", "fix_imports", False)
                result = get_ranked_strategies("lint", "style")
                assert len(result) == 1
                assert result[0][0] == "fix_imports"
                assert abs(result[0][1] - 2 / 3) < 1e-9
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_recording_isolates_different_job_error_pairs(self):
        tmp_path = _make_history_file()
        try:
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                record_strategy_outcome("lint", "style", "fix_imports", True)
                record_strategy_outcome("build", "compile", "patch_dep", False)
                lint_result = get_ranked_strategies("lint", "style")
                build_result = get_ranked_strategies("build", "compile")
                assert len(lint_result) == 1
                assert lint_result[0][0] == "fix_imports"
                assert abs(lint_result[0][1] - 1.0) < 1e-9
                assert len(build_result) == 1
                assert build_result[0][0] == "patch_dep"
                assert abs(build_result[0][1] - 0.0) < 1e-9
        finally:
            tmp_path.unlink(missing_ok=True)


class TestComputeFeasibilityEdgeCases:
    def test_zero_attempts(self):
        tmp_path = _make_history_file()
        try:
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = compute_feasibility("build", "compile")
                assert abs(result.score - 1.0) < 1e-9
                assert result.recommendation == "proceed"
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_four_attempts_below_minimum(self):
        tmp_path = _make_history_file()
        try:
            _seed_feasibility_outcomes(tmp_path, "build", "compile", [False, False, False, False])
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = compute_feasibility("build", "compile")
                assert abs(result.score - 1.0) < 1e-9
                assert result.recommendation == "proceed"
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_all_failures(self):
        tmp_path = _make_history_file()
        try:
            _seed_feasibility_outcomes(tmp_path, "build", "compile", [False] * 10)
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = compute_feasibility("build", "compile")
                assert abs(result.score - 0.0) < 1e-9
                assert result.recommendation == "skip"
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_low_success_rate(self):
        tmp_path = _make_history_file()
        try:
            outcomes = [True] * 2 + [False] * 8
            _seed_feasibility_outcomes(tmp_path, "build", "compile", outcomes)
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = compute_feasibility("build", "compile")
                assert abs(result.score - 0.20) < 1e-9
                assert result.recommendation == "warn"
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_half_success_rate(self):
        tmp_path = _make_history_file()
        try:
            outcomes = [True] * 5 + [False] * 5
            _seed_feasibility_outcomes(tmp_path, "build", "compile", outcomes)
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = compute_feasibility("build", "compile")
                assert abs(result.score - 0.50) < 1e-9
                assert result.recommendation == "proceed"
        finally:
            tmp_path.unlink(missing_ok=True)
from scripts.quality_webhook.decisions import (
    REPRO_PROFILE_HISTORY,
    get_steps_to_skip,
    record_repro_outcome,
)


class TestReproProfileEdgeCases:
    def test_skip_recommended_with_all_passes(self):
        """Validates: Requirements 17.3"""
        tmp_path = _make_history_file()
        try:
            history = {
                "repro_profiles": {
                    "build": {
                        "cargo-test": [True] * REPRO_PROFILE_HISTORY,
                    }
                }
            }
            tmp_path.write_text(json.dumps(history), encoding="utf-8")
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                skips = get_steps_to_skip("build")
                assert "cargo-test" in skips
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_no_skip_with_one_failure(self):
        """Validates: Requirements 17.4"""
        tmp_path = _make_history_file()
        try:
            outcomes = [True] * 9 + [False]
            history = {
                "repro_profiles": {
                    "build": {
                        "cargo-test": outcomes,
                    }
                }
            }
            tmp_path.write_text(json.dumps(history), encoding="utf-8")
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                skips = get_steps_to_skip("build")
                assert "cargo-test" not in skips
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_no_skip_with_insufficient_data(self):
        """Validates: Requirements 17.5"""
        tmp_path = _make_history_file()
        try:
            history = {
                "repro_profiles": {
                    "build": {
                        "cargo-test": [True] * 8,
                    }
                }
            }
            tmp_path.write_text(json.dumps(history), encoding="utf-8")
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                skips = get_steps_to_skip("build")
                assert "cargo-test" not in skips
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_skip_revoked_on_new_failure(self):
        """Validates: Requirements 17.4"""
        tmp_path = _make_history_file()
        try:
            history = {
                "repro_profiles": {
                    "lint": {
                        "clippy": [True] * REPRO_PROFILE_HISTORY,
                    }
                }
            }
            tmp_path.write_text(json.dumps(history), encoding="utf-8")
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                skips_before = get_steps_to_skip("lint")
                assert "clippy" in skips_before
                record_repro_outcome("lint", "clippy", False)
                skips_after = get_steps_to_skip("lint")
                assert "clippy" not in skips_after
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_empty_profile_returns_no_skips(self):
        """Validates: Requirements 17.3"""
        tmp_path = _make_history_file()
        try:
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                skips = get_steps_to_skip("nonexistent-job")
                assert skips == []
        finally:
            tmp_path.unlink(missing_ok=True)


from scripts.quality_webhook.decisions import (
    CO_FAILURE_HOLD_S,
    CO_FAILURE_MIN_EVENTS,
    CO_FAILURE_THRESHOLD,
    get_correlated_jobs,
    record_co_failure_event,
    should_hold_for_correlation,
)


class TestCoFailureEdgeCases:
    def test_pair_detected_at_above_70_percent_with_5_events(self):
        tmp_path = _make_history_file()
        try:
            history = {
                "co_failure_matrix": {
                    "job-a|job-b": {
                        "co_failures": 5,
                        "total_a": 7,
                        "total_b": 7,
                    }
                }
            }
            tmp_path.write_text(json.dumps(history), encoding="utf-8")
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = get_correlated_jobs("job-a")
                assert "job-b" in result
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_pair_not_detected_at_exactly_70_percent(self):
        tmp_path = _make_history_file()
        try:
            history = {
                "co_failure_matrix": {
                    "job-a|job-b": {
                        "co_failures": 7,
                        "total_a": 10,
                        "total_b": 10,
                    }
                }
            }
            tmp_path.write_text(json.dumps(history), encoding="utf-8")
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = get_correlated_jobs("job-a")
                assert "job-b" not in result
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_pair_not_detected_with_only_4_events(self):
        tmp_path = _make_history_file()
        try:
            history = {
                "co_failure_matrix": {
                    "job-a|job-b": {
                        "co_failures": 4,
                        "total_a": 4,
                        "total_b": 4,
                    }
                }
            }
            tmp_path.write_text(json.dumps(history), encoding="utf-8")
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                result = get_correlated_jobs("job-a")
                assert result == []
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_hold_recommendation_returns_correct_jobs_and_duration(self):
        tmp_path = _make_history_file()
        try:
            history = {
                "co_failure_matrix": {
                    "lint|test-backend": {
                        "co_failures": 8,
                        "total_a": 10,
                        "total_b": 10,
                    }
                }
            }
            tmp_path.write_text(json.dumps(history), encoding="utf-8")
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                hold, jobs, duration = should_hold_for_correlation("lint")
                assert hold is True
                assert "test-backend" in jobs
                assert duration == CO_FAILURE_HOLD_S
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_no_hold_when_no_correlated_pairs(self):
        tmp_path = _make_history_file()
        try:
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                hold, jobs, duration = should_hold_for_correlation("lint")
                assert hold is False
                assert jobs == []
                assert duration == 0
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_matrix_update_with_3_jobs_creates_3_pairs(self):
        tmp_path = _make_history_file()
        try:
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                record_co_failure_event("abc123", ["lint", "test-backend", "build"])
                data = json.loads(tmp_path.read_text(encoding="utf-8"))
                matrix = data["co_failure_matrix"]
                assert len(matrix) == 3
                assert "build|lint" in matrix
                assert "build|test-backend" in matrix
                assert "lint|test-backend" in matrix
                for entry in matrix.values():
                    assert entry["co_failures"] == 1
                    assert entry["total_a"] == 1
                    assert entry["total_b"] == 1
        finally:
            tmp_path.unlink(missing_ok=True)


from scripts.quality_webhook.decisions import (
    FEASIBILITY_WARN_THRESHOLD,
    FixDecision,
    build_decision,
    get_decision_cache_summary,
)


class TestBuildDecisionIntegration:
    def _seed_full_history(self, tmp_path, strategy_entries, repro_profiles, co_failure_matrix):
        from datetime import datetime
        now = datetime.now().isoformat()
        strategy_stats = {}
        for key, strategies in strategy_entries.items():
            records = []
            for strategy, (successes, failures) in strategies.items():
                records.extend(
                    {"strategy": strategy, "ts": now, "success": True}
                    for _ in range(successes)
                )
                records.extend(
                    {"strategy": strategy, "ts": now, "success": False}
                    for _ in range(failures)
                )
            strategy_stats[key] = records
        history = {
            "strategy_stats": strategy_stats,
            "repro_profiles": repro_profiles,
            "co_failure_matrix": co_failure_matrix,
        }
        import json
        tmp_path.write_text(json.dumps(history), encoding="utf-8")

    def test_build_decision_with_all_signals(self):
        import json
        from datetime import datetime
        from unittest.mock import patch
        from scripts.quality_webhook.decisions import REPRO_PROFILE_HISTORY
        tmp_path = _make_history_file()
        try:
            self._seed_full_history(
                tmp_path,
                strategy_entries={
                    "lint|code_quality": {
                        "ran cargo fmt": (8, 2),
                        "ran cargo clippy": (5, 5),
                        "disabled rule": (2, 8),
                    },
                },
                repro_profiles={
                    "lint": {
                        "fmt-check": [True] * REPRO_PROFILE_HISTORY,
                    },
                },
                co_failure_matrix={
                    "lint|test-backend": {
                        "co_failures": 8,
                        "total_a": 10,
                        "total_b": 10,
                    },
                },
            )
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                decision = build_decision("lint", "code_quality")
                assert isinstance(decision, FixDecision)
                assert decision.feasibility.recommendation == "proceed"
                assert len(decision.ranked_strategies) > 0
                rates = [r for _, r in decision.ranked_strategies]
                assert rates == sorted(rates, reverse=True)
                assert "fmt-check" in decision.steps_to_skip
                assert "test-backend" in decision.hold_for_jobs
                assert "PREFERRED STRATEGIES" in decision.prompt_additions
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_build_decision_with_warn_feasibility(self):
        import json
        from datetime import datetime
        from unittest.mock import patch
        tmp_path = _make_history_file()
        try:
            now = datetime.now().isoformat()
            outcomes = (
                [{"strategy": "default", "ts": now, "success": True}] * 2
                + [{"strategy": "default", "ts": now, "success": False}] * 8
            )
            history = {
                "strategy_stats": {"build|compile": outcomes},
            }
            tmp_path.write_text(json.dumps(history), encoding="utf-8")
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                decision = build_decision("build", "compile")
                assert decision.feasibility.recommendation == "warn"
                assert decision.feasibility.score >= 0.10
                assert decision.feasibility.score < FEASIBILITY_WARN_THRESHOLD
                assert "Low historical success rate" in decision.prompt_additions
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_get_decision_cache_summary_returns_all_keys(self):
        import json
        from unittest.mock import patch
        tmp_path = _make_history_file()
        try:
            self._seed_full_history(
                tmp_path,
                strategy_entries={
                    "lint|code_quality": {"fix_imports": (3, 1)},
                },
                repro_profiles={
                    "lint": {"fmt-check": [True] * 5},
                },
                co_failure_matrix={
                    "lint|test-backend": {
                        "co_failures": 2,
                        "total_a": 3,
                        "total_b": 3,
                    },
                },
            )
            with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path):
                summary = get_decision_cache_summary()
                assert "strategy_stats" in summary
                assert "feasibility" in summary
                assert "repro_profiles" in summary
                assert "co_failure_pairs" in summary
        finally:
            tmp_path.unlink(missing_ok=True)
