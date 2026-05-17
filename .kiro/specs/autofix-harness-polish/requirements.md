# Requirements Document

## Introduction

Polish the existing CI autofix harness by introducing a dedicated kiro-cli custom agent configuration with a file-based system prompt, pre-approved tools, resource references, and hooks. Expand the verify-fix loop from basic rustfmt+clippy into a comprehensive multi-language feedback sensor suite (Rust, TypeScript, Kotlin) that iterates until clean or hits a configurable iteration cap. Structure the agent around the generator/evaluator pattern with computational feedforward guides and feedback sensors.

## Glossary

- **Autofix_Agent**: The kiro-cli custom agent defined in `.kiro/agents/autofix.json` that runs in CI to remediate failures.
- **Feedback_Sensor**: A deterministic tool (compiler, linter, formatter, test runner) whose output the Autofix_Agent consumes to evaluate whether a fix attempt succeeded.
- **Feedforward_Guide**: A resource (steering file, AGENTS.md, project structure doc) provided to the Autofix_Agent before it begins work, reducing inferential load.
- **Verify_Loop**: The iterative cycle where the Autofix_Agent applies a fix, runs feedback sensors, and either commits (on clean) or retries (on failure) up to a maximum iteration count.
- **System_Prompt**: The file-based prompt at `.kiro/prompts/autofix-system.md` that defines the Autofix_Agent's behavior, constraints, and workflow.
- **Trigger_Workflow**: The GitHub Actions workflow at `.github/workflows/kiro-autofix-trigger.yml` that discovers failing-job artifacts and invokes the Autofix_Agent.
- **Reusable_Workflow**: The GitHub Actions workflow at `.github/workflows/kiro-autofix.yml` used for single-artifact deploy-stage failures.
- **Runner_Image**: The custom Docker image built from `infra/docker/Dockerfile.runner` containing all CI tooling.
- **PostToolUse_Hook**: A hook in the agent config that fires after write operations to auto-format affected files.
- **Rust_Fix_Skill**: The `warpdotdev/common-skills@fix-errors` skill (942 installs, from Warp's 25k-star Rust codebase) for resolving compilation errors, clippy warnings, formatting violations, and test failures.
- **Diagnose_CI_Skill**: The `warpdotdev/common-skills@diagnose-ci-failures` skill (943 installs) for reading CI logs, identifying root causes, and structuring diagnostic findings.
- **CI_Fix_Skill**: The `warpdotdev/oz-skills@ci-fix` skill (122 installs, MIT licensed) for the end-to-end CI fix workflow: locate run → extract logs → diagnose → fix → push.
- **TypeScript_Fix_Skill**: The on-demand skill at `.kiro/skills/ci-fix-typescript/SKILL.md` encoding TypeScript/baileys-service diagnostic commands and fix strategies.
- **Verify_Loop_Skill**: The on-demand skill at `.kiro/skills/verify-fix-loop/SKILL.md` encoding the iterative diagnose → fix → verify → repeat workflow pattern.

## Requirements

### Requirement 1: Custom Agent Configuration File

**User Story:** As a CI maintainer, I want the autofix agent defined as a JSON configuration file, so that its behavior is version-controlled, reviewable, and decoupled from workflow YAML.

#### Acceptance Criteria

1. THE Autofix_Agent SHALL be defined in `.kiro/agents/autofix.json` with fields: `prompt`, `tools`, `allowedTools`, `resources`, `hooks`, and `model`.
2. WHEN the `prompt` field is specified, THE Autofix_Agent SHALL reference the System_Prompt via a `file://` URI pointing to `.kiro/prompts/autofix-system.md`.
3. THE Autofix_Agent SHALL declare `resources` that include paths to `AGENTS.md`, `.kiro/steering/structure.md`, and `.kiro/steering/workflow-retries.md`.
4. WHEN the Autofix_Agent is invoked, THE Trigger_Workflow SHALL use the command `kiro-cli --agent autofix chat --no-interactive --trust-all-tools` followed by the per-artifact prompt.

### Requirement 2: File-Based System Prompt

**User Story:** As a CI maintainer, I want the autofix system prompt stored as a markdown file, so that it is readable, editable, and not buried in shell string interpolation.

#### Acceptance Criteria

1. THE System_Prompt SHALL be stored at `.kiro/prompts/autofix-system.md`.
2. THE System_Prompt SHALL define the Autofix_Agent's role, constraints (surgical changes, no new features, match existing style), the verify-fix-loop workflow, and escalation behavior (stop after max iterations).
3. THE System_Prompt SHALL instruct the Autofix_Agent to read diagnostic context from the environment variable `KIRO_DIAGNOSTICS_FILE` before making changes.
4. THE System_Prompt SHALL instruct the Autofix_Agent to write its commit message to the path in the environment variable `KIRO_COMMIT_MSG_FILE`.
5. THE System_Prompt SHALL instruct the Autofix_Agent to run the Verify_Loop internally rather than relying on the workflow to re-invoke it.

### Requirement 3: PostToolUse Hook for Auto-Formatting

**User Story:** As a CI maintainer, I want write operations to trigger auto-formatting immediately, so that the agent never commits unformatted code and does not waste a verify iteration on formatting issues.

#### Acceptance Criteria

1. THE Autofix_Agent configuration SHALL include a `postToolUse` hook that triggers after file-write tool invocations.
2. WHEN a Rust file (`.rs`) is written, THE PostToolUse_Hook SHALL execute `cargo fmt -- {file}` on the written file.
3. WHEN a TypeScript file (`.ts` or `.tsx`) is written in the `baileys-service/` directory, THE PostToolUse_Hook SHALL execute `npx eslint --fix {file}` on the written file.
4. WHEN a Kotlin file (`.kt`) is written in the `android/` directory, THE PostToolUse_Hook SHALL execute `./gradlew spotlessApply` scoped to the affected module.
5. IF the PostToolUse_Hook command fails, THEN THE Autofix_Agent SHALL log the failure and continue without blocking the fix attempt.

### Requirement 4: Rust Feedback Sensors

**User Story:** As a CI maintainer, I want the autofix agent to run Rust-specific feedback sensors after each fix attempt, so that type errors, lint violations, and test failures are caught before committing.

#### Acceptance Criteria

1. THE Verify_Loop SHALL execute `cargo fmt --all -- --check` as the first Rust feedback sensor.
2. THE Verify_Loop SHALL execute `cargo clippy --locked -p realestate-backend -- -D warnings` as the second Rust feedback sensor.
3. THE Verify_Loop SHALL execute `cargo clippy --locked --target wasm32-unknown-unknown -p realestate-frontend -- -D warnings` as the third Rust feedback sensor.
4. THE Verify_Loop SHALL execute `cargo test --locked -p realestate-backend --no-fail-fast` as the fourth Rust feedback sensor.
5. IF any Rust feedback sensor reports errors, THEN THE Autofix_Agent SHALL parse the output and attempt a corrective fix in the next iteration.
6. WHEN all Rust feedback sensors pass, THE Verify_Loop SHALL mark the Rust verification as clean.

### Requirement 5: TypeScript Feedback Sensors

**User Story:** As a CI maintainer, I want the autofix agent to run TypeScript-specific feedback sensors for the baileys-service sidecar, so that type errors and lint issues are caught before committing.

#### Acceptance Criteria

1. WHEN the diagnostic artifact references `baileys-service` or when files in `baileys-service/` were modified, THE Verify_Loop SHALL execute `cd baileys-service && npm run build` as the TypeScript type-check sensor.
2. WHEN the diagnostic artifact references `baileys-service` or when files in `baileys-service/` were modified, THE Verify_Loop SHALL execute `cd baileys-service && npx eslint . --max-warnings 0` as the TypeScript lint sensor.
3. WHEN the diagnostic artifact references `baileys-service` or when files in `baileys-service/` were modified, THE Verify_Loop SHALL execute `cd baileys-service && npm test` as the TypeScript test sensor.
4. IF any TypeScript feedback sensor reports errors, THEN THE Autofix_Agent SHALL parse the output and attempt a corrective fix in the next iteration.
5. THE Autofix_Agent SHALL also attempt fixes when sensors pass but additional heuristics (e.g., runtime warnings, deprecated API usage detected during parsing, or patterns matching known failure modes from `lessons-learned.md`) suggest latent problems exist in the modified files.

### Requirement 6: Verify-Fix Loop Iteration Control

**User Story:** As a CI maintainer, I want the verify-fix loop to have a configurable maximum iteration count, so that the agent does not loop indefinitely on unfixable issues.

#### Acceptance Criteria

1. THE Verify_Loop SHALL accept a maximum iteration count configurable via the environment variable `KIRO_MAX_FIX_ITERATIONS` with a default value of 3.
2. WHILE the iteration count is below the maximum, THE Autofix_Agent SHALL re-run all applicable feedback sensors after each fix attempt.
3. WHEN the iteration count reaches the maximum without all sensors passing, THE Autofix_Agent SHALL stop, write a commit message describing partial progress, and exit with a non-zero status.
4. WHEN all feedback sensors pass before reaching the maximum iteration count, THE Autofix_Agent SHALL write the final commit message and exit with status 0.
5. THE Autofix_Agent SHALL include the iteration count and which sensors failed in its commit message when exiting after reaching the maximum.

### Requirement 7: Workflow Integration with Custom Agent

**User Story:** As a CI maintainer, I want the trigger and reusable workflows updated to invoke the custom agent instead of passing inline prompts, so that prompt maintenance is centralized in the system prompt file.

#### Acceptance Criteria

1. THE Trigger_Workflow SHALL set the environment variable `KIRO_DIAGNOSTICS_FILE` to the path of the effective diagnostics context file before invoking the Autofix_Agent.
2. THE Trigger_Workflow SHALL invoke the Autofix_Agent using `kiro-cli --agent autofix chat --no-interactive --trust-all-tools` with a minimal positional prompt referencing the artifact name and queue position.
3. THE Reusable_Workflow SHALL invoke the Autofix_Agent using the same `kiro-cli --agent autofix` pattern, replacing the current inline prompt.
4. THE Trigger_Workflow SHALL remove the existing inline verify step (rustfmt + clippy) since verification is now handled inside the Autofix_Agent's Verify_Loop.
5. THE Trigger_Workflow SHALL interpret the Autofix_Agent's exit code: 0 means clean fix (commit and continue queue), non-zero means partial fix or failure (attempt commit if changes exist). IF the Autofix_Agent exits non-zero OR the commit of partial changes fails, THEN THE Trigger_Workflow SHALL stop processing the remaining artifact queue and exit with a non-zero status.

### Requirement 8: Feedforward Guide Resources

**User Story:** As a CI maintainer, I want the autofix agent to have access to project structure and conventions documentation before it starts fixing, so that it makes informed decisions without inferring project layout.

#### Acceptance Criteria

1. THE Autofix_Agent configuration SHALL list `.kiro/steering/structure.md` as a resource so the agent knows the workspace layout.
2. THE Autofix_Agent configuration SHALL list `AGENTS.md` as a resource so the agent follows behavioral guidelines.
3. THE Autofix_Agent configuration SHALL list `.kiro/steering/workflow-retries.md` as a resource so the agent understands retry conventions when modifying workflow files.
4. THE Autofix_Agent configuration SHALL list `.kiro/steering/github-actions-security.md` as a resource so the agent follows security best practices when modifying workflow files.
5. THE System_Prompt SHALL instruct the Autofix_Agent to read its resource files before making any changes.

### Requirement 9: Selective Sensor Activation

**User Story:** As a CI maintainer, I want the verify loop to only run sensors relevant to the files that were modified, so that verification is fast and does not waste time on unaffected stacks.

#### Acceptance Criteria

1. WHEN only Rust files (`.rs`) are modified, THE Verify_Loop SHALL run only Rust feedback sensors.
2. WHEN only TypeScript files (`.ts`, `.tsx`, `.json` in `baileys-service/`) are modified, THE Verify_Loop SHALL run only TypeScript feedback sensors.
3. WHEN only Kotlin files (`.kt`, `.kts`) are modified, THE Verify_Loop SHALL run only Kotlin feedback sensors (gradle build for the affected module).
4. WHEN files from multiple stacks are modified, THE Verify_Loop SHALL run feedback sensors for all affected stacks.
5. WHEN only non-code files are modified (YAML, Markdown, Dockerfile, Kubernetes manifests), THE Verify_Loop SHALL skip all code feedback sensors and mark verification as clean.

### Requirement 10: Commit Message Quality Enforcement

**User Story:** As a CI maintainer, I want the autofix agent to always produce structured, traceable commit messages, so that the git history remains useful for debugging.

#### Acceptance Criteria

1. THE Autofix_Agent SHALL write a commit message following the format: `fix(<scope>): <subject under 70 chars>` with body sections for Root cause, Changes, Verification, and Workflow run URL.
2. THE Autofix_Agent SHALL reference specific error identifiers (clippy lint IDs, test names, file:line, TypeScript error codes) in the Root cause section.
3. THE Autofix_Agent SHALL list each modified file or logical change as a bullet in the Changes section.
4. THE Autofix_Agent SHALL list which feedback sensors were run and their pass/fail status in the Verification section.
5. IF the Autofix_Agent exits after reaching the maximum iteration count, THEN the commit message SHALL include a `Status: PARTIAL` header and list which sensors still fail.

### Requirement 11: Install Proven CI Fix Skills

**User Story:** As a CI maintainer, I want the autofix agent to use battle-tested, community-proven skills from high-star repositories, so that it benefits from established patterns for diagnosing and fixing CI failures without reinventing the wheel.

#### Acceptance Criteria

1. THE harness SHALL install `warpdotdev/common-skills@diagnose-ci-failures` (943 installs, from Warp's 25k-star Rust codebase) as a skill available to the Autofix_Agent for reading CI logs, identifying root causes, and structuring diagnostic findings.
2. THE harness SHALL install `warpdotdev/common-skills@fix-errors` (942 installs, from Warp's 25k-star Rust codebase) as a skill available to the Autofix_Agent for resolving compilation errors, clippy warnings, formatting violations, and test failures.
3. THE harness SHALL install `warpdotdev/oz-skills@ci-fix` (122 installs, MIT licensed, from Warp's dedicated skills repo with 788 stars) as a skill available to the Autofix_Agent for the end-to-end CI fix workflow: locate failing run → extract failure logs → diagnose root cause → implement minimal fix → push.
4. THE Autofix_Agent configuration SHALL reference all three installed skills via `skill://` URIs in its `resources` array so they are loaded on demand when relevant to the current task.
5. THE Runner_Image or workflow setup step SHALL run `npx skills add warpdotdev/common-skills@diagnose-ci-failures -g -y`, `npx skills add warpdotdev/common-skills@fix-errors -g -y`, and `npx skills add warpdotdev/oz-skills@ci-fix -g -y` to ensure skills are available in the CI environment.

### Requirement 12: TypeScript CI Fix Skill

**User Story:** As a CI maintainer, I want a dedicated TypeScript CI fix skill loaded on demand, so that the agent has specialized knowledge for diagnosing and resolving baileys-service build/lint/test failures.

#### Acceptance Criteria

1. THE skill SHALL be defined at `.kiro/skills/ci-fix-typescript/SKILL.md` with frontmatter fields `name: ci-fix-typescript` and a description indicating when to activate (TypeScript compilation errors, ESLint violations, test failures in baileys-service).
2. THE skill SHALL document the exact verification commands: `cd baileys-service && npm run build`, `cd baileys-service && npx eslint . --max-warnings 0`, and `cd baileys-service && npm test`.
3. THE skill SHALL instruct the agent to run `npx eslint --fix .` for auto-fixable lint issues before attempting manual fixes.
4. THE skill SHALL instruct the agent to parse TypeScript compiler errors by file:line:column format and address them in dependency order (imports before usages, type definitions before implementations).
5. THE skill SHALL instruct the agent to never add `// @ts-ignore` or `// eslint-disable` comments unless the error is a confirmed false positive with documented justification.
6. THE Autofix_Agent configuration SHALL reference this skill via `skill://.kiro/skills/ci-fix-typescript/SKILL.md` in its `resources` array.

### Requirement 13: Verify-Fix Loop Skill

**User Story:** As a CI maintainer, I want a dedicated verify-fix loop skill that encodes the iterative verification pattern, so that the agent follows a consistent diagnose → fix → verify → repeat workflow regardless of the language stack.

#### Acceptance Criteria

1. THE skill SHALL be defined at `.kiro/skills/verify-fix-loop/SKILL.md` with frontmatter fields `name: verify-fix-loop` and a description indicating when to activate (after applying any code fix, when running verification sensors, when iterating on failed checks).
2. THE skill SHALL define the loop workflow: (1) detect modified file types, (2) select applicable sensors, (3) run all sensors capturing output, (4) parse failures into actionable findings, (5) apply fixes for highest-priority findings first, (6) re-run sensors, (7) repeat until clean or max iterations reached.
3. THE skill SHALL instruct the agent to prioritize compilation errors over lint warnings, and lint warnings over test failures, when multiple sensors fail simultaneously.
4. THE skill SHALL instruct the agent to attempt a fundamentally different approach if the same sensor fails with the same error across two consecutive iterations (avoid incremental patching).
5. THE skill SHALL define exit conditions: exit 0 when all sensors pass, exit 1 when max iterations reached with partial progress, exit 2 when no progress is made across iterations (same errors persist).
6. THE Autofix_Agent configuration SHALL reference this skill via `skill://.kiro/skills/verify-fix-loop/SKILL.md` in its `resources` array.

### Requirement 14: LSP Integration for Real-Time Diagnostics

**User Story:** As a CI maintainer, I want the autofix agent to leverage the project's configured LSPs (rust-analyzer, typescript-language-server, pyrefly) as feedback sensors, so that it gets rich type-aware diagnostics beyond what CLI tools alone provide.

#### Acceptance Criteria

1. THE Runner_Image SHALL include `rust-analyzer` pre-installed and available in PATH, configured with `check.command: clippy` and `diagnostics.enableExperimental: true` matching the project's `.kiro/settings/lsp.json` configuration.
2. THE Runner_Image SHALL include `typescript-language-server` (via npm global install) available in PATH for TypeScript/JavaScript diagnostics on `baileys-service/` (file extensions: `.ts`, `.js`, `.tsx`, `.jsx`).
3. THE Runner_Image SHALL include `pyrefly` (via pip or uv install) available in PATH for Python diagnostics with `typeCheckingMode: default` (file extension: `.py`), used for any Python scripts in the project.
4. THE Autofix_Agent SHALL use rust-analyzer diagnostics as a feedback sensor when Rust files are modified, providing file:line:column errors with suggested fixes (code actions) that the agent can apply directly.
5. THE Autofix_Agent SHALL use typescript-language-server diagnostics as a feedback sensor when TypeScript files in `baileys-service/` are modified, providing type errors with diagnostic codes (TS2304, TS2345, etc.) that guide targeted fixes.
6. THE Autofix_Agent SHALL use pyrefly diagnostics as a feedback sensor when Python files are modified, providing type-checking errors that guide targeted fixes.
7. THE System_Prompt SHALL instruct the Autofix_Agent to prefer LSP-provided code actions (auto-fixes) over manual edits when available, as they are guaranteed to be type-correct.
8. THE `.kiro/settings/lsp.json` configuration SHALL be included as a resource in the Autofix_Agent config so the agent knows which LSPs are available and how they are configured.

### Requirement 15: Comprehensive Steering File Inclusion

**User Story:** As a CI maintainer, I want the autofix agent to have access to all relevant project steering files as feedforward guides, so that it follows project conventions, understands domain context, and makes informed decisions across all stacks.

#### Acceptance Criteria

1. THE Autofix_Agent configuration SHALL include `file://.kiro/steering/**/*.md` as a glob resource pattern to load all steering files.
2. THE following steering files SHALL be available to the Autofix_Agent as feedforward guides:
   - `structure.md` — workspace layout (Rust backend/frontend, baileys-service, android, infra)
   - `backend.md` — Rust backend conventions (Actix-web, SeaORM, handler/service/entity patterns)
   - `frontend.md` — Leptos/WASM frontend conventions
   - `code-style.md` — naming, formatting, and style rules across all stacks
   - `testing.md` — test conventions (PBT naming, test file locations, assertion patterns)
   - `workflow-retries.md` — GitHub Actions retry policy (which steps to wrap, which not to)
   - `github-actions-security.md` — Actions security best practices (permissions, SHA pinning)
   - `product.md` — domain model, entity relationships, business invariants
   - `baileys-sessions.md` — WhatsApp sidecar session management conventions
   - `lessons-learned.md` — past mistakes and their resolutions (high-value gotchas)
3. THE System_Prompt SHALL instruct the Autofix_Agent to consult `lessons-learned.md` before attempting a fix, as it contains previously encountered failure modes and their correct resolutions.
4. THE System_Prompt SHALL instruct the Autofix_Agent to consult `code-style.md` and `testing.md` when modifying code to ensure new code matches project conventions.
5. THE Autofix_Agent SHALL NOT load steering files that are irrelevant to CI autofix (e.g., `context7.md`, `ocr-openvino-conversion.md`, `rig-provider.md`) to avoid context bloat — these SHALL be excluded via specific `file://` paths rather than the glob pattern, or the glob SHALL be used with the understanding that progressive loading handles relevance filtering.
