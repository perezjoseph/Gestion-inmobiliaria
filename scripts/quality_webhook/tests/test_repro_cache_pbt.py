# Feature: ci-pipeline-optimization, Property 7: Reproduction cache returns cached results for previously-succeeded steps
"""
Property-based test for reproduction cache behaviour.

**Validates: Requirements 8.1, 8.2**

Property 7: For any sequence of reproduction steps within a single fix cycle,
if a step has previously succeeded (exit code 0), subsequent calls to reproduce
that step return the cached ReproResult without re-execution. The cached result
is identical to the original result.
"""

from hypothesis import given, settings
from hypothesis import strategies as st

from scripts.quality_webhook.reproducer import ReproCache, ReproResult

STEPS = ["fmt", "clippy", "test", "audit", "deny"]


@given(
    step_outcomes=st.lists(
        st.tuples(st.sampled_from(STEPS), st.booleans()),
        min_size=1,
        max_size=20,
    )
)
@settings(max_examples=100, deadline=None)
def test_cache_returns_identical_result_for_succeeded_steps(step_outcomes):
    """Property 7: after storing a successful result, get() returns the identical ReproResult.

    **Validates: Requirements 8.1, 8.2**

    For each (step, success) pair:
    - Build a ReproResult with exit_code=0 for success, exit_code=1 for failure
    - Store it in the cache
    - Verify get() returns the exact same object for successful steps
    """
    cache = ReproCache(error_hash="test-hash")

    for step_name, success in step_outcomes:
        exit_code = 0 if success else 1
        result = ReproResult(
            reproduced=not success,
            exit_code=exit_code,
            raw_output=f"output for {step_name}",
            errors=[] if success else [f"error in {step_name}"],
            warnings=[],
            summary=f"{step_name} {'passed' if success else 'failed'}",
            error_class="" if success else "code_quality",
        )

        cache.store(step_name, result)

        # Core property: immediately after storing a successful result,
        # get() returns the identical object without re-execution
        if success:
            cached = cache.get(step_name)
            assert cached is not None, (
                f"get('{step_name}') returned None right after storing a successful result"
            )
            assert cached is result, (
                f"get('{step_name}') returned a different object than what was stored"
            )
            assert cached.exit_code == 0, (
                f"Cached result for '{step_name}' has exit_code={cached.exit_code}, expected 0"
            )
            assert cached.reproduced == result.reproduced
            assert cached.raw_output == result.raw_output
            assert cached.errors == result.errors
            assert cached.warnings == result.warnings
            assert cached.summary == result.summary
            assert cached.error_class == result.error_class


@given(
    step_outcomes=st.lists(
        st.tuples(st.sampled_from(STEPS), st.booleans()),
        min_size=1,
        max_size=20,
    )
)
@settings(max_examples=100, deadline=None)
def test_cache_preserves_last_stored_result_per_step(step_outcomes):
    """Property 7 (overwrite): the cache holds the most recent result for each step.

    **Validates: Requirements 8.1, 8.2**

    When the same step is stored multiple times, get() returns the last stored
    result. This ensures the cache correctly tracks the latest outcome.
    """
    cache = ReproCache(error_hash="test-hash")

    last_result: dict[str, ReproResult] = {}

    for step_name, success in step_outcomes:
        exit_code = 0 if success else 1
        result = ReproResult(
            reproduced=not success,
            exit_code=exit_code,
            raw_output=f"output for {step_name} success={success}",
            errors=[] if success else [f"error in {step_name}"],
            warnings=[],
            summary=f"{step_name} {'passed' if success else 'failed'}",
            error_class="" if success else "code_quality",
        )

        cache.store(step_name, result)
        last_result[step_name] = result

    for step_name, expected in last_result.items():
        cached = cache.get(step_name)
        assert cached is expected, (
            f"get('{step_name}') did not return the last stored result"
        )


@given(
    step_outcomes=st.lists(
        st.tuples(st.sampled_from(STEPS), st.booleans()),
        min_size=0,
        max_size=20,
    )
)
@settings(max_examples=100, deadline=None)
def test_cache_returns_none_for_unstored_steps(step_outcomes):
    """Property 7 (miss): get() returns None for steps never stored.

    **Validates: Requirements 8.1, 8.2**

    After storing results for a subset of steps, get() on any step not in
    that subset returns None.
    """
    cache = ReproCache(error_hash="test-hash")

    stored_steps = set()
    for step_name, success in step_outcomes:
        exit_code = 0 if success else 1
        result = ReproResult(
            reproduced=not success,
            exit_code=exit_code,
            raw_output=f"output for {step_name}",
            errors=[],
            warnings=[],
            summary="",
            error_class="",
        )
        cache.store(step_name, result)
        stored_steps.add(step_name)

    for step_name in STEPS:
        if step_name not in stored_steps:
            assert cache.get(step_name) is None, (
                f"get('{step_name}') should return None for an unstored step"
            )
