"""Integration and smoke tests for CI pipeline optimization.

Task 14.1: Concurrent fix processing integration tests.
  Requirements: 5.1, 5.3, 5.4

Task 14.2: Smoke tests for configuration values.
  Requirements: 3.1, 9.2, 6.1, 1.2, 7.1
"""

import threading
import time
from pathlib import Path

import pytest
import yaml


REPO_ROOT = Path(__file__).resolve().parent.parent.parent.parent


class TestConcurrentFixProcessing:
    """Integration tests for per-job locking and concurrent fix processing.

    Validates Requirements 5.1, 5.3, 5.4.
    """

    def test_three_jobs_acquire_locks_in_parallel(self):
        """Three simultaneous fix requests for different jobs run in parallel."""
        from scripts.quality_webhook.fixers import _get_job_lock

        job_names = ["lint", "test-backend", "test-frontend"]
        started = threading.Barrier(len(job_names), timeout=5)
        results = {}
        errors = []

        def worker(job: str):
            lock = _get_job_lock(job)
            acquired = lock.acquire(blocking=False)
            if not acquired:
                errors.append(f"{job}: failed to acquire per-job lock")
                return
            try:
                started.wait()
                results[job] = time.monotonic()
                time.sleep(0.02)
            finally:
                lock.release()

        threads = [threading.Thread(target=worker, args=(j,)) for j in job_names]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=10)

        assert not errors, errors
        assert set(results.keys()) == set(job_names), (
            f"Expected all 3 jobs to complete, got {list(results.keys())}"
        )
        timestamps = list(results.values())
        spread = max(timestamps) - min(timestamps)
        assert spread < 1.0, (
            f"Jobs should run in parallel (spread {spread:.3f}s), not serially"
        )

    def test_same_job_blocks_duplicate(self):
        """A second request for the same job is blocked while the first holds the lock."""
        from scripts.quality_webhook.fixers import _get_job_lock

        job = f"dup-test-{time.monotonic_ns()}"
        lock = _get_job_lock(job)

        assert lock.acquire(blocking=False), "First acquire should succeed"
        try:
            second = lock.acquire(blocking=False)
            assert not second, "Second non-blocking acquire should fail"
        finally:
            lock.release()

    def test_git_lock_serializes_access(self):
        """Git operations are serialized: only one thread holds _git_lock at a time."""
        from scripts.quality_webhook.fixers import _git_lock

        counter_lock = threading.Lock()
        current = [0]
        max_concurrent = [0]
        violations = []

        def git_operation():
            with _git_lock:
                with counter_lock:
                    current[0] += 1
                    if current[0] > max_concurrent[0]:
                        max_concurrent[0] = current[0]
                    if current[0] > 1:
                        violations.append(current[0])
                time.sleep(0.01)
                with counter_lock:
                    current[0] -= 1

        threads = [threading.Thread(target=git_operation) for _ in range(5)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=10)

        assert not violations, (
            f"_git_lock should serialize access, but saw {violations} concurrent holders"
        )
        assert max_concurrent[0] == 1, (
            f"Expected max 1 concurrent git operation, got {max_concurrent[0]}"
        )

    def test_parallel_jobs_with_serialized_git(self):
        """Multiple jobs run in parallel but serialize through _git_lock for git ops."""
        from scripts.quality_webhook.fixers import _get_job_lock, _git_lock

        job_names = ["par-a", "par-b", "par-c"]
        job_phase_times = {}
        git_phase_order = []
        git_order_lock = threading.Lock()
        barrier = threading.Barrier(len(job_names), timeout=5)

        def worker(job: str):
            lock = _get_job_lock(job)
            lock.acquire()
            try:
                barrier.wait()
                job_phase_times[job] = {"start": time.monotonic()}
                time.sleep(0.01)

                with _git_lock:
                    with git_order_lock:
                        git_phase_order.append(job)
                    time.sleep(0.01)

                job_phase_times[job]["end"] = time.monotonic()
            finally:
                lock.release()

        threads = [threading.Thread(target=worker, args=(j,)) for j in job_names]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=10)

        assert set(job_phase_times.keys()) == set(job_names)
        assert len(git_phase_order) == 3, (
            f"All 3 jobs should have completed git phase, got {len(git_phase_order)}"
        )
        assert set(git_phase_order) == set(job_names)

    def test_max_concurrent_fixes_semaphore(self):
        """_max_concurrent_fixes limits to MAX_CONCURRENT_FIXES (3) simultaneous fixes."""
        from scripts.quality_webhook.fixers import _max_concurrent_fixes
        from scripts.quality_webhook.config import MAX_CONCURRENT_FIXES

        counter_lock = threading.Lock()
        current = [0]
        max_observed = [0]
        violations = []

        def fix_task():
            _max_concurrent_fixes.acquire()
            try:
                with counter_lock:
                    current[0] += 1
                    if current[0] > max_observed[0]:
                        max_observed[0] = current[0]
                    if current[0] > MAX_CONCURRENT_FIXES:
                        violations.append(current[0])
                time.sleep(0.02)
                with counter_lock:
                    current[0] -= 1
            finally:
                _max_concurrent_fixes.release()

        threads = [threading.Thread(target=fix_task) for _ in range(6)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=10)

        assert not violations, (
            f"Semaphore should cap at {MAX_CONCURRENT_FIXES}, saw {violations}"
        )
        assert max_observed[0] <= MAX_CONCURRENT_FIXES


class TestSmokeConfiguration:
    """Smoke tests verifying key configuration values are set correctly.

    Validates Requirements 3.1, 9.2, 6.1, 1.2, 7.1.
    """

    def test_kiro_timeout_is_disabled(self):
        """KIRO_TIMEOUT should be 0 (no timeout). Requirement 3.1."""
        from scripts.quality_webhook.config import KIRO_TIMEOUT

        assert KIRO_TIMEOUT == 0, f"Expected KIRO_TIMEOUT=0, got {KIRO_TIMEOUT}"

    def test_thread_semaphore_initial_value_is_8(self):
        """_thread_semaphore should start at 8. Requirement 9.2."""
        from scripts.quality_webhook.server import _thread_semaphore
        from scripts.quality_webhook.config import THREAD_POOL_SIZE

        assert THREAD_POOL_SIZE == 8, (
            f"Expected THREAD_POOL_SIZE=8, got {THREAD_POOL_SIZE}"
        )
        assert _thread_semaphore._value == THREAD_POOL_SIZE, (
            f"Expected semaphore value={THREAD_POOL_SIZE}, got {_thread_semaphore._value}"
        )

    def test_composite_action_file_exists(self):
        """Composite action for Rust setup must exist. Requirement 6.1."""
        action_path = REPO_ROOT / ".github" / "actions" / "rust-setup" / "action.yml"
        assert action_path.is_file(), f"Composite action not found at {action_path}"

    def test_notify_failure_job_absent_from_ci(self):
        """notify-failure job must not exist in ci.yml. Requirement 1.2."""
        ci_path = REPO_ROOT / ".github" / "workflows" / "ci.yml"
        with open(ci_path, encoding="utf-8") as f:
            ci = yaml.safe_load(f)

        jobs = ci.get("jobs", {})
        assert "notify-failure" not in jobs, (
            "notify-failure job should have been removed from ci.yml"
        )

    def test_build_jobs_depend_on_lint(self):
        """Build job must include lint in needs. Requirement 7.1."""
        ci_path = REPO_ROOT / ".github" / "workflows" / "ci.yml"
        with open(ci_path, encoding="utf-8") as f:
            ci = yaml.safe_load(f)

        jobs = ci.get("jobs", {})

        for build_job in ("build",):
            assert build_job in jobs, f"{build_job} not found in ci.yml jobs"
            needs = jobs[build_job].get("needs", [])
            if isinstance(needs, str):
                needs = [needs]
            assert "lint" in needs, (
                f"{build_job} needs={needs} does not include 'lint'"
            )
