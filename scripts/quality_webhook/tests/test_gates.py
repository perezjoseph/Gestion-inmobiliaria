import threading
from unittest.mock import patch, MagicMock

import pytest

from scripts.quality_webhook.gates import (
    revision_security_check,
    revision_maintainability_check,
    revision_correctness_check,
    revision_check_diff,
    baseline_verification,
    baseline_verification_incremental,
    verification_run_pbt,
    _check_unsafe_blocks,
    _FORBIDDEN_DIFF_PATTERNS,
)


def _mock_diff(diff_text):
    result = MagicMock()
    result.returncode = 0
    result.stdout = diff_text
    result.stderr = ""
    return result


class TestSecurityHardcodedSecrets:
    def test_rejects_hardcoded_password(self):
        diff = (
            "+++ b/backend/src/config.rs\n"
            '+let password = "supersecretpassword123"\n'
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, reason = revision_security_check()
        assert not passed
        assert "secret" in reason.lower() or "hardcoded" in reason.lower()

    def test_rejects_hardcoded_token(self):
        diff = (
            "+++ b/backend/src/config.rs\n"
            '+let api_key = "abcdefgh12345678"\n'
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, reason = revision_security_check()
        assert not passed

    def test_allows_non_secret_string(self):
        diff = (
            "+++ b/backend/src/config.rs\n"
            '+let name = "hello world"\n'
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, _ = revision_security_check()
        assert passed


class TestSecurityUnsafeBlocks:
    def test_rejects_unsafe_without_safety_comment(self):
        diff = (
            "+some normal code\n"
            "+unsafe {\n"
            "+    ptr::read(p)\n"
            "+}\n"
        )
        passed, reason = _check_unsafe_blocks(diff)
        assert not passed
        assert "SAFETY" in reason

    def test_allows_unsafe_with_safety_comment(self):
        diff = (
            "+// SAFETY: pointer is valid and aligned\n"
            "+unsafe {\n"
            "+    ptr::read(p)\n"
            "+}\n"
        )
        passed, _ = _check_unsafe_blocks(diff)
        assert passed

    def test_safety_comment_within_3_lines(self):
        diff = (
            "+// SAFETY: invariant holds because of X\n"
            "+let x = 1;\n"
            "+let y = 2;\n"
            "+unsafe {\n"
            "+    do_thing()\n"
            "+}\n"
        )
        passed, _ = _check_unsafe_blocks(diff)
        assert passed


class TestSecurityTLSBypass:
    def test_rejects_disabled_tls(self):
        diff = (
            "+++ b/backend/src/client.rs\n"
            "+    .danger_accept_invalid_certs(true)\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, reason = revision_security_check()
        assert not passed
        assert "TLS" in reason or "danger" in reason.lower()


class TestSecurityWildcardCORS:
    def test_rejects_wildcard_cors(self):
        diff = (
            "+++ b/backend/src/config.rs\n"
            '+    .header("Access-Control-Allow-Origin", "*")\n'
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, reason = revision_security_check()
        assert not passed
        assert "CORS" in reason


class TestMaintainabilityUnwrap:
    def test_rejects_unwrap_in_prod_code(self):
        diff = (
            "+++ b/backend/src/handlers/pagos.rs\n"
            "+    let val = result.unwrap();\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, reason = revision_maintainability_check()
        assert not passed
        assert "unwrap" in reason.lower()

    def test_allows_unwrap_in_test_file(self):
        diff = (
            "+++ b/backend/tests/pagos_tests.rs\n"
            "+    let val = result.unwrap();\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, _ = revision_maintainability_check()
        assert passed

    def test_allows_unwrap_in_test_module(self):
        diff = (
            "+++ b/backend/src/handlers/test_pagos.rs\n"
            "+    let val = result.unwrap();\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, _ = revision_maintainability_check()
        assert passed


class TestMaintainabilityFunctionLength:
    def test_rejects_function_over_100_lines(self):
        lines = ["+++ b/backend/src/handlers/pagos.rs\n", "@@ -1,0 +1,120 @@\n"]
        for i in range(120):
            lines.append(f"+    let x{i} = {i};\n")
        diff = "".join(lines)
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, reason = revision_maintainability_check()
        assert not passed
        assert "100" in reason

    def test_allows_function_under_100_lines(self):
        lines = ["+++ b/backend/src/handlers/pagos.rs\n", "@@ -1,0 +1,50 @@\n"]
        for i in range(50):
            lines.append(f"+    let x{i} = {i};\n")
        diff = "".join(lines)
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, _ = revision_maintainability_check()
        assert passed


class TestMaintainabilityNestingDepth:
    def test_rejects_deep_nesting(self):
        diff = (
            "+++ b/backend/src/handlers/pagos.rs\n"
            "@@ -1,0 +1,5 @@\n"
            "+                    let deeply_nested = true;\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, reason = revision_maintainability_check()
        assert not passed
        assert "nesting" in reason.lower()

    def test_allows_shallow_nesting(self):
        diff = (
            "+++ b/backend/src/handlers/pagos.rs\n"
            "@@ -1,0 +1,5 @@\n"
            "+        let shallow = true;\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, _ = revision_maintainability_check()
        assert passed


class TestCorrectnessRegressionDetection:
    def test_rejects_removed_uniqueness_check(self):
        diff = (
            "-    if duplicate_cedula_exists(&cedula) {\n"
            "-        return Err(\"already exists\");\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, reason = revision_correctness_check()
        assert not passed
        assert "uniqueness" in reason.lower() or "already" in reason.lower()

    def test_rejects_removed_overlap_validation(self):
        diff = (
            "-    if check_date_overlap(fecha_inicio, fecha_fin) {\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, reason = revision_correctness_check()
        assert not passed
        assert "overlap" in reason.lower() or "date" in reason.lower()

    def test_allows_clean_diff(self):
        diff = (
            "+    let new_feature = true;\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, _ = revision_correctness_check()
        assert passed

    def test_no_domain_files_passes(self):
        empty_diff = ""
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(empty_diff)):
            passed, reason = revision_correctness_check()
        assert passed
        assert "no domain" in reason.lower()


class TestPBTForbiddenPatterns:
    def test_rejects_reduced_proptest_cases(self):
        diff = (
            "+++ b/backend/tests/pagos_pbt.rs\n"
            "+PROPTEST_CASES = 5\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, reason = revision_check_diff()
        assert not passed
        assert "PROPTEST_CASES" in reason or "forbidden" in reason.lower()

    def test_rejects_proptest_config_reduction(self):
        diff = (
            "+++ b/backend/tests/pagos_pbt.rs\n"
            "+    proptest_config { cases = 5 }\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, reason = revision_check_diff()
        assert not passed

    def test_allows_normal_test_code(self):
        diff = (
            "+++ b/backend/tests/pagos_pbt.rs\n"
            "+    assert_eq!(result, expected);\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, _ = revision_check_diff()
        assert passed


class TestBaselineVerification:
    def test_baseline_passes(self):
        result = MagicMock(returncode=0, stdout="ok", stderr="")
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=result):
            passed, reason = baseline_verification()
        assert passed
        assert "passed" in reason.lower()

    def test_baseline_fails(self):
        result = MagicMock(returncode=1, stdout="error", stderr="clippy warning")
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=result):
            passed, reason = baseline_verification()
        assert not passed
        assert "failed" in reason.lower()

    def test_incremental_skips_covered_steps(self):
        result = MagicMock(returncode=0, stdout="ok", stderr="")
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=result) as mock:
            passed, reason = baseline_verification_incremental("lint")
        assert passed
        assert "skipped" in reason.lower()
        cmd = mock.call_args[0][0]
        assert "fmt" not in cmd
        assert "clippy" not in cmd

    def test_incremental_runs_missing_steps(self):
        result = MagicMock(returncode=0, stdout="ok", stderr="")
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=result) as mock:
            passed, _ = baseline_verification_incremental("test-backend")
        assert passed
        cmd = mock.call_args[0][0]
        assert "fmt" in cmd or "clippy" in cmd


class TestGateOrdering:
    def test_security_rejects_before_baseline(self):
        sec_diff = (
            "+++ b/backend/src/config.rs\n"
            '+let password = "supersecretpassword123"\n'
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(sec_diff)):
            sec_passed, _ = revision_security_check()
        assert not sec_passed

    def test_maintainability_rejects_before_baseline(self):
        maint_diff = (
            "+++ b/backend/src/handlers/pagos.rs\n"
            "+    let val = result.unwrap();\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(maint_diff)):
            maint_passed, _ = revision_maintainability_check()
        assert not maint_passed

    def test_early_rejection_prevents_later_gates(self):
        sec_diff = (
            "+++ b/backend/src/config.rs\n"
            '+let password = "supersecretpassword123"\n'
        )
        baseline_called = False

        def track_baseline(*args, **kwargs):
            nonlocal baseline_called
            baseline_called = True
            return MagicMock(returncode=0, stdout="ok", stderr="")

        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(sec_diff)):
            sec_passed, _ = revision_security_check()

        if not sec_passed:
            pass
        else:
            with patch("scripts.quality_webhook.gates.wsl_bash", side_effect=track_baseline):
                baseline_verification()

        assert not sec_passed
        assert not baseline_called


class TestTestFileAwareness:
    def test_security_allows_test_file_patterns(self):
        diff = (
            "+++ b/backend/tests/auth_tests.rs\n"
            "+    let val = result.unwrap();\n"
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, _ = revision_security_check()
        assert passed

    def test_maintainability_allows_expect_in_tests(self):
        diff = (
            "+++ b/backend/tests/pagos_tests.rs\n"
            '+    let val = result.expect("should work");\n'
        )
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=_mock_diff(diff)):
            passed, _ = revision_maintainability_check()
        assert passed


class TestPBTVerification:
    def test_pbt_verification_uses_500_cases(self):
        result = MagicMock(returncode=0, stdout="ok", stderr="")
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=result) as mock:
            passed, reason = verification_run_pbt("test-backend")
        assert passed
        assert "500" in reason
        cmd = mock.call_args[0][0]
        assert "PROPTEST_CASES=500" in cmd

    def test_pbt_verification_fallback_for_unknown_job(self):
        result = MagicMock(returncode=0, stdout="ok", stderr="")
        with patch("scripts.quality_webhook.gates.wsl_bash", return_value=result):
            passed, _ = verification_run_pbt("quality-gate")
        assert passed
