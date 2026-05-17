# Implementation Plan: Autofix Harness Polish

## Overview

Transform the CI autofix harness into a structured, multi-language agent harness with a dedicated kiro-cli custom agent configuration, file-based system prompt, on-demand skills, selective feedback sensors, and an iterative verify-fix loop. Pure logic functions are implemented in Rust with property-based tests using `proptest`.

## Tasks

- [x] 1. Create agent configuration and system prompt
  - [x] 1.1 Create the agent configuration file at `.kiro/agents/autofix.json`
    - Define the JSON structure with fields: `name`, `description`, `prompt`, `model`, `tools`, `allowedTools`, `resources`, `hooks`
    - Set `prompt` to `file://../prompts/autofix-system.md`
    - Set `tools` to `["*"]` and `allowedTools` to `["read", "write", "shell", "@builtin"]`
    - Add all resource URIs: `AGENTS.md`, steering glob, `lsp.json`, skill URIs, community skill URIs
    - Add the `postToolUse` hook with `matcher: "fs_write"` and the bash dispatch command handling `.rs`, `.ts`/`.tsx` (baileys-service), and `.kt` (android) files with `|| true` for failure swallowing
    - _Requirements: 1.1, 1.2, 1.3, 3.1, 3.2, 3.3, 3.4, 3.5, 8.1, 8.2, 8.3, 8.4, 8.5, 11.4, 12.6, 13.6, 14.8, 15.1_

  - [x] 1.2 Create the system prompt file at `.kiro/prompts/autofix-system.md`
    - Define the agent's role as automated CI remediation making surgical, minimal fixes
    - Document constraints: no new features, match existing style, no suppressed warnings
    - Document environment variables: `KIRO_DIAGNOSTICS_FILE`, `KIRO_COMMIT_MSG_FILE`, `KIRO_MAX_FIX_ITERATIONS`
    - Define the verify-fix loop workflow: read guides → read diagnostics → fix → verify → iterate or commit
    - Document all per-language sensor commands (Rust, TypeScript, Kotlin)
    - Define commit message format with scope, root cause, changes, verification, status
    - Define escalation behavior: stop after max iterations, write partial commit, exit non-zero
    - Instruct to prefer LSP code actions over manual edits when available
    - Instruct to consult `lessons-learned.md` before attempting fixes
    - Instruct to consult `code-style.md` and `testing.md` when modifying code
    - Instruct to read resource files before making any changes
    - Instruct to run the verify loop internally rather than relying on workflow re-invocation
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 8.5, 14.7, 15.3, 15.4_

- [x] 2. Create on-demand skills
  - [x] 2.1 Create the TypeScript CI fix skill at `.kiro/skills/ci-fix-typescript/SKILL.md`
    - Add YAML frontmatter with `name: ci-fix-typescript` and description for activation triggers (TS compilation errors, ESLint violations, test failures in baileys-service)
    - Document verification commands: `cd baileys-service && npm run build`, `cd baileys-service && npx eslint . --max-warnings 0`, `cd baileys-service && npm test`
    - Instruct to run `npx eslint --fix .` for auto-fixable lint issues before manual fixes
    - Instruct to parse TS compiler errors by file:line:column and address in dependency order
    - Instruct to never add `// @ts-ignore` or `// eslint-disable` unless confirmed false positive with justification
    - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5_

  - [x] 2.2 Create the verify-fix loop skill at `.kiro/skills/verify-fix-loop/SKILL.md`
    - Add YAML frontmatter with `name: verify-fix-loop` and description for activation triggers (after applying fixes, running sensors, iterating on failed checks)
    - Define the loop workflow: detect modified types → select sensors → run sensors → parse failures → apply fixes → re-run → repeat
    - Define priority ordering: compilation errors > lint warnings > test failures
    - Instruct to attempt fundamentally different approach if same error persists across two consecutive iterations
    - Define exit conditions: exit 0 (all pass), exit 1 (max reached, partial progress), exit 2 (no progress)
    - _Requirements: 13.1, 13.2, 13.3, 13.4, 13.5_

- [ ] 3. Implement pure logic functions and property-based tests
  - [x] 3.1 Create the harness logic module with pure functions
    - Create a new Rust source file (e.g., `backend/src/harness/mod.rs` or a dedicated crate) containing the pure logic functions
    - Implement `dispatch_formatter(file_path) -> Option<String>`: returns formatter command based on file extension and path
    - Implement `select_sensors(modified_files) -> HashSet<SensorSuite>`: returns applicable sensor suites based on file paths
    - Implement `parse_max_iterations(env_value: Option<&str>) -> u32`: parses env var with default 3
    - Implement `should_continue_loop(iteration, max, sensor_results) -> LoopDecision`: determines continue/stop + exit code
    - Implement `determine_exit_code(all_pass, max_reached, progress_made) -> u8`: returns 0, 1, or 2
    - Implement `format_commit_message(scope, subject, root_cause, changes, sensors, status) -> String`: produces structured commit message
    - Implement `prioritize_failures(failures) -> Vec<Failure>`: stable-sorts by type priority (compile > lint > test)
    - _Requirements: 3.2, 3.3, 3.4, 6.1, 6.2, 6.3, 6.4, 9.1, 9.2, 9.3, 9.4, 9.5, 10.1, 10.3, 10.4, 10.5, 13.3, 13.5_

  - [~] 3.2 Write property test for dispatch_formatter
    - **Property 1: Hook dispatch produces correct formatter command**
    - Use proptest to generate random file paths with various extensions (.rs, .ts, .tsx, .kt, .py, .md, .yml) and directory prefixes (baileys-service/, android/, src/, etc.)
    - Assert correct command returned for each file type/path combination
    - **Validates: Requirements 3.2, 3.3, 3.4**

  - [~] 3.3 Write property test for select_sensors
    - **Property 2: Selective sensor activation by modified file set**
    - Use proptest to generate random sets of 1-20 file paths from all stacks plus non-code files
    - Assert exactly the correct sensor suites are returned for each file set composition
    - **Validates: Requirements 5.1, 5.2, 5.3, 9.1, 9.2, 9.3, 9.4, 9.5**

  - [~] 3.4 Write property test for parse_max_iterations
    - **Property 3: Iteration configuration parsing**
    - Use proptest to generate random strings: valid positive integers, empty strings, non-numeric strings, negative numbers, zero
    - Assert returns parsed integer for valid positive values, 3 for all invalid/missing/zero/negative
    - **Validates: Requirements 6.1**

  - [~] 3.5 Write property test for should_continue_loop
    - **Property 4: Loop termination correctness**
    - Use proptest to generate random (iteration, max, Vec<SensorResult>) tuples
    - Assert: continues when iteration < max and sensors fail; exit 0 when all pass; exit 1 when max reached with progress; exit 2 when max reached without progress
    - **Validates: Requirements 4.6, 6.2, 6.3, 6.4**

  - [~] 3.6 Write property test for determine_exit_code
    - **Property 5: Exit code determination**
    - Exhaustively test all 8 combinations of (all_pass, max_reached, progress_made) booleans
    - Assert: 0 when all_pass=true; 1 when !all_pass && max_reached && progress_made; 2 when !all_pass && max_reached && !progress_made
    - **Validates: Requirements 7.5, 13.5**

  - [~] 3.7 Write property test for format_commit_message (format validity)
    - **Property 6: Commit message format validity**
    - Use proptest to generate random scope (alphanumeric 1-20 chars) and subject (1-70 chars)
    - Assert first line matches `fix(<scope>): <subject>` and does not exceed 70 characters
    - **Validates: Requirements 10.1**

  - [~] 3.8 Write property test for format_commit_message (completeness)
    - **Property 7: Commit message completeness**
    - Use proptest to generate random file sets (1-30 paths) and sensor result sets (1-10 sensors with pass/fail)
    - Assert: each file appears in Changes section, each sensor appears in Verification section, PARTIAL status includes failing sensors, iteration count present
    - **Validates: Requirements 6.5, 10.3, 10.4, 10.5**

  - [~] 3.9 Write property test for prioritize_failures
    - **Property 8: Failure priority ordering**
    - Use proptest to generate random lists of 1-50 failures with types {compilation, lint, test}
    - Assert: all compilation errors precede lint warnings, all lint warnings precede test failures, stable sort within same type
    - **Validates: Requirements 13.3**

- [~] 4. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 5. Update CI workflows
  - [~] 5.1 Update the trigger workflow at `.github/workflows/kiro-autofix-trigger.yml`
    - Set `KIRO_DIAGNOSTICS_FILE` environment variable to the path of the effective diagnostics context file
    - Set `KIRO_COMMIT_MSG_FILE` environment variable for the agent to write commit messages
    - Set `KIRO_MAX_FIX_ITERATIONS` environment variable (default 3)
    - Replace inline prompt invocation with `kiro-cli --agent autofix chat --no-interactive --trust-all-tools` followed by minimal positional prompt
    - Remove the existing inline verify step (rustfmt + clippy) since verification is now internal to the agent
    - Interpret agent exit codes: 0 → commit and continue queue; non-zero → attempt commit if changes exist, then stop queue
    - Add community skills installation step: `npx skills add warpdotdev/common-skills@diagnose-ci-failures -g -y`, `npx skills add warpdotdev/common-skills@fix-errors -g -y`, `npx skills add warpdotdev/oz-skills@ci-fix -g -y`
    - _Requirements: 1.4, 7.1, 7.2, 7.4, 7.5, 11.5_

  - [~] 5.2 Update the reusable workflow at `.github/workflows/kiro-autofix.yml`
    - Replace the current inline prompt with `kiro-cli --agent autofix chat --no-interactive --trust-all-tools` invocation pattern
    - Set the same environment variables (`KIRO_DIAGNOSTICS_FILE`, `KIRO_COMMIT_MSG_FILE`, `KIRO_MAX_FIX_ITERATIONS`)
    - Interpret agent exit codes consistently with the trigger workflow
    - _Requirements: 7.3_

- [ ] 6. Update runner image
  - [~] 6.1 Update `infra/docker/Dockerfile.runner` with LSP tooling
    - Install `rust-analyzer` (download pre-built binary from GitHub releases, place in PATH)
    - Install `typescript-language-server` and `typescript` globally via npm
    - Install `pyrefly` via pip or uv
    - Ensure Node.js 20 LTS is available (for TS tools and skills CLI)
    - _Requirements: 14.1, 14.2, 14.3_

- [~] 7. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties using `proptest` (Rust)
- Unit tests validate specific examples and edge cases
- The pure logic functions (task 3.1) are the testable core; the agent config, prompts, and skills are declarative artifacts
- Community skills are installed at CI runtime via `npx skills add` rather than vendored into the repo

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1", "1.2", "2.1", "2.2"] },
    { "id": 1, "tasks": ["3.1"] },
    { "id": 2, "tasks": ["3.2", "3.3", "3.4", "3.5", "3.6", "3.7", "3.8", "3.9"] },
    { "id": 3, "tasks": ["5.1", "5.2", "6.1"] }
  ]
}
```
