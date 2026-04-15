"""
Local reproduction engine for CI failures.

Runs the same commands that CI runs, captures structured output, and
determines whether the failure reproduces locally. This gives the fixer
agent real local error output instead of truncated webhook logs.
"""

import re
from dataclasses import dataclass, field

from .config import log
from .decisions import get_steps_to_skip, record_repro_outcome
from .runner import wsl_bash


@dataclass
class ReproResult:
    """Structured result from a local reproduction attempt."""
    reproduced: bool
    exit_code: int
    raw_output: str
    errors: list = field(default_factory=list)
    warnings: list = field(default_factory=list)
    error_class: str = ""
    summary: str = ""


@dataclass
class ReproCache:
    """Cache for reproduction results within a single fix cycle."""
    error_hash: str
    results: dict[str, ReproResult] = field(default_factory=dict)

    def get(self, step_name: str) -> ReproResult | None:
        return self.results.get(step_name)

    def store(self, step_name: str, result: ReproResult):
        self.results[step_name] = result


@dataclass
class ParsedError:
    """A single error extracted from compiler/test output."""
    file: str = ""
    line: int = 0
    column: int = 0
    code: str = ""
    message: str = ""
    severity: str = "error"

    def __str__(self):
        loc = self.file
        if self.line:
            loc += f":{self.line}"
            if self.column:
                loc += f":{self.column}"
        code_part = f" [{self.code}]" if self.code else ""
        return f"{self.severity}: {loc}{code_part} {self.message}"


# Maps CI job names to the local commands that reproduce them.
# These mirror _VERIFY_COMMANDS in gates.py but are tuned for
# capturing structured output (JSON where possible).
_REPRO_COMMANDS = {
    "lint": [
        ("fmt-check", "cargo fmt --all -- --check", 120),
        ("clippy-backend", (
            "cargo clippy --locked -p realestate-backend "
            "--message-format=json -- -D warnings"
        ), 300),
        ("clippy-frontend", (
            "cargo clippy --locked -p realestate-frontend "
            "--target wasm32-unknown-unknown "
            "--message-format=json -- -D warnings"
        ), 300),
    ],
    "test-backend": [
        ("backend-tests", "cargo test --locked -p realestate-backend --all-targets 2>&1", 300),
    ],
    "test-frontend": [
        ("frontend-tests", "cargo test --locked -p realestate-frontend --all-targets 2>&1", 300),
    ],
    "build-backend": [
        ("build-backend", "cargo build -p realestate-backend --release 2>&1", 600),
    ],
    "build-frontend": [
        ("build-frontend", "trunk build --release 2>&1", 600),
    ],
    "quality-gate": [
        ("cargo-audit", "cargo audit 2>&1", 120),
        ("cargo-deny", "cargo deny check 2>&1", 120),
    ],
    "android-lint": [
        ("android-lint", "(cd android && ./gradlew lint detekt) 2>&1", 300),
    ],
    "android-unit-test": [
        ("android-tests", "(cd android && ./gradlew testDebugUnitTest) 2>&1", 300),
    ],
    "android-build": [
        ("android-build", "(cd android && ./gradlew assembleDebug) 2>&1", 600),
    ],
}

# Regex patterns for extracting structured errors from various tools.
_RUST_ERROR_RE = re.compile(
    r"^error(?:\[(?P<code>E\d{4})\])?: (?P<message>.+)\n"
    r"\s*--> (?P<file>[^:]+):(?P<line>\d+):(?P<col>\d+)",
    re.MULTILINE,
)

_RUST_WARNING_RE = re.compile(
    r"^warning(?:\[(?P<code>[^\]]+)\])?: (?P<message>.+)\n"
    r"\s*--> (?P<file>[^:]+):(?P<line>\d+):(?P<col>\d+)",
    re.MULTILINE,
)

_CLIPPY_JSON_RE = re.compile(
    r'"reason"\s*:\s*"compiler-message"'
)

_TEST_FAIL_RE = re.compile(
    r"^test (?P<name>\S+) \.\.\. FAILED$",
    re.MULTILINE,
)

_TEST_PANIC_RE = re.compile(
    r"thread '(?P<test>[^']+)' panicked at (?:'(?P<msg>[^']*)'|(?P<msg2>.+?)),?\s*"
    r"(?P<file>[^:]+):(?P<line>\d+):(?P<col>\d+)",
    re.MULTILINE,
)

_CARGO_AUDIT_RE = re.compile(
    r"(?P<id>(?:RUSTSEC|CVE|GHSA)-[\w\-]+).*?(?:Crate|Package):\s*(?P<crate>\S+)",
    re.DOTALL,
)

_ANDROID_ERROR_RE = re.compile(
    r"^e:\s*(?P<file>[^:]+):(?P<line>\d+):(?P<col>\d+):\s*(?P<message>.+)$",
    re.MULTILINE,
)

_FMT_DIFF_RE = re.compile(
    r"^Diff in (?P<file>\S+)",
    re.MULTILINE,
)


def _parse_rust_errors(output):
    """Extract structured errors from plain-text rustc/clippy output."""
    errors = []
    for m in _RUST_ERROR_RE.finditer(output):
        errors.append(ParsedError(
            file=m.group("file"),
            line=int(m.group("line")),
            column=int(m.group("col")),
            code=m.group("code") or "",
            message=m.group("message").strip(),
            severity="error",
        ))
    return errors


def _parse_rust_warnings(output):
    """Extract structured warnings from plain-text rustc/clippy output."""
    warnings = []
    for m in _RUST_WARNING_RE.finditer(output):
        warnings.append(ParsedError(
            file=m.group("file"),
            line=int(m.group("line")),
            column=int(m.group("col")),
            code=m.group("code") or "",
            message=m.group("message").strip(),
            severity="warning",
        ))
    return warnings


def _parse_clippy_json(output):
    """Extract errors from clippy JSON (--message-format=json) output."""
    import json as _json
    errors = []
    for line in output.splitlines():
        line = line.strip()
        if not line or not line.startswith("{"):
            continue
        try:
            msg = _json.loads(line)
        except _json.JSONDecodeError:
            continue
        if msg.get("reason") != "compiler-message":
            continue
        inner = msg.get("message", {})
        level = inner.get("level", "")
        if level not in ("error", "warning"):
            continue
        spans = inner.get("spans", [])
        primary = next((s for s in spans if s.get("is_primary")), None)
        if not primary and spans:
            primary = spans[0]
        pe = ParsedError(
            message=inner.get("message", ""),
            code=inner.get("code", {}).get("code", "") if inner.get("code") else "",
            severity=level,
        )
        if primary:
            pe.file = primary.get("file_name", "")
            pe.line = primary.get("line_start", 0)
            pe.column = primary.get("column_start", 0)
        errors.append(pe)
    return errors


def _parse_test_failures(output):
    """Extract failed test names and panic locations."""
    errors = []
    failed_names = set()
    for m in _TEST_FAIL_RE.finditer(output):
        failed_names.add(m.group("name"))

    for m in _TEST_PANIC_RE.finditer(output):
        errors.append(ParsedError(
            file=m.group("file"),
            line=int(m.group("line")),
            column=int(m.group("col")),
            message=f"test '{m.group('test')}' panicked: {(m.group('msg') or m.group('msg2') or '').strip()}",
            severity="error",
        ))

    for name in failed_names:
        if not any(name in e.message for e in errors):
            errors.append(ParsedError(
                message=f"test {name} FAILED",
                severity="error",
            ))

    return errors


def _parse_fmt_diff(output):
    """Extract files with formatting issues from cargo fmt --check."""
    errors = []
    for m in _FMT_DIFF_RE.finditer(output):
        errors.append(ParsedError(
            file=m.group("file"),
            message="formatting differs from cargo fmt",
            severity="warning",
        ))
    return errors


def _parse_audit_output(output):
    """Extract advisory IDs and affected crates from cargo audit."""
    errors = []
    for m in _CARGO_AUDIT_RE.finditer(output):
        errors.append(ParsedError(
            code=m.group("id"),
            message=f"vulnerability {m.group('id')} in crate {m.group('crate')}",
            severity="error",
        ))
    return errors


def _parse_android_errors(output):
    """Extract Kotlin/Gradle compilation errors."""
    errors = []
    for m in _ANDROID_ERROR_RE.finditer(output):
        errors.append(ParsedError(
            file=m.group("file"),
            line=int(m.group("line")),
            column=int(m.group("col")),
            message=m.group("message").strip(),
            severity="error",
        ))
    return errors


def _pick_parser(step_name, output):
    """Choose the right parser based on the step and output content."""
    errors = []
    warnings = []

    if "clippy" in step_name and _CLIPPY_JSON_RE.search(output):
        parsed = _parse_clippy_json(output)
        errors = [e for e in parsed if e.severity == "error"]
        warnings = [e for e in parsed if e.severity == "warning"]
    elif "fmt" in step_name:
        errors = _parse_fmt_diff(output)
    elif "audit" in step_name:
        errors = _parse_audit_output(output)
    elif "deny" in step_name:
        errors = _parse_audit_output(output)
    elif "android" in step_name:
        errors = _parse_android_errors(output)
    elif "test" in step_name:
        errors = _parse_test_failures(output)
    else:
        errors = _parse_rust_errors(output)
        warnings = _parse_rust_warnings(output)

    if not errors and "error" in output.lower():
        errors.extend(_parse_rust_errors(output))
    if not errors and "FAILED" in output:
        errors.extend(_parse_test_failures(output))

    return errors, warnings


def _reclassify_from_local(errors, original_class):
    """Refine the error classification using parsed local errors."""
    if not errors:
        return original_class

    codes = {e.code for e in errors if e.code}
    messages = " ".join(e.message for e in errors).lower()

    if any(c.startswith("RUSTSEC") or c.startswith("CVE") or c.startswith("GHSA") for c in codes):
        return "dependency"

    if any("panicked" in e.message or "FAILED" in e.message for e in errors):
        return "test_failure"

    if any(c.startswith("E") and c[1:].isdigit() for c in codes):
        return "code_quality"

    if "formatting" in messages or "fmt" in messages:
        return "code_quality"

    return original_class


def _build_summary(step_results):
    """Build a human-readable summary of all reproduction steps."""
    lines = []
    total_errors = 0
    total_warnings = 0
    for step_name, exit_code, errors, warnings in step_results:
        status = "PASS" if exit_code == 0 else "FAIL"
        total_errors += len(errors)
        total_warnings += len(warnings)
        lines.append(f"  {step_name}: {status} (exit {exit_code}, {len(errors)} errors, {len(warnings)} warnings)")
        for e in errors[:5]:
            lines.append(f"    - {e}")
        if len(errors) > 5:
            lines.append(f"    ... and {len(errors) - 5} more errors")
    header = f"Local reproduction: {total_errors} errors, {total_warnings} warnings"
    return header + "\n" + "\n".join(lines)


def reproduce_locally(job, error_class="", cache: ReproCache | None = None):
    """Run the CI commands for a job locally and return structured results.

    Args:
        job: CI job name to reproduce.
        error_class: Optional error classification for reclassification.
        cache: Optional ReproCache to skip previously-succeeded steps
               and store new results within the same fix cycle.

    Returns a ReproResult with parsed errors, or a result with
    reproduced=False if the job has no local reproduction commands
    or if all steps pass.
    """
    steps = _REPRO_COMMANDS.get(job)
    if not steps:
        log.info(f"No local reproduction commands for job '{job}'")
        return ReproResult(
            reproduced=False,
            exit_code=0,
            raw_output="",
            summary=f"No local reproduction commands for job '{job}'",
        )

    log.info(f"Reproducing '{job}' locally ({len(steps)} steps)")

    skip_list = get_steps_to_skip(job) if job else []

    all_errors = []
    all_warnings = []
    all_output_parts = []
    step_results = []
    any_failed = False

    for step_name, cmd, timeout in steps:
        if step_name in skip_list:
            log.info(f"  step '{step_name}' skipped (100% pass rate across last 10 cycles)")
            record_repro_outcome(job, step_name, True)
            all_output_parts.append(f"--- {step_name} (skipped, 100% pass rate) ---\n")
            step_results.append((step_name, 0, [], []))
            continue

        cached_result = cache.get(step_name) if cache else None
        if cached_result is not None and cached_result.exit_code == 0:
            log.info(f"  Cache hit for '{step_name}' (exit 0), skipping execution")
            all_output_parts.append(f"--- {step_name} (cached, exit 0) ---\n")
            step_results.append((step_name, 0, [], []))
            continue

        log.info(f"  Running: {step_name}")
        try:
            result = wsl_bash(cmd, timeout=timeout)
            output = (result.stdout or "") + (result.stderr or "")
            exit_code = result.returncode
        except Exception as e:
            log.warning(f"  {step_name} execution error: {e}")
            output = str(e)
            exit_code = -1

        all_output_parts.append(f"--- {step_name} (exit {exit_code}) ---\n{output}\n")

        errors, warnings = _pick_parser(step_name, output)
        all_errors.extend(errors)
        all_warnings.extend(warnings)
        step_results.append((step_name, exit_code, errors, warnings))

        if job:
            record_repro_outcome(job, step_name, exit_code == 0)

        if cache:
            step_result = ReproResult(
                reproduced=exit_code != 0,
                exit_code=exit_code,
                raw_output=output,
                errors=errors,
                warnings=warnings,
            )
            cache.store(step_name, step_result)

        if exit_code != 0:
            any_failed = True
            log.info(f"  {step_name}: FAILED (exit {exit_code}, {len(errors)} errors)")
        else:
            log.info(f"  {step_name}: PASSED")

    raw_output = "\n".join(all_output_parts)
    refined_class = _reclassify_from_local(all_errors, error_class)
    summary = _build_summary(step_results)

    log.info(f"Reproduction complete for '{job}': reproduced={any_failed}, "
             f"{len(all_errors)} errors, class={refined_class}")

    return ReproResult(
        reproduced=any_failed,
        exit_code=1 if any_failed else 0,
        raw_output=raw_output,
        errors=all_errors,
        warnings=all_warnings,
        error_class=refined_class,
        summary=summary,
    )


def format_errors_for_prompt(repro):
    """Format a ReproResult into a concise section for the fix prompt.

    Prioritizes structured errors over raw output to keep the prompt
    focused and within token limits.
    """
    if not repro.reproduced:
        return (
            "\n\nLOCAL REPRODUCTION: Could not reproduce locally. "
            "The failure may be environment-specific (CI runner, Docker, network). "
            "Use gh CLI to fetch the full CI logs before attempting a fix.\n"
        )

    parts = ["\n\nLOCAL REPRODUCTION (ran the same commands CI runs):\n"]
    parts.append(repro.summary)

    if repro.errors:
        parts.append("\nSTRUCTURED ERRORS (parsed from local output):")
        seen = set()
        for e in repro.errors[:20]:
            key = f"{e.file}:{e.line}:{e.code}:{e.message[:80]}"
            if key in seen:
                continue
            seen.add(key)
            parts.append(f"  {e}")
        if len(repro.errors) > 20:
            parts.append(f"  ... and {len(repro.errors) - 20} more")

    files_affected = sorted({e.file for e in repro.errors if e.file})
    if files_affected:
        parts.append(f"\nFILES TO FIX: {', '.join(files_affected[:15])}")

    error_codes = sorted({e.code for e in repro.errors if e.code})
    if error_codes:
        parts.append(f"ERROR CODES: {', '.join(error_codes[:10])}")

    raw_tail = repro.raw_output[-2000:] if len(repro.raw_output) > 2000 else repro.raw_output
    parts.append(f"\nRAW OUTPUT (last 2000 chars):\n```\n{raw_tail}\n```")

    return "\n".join(parts)
