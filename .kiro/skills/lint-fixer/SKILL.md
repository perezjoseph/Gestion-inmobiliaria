---
name: lint-fixer
description: >
  Detects and fixes all Rust linting issues by directly editing source files.
  Runs cargo clippy, resolves every warning and error, runs cargo fmt, and
  validates with cargo test. Handles dead code, unused imports, clippy lints,
  collapsible ifs, redundant clones, and type mismatches. Use when fixing
  lint warnings, resolving clippy errors, cleaning up after refactoring, or
  preparing code for merge.
license: MIT
allowed-tools: Read Write Grep Glob Shell
metadata:
  author: project
  version: "1.0.0"
  domain: quality
  triggers: lint, clippy, warnings, dead code, unused imports, cargo clippy, fmt, formatting
  role: specialist
  scope: implementation
  output-format: code
  related-skills: perf-optimizer, maintainability-reviewer, code-reviewer
---

# Lint Fixer

Specialist skill for resolving all Rust linting issues. Runs cargo clippy, parses every warning and error, directly edits source files to fix them, then validates the fixes compile and pass tests.

## Core Workflow

1. **Run cargo clippy** — `cargo clippy --all-targets --all-features -- -D warnings 2>&1` to capture all warnings and errors
2. **Parse output** — Extract file path, line number, lint name, and suggested fix from each diagnostic
3. **Fix each issue** — Read the file, apply the fix directly, move to the next
4. **Run cargo fmt** — `cargo fmt --all` to ensure consistent formatting after fixes
5. **Re-run clippy** — Verify zero warnings remain. If new warnings appeared from fixes, repeat.
6. **Run cargo test** — `cargo test --workspace` to verify fixes don't break anything
7. **Update memory** — Append fixed issues to `.kiro/optimization-memory.md` and `.kiro/agent-notepads/learnings.md`

## Common Lint Fixes

### Dead Code
Remove the function if truly unused, or keep with justification if planned for future use.

### Unused Imports
Remove the import line entirely.

### Collapsible If
Merge nested if-let chains using let-chains syntax (Rust 2024 edition).

### Redundant Clone
Replace `.clone()` with a borrow when the value is only read.

### Needless Borrow
Remove unnecessary `&*` or `&` on already-borrowed values.

### Type Mismatches
Fix type conversions, missing `.into()`, or incorrect generic bounds.

## Constraints

### MUST DO
- Fix EVERY warning and error — zero tolerance for lint issues
- Read each file before editing it
- Run cargo fmt after all fixes
- Run cargo test to verify nothing breaks
- If a fix breaks tests, revert that specific fix and move on
- Update shared memory with what was fixed

### MUST NOT DO
- Suppress warnings with #[allow(...)] unless the code is intentionally kept for future use
- Modify generated entity files in backend/src/entities/
- Change code logic while fixing lints — lint fixes only
- Skip re-running clippy after fixes — new warnings can appear
