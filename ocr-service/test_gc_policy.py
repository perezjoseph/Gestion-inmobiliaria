"""Property 7: Bounded memory store — after a GC run the store is trimmed to the configured age and/or size cap.

Validates: Requirements 10.12
"""

import os
import time
from pathlib import Path

import pytest


def gc_run(kb_path: Path, age_days: int = 30, max_size_mb: int = 500) -> dict:
    age_removed = 0
    size_removed = 0

    if not kb_path.is_dir():
        return {"age_removed": age_removed, "size_removed": size_removed}

    now = time.time()
    age_cutoff = now - (age_days * 86400)

    for f in list(kb_path.rglob("*")):
        if f.is_file() and f.stat().st_mtime < age_cutoff:
            f.unlink()
            age_removed += 1

    max_size_bytes = max_size_mb * 1024 * 1024
    files = [f for f in kb_path.rglob("*") if f.is_file()]
    total = sum(f.stat().st_size for f in files)

    if total > max_size_bytes:
        files_by_age = sorted(files, key=lambda f: f.stat().st_mtime)
        for f in files_by_age:
            if total <= max_size_bytes:
                break
            sz = f.stat().st_size
            f.unlink()
            total -= sz
            size_removed += 1

    for d in sorted(kb_path.rglob("*"), reverse=True):
        if d.is_dir() and not any(d.iterdir()):
            d.rmdir()

    return {"age_removed": age_removed, "size_removed": size_removed}


def _create_file(path: Path, size_bytes: int, age_days: float) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"\x00" * size_bytes)
    mtime = time.time() - (age_days * 86400)
    os.utime(path, (mtime, mtime))


class TestAgeCap:
    def test_files_older_than_age_cap_removed(self, tmp_path):
        kb = tmp_path / "knowledge_bases"
        kb.mkdir()
        _create_file(kb / "old_lesson.txt", 100, age_days=45)
        _create_file(kb / "ancient.txt", 100, age_days=90)

        result = gc_run(kb, age_days=30)

        assert result["age_removed"] == 2
        assert not (kb / "old_lesson.txt").exists()
        assert not (kb / "ancient.txt").exists()

    def test_files_within_age_cap_retained(self, tmp_path):
        kb = tmp_path / "knowledge_bases"
        kb.mkdir()
        _create_file(kb / "recent.txt", 100, age_days=5)
        _create_file(kb / "fresh.txt", 100, age_days=0.5)

        result = gc_run(kb, age_days=30)

        assert result["age_removed"] == 0
        assert (kb / "recent.txt").exists()
        assert (kb / "fresh.txt").exists()


class TestSizeCap:
    def test_oldest_files_removed_when_over_size_cap(self, tmp_path):
        kb = tmp_path / "knowledge_bases"
        kb.mkdir()
        _create_file(kb / "oldest.bin", 600 * 1024, age_days=10)
        _create_file(kb / "middle.bin", 600 * 1024, age_days=5)
        _create_file(kb / "newest.bin", 600 * 1024, age_days=1)

        result = gc_run(kb, age_days=30, max_size_mb=1)

        assert result["size_removed"] >= 1
        assert (kb / "newest.bin").exists()
        remaining_size = sum(
            f.stat().st_size for f in kb.rglob("*") if f.is_file()
        )
        assert remaining_size <= 1 * 1024 * 1024

    def test_no_eviction_when_under_size_cap(self, tmp_path):
        kb = tmp_path / "knowledge_bases"
        kb.mkdir()
        _create_file(kb / "small1.txt", 100, age_days=5)
        _create_file(kb / "small2.txt", 100, age_days=3)

        result = gc_run(kb, age_days=30, max_size_mb=1)

        assert result["size_removed"] == 0
        assert (kb / "small1.txt").exists()
        assert (kb / "small2.txt").exists()


class TestEmptyStore:
    def test_empty_kb_dir_no_error(self, tmp_path):
        kb = tmp_path / "knowledge_bases"
        kb.mkdir()

        result = gc_run(kb, age_days=30, max_size_mb=500)

        assert result["age_removed"] == 0
        assert result["size_removed"] == 0

    def test_nonexistent_kb_dir_no_error(self, tmp_path):
        kb = tmp_path / "knowledge_bases"

        result = gc_run(kb, age_days=30, max_size_mb=500)

        assert result["age_removed"] == 0
        assert result["size_removed"] == 0


class TestMixedAgeAndSize:
    def test_both_phases_work_together(self, tmp_path):
        kb = tmp_path / "knowledge_bases"
        kb.mkdir()
        _create_file(kb / "expired.bin", 400 * 1024, age_days=45)
        _create_file(kb / "old_but_valid.bin", 700 * 1024, age_days=10)
        _create_file(kb / "recent_large.bin", 700 * 1024, age_days=2)
        _create_file(kb / "recent_small.bin", 100 * 1024, age_days=1)

        result = gc_run(kb, age_days=30, max_size_mb=1)

        assert result["age_removed"] == 1
        assert not (kb / "expired.bin").exists()
        assert result["size_removed"] >= 1
        remaining = sum(
            f.stat().st_size for f in kb.rglob("*") if f.is_file()
        )
        assert remaining <= 1 * 1024 * 1024

    def test_subdirectories_handled(self, tmp_path):
        kb = tmp_path / "knowledge_bases"
        sub = kb / "branch_main"
        sub.mkdir(parents=True)
        _create_file(sub / "old.txt", 100, age_days=60)
        _create_file(sub / "new.txt", 100, age_days=1)

        result = gc_run(kb, age_days=30)

        assert result["age_removed"] == 1
        assert not (sub / "old.txt").exists()
        assert (sub / "new.txt").exists()
