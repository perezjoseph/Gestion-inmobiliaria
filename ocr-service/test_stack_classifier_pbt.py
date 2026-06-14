"""Property-based tests for the shared stack classifier.

**Validates: Requirements 4.1, 4.2**
"""

import fnmatch
import subprocess
import sys
from pathlib import Path

import pytest
from hypothesis import given, settings
from hypothesis import strategies as st


ALLOWED_STACKS = frozenset({"rust", "ts", "kotlin", "python", "docker", "k8s", "shell", "none"})

CLASSIFIER_SCRIPT = Path(__file__).resolve().parent.parent / ".kiro" / "shared" / "autofix-stack-classify.sh"


def classify_stack(file: str) -> str:
    patterns = [
        (["*.rs"], "rust"),
        (["baileys-service/*.ts", "baileys-service/*.tsx",
          "*/baileys-service/*.ts", "*/baileys-service/*.tsx"], "ts"),
        (["android/*.kt", "android/*.kts",
          "*/android/*.kt", "*/android/*.kts"], "kotlin"),
        (["ocr-service/*.py", "*/ocr-service/*.py"], "python"),
        (["*Dockerfile", "*.Dockerfile"], "docker"),
        (["infra/k8s/*.yml", "infra/k8s/*.yaml",
          "*/infra/k8s/*.yml", "*/infra/k8s/*.yaml"], "k8s"),
        (["*.sh"], "shell"),
    ]
    for globs, stack in patterns:
        for pattern in globs:
            if fnmatch.fnmatch(file, pattern):
                return stack
    return "none"


KNOWN_STACK_PATHS = {
    "rust": [
        "backend/src/main.rs",
        "frontend/src/lib.rs",
        "deep/nested/path/module.rs",
    ],
    "ts": [
        "baileys-service/src/index.ts",
        "baileys-service/components/App.tsx",
        "some/prefix/baileys-service/util.ts",
        "some/prefix/baileys-service/Component.tsx",
    ],
    "kotlin": [
        "android/app/src/main/MainActivity.kt",
        "android/build.gradle.kts",
        "some/prefix/android/Module.kt",
        "some/prefix/android/settings.gradle.kts",
    ],
    "python": [
        "ocr-service/main.py",
        "ocr-service/test_main.py",
        "some/prefix/ocr-service/engine.py",
    ],
    "docker": [
        "Dockerfile",
        "backend/Dockerfile",
        "ocr-service/Dockerfile",
        "custom.Dockerfile",
        "infra/app.Dockerfile",
    ],
    "k8s": [
        "infra/k8s/app/backend.yml",
        "infra/k8s/app/frontend.yaml",
        "some/prefix/infra/k8s/manifests/deploy.yml",
        "some/prefix/infra/k8s/service.yaml",
    ],
    "shell": [
        "scripts/deploy.sh",
        "run.sh",
        "deeply/nested/util.sh",
    ],
}

KNOWN_NONE_PATHS = [
    "README.md",
    "docs/architecture.md",
    "notes.txt",
    "config.toml",
    "data/sample.json",
    "assets/logo.png",
    ".gitignore",
    "infra/terraform/main.tf",
    "some/random/file.xml",
    "baileys-service/package.json",
    "android/build.gradle.kts.bak",
    "ocr-service/requirements.txt",
]


known_stack_path_strategy = st.one_of(
    *[
        st.sampled_from(paths).map(lambda p, s=stack: (s, p))
        for stack, paths in KNOWN_STACK_PATHS.items()
    ]
)

none_path_strategy = st.sampled_from(KNOWN_NONE_PATHS)

path_segment = st.text(
    alphabet=st.characters(whitelist_categories=("Ll",), min_codepoint=97, max_codepoint=122),
    min_size=1,
    max_size=8,
)

extension = st.sampled_from([
    ".rs", ".ts", ".tsx", ".kt", ".kts", ".py",
    ".sh", ".Dockerfile",
    ".md", ".txt", ".json", ".toml", ".xml", ".png", ".jpg",
    ".yml", ".yaml", ".html", ".css", ".js", ".lock",
])

random_path = st.builds(
    lambda segments, ext: "/".join(segments) + ext,
    segments=st.lists(path_segment, min_size=1, max_size=4),
    ext=extension,
)


@given(data=known_stack_path_strategy)
@settings(max_examples=200)
def test_known_stack_paths_never_return_none(data):
    """Property (Classifier totality — known paths): classify_stack returns the
    expected stack for paths that belong to a known sensor suite.

    **Validates: Requirements 4.1, 4.2**
    """
    expected_stack, path = data
    result = classify_stack(path)
    assert result == expected_stack, f"Expected {expected_stack} for {path}, got {result}"


@given(path=none_path_strategy)
@settings(max_examples=50)
def test_known_none_paths_return_none(path):
    """Property (Classifier totality — none paths): classify_stack returns none
    for paths that have no associated sensor suite.

    **Validates: Requirements 4.1, 4.2**
    """
    result = classify_stack(path)
    assert result == "none", f"Expected none for {path}, got {result}"


@given(path=random_path)
@settings(max_examples=500)
def test_classifier_always_returns_exactly_one_allowed_value(path):
    """Property (Classifier totality): classify_stack returns exactly one value
    from the allowed set for any arbitrary path.

    **Validates: Requirements 4.1, 4.2**
    """
    result = classify_stack(path)
    assert result in ALLOWED_STACKS, f"Got unexpected value '{result}' for path '{path}'"


@pytest.mark.skipif(sys.platform == "win32", reason="bash integration requires Unix")
def test_python_matches_bash_on_corpus():
    """Integration: confirm the Python classify_stack mirrors the bash script
    for the full corpus of known paths.

    **Validates: Requirements 4.1, 4.2**
    """
    corpus = []
    for stack, paths in KNOWN_STACK_PATHS.items():
        for p in paths:
            corpus.append((p, stack))
    for p in KNOWN_NONE_PATHS:
        corpus.append((p, "none"))

    script = str(CLASSIFIER_SCRIPT).replace("\\", "/")
    for file_path, expected in corpus:
        result = subprocess.run(
            ["bash", "-c", f'source "{script}" && classify_stack "{file_path}"'],
            capture_output=True,
            text=True,
            timeout=5,
        )
        actual = result.stdout.strip()
        assert actual == expected, (
            f"Bash/Python mismatch for '{file_path}': bash={actual}, python={expected}"
        )
