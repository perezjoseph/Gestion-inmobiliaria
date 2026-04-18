import pytest

from scripts.quality_webhook.classifier import classify_error


class TestPBTFailureClassification:
    def test_proptest_keyword(self):
        log = "thread 'test_overlap' panicked at proptest failure"
        assert classify_error(log) == "pbt_failure"

    def test_minimal_failing_input(self):
        log = "minimal failing input: (Decimal(-1, 2), '')"
        assert classify_error(log) == "pbt_failure"

    def test_shrunk_to(self):
        log = "test failed: attempt 42, shrunk to: [0, 1]"
        assert classify_error(log) == "pbt_failure"

    def test_proptest_config(self):
        log = "proptest_config { cases: 256 } failed after 10 iterations"
        assert classify_error(log) == "pbt_failure"

    def test_testrunner(self):
        log = "TestRunner encountered a failure"
        assert classify_error(log) == "pbt_failure"


class TestAppBugClassification:
    def test_health_check_failure(self):
        log = "[health-check] App health check failed. /health not responding."
        assert classify_error(log) == "app_bug"

    def test_health_endpoint(self):
        log = "GET /health returned 503"
        assert classify_error(log) == "app_bug"

    def test_not_responding(self):
        log = "Service not responding after 30s timeout"
        assert classify_error(log) == "app_bug"


class TestExistingClassesUnchanged:
    def test_runner_environment(self):
        log = "command not found: cargo"
        assert classify_error(log) == "runner_environment"

    def test_dependency(self):
        log = "RUSTSEC-2024-0001 found in crate foo"
        assert classify_error(log) == "dependency"

    def test_flaky(self):
        log = "connection reset by peer during test"
        assert classify_error(log) == "flaky"

    def test_test_failure(self):
        log = "test result: FAILED. 2 passed; 1 failed"
        assert classify_error(log) == "test_failure"

    def test_code_quality(self):
        log = "clippy warning: unused variable"
        assert classify_error(log) == "code_quality"

    def test_build_failure(self):
        log = "error[E0308]: mismatched types"
        assert classify_error(log) == "build_failure"

    def test_unknown(self):
        log = "something completely unrecognizable happened"
        assert classify_error(log) == "unknown"


class TestDeployClassification:
    def test_docker_compose_is_runner_env(self):
        log = "[deploy] docker compose up failed with exit code 1"
        assert classify_error(log) == "runner_environment"

    def test_attestation_is_security(self):
        log = "[attestation] Backend image attestation verification failed"
        assert classify_error(log) == "security"

    def test_health_check_is_app_bug(self):
        log = "health-check failed: container started but /health returns 500"
        assert classify_error(log) == "app_bug"
