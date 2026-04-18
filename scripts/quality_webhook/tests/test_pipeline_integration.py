import json
import threading
from unittest.mock import patch, MagicMock, PropertyMock

import pytest

from scripts.quality_webhook.tracker import (
    PipelineTracker, PipelineRun, JobState, QualitySnapshot,
)
from scripts.quality_webhook.classifier import classify_error
from scripts.quality_webhook.fixers import build_invariant_section


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


class TestTrackerStateTransitions:
    def test_full_lifecycle(self, tracker):
        run = tracker.get_or_create("commit_abc")
        assert run.job_states == {}

        tracker.register_failure("commit_abc", "lint", "error log", run_url="run1")
        run = tracker.get_or_create("commit_abc")
        assert run.job_states["lint"] == JobState.FAILED

        tracker.mark_job_fixing("commit_abc", "lint")
        run = tracker.get_or_create("commit_abc")
        assert run.job_states["lint"] == JobState.FIXING

        tracker.mark_job_fixed("commit_abc", "lint", "new_sha", ["file.rs"], "quick_fix")
        run = tracker.get_or_create("commit_abc")
        assert run.job_states["lint"] == JobState.PASSED

    def test_deploy_success_terminates_pipeline(self, tracker):
        tracker.get_or_create("commit_abc")
        tracker.update_from_report("commit_abc", {
            "lint": "success",
            "test-backend": "success",
            "deploy": "success",
        })
        tracker.mark_deployed("commit_abc")
        run = tracker.get_or_create("commit_abc")
        assert run.deployed is True


class TestDeploySuccessTermination:
    def test_deployed_pipeline_skips_further_fixes(self, tracker):
        tracker.get_or_create("commit_abc")
        tracker.mark_deployed("commit_abc")
        run = tracker.get_or_create("commit_abc")
        assert run.deployed is True

    def test_strategy_confirmed_on_deploy(self, tracker):
        tracker.get_or_create("commit_abc")
        tracker.mark_job_fixed("commit_abc", "lint", "new_sha", ["f.rs"], "strat1")
        run = tracker.get_or_create("commit_abc")
        assert "lint" in run.pending_strategies

        with patch("scripts.quality_webhook.decisions.record_strategy_outcome"):
            tracker.confirm_strategy("commit_abc", "lint", True)
        run = tracker.get_or_create("commit_abc")
        assert "lint" not in run.pending_strategies


class TestQualityGateEnforcement:
    def test_security_gate_rejects_hardcoded_secret(self):
        from scripts.quality_webhook.gates import revision_security_check
        diff = (
            "+++ b/backend/src/config.rs\n"
            '+let password = "supersecretpassword123"\n'
        )
        mock_result = MagicMock(returncode=0, stdout=diff, stderr="")
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=mock_result):
            passed, _ = revision_security_check()
        assert not passed

    def test_maintainability_gate_rejects_unwrap(self):
        from scripts.quality_webhook.gates import revision_maintainability_check
        diff = (
            "+++ b/backend/src/handlers/pagos.rs\n"
            "+    let val = result.unwrap();\n"
        )
        mock_result = MagicMock(returncode=0, stdout=diff, stderr="")
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=mock_result):
            passed, _ = revision_maintainability_check()
        assert not passed

    def test_correctness_gate_rejects_removed_validation(self):
        from scripts.quality_webhook.gates import revision_correctness_check
        diff = "-    if duplicate_cedula_exists(&cedula) {\n"
        mock_result = MagicMock(returncode=0, stdout=diff, stderr="")
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=mock_result):
            passed, _ = revision_correctness_check()
        assert not passed


class TestDomainInvariantInjection:
    def test_contratos_invariants_injected(self):
        files = ["backend/src/handlers/contratos.rs"]
        section = build_invariant_section(files)
        assert "overlapping" in section.lower()
        assert "contrato" in section.lower()

    def test_inquilinos_invariants_injected(self):
        files = ["backend/src/services/inquilinos.rs"]
        section = build_invariant_section(files)
        assert "cedula" in section.lower()

    def test_no_invariants_for_unrelated_files(self):
        files = ["backend/src/config.rs"]
        section = build_invariant_section(files)
        assert section == ""

    def test_multiple_domains_combined(self):
        files = [
            "backend/src/handlers/contratos.rs",
            "backend/src/services/pagos.rs",
        ]
        section = build_invariant_section(files)
        assert "contrato" in section.lower()
        assert "pago" in section.lower() or "payment" in section.lower()


class TestBackwardCompatibility:
    def test_webhook_without_lineage_tag_creates_new(self, tracker, mock_wsl):
        mock_wsl.return_value = MagicMock(returncode=0, stdout="feat: add new feature\n")
        run = tracker.get_or_create("aabb11223344")
        assert run.lineage_id == "aabb11223344"
        assert run.round_number == 1

    def test_webhook_without_lineage_works_normally(self, tracker, mock_wsl):
        mock_wsl.return_value = MagicMock(returncode=0, stdout="feat: add new feature\n")
        run = tracker.get_or_create("aabb11223344")
        tracker.register_failure("aabb11223344", "lint", "error log")
        run = tracker.get_or_create("aabb11223344")
        assert run.job_states["lint"] == JobState.FAILED

        tracker.mark_job_fixing("aabb11223344", "lint")
        run = tracker.get_or_create("aabb11223344")
        assert run.job_states["lint"] == JobState.FIXING


class TestConcurrentLineageAccess:
    def test_multiple_threads_same_lineage(self, tracker):
        tracker.get_or_create("commit_abc")
        errors = []
        barrier = threading.Barrier(5, timeout=5)

        def worker(idx):
            try:
                barrier.wait()
                tracker.register_failure("commit_abc", f"job-{idx}", "error")
                tracker.mark_job_fixing("commit_abc", f"job-{idx}")
            except Exception as e:
                errors.append(str(e))

        threads = [threading.Thread(target=worker, args=(i,)) for i in range(5)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=10)

        assert not errors
        run = tracker.get_or_create("commit_abc")
        assert len(run.job_states) == 5


class TestQualitySnapshotCapture:
    def test_snapshot_stored_and_retrieved(self, tracker):
        tracker.get_or_create("commit_abc")
        snap = QualitySnapshot(
            clippy_warning_count=3, test_count=200,
            sonar_issue_count=5, vuln_count=1,
        )
        tracker.record_quality_snapshot("commit_abc", 1, snap)
        run = tracker.get_or_create("commit_abc")
        assert 1 in run.quality_snapshots
        stored = run.quality_snapshots[1]
        assert stored.clippy_warning_count == 3
        assert stored.test_count == 200

    def test_regression_detected_between_snapshots(self):
        from scripts.quality_webhook.tracker import compare_snapshots
        before = QualitySnapshot(clippy_warning_count=0, test_count=100, vuln_count=0)
        after = QualitySnapshot(clippy_warning_count=5, test_count=100, vuln_count=0)
        regressions = compare_snapshots(before, after)
        assert len(regressions) == 1
        assert "clippy" in regressions[0]

    def test_snapshot_in_health_summary(self, tracker):
        tracker.get_or_create("commit_abc")
        snap = QualitySnapshot(
            clippy_warning_count=2, test_count=50,
            sonar_issue_count=0, vuln_count=0,
        )
        tracker.record_quality_snapshot("commit_abc", 1, snap)
        summary = tracker.get_summary()
        pipeline_data = summary["pipelines"]["commit_abc"]
        assert pipeline_data["quality_snapshot"]["clippy_warning_count"] == 2


class TestPBTSeedPersistence:
    def test_pbt_seeds_parsed_from_output(self):
        from scripts.quality_webhook.reproducer import _parse_pbt_failures
        output = (
            "test test_overlap ... FAILED\n"
            "PROPTEST_SEED=12345\n"
            "minimal failing input: (Decimal(-1, 2), \"\")\n"
        )
        errors = _parse_pbt_failures(output)
        assert len(errors) == 2
        seed_error = [e for e in errors if "seed" in e.code]
        assert len(seed_error) == 1
        assert "12345" in seed_error[0].message

    def test_pbt_seeds_stored_in_fix_history(self):
        from scripts.quality_webhook.history import record_fix_attempt, _load_fix_history, _fix_history_lock
        import tempfile
        from pathlib import Path

        with tempfile.NamedTemporaryFile(suffix=".json", delete=False, mode="w") as f:
            f.write("{}")
            tmp_path = Path(f.name)

        try:
            with (
                patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", tmp_path),
                patch("scripts.quality_webhook.history._fix_history_lock", threading.Lock()),
            ):
                record_fix_attempt(
                    "test-backend", "proptest error", 1, False,
                    "class=pbt_failure. PBT: seed:12345 proptest failure",
                )
                history = _load_fix_history()
                found_pbt = False
                for key, entry in history.items():
                    for attempt in entry.get("attempts", []):
                        if "PBT" in attempt.get("notes", "") or "seed" in attempt.get("notes", ""):
                            found_pbt = True
                assert found_pbt
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_pbt_seed_included_in_prompt_extras(self):
        from scripts.quality_webhook.reproducer import _parse_pbt_failures, ParsedError
        output = "PROPTEST_SEED=99999\nminimal failing input: (42, true)\n"
        errors = _parse_pbt_failures(output)
        seed_notes = "\n".join(str(e) for e in errors[:5])
        assert "99999" in seed_notes
        assert "minimal failing input" in seed_notes


class TestHealthEndpointData:
    def test_health_includes_pipeline_tracker(self, tracker):
        tracker.get_or_create("commit_abc")
        summary = tracker.get_summary()
        assert "active_lineages" in summary
        assert "deployed_count" in summary
        assert "budget_exhausted_count" in summary
        assert "pipelines" in summary

    def test_health_pipeline_includes_quality_and_sonar(self, tracker):
        run = tracker.get_or_create("commit_abc")
        with tracker._lock:
            run.sonarqube_passed = False
        snap = QualitySnapshot(clippy_warning_count=1, test_count=50)
        tracker.record_quality_snapshot("commit_abc", 1, snap)

        summary = tracker.get_summary()
        p = summary["pipelines"]["commit_abc"]
        assert "sonarqube_passed" in p
        assert p["sonarqube_passed"] is False
        assert p["quality_snapshot"] is not None
        assert p["quality_snapshot"]["clippy_warning_count"] == 1
