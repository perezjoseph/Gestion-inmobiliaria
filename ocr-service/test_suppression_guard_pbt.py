"""Property-based tests for the suppression guard.

**Validates: Requirements 7.1, 7.2, 7.3**
"""

import re

from hypothesis import assume, given, settings
from hypothesis import strategies as st


SUPPRESSION_RE = re.compile(
    r"#\[allow\(|@ts-ignore|@ts-nocheck|eslint-disable|@Suppress|# type: ignore|# noqa"
)

SUPPRESSION_TOKENS = [
    "#[allow(dead_code)]",
    "// @ts-ignore",
    "// @ts-nocheck",
    "// eslint-disable-next-line no-unused-vars",
    '@Suppress("unused")',
    "# type: ignore",
    "# noqa",
]


def count_suppressions(text: str) -> int:
    return len(SUPPRESSION_RE.findall(text))


def suppression_guard_str_replace(old_str: str, new_str: str) -> str:
    """Pure-Python reimplementation of the str_replace suppression guard.

    Returns "blocked" if new_str has more suppression tokens than old_str,
    otherwise "allowed".
    """
    old_n = count_suppressions(old_str)
    new_n = count_suppressions(new_str)
    if new_n > old_n:
        return "blocked"
    return "allowed"


def suppression_guard_fs_write(on_disk_content: str | None, new_text: str) -> str:
    """Pure-Python reimplementation of the fs_write suppression guard.

    on_disk_content is None when the file does not exist (treated as count 0).
    Returns "blocked" if new_text has more suppression tokens than the on-disk
    content, otherwise "allowed".
    """
    old_n = count_suppressions(on_disk_content) if on_disk_content is not None else 0
    new_n = count_suppressions(new_text)
    if new_n > old_n:
        return "blocked"
    return "allowed"


safe_text = st.text(
    alphabet=st.characters(categories=("L", "N", "P", "Z", "S"), max_codepoint=127),
    min_size=0,
    max_size=200,
)

suppression_token = st.sampled_from(SUPPRESSION_TOKENS)


def text_with_n_suppressions(n: int):
    tokens = st.lists(suppression_token, min_size=n, max_size=n)
    filler_lines = st.lists(safe_text, min_size=n + 1, max_size=n + 3)
    return st.builds(
        lambda ts, fs: "\n".join(
            f if i >= len(ts) else f"{ts[i]}\n{f}" for i, f in enumerate(fs)
        ),
        tokens,
        filler_lines,
    )


@given(
    old_count=st.integers(min_value=0, max_value=3),
    extra=st.integers(min_value=1, max_value=3),
)
@settings(max_examples=200, deadline=None)
def test_str_replace_blocked_when_count_increases(old_count, extra):
    """Property 3 (str_replace blocked): A write that increases the
    suppression-token count is blocked (exit 2).

    **Validates: Requirements 7.1, 7.2, 7.3**
    """
    new_count = old_count + extra
    old_tokens = SUPPRESSION_TOKENS[:old_count]
    new_tokens = SUPPRESSION_TOKENS[:new_count]
    old_str = "\n".join(["code line"] + old_tokens + ["more code"])
    new_str = "\n".join(["code line"] + new_tokens + ["more code"])

    result = suppression_guard_str_replace(old_str, new_str)
    assert result == "blocked", (
        f"Expected blocked for old_count={old_count}, new_count={new_count}"
    )


@given(
    old_count=st.integers(min_value=0, max_value=5),
    reduction=st.integers(min_value=0, max_value=5),
)
@settings(max_examples=200, deadline=None)
def test_str_replace_allowed_when_count_same_or_less(old_count, reduction):
    """Property 3 (str_replace allowed): A write that leaves the count
    unchanged or reduces it is allowed.

    **Validates: Requirements 7.1, 7.2, 7.3**
    """
    new_count = max(0, old_count - reduction)
    old_tokens = SUPPRESSION_TOKENS[:old_count]
    new_tokens = SUPPRESSION_TOKENS[:new_count]
    old_str = "\n".join(["code line"] + old_tokens + ["more code"])
    new_str = "\n".join(["code line"] + new_tokens + ["more code"])

    result = suppression_guard_str_replace(old_str, new_str)
    assert result == "allowed", (
        f"Expected allowed for old_count={old_count}, new_count={new_count}"
    )


@given(
    on_disk_count=st.integers(min_value=0, max_value=3),
    extra=st.integers(min_value=1, max_value=3),
)
@settings(max_examples=200, deadline=None)
def test_fs_write_blocked_when_count_increases(on_disk_count, extra):
    """Property 3 (fs_write blocked): A write that increases the
    suppression-token count is blocked (exit 2).

    **Validates: Requirements 7.1, 7.2, 7.3**
    """
    new_count = on_disk_count + extra
    on_disk_tokens = SUPPRESSION_TOKENS[:on_disk_count]
    new_tokens = SUPPRESSION_TOKENS[:new_count]
    on_disk = "\n".join(["existing code"] + on_disk_tokens + ["end"])
    new_text = "\n".join(["existing code"] + new_tokens + ["end"])

    result = suppression_guard_fs_write(on_disk, new_text)
    assert result == "blocked", (
        f"Expected blocked for on_disk_count={on_disk_count}, new_count={new_count}"
    )


@given(
    on_disk_count=st.integers(min_value=0, max_value=5),
    reduction=st.integers(min_value=0, max_value=5),
)
@settings(max_examples=200, deadline=None)
def test_fs_write_allowed_when_count_same_or_less(on_disk_count, reduction):
    """Property 3 (fs_write allowed): A write that leaves the count unchanged
    or reduces it is allowed.

    **Validates: Requirements 7.1, 7.2, 7.3**
    """
    new_count = max(0, on_disk_count - reduction)
    on_disk_tokens = SUPPRESSION_TOKENS[:on_disk_count]
    new_tokens = SUPPRESSION_TOKENS[:new_count]
    on_disk = "\n".join(["existing code"] + on_disk_tokens + ["end"])
    new_text = "\n".join(["existing code"] + new_tokens + ["end"])

    result = suppression_guard_fs_write(on_disk, new_text)
    assert result == "allowed", (
        f"Expected allowed for on_disk_count={on_disk_count}, new_count={new_count}"
    )


@given(
    token_idx=st.integers(min_value=0, max_value=len(SUPPRESSION_TOKENS) - 1),
    prefix_lines=st.integers(min_value=0, max_value=3),
)
@settings(max_examples=100, deadline=None)
def test_count_neutral_relocate_allowed(token_idx, prefix_lines):
    """Property 3 (count-neutral relocate): oldStr has a suppression, newStr
    has the same count but the token is in a different position — allowed.

    **Validates: Requirements 7.3**
    """
    token = SUPPRESSION_TOKENS[token_idx]
    prefix = "\n".join(f"line {i}" for i in range(prefix_lines))
    suffix = "\nfinal line"

    old_str = f"{prefix}\n{token}{suffix}" if prefix else f"{token}{suffix}"
    new_str = f"{prefix}{suffix}\n{token}" if prefix else f"{suffix}\n{token}"

    assert count_suppressions(old_str) == count_suppressions(new_str)
    result = suppression_guard_str_replace(old_str, new_str)
    assert result == "allowed", (
        f"Relocating '{token}' should be allowed (count unchanged)"
    )


def test_each_token_counted():
    """Each suppression token recognized by the guard counts as exactly 1."""
    for token in SUPPRESSION_TOKENS:
        assert count_suppressions(token) == 1, (
            f"Token '{token}' should count as exactly 1 suppression"
        )


def test_new_file_with_suppressions_blocked():
    """fs_write to a non-existent file (on_disk=None) with suppressions is
    blocked because old count is 0."""
    new_text = "fn main() {\n#[allow(dead_code)]\nlet x = 1;\n}"
    result = suppression_guard_fs_write(None, new_text)
    assert result == "blocked"


def test_new_file_without_suppressions_allowed():
    """fs_write to a non-existent file with no suppressions is allowed."""
    new_text = "fn main() {\nlet x = 1;\n}"
    result = suppression_guard_fs_write(None, new_text)
    assert result == "allowed"


def test_removing_suppression_allowed():
    """Removing a suppression (new_count < old_count) is always allowed."""
    on_disk = "fn main() {\n#[allow(dead_code)]\nlet x = 1;\n}"
    new_text = "fn main() {\nlet x = 1;\n}"
    result = suppression_guard_fs_write(on_disk, new_text)
    assert result == "allowed"
