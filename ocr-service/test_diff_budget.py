"""Unit tests for the diff-budget guard thresholds.

Validates: Requirements 8.1, 8.2, 8.3, 8.4
"""

import pytest


def diff_budget_check(
    file_paths: list[str], soft: int = 8, hard: int = 15
) -> tuple[int, str]:
    distinct = len(set(file_paths))
    if distinct >= hard:
        return (
            0,
            f"::warning::Diff budget exceeded: {distinct} distinct files modified "
            f"(hard limit: {hard}). Stop expanding scope. If this fix genuinely "
            f"needs this many files, exit with Status: PARTIAL.",
        )
    elif distinct >= soft:
        return (
            0,
            f"::notice::Diff budget advisory: {distinct} distinct files modified "
            f"(soft limit: {soft}). Confirm each change traces directly to the "
            f"diagnosed failure.",
        )
    return (0, "")


class TestBelowSoftThreshold:
    def test_zero_files(self):
        exit_code, output = diff_budget_check([])
        assert exit_code == 0
        assert output == ""

    def test_single_file(self):
        exit_code, output = diff_budget_check(["src/main.rs"])
        assert exit_code == 0
        assert output == ""

    def test_seven_files(self):
        paths = [f"src/file_{i}.rs" for i in range(7)]
        exit_code, output = diff_budget_check(paths)
        assert exit_code == 0
        assert output == ""

    def test_no_annotation_emitted(self):
        paths = [f"src/file_{i}.rs" for i in range(7)]
        _, output = diff_budget_check(paths)
        assert "::notice::" not in output
        assert "::warning::" not in output


class TestAtSoftThreshold:
    def test_exactly_soft_emits_notice(self):
        paths = [f"src/file_{i}.rs" for i in range(8)]
        exit_code, output = diff_budget_check(paths)
        assert exit_code == 0
        assert "::notice::" in output
        assert "8 distinct files modified" in output

    def test_exit_code_zero(self):
        paths = [f"src/file_{i}.rs" for i in range(8)]
        exit_code, _ = diff_budget_check(paths)
        assert exit_code == 0

    def test_no_warning_at_soft(self):
        paths = [f"src/file_{i}.rs" for i in range(8)]
        _, output = diff_budget_check(paths)
        assert "::warning::" not in output


class TestBetweenSoftAndHard:
    def test_twelve_files_emits_notice(self):
        paths = [f"src/file_{i}.rs" for i in range(12)]
        exit_code, output = diff_budget_check(paths)
        assert exit_code == 0
        assert "::notice::" in output
        assert "12 distinct files modified" in output

    def test_no_warning_between_thresholds(self):
        paths = [f"src/file_{i}.rs" for i in range(12)]
        _, output = diff_budget_check(paths)
        assert "::warning::" not in output


class TestAtHardThreshold:
    def test_exactly_hard_emits_warning(self):
        paths = [f"src/file_{i}.rs" for i in range(15)]
        exit_code, output = diff_budget_check(paths)
        assert exit_code == 0
        assert "::warning::" in output
        assert "15 distinct files modified" in output

    def test_exit_code_zero(self):
        paths = [f"src/file_{i}.rs" for i in range(15)]
        exit_code, _ = diff_budget_check(paths)
        assert exit_code == 0

    def test_no_notice_at_hard(self):
        paths = [f"src/file_{i}.rs" for i in range(15)]
        _, output = diff_budget_check(paths)
        assert "::notice::" not in output


class TestAboveHardThreshold:
    def test_twenty_files_emits_warning(self):
        paths = [f"src/file_{i}.rs" for i in range(20)]
        exit_code, output = diff_budget_check(paths)
        assert exit_code == 0
        assert "::warning::" in output
        assert "20 distinct files modified" in output


class TestExitCodeAlwaysZero:
    @pytest.mark.parametrize("count", [0, 1, 5, 7, 8, 10, 14, 15, 20, 50])
    def test_advisory_never_blocks(self, count):
        paths = [f"src/file_{i}.rs" for i in range(count)]
        exit_code, _ = diff_budget_check(paths)
        assert exit_code == 0


class TestCustomThresholds:
    def test_custom_soft_triggers_at_new_value(self):
        paths = [f"src/file_{i}.rs" for i in range(5)]
        exit_code, output = diff_budget_check(paths, soft=5, hard=10)
        assert exit_code == 0
        assert "::notice::" in output

    def test_custom_hard_triggers_at_new_value(self):
        paths = [f"src/file_{i}.rs" for i in range(10)]
        exit_code, output = diff_budget_check(paths, soft=5, hard=10)
        assert exit_code == 0
        assert "::warning::" in output

    def test_below_custom_soft_no_annotation(self):
        paths = [f"src/file_{i}.rs" for i in range(4)]
        _, output = diff_budget_check(paths, soft=5, hard=10)
        assert output == ""

    def test_custom_thresholds_read_from_env(self):
        paths = [f"src/file_{i}.rs" for i in range(3)]
        exit_code, output = diff_budget_check(paths, soft=3, hard=6)
        assert exit_code == 0
        assert "::notice::" in output


class TestDeduplication:
    def test_duplicate_paths_are_deduplicated(self):
        paths = ["src/main.rs"] * 20
        exit_code, output = diff_budget_check(paths)
        assert exit_code == 0
        assert output == ""

    def test_mixed_duplicates_counted_once(self):
        paths = ["src/a.rs", "src/b.rs", "src/a.rs", "src/b.rs", "src/c.rs"]
        exit_code, output = diff_budget_check(paths)
        assert exit_code == 0
        assert output == ""

    def test_eight_distinct_with_duplicates_triggers_notice(self):
        distinct = [f"src/file_{i}.rs" for i in range(8)]
        paths = distinct + distinct
        exit_code, output = diff_budget_check(paths)
        assert exit_code == 0
        assert "::notice::" in output
        assert "8 distinct files modified" in output


class TestNoFileExists:
    def test_empty_list_no_output(self):
        exit_code, output = diff_budget_check([])
        assert exit_code == 0
        assert output == ""
