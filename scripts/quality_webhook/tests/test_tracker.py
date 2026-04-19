import json
import tempfile
import threading
from datetime import datetime, timedelta
from pathlib import Path
from unittest.mock import patch, MagicMock

import pytest

from scripts.quality_webhook.tracker import (
    PipelineTracker, PipelineRun, JobState, QualitySnapshot,
    capture_quality_snapshot, compare_snapshots,
    _serialize_pipeline, _deserialize_pipeline,
)


@pytest.fixture()
def tmp_state_file(tmp_path):
    state_file = tmp_path / ".pipeline-state.json"
    with patch("scripts.quality_webhook.tracker._PIPELINE_STATE_FILE", state_file):
        yield state_file


@pytest.fixture()
def mock_wsl():
    with patch("scripts.quality_webhook.tracker.wsl_bash") as m:
        m.return_value = MagicMock(returncode=1, stdout="", stderr="")
        yield m


@pytest.fixture()
def tracker(tmp_state_file, mock_wsl):
    with patch("scripts.quality_webhook.tracker._tracker_instance", None):
        t = PipelineTracker()
        yield t


class TestLineageCreation:
    def test_new_commit_creates_lineage(self, tracker):
        run = tracker.get_or_create("abc123def456")
        assert run.lineage_id == "abc123def456"
        assert run.current_commit == "abc123def456"
        assert run.round_number == 1
        assert run.deployed is False

    def test_same_commit_returns_existing(self, tracker):
        run1 = tracker.get_or_create("abc123def456")
        run2 = tracker.get_or_create("abc123def456")
        assert run1.lineage_id == run2.lineage_id

    def test_lineage_tag_in_commit_message(self, tracker, mock_wsl):
        tracker.get_or_create("aabbccdd1234")
        mock_wsl.return_value = MagicMock(
            returncode=0,
            stdout="fix: resolve CI failures [lineage:aabbccdd1234]\n",
        )
        run = tracker.get_or_create("eeff00112233")
        assert run.lineage_id == "aabbccdd1234"
        assert run.current_commit == "eeff00112233"

    def test_empty_commit_creates_timestamp_lineage(self, tracker):
        run = tracker.get_or_create("")
        assert run.lineage_id
        assert len(run.lineage_id) > 0


class TestRoundIncrementing:
    def test_update_from_report_increments_round(self, tracker):
        tracker.get_or_create("abc123def456")
        pipeline = tracker.update_from_report("abc123def456", {"lint": "success"})
        assert pipeline.round_number == 2

    def test_register_failure_increments_on_new_run_url(self, tracker):
        run = tracker.get_or_create("abc123def456")
        tracker.register_failure("abc123def456", "lint", "error", run_url="url1")
        pipeline = tracker.register_failure("abc123def456", "lint", "error", run_url="url2")
        assert pipeline.round_number == 2

    def test_register_failure_no_increment_same_run_url(self, tracker):
        tracker.get_or_create("abc123def456")
        tracker.register_failure("abc123def456", "lint", "error", run_url="url1")
        pipeline = tracker.register_failure("abc123def456", "lint", "error", run_url="url1")
        assert pipeline.round_number == 1


class TestDeployTermination:
    def test_mark_deployed_sets_flag(self, tracker):
        tracker.get_or_create("abc123def456")
        tracker.mark_deployed("abc123def456")
        run = tracker.get_or_create("abc123def456")
        assert run.deployed is True

    def test_deployed_pipeline_in_summary(self, tracker):
        tracker.get_or_create("abc123def456")
        tracker.mark_deployed("abc123def456")
        summary = tracker.get_summary()
        assert summary["deployed_count"] == 1


class TestBudgetExhaustion:
    def test_budget_not_exhausted_initially(self, tracker):
        tracker.get_or_create("abc123def456")
        assert tracker.is_budget_exhausted("abc123def456") is False

    def test_budget_exhausted_at_limit(self, tracker):
        run = tracker.get_or_create("abc123def456")
        with tracker._lock:
            run.round_number = 10
        assert tracker.is_budget_exhausted("abc123def456") is True

    def test_budget_exhausted_count_in_summary(self, tracker):
        run = tracker.get_or_create("abc123def456")
        with tracker._lock:
            run.round_number = 10
        summary = tracker.get_summary()
        assert summary["budget_exhausted_count"] == 1


class TestBackoffCalculation:
    def test_round_1_no_backoff(self, tracker):
        tracker.get_or_create("abc123def456")
        should_wait, seconds = tracker.should_backoff("abc123def456", "lint")
        assert should_wait is False
        assert seconds == 0.0

    def test_round_3_backoff_120s(self, tracker):
        run = tracker.get_or_create("abc123def456")
        with tracker._lock:
            run.round_number = 2
        should_wait, seconds = tracker.should_backoff("abc123def456", "lint")
        assert should_wait is True
        assert seconds == 120.0

    def test_round_4_backoff_300s(self, tracker):
        run = tracker.get_or_create("abc123def456")
        with tracker._lock:
            run.round_number = 3
        should_wait, seconds = tracker.should_backoff("abc123def456", "lint")
        assert should_wait is True
        assert seconds == 300.0

    def test_round_5_backoff_600s(self, tracker):
        run = tracker.get_or_create("abc123def456")
        with tracker._lock:
            run.round_number = 4
        should_wait, seconds = tracker.should_backoff("abc123def456", "lint")
        assert should_wait is True
        assert seconds == 600.0

    def test_round_beyond_schedule_clamps(self, tracker):
        run = tracker.get_or_create("abc123def456")
        with tracker._lock:
            run.round_number = 20
        should_wait, seconds = tracker.should_backoff("abc123def456", "lint")
        assert should_wait is True
        assert seconds == 600.0

    def test_new_run_url_skips_backoff(self, tracker):
        run = tracker.get_or_create("abc123def456")
        with tracker._lock:
            run.round_number = 4
            run.last_run_url = "old_url"
        should_wait, _ = tracker.should_backoff("abc123def456", "lint", run_url="new_url")
        assert should_wait is False


class TestRegressionDetection:
    def test_no_regression_on_first_round(self, tracker):
        tracker.get_or_create("abc123def456")
        result = tracker.detect_regression("abc123def456", "test-backend", {"file.rs"})
        assert result == []

    def test_regression_detected_on_overlapping_files(self, tracker):
        run = tracker.get_or_create("abc123def456")
        with tracker._lock:
            run.round_number = 2
            run.files_changed[1] = ["backend/src/handlers/pagos.rs"]
            run.job_states["lint"] = JobState.PASSED
        result = tracker.detect_regression(
            "abc123def456", "test-backend",
            {"backend/src/handlers/pagos.rs"},
        )
        assert "lint" in result

    def test_no_regression_without_overlap(self, tracker):
        run = tracker.get_or_create("abc123def456")
        with tracker._lock:
            run.round_number = 2
            run.files_changed[1] = ["backend/src/handlers/pagos.rs"]
        result = tracker.detect_regression(
            "abc123def456", "test-backend",
            {"backend/src/handlers/contratos.rs"},
        )
        assert result == []


class TestQualitySnapshotComparison:
    def test_no_regressions_when_equal(self):
        before = QualitySnapshot(clippy_warning_count=5, test_count=100, vuln_count=0)
        after = QualitySnapshot(clippy_warning_count=5, test_count=100, vuln_count=0)
        assert compare_snapshots(before, after) == []

    def test_clippy_regression_detected(self):
        before = QualitySnapshot(clippy_warning_count=0, test_count=100, vuln_count=0)
        after = QualitySnapshot(clippy_warning_count=3, test_count=100, vuln_count=0)
        regressions = compare_snapshots(before, after)
        assert len(regressions) == 1
        assert "clippy" in regressions[0]

    def test_test_count_decrease_detected(self):
        before = QualitySnapshot(clippy_warning_count=0, test_count=100, vuln_count=0)
        after = QualitySnapshot(clippy_warning_count=0, test_count=95, vuln_count=0)
        regressions = compare_snapshots(before, after)
        assert len(regressions) == 1
        assert "test count" in regressions[0]

    def test_vuln_increase_detected(self):
        before = QualitySnapshot(clippy_warning_count=0, test_count=100, vuln_count=0)
        after = QualitySnapshot(clippy_warning_count=0, test_count=100, vuln_count=2)
        regressions = compare_snapshots(before, after)
        assert len(regressions) == 1
        assert "vulnerabilities" in regressions[0]

    def test_multiple_regressions(self):
        before = QualitySnapshot(clippy_warning_count=0, test_count=100, vuln_count=0)
        after = QualitySnapshot(clippy_warning_count=5, test_count=90, vuln_count=3)
        regressions = compare_snapshots(before, after)
        assert len(regressions) == 3

    def test_improvement_not_flagged(self):
        before = QualitySnapshot(clippy_warning_count=5, test_count=100, vuln_count=3)
        after = QualitySnapshot(clippy_warning_count=2, test_count=110, vuln_count=0)
        assert compare_snapshots(before, after) == []


class TestPersistence:
    def test_save_and_load(self, tmp_state_file, mock_wsl):
        t1 = PipelineTracker()
        t1.get_or_create("abc123def456")
        t1.mark_deployed("abc123def456")

        t2 = PipelineTracker()
        summary = t2.get_summary()
        assert summary["active_lineages"] == 1
        assert summary["deployed_count"] == 1

    def test_prune_expired_entries(self, tmp_state_file, mock_wsl):
        old_run = PipelineRun(
            lineage_id="old_lineage",
            current_commit="oldcommit1234",
        )
        old_run.last_updated = datetime.now() - timedelta(hours=48)
        data = {"pipelines": {"old_lineage": _serialize_pipeline(old_run)}}
        tmp_state_file.write_text(json.dumps(data), encoding="utf-8")

        t = PipelineTracker()
        assert t.get_summary()["active_lineages"] == 0


class TestSummaryFormat:
    def test_summary_includes_quality_snapshot(self, tracker):
        tracker.get_or_create("abc123def456")
        snap = QualitySnapshot(
            clippy_warning_count=2, test_count=50,
            sonar_issue_count=1, vuln_count=0,
        )
        tracker.record_quality_snapshot("abc123def456", 1, snap)
        summary = tracker.get_summary()
        pipeline_data = summary["pipelines"]["abc123def456"]
        assert pipeline_data["quality_snapshot"] is not None
        assert pipeline_data["quality_snapshot"]["clippy_warning_count"] == 2

    def test_summary_includes_sonarqube_status(self, tracker):
        run = tracker.get_or_create("abc123def456")
        with tracker._lock:
            run.sonarqube_passed = True
        summary = tracker.get_summary()
        assert summary["pipelines"]["abc123def456"]["sonarqube_passed"] is True

    def test_summary_structure(self, tracker):
        tracker.get_or_create("abc123def456")
        summary = tracker.get_summary()
        assert "active_lineages" in summary
        assert "deployed_count" in summary
        assert "budget_exhausted_count" in summary
        assert "pipelines" in summary


class TestConcurrentAccess:
    def test_parallel_updates_same_lineage(self, tracker):
        tracker.get_or_create("abc123def456")
        errors = []
        barrier = threading.Barrier(4, timeout=5)

        def update_job(job_name):
            try:
                barrier.wait()
                tracker.mark_job_fixing("abc123def456", job_name)
                tracker.mark_job_fixed(
                    "abc123def456", job_name,
                    f"new_{job_name}", [f"{job_name}.rs"], f"strategy_{job_name}",
                )
            except Exception as e:
                errors.append(str(e))

        threads = [
            threading.Thread(target=update_job, args=(f"job-{i}",))
            for i in range(4)
        ]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=10)

        assert not errors
        run = tracker.get_or_create("abc123def456")
        assert len(run.job_states) == 4
        for i in range(4):
            assert run.job_states[f"job-{i}"] == JobState.PASSED
