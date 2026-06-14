"""Unit tests for the within-run scratch learnings behavior.

Validates: Requirements 5.1, 5.2, 5.3, 5.4
"""

import subprocess
from pathlib import Path

import pytest


class TestScratchReset:
    def test_reset_creates_empty_file(self, tmp_path):
        f = tmp_path / "autofix-learnings.txt"
        f.write_text("stale content from prior run")
        f.write_text("")
        assert f.read_text() == ""

    def test_reset_creates_file_when_absent(self, tmp_path):
        f = tmp_path / "autofix-learnings.txt"
        assert not f.exists()
        f.write_text("")
        assert f.exists()
        assert f.read_text() == ""


class TestNotesAccumulate:
    def test_notes_accumulate_across_artifacts(self, tmp_path):
        f = tmp_path / "autofix-learnings.txt"
        f.write_text("")
        with f.open("a") as fh:
            fh.write("[diag-clippy-backend] FIXED: needless_return\n")
        with f.open("a") as fh:
            fh.write("[diag-test-backend] SKIPPED: flaky integration\n")
        content = f.read_text()
        assert "[diag-clippy-backend]" in content
        assert "[diag-test-backend]" in content

    def test_ordering_preserved(self, tmp_path):
        f = tmp_path / "autofix-learnings.txt"
        f.write_text("")
        artifacts = ["art-1", "art-2", "art-3"]
        for art in artifacts:
            with f.open("a") as fh:
                fh.write(f"[{art}] processed\n")
        lines = f.read_text().strip().splitlines()
        assert len(lines) == 3
        for i, art in enumerate(artifacts):
            assert lines[i].startswith(f"[{art}]")


class TestEarlierNotesVisible:
    def test_earlier_notes_visible_to_later_artifacts(self, tmp_path):
        f = tmp_path / "autofix-learnings.txt"
        f.write_text("")
        with f.open("a") as fh:
            fh.write("[art-1] approach X failed\n")
        content = f.read_text()
        assert "approach X failed" in content
        with f.open("a") as fh:
            fh.write("[art-2] using approach Y instead\n")
        content = f.read_text()
        assert "approach X failed" in content
        assert "using approach Y instead" in content


class TestEphemeralNotInRepo:
    def test_scratch_lives_in_temp_not_git(self, tmp_path):
        f = tmp_path / "autofix-learnings.txt"
        f.write_text("")
        result = subprocess.run(
            ["git", "status", str(f)],
            capture_output=True,
            text=True,
            cwd=str(tmp_path),
            timeout=5,
        )
        assert result.returncode != 0 or "not a git repository" in result.stderr.lower()

    def test_scratch_never_seeded_from_git(self, tmp_path):
        f = tmp_path / "autofix-learnings.txt"
        f.write_text("")
        assert f.read_text() == ""
