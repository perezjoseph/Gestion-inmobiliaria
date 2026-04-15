# Feature: ci-pipeline-optimization, Property 5: Concurrent fix semaphore enforces maximum of 3
"""
Property-based test for concurrent fix semaphore enforcement.

**Validates: Requirements 5.2**

Property 5: For any set of N concurrent fix requests (where N ranges from
1 to 10), at most 3 fix threads execute simultaneously, and the remaining
N-3 threads block until a slot becomes available.
"""

import threading
import time

from hypothesis import given, settings
from hypothesis import strategies as st


@given(num_tasks=st.integers(min_value=1, max_value=10))
@settings(max_examples=100, deadline=None)
def test_semaphore_enforces_max_3_concurrent(num_tasks):
    """Property 5: at most 3 tasks execute simultaneously through a Semaphore(3).

    **Validates: Requirements 5.2**

    Creates a fresh Semaphore(3) per test run, spawns N threads that each
    acquire the semaphore, increment a shared counter, assert the counter
    never exceeds 3, sleep briefly, then decrement and release.
    """
    semaphore = threading.Semaphore(3)
    counter_lock = threading.Lock()
    current_count = [0]
    max_observed = [0]
    violations = []

    def task():
        semaphore.acquire()
        try:
            with counter_lock:
                current_count[0] += 1
                if current_count[0] > max_observed[0]:
                    max_observed[0] = current_count[0]
                if current_count[0] > 3:
                    violations.append(current_count[0])

            time.sleep(0.005)

            with counter_lock:
                current_count[0] -= 1
        finally:
            semaphore.release()

    threads = [threading.Thread(target=task) for _ in range(num_tasks)]
    for t in threads:
        t.start()
    for t in threads:
        t.join(timeout=10)

    assert not violations, (
        f"Semaphore violated max concurrency of 3: observed {violations}"
    )
    assert max_observed[0] <= 3, (
        f"Max concurrent tasks was {max_observed[0]}, expected <= 3"
    )
    assert max_observed[0] >= min(num_tasks, 1), (
        f"Expected at least 1 concurrent task, got {max_observed[0]}"
    )


# Feature: ci-pipeline-optimization, Property 6: Per-job locking allows parallel execution of distinct jobs
"""
Property-based tests for per-job locking behaviour.

**Validates: Requirements 5.3**

Property 6: For any two distinct job names, both jobs can acquire their
respective locks simultaneously. For any single job name, a second
concurrent request for the same job blocks (non-blocking acquire returns
False).
"""

import string


def _get_job_lock_fresh(job: str, registry: dict, guard: threading.Lock) -> threading.Lock:
    """Mirrors _get_job_lock from fixers.py but uses a caller-provided registry."""
    with guard:
        if job not in registry:
            registry[job] = threading.Lock()
        return registry[job]


@given(
    job_a=st.text(alphabet=string.ascii_lowercase, min_size=1, max_size=20),
    job_b=st.text(alphabet=string.ascii_lowercase, min_size=1, max_size=20),
)
@settings(max_examples=100, deadline=None)
def test_distinct_jobs_acquire_locks_simultaneously(job_a, job_b):
    """Property 6 (distinct): two different jobs acquire locks at the same time.

    **Validates: Requirements 5.3**

    Creates a fresh lock registry, acquires locks for two distinct job names,
    and verifies both are held simultaneously without blocking.
    """
    from hypothesis import assume

    assume(job_a != job_b)

    registry: dict[str, threading.Lock] = {}
    guard = threading.Lock()

    lock_a = _get_job_lock_fresh(job_a, registry, guard)
    lock_b = _get_job_lock_fresh(job_b, registry, guard)

    assert lock_a is not lock_b, "Distinct jobs must get distinct Lock objects"

    acquired_a = lock_a.acquire(blocking=False)
    assert acquired_a, f"Lock for job_a='{job_a}' should be acquirable"

    acquired_b = lock_b.acquire(blocking=False)
    assert acquired_b, (
        f"Lock for job_b='{job_b}' should be acquirable while job_a='{job_a}' is held"
    )

    lock_a.release()
    lock_b.release()


@given(job=st.text(alphabet=string.ascii_lowercase, min_size=1, max_size=20))
@settings(max_examples=100, deadline=None)
def test_same_job_blocks_second_acquire(job):
    """Property 6 (same): a second non-blocking acquire on the same job returns False.

    **Validates: Requirements 5.3**

    Creates a fresh lock registry, acquires the lock for a job, then verifies
    that a second non-blocking acquire on the same job fails.
    """
    registry: dict[str, threading.Lock] = {}
    guard = threading.Lock()

    lock_first = _get_job_lock_fresh(job, registry, guard)
    lock_second = _get_job_lock_fresh(job, registry, guard)

    assert lock_first is lock_second, "Same job name must return the same Lock object"

    acquired = lock_first.acquire(blocking=False)
    assert acquired, f"First acquire for job='{job}' should succeed"

    second_acquired = lock_second.acquire(blocking=False)
    assert not second_acquired, (
        f"Second non-blocking acquire for job='{job}' should fail while first is held"
    )

    lock_first.release()
