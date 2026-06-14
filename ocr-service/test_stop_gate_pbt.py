"""Property-based tests for the autofix stop gate.

**Validates: Requirements 3.2, 3.3, 3.4, 3.5, 3.6**
"""

from hypothesis import assume, given, settings
from hypothesis import strategies as st

from test_stack_classifier_pbt import classify_stack


ALL_STACKS = ["rust", "ts", "kotlin", "python", "docker", "k8s", "shell"]

STACK_FILES = {
    "rust": "backend/src/main.rs",
    "ts": "baileys-service/src/index.ts",
    "kotlin": "android/app/src/main/MainActivity.kt",
    "python": "ocr-service/main.py",
    "docker": "infra/Dockerfile",
    "k8s": "infra/k8s/app/backend.yml",
    "shell": "scripts/deploy.sh",
}

NONE_FILES = ["README.md", "docs/notes.txt", "config.toml"]


def stop_gate(modified_files: list[str], sensors_ran: set[str], block_count: int) -> dict:
    """Pure-Python reimplementation of the stop gate logic from autofix.json.

    Returns a dict with:
      - "allowed": bool (True = exit allowed, False = blocked)
      - "missing": list[str] of stacks that were not covered
      - "cleanup": bool (True = state files should be removed)
    """
    if block_count >= 2:
        return {"allowed": True, "missing": [], "cleanup": True}

    if not modified_files:
        return {"allowed": True, "missing": [], "cleanup": True}

    has_content = any(f.strip() for f in modified_files)
    if not has_content:
        return {"allowed": True, "missing": [], "cleanup": True}

    missing = []
    for f in modified_files:
        f = f.strip()
        if not f:
            continue
        stack = classify_stack(f)
        if stack == "none":
            continue
        if stack not in sensors_ran and stack not in missing:
            missing.append(stack)

    if missing:
        return {"allowed": False, "missing": missing, "cleanup": False}

    return {"allowed": True, "missing": [], "cleanup": True}


stack_subset = st.lists(
    st.sampled_from(ALL_STACKS), min_size=1, max_size=len(ALL_STACKS), unique=True
)

sensor_subset = st.lists(
    st.sampled_from(ALL_STACKS), min_size=0, max_size=len(ALL_STACKS), unique=True
)


@given(
    stacks_modified=stack_subset,
    sensors_ran=sensor_subset,
    block_count=st.integers(min_value=0, max_value=5),
)
@settings(max_examples=300, deadline=None)
def test_sensor_coverage_completeness(stacks_modified, sensors_ran, block_count):
    """Property 1: Sensor coverage completeness — exit allowed implies
    sensor-ran[s] set for every modified stack s (or block count >= 2).

    **Validates: Requirements 3.2, 3.3, 3.4, 3.5, 3.6**
    """
    modified_files = [STACK_FILES[s] for s in stacks_modified]
    result = stop_gate(modified_files, set(sensors_ran), block_count)

    if result["allowed"]:
        escape_hatch = block_count >= 2
        all_covered = all(s in sensors_ran for s in stacks_modified)
        assert escape_hatch or all_covered, (
            f"Gate allowed exit but not all stacks covered and block_count={block_count}. "
            f"Modified stacks: {stacks_modified}, sensors ran: {sensors_ran}"
        )


@given(
    extra_stacks=st.lists(
        st.sampled_from(["rust", "ts", "kotlin", "docker", "k8s", "shell"]),
        min_size=0,
        max_size=3,
        unique=True,
    ),
    wrong_sensor=st.sampled_from(["rust", "ts", "kotlin", "docker", "k8s", "shell"]),
)
@settings(max_examples=200, deadline=None)
def test_no_silent_stack(extra_stacks, wrong_sensor):
    """Property 2: No silent stack — a modified Python file with only a
    non-Python sensor run implies the gate blocks.

    **Validates: Requirements 3.2, 3.3, 3.4, 3.5, 3.6**
    """
    assume(wrong_sensor != "python")
    sensors_ran_list = [wrong_sensor]

    modified_files = [STACK_FILES["python"]]
    for s in extra_stacks:
        modified_files.append(STACK_FILES[s])
        if s not in sensors_ran_list:
            sensors_ran_list.append(s)

    assume("python" not in sensors_ran_list)

    result = stop_gate(modified_files, set(sensors_ran_list), block_count=0)
    assert not result["allowed"], (
        f"Gate did not block despite python stack uncovered. "
        f"Sensors ran: {sensors_ran_list}"
    )
    assert "python" in result["missing"], (
        f"Missing stacks should include 'python'. Got: {result['missing']}"
    )


def test_empty_modified_files_allows_exit():
    """Exit allowed when modified files list is empty.

    **Validates: Requirements 3.5**
    """
    result = stop_gate([], set(), 0)
    assert result["allowed"]
    assert result["cleanup"]


def test_all_none_files_allows_exit():
    """Exit allowed when all modified files classify to 'none' stack.

    **Validates: Requirements 3.3**
    """
    result = stop_gate(NONE_FILES, set(), 0)
    assert result["allowed"]
    assert result["cleanup"]


def test_escape_hatch_at_two_blocks():
    """Exit allowed after 2 prior blocks even with uncovered stacks.

    **Validates: Requirements 3.4**
    """
    modified = [STACK_FILES["rust"], STACK_FILES["python"]]
    result = stop_gate(modified, set(), block_count=2)
    assert result["allowed"]
    assert result["cleanup"]


def test_cleanup_signaled_on_allowed_exit():
    """Cleanup is signaled when gate allows exit.

    **Validates: Requirements 3.6**
    """
    result = stop_gate([STACK_FILES["rust"]], {"rust"}, block_count=0)
    assert result["allowed"]
    assert result["cleanup"]


def test_no_cleanup_on_block():
    """No cleanup when gate blocks (state preserved for retry).

    **Validates: Requirements 3.2**
    """
    result = stop_gate([STACK_FILES["python"]], set(), block_count=0)
    assert not result["allowed"]
    assert not result["cleanup"]
    assert "python" in result["missing"]
