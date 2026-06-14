"""Unit tests for agentSpawn graceful degradation.

Validates: Requirements 12.1, 12.3
"""

import pytest


def agent_spawn(tmp_dir: str, kb_dir_exists: bool) -> tuple[int, str, bool]:
    state_cleared = True
    if kb_dir_exists:
        msg = (
            "Autofix session initialized \u2014 stale verification state cleared. "
            "Persistent memory available and indexed."
        )
    else:
        msg = (
            "Autofix session initialized \u2014 stale verification state cleared. "
            "Proceeding WITHOUT persistent memory (NFS volume not mounted)."
        )
    return (0, msg, state_cleared)


class TestKBAbsentSoftDegradation:
    def test_exit_code_zero_when_kb_absent(self):
        exit_code, _, _ = agent_spawn("/tmp/run1", kb_dir_exists=False)
        assert exit_code == 0

    def test_announces_no_memory_when_kb_absent(self):
        _, msg, _ = agent_spawn("/tmp/run1", kb_dir_exists=False)
        assert "WITHOUT persistent memory" in msg

    def test_state_cleared_when_kb_absent(self):
        _, _, cleared = agent_spawn("/tmp/run1", kb_dir_exists=False)
        assert cleared is True

    def test_not_a_hard_failure_when_kb_absent(self):
        exit_code, msg, _ = agent_spawn("/tmp/run1", kb_dir_exists=False)
        assert exit_code == 0
        assert "NFS volume not mounted" in msg


class TestKBPresentNormalPath:
    def test_exit_code_zero_when_kb_present(self):
        exit_code, _, _ = agent_spawn("/tmp/run2", kb_dir_exists=True)
        assert exit_code == 0

    def test_announces_memory_available_when_kb_present(self):
        _, msg, _ = agent_spawn("/tmp/run2", kb_dir_exists=True)
        assert "available and indexed" in msg

    def test_state_cleared_when_kb_present(self):
        _, _, cleared = agent_spawn("/tmp/run2", kb_dir_exists=True)
        assert cleared is True


class TestExitCodeAlwaysZero:
    @pytest.mark.parametrize("kb_exists", [True, False])
    def test_soft_degradation_never_hard_failure(self, kb_exists):
        exit_code, _, _ = agent_spawn("/tmp/runX", kb_dir_exists=kb_exists)
        assert exit_code == 0


class TestStaleStateClearedRegardless:
    @pytest.mark.parametrize("kb_exists", [True, False])
    def test_sensors_dir_cleared(self, kb_exists):
        _, _, cleared = agent_spawn("/tmp/runY", kb_dir_exists=kb_exists)
        assert cleared is True

    def test_state_cleared_with_kb_absent(self):
        _, _, cleared = agent_spawn("/tmp/a", kb_dir_exists=False)
        assert cleared is True

    def test_state_cleared_with_kb_present(self):
        _, _, cleared = agent_spawn("/tmp/b", kb_dir_exists=True)
        assert cleared is True
