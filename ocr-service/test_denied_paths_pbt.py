"""Property-based tests for dependency immutability via deniedPaths.

**Validates: Requirements 9.1, 9.2, 9.3, 9.5, 9.6**
"""

import fnmatch

from hypothesis import given, settings
from hypothesis import strategies as st


DENIED_PATHS = [
    ".github/workflows/**",
    ".github/actions/**",
    ".kiro/agents/**",
    ".kiro/steering/**",
    ".kiro/skills/**",
    "infra/**",
    "Cargo.toml",
    "backend/Cargo.toml",
    "frontend/Cargo.toml",
    "Cargo.lock",
    "baileys-service/package-lock.json",
    "ocr-service/requirements.txt",
    "android/gradle/libs.versions.toml",
    "android/**/*.lockfile",
]

PATHS_THAT_MUST_BE_DENIED = [
    ".github/workflows/ci.yml",
    ".github/actions/rust-setup/action.yml",
    ".kiro/agents/autofix.json",
    ".kiro/steering/structure.md",
    ".kiro/skills/verify-fix-loop/SKILL.md",
    "infra/k8s/app/backend.yml",
    "Cargo.toml",
    "backend/Cargo.toml",
    "frontend/Cargo.toml",
    "Cargo.lock",
    "baileys-service/package-lock.json",
    "ocr-service/requirements.txt",
    "android/gradle/libs.versions.toml",
    "android/app/gradle.lockfile",
]

PATHS_THAT_MUST_BE_ALLOWED = [
    "backend/src/main.rs",
    "frontend/src/lib.rs",
    "baileys-service/src/index.ts",
    "ocr-service/main.py",
    "android/app/src/main/MainActivity.kt",
    "README.md",
]


def is_denied(path: str) -> bool:
    return any(fnmatch.fnmatch(path, pattern) for pattern in DENIED_PATHS)


allowed_source_paths = st.sampled_from(PATHS_THAT_MUST_BE_ALLOWED)

denied_target_paths = st.sampled_from(PATHS_THAT_MUST_BE_DENIED)

source_file_segments = st.sampled_from([
    "main.rs", "lib.rs", "mod.rs", "index.ts", "utils.ts",
    "main.py", "helpers.py", "MainActivity.kt", "App.kt",
])

source_dirs = st.sampled_from([
    "backend/src/", "backend/src/handlers/", "backend/src/services/",
    "frontend/src/", "frontend/src/pages/",
    "baileys-service/src/", "baileys-service/src/lib/",
    "ocr-service/", "ocr-service/lib/",
    "android/app/src/main/",
])

generated_allowed_paths = st.builds(lambda d, f: d + f, source_dirs, source_file_segments)


@given(path=denied_target_paths)
@settings(max_examples=100, deadline=None)
def test_denied_paths_are_matched(path):
    """Property 4: Dependency immutability — any write targeting a lockfile
    or pinned-dependency manifest is denied.

    Every known denied path must match at least one deniedPaths pattern.

    **Validates: Requirements 9.1, 9.2, 9.3, 9.5, 9.6**
    """
    assert is_denied(path), (
        f"Path '{path}' should be denied but matched no deniedPaths pattern"
    )


@given(path=allowed_source_paths)
@settings(max_examples=100, deadline=None)
def test_allowed_paths_are_not_denied(path):
    """Property 4 (inverse): Non-denied source paths are never matched by
    any denied pattern.

    **Validates: Requirements 9.1, 9.2, 9.3, 9.5, 9.6**
    """
    assert not is_denied(path), (
        f"Path '{path}' should be allowed but was matched by a deniedPaths pattern"
    )


@given(path=generated_allowed_paths)
@settings(max_examples=200, deadline=None)
def test_generated_source_paths_are_not_denied(path):
    """Property 4 (generative): Arbitrary source file paths in non-denied
    directories are never matched by any denied pattern.

    **Validates: Requirements 9.5, 9.6**
    """
    assert not is_denied(path), (
        f"Generated source path '{path}' should be allowed but was matched by a deniedPaths pattern"
    )


def test_all_pre_existing_denied_paths_covered():
    """Every pre-existing deniedPaths entry from before the expansion still
    denies at least one concrete path.

    **Validates: Requirements 9.6**
    """
    pre_existing_patterns = [
        ".github/workflows/**",
        ".github/actions/**",
        ".kiro/agents/**",
        ".kiro/steering/**",
        ".kiro/skills/**",
        "infra/**",
        "Cargo.toml",
        "backend/Cargo.toml",
        "frontend/Cargo.toml",
        "Cargo.lock",
    ]
    for pattern in pre_existing_patterns:
        matched = any(fnmatch.fnmatch(p, pattern) for p in PATHS_THAT_MUST_BE_DENIED)
        assert matched, (
            f"Pre-existing pattern '{pattern}' has no concrete denied path exercising it"
        )


def test_new_dependency_denied_paths_covered():
    """Each new dependency-related deniedPaths entry denies its target.

    **Validates: Requirements 9.1, 9.2, 9.3**
    """
    assert is_denied("baileys-service/package-lock.json")
    assert is_denied("ocr-service/requirements.txt")
    assert is_denied("android/gradle/libs.versions.toml")
    assert is_denied("android/app/gradle.lockfile")


def test_forward_looking_lockfile_pattern():
    """The android/**/*.lockfile pattern matches nested lockfiles that may
    appear in the future.

    **Validates: Requirements 9.5**
    """
    future_lockfiles = [
        "android/app/gradle.lockfile",
        "android/lib/gradle.lockfile",
        "android/feature/auth/deps.lockfile",
    ]
    for path in future_lockfiles:
        assert is_denied(path), (
            f"Future lockfile path '{path}' should be denied by android/**/*.lockfile"
        )
