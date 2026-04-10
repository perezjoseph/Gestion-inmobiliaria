# Requirements Document

## Introduction

This feature creates a non-interactive optimization background worker powered by kiro-cli with custom agent configuration and skills. The system provides automated Rust code analysis for performance issues, algorithm efficiency, data structure selection, memory optimization, and maintainability — triggered by IDE hooks after task completion, on agent stop, or on manual trigger. The optimization agent runs unattended using `kiro-cli chat --no-interactive --trust-all-tools --agent <agent-name>` and produces actionable reports without requiring developer interaction. The agent maintains persistent memory across runs via an Optimization_Memory_File, accumulating findings, tracking issue resolution, and learning project-specific patterns to avoid duplicate reports. Significant discoveries are also appended to the project's `lessons-learned.md` file following established conventions, enabling knowledge sharing across all agents and developers.

## Glossary

- **Optimization_Agent**: A custom kiro-cli agent configuration (JSON) stored in `.kiro/agents/` that defines skills, resources, and MCP servers for non-interactive Rust code optimization analysis.
- **Performance_Skill**: A custom SKILL.md file in `.kiro/skills/perf-optimizer/` encoding Rust-specific performance patterns including iterator optimization, allocation reduction, clone elimination, and concurrency best practices.
- **Algorithm_Skill**: A custom SKILL.md file in `.kiro/skills/algorithm-advisor/` encoding algorithm analysis, data structure selection guidance, and computational complexity evaluation.
- **Maintainability_Skill**: A custom SKILL.md file in `.kiro/skills/maintainability-reviewer/` encoding SOLID principles, error handling patterns, code organization, and long-term code health practices.
- **Optimization_Hook**: An IDE hook (JSON file in `.kiro/hooks/`) that triggers the Optimization_Agent to run non-interactively as a background process.
- **Optimization_Report**: A structured markdown output produced by the Optimization_Agent containing categorized findings with severity, location, description, and suggested fix.
- **Reference_Guide**: A markdown file in a skill's `references/` subdirectory providing detailed domain knowledge the agent loads contextually.
- **kiro-cli**: The command-line interface for Kiro that supports `--no-interactive` mode with `--trust-all-tools` for unattended agent execution.
- **Optimization_Memory_File**: A persistent markdown file at `.kiro/optimization-memory.md` where the Optimization_Agent records previously identified issues, resolution status, recurring patterns, and project-specific optimization insights across runs.
- **Lessons_Learned_File**: The project-level knowledge log at `lessons-learned.md` in the project root where significant, non-obvious discoveries are appended using the format `### YYYY-MM-DD — <Topic Title>` followed by a brief description.
- **Finding_Fingerprint**: A unique identifier derived from a finding's file path, line range, category, and description used by the Optimization_Agent to detect duplicate findings across runs.

## Requirements

### Requirement 1: Optimization Agent Configuration

**User Story:** As a developer, I want a dedicated kiro-cli agent configured for code optimization, so that I can run automated performance and maintainability analysis without manual setup.

#### Acceptance Criteria

1. THE Optimization_Agent SHALL be defined as a JSON file at `.kiro/agents/optimization-agent.json` conforming to the kiro-cli agent schema (name, resources, skills, hooks fields).
2. THE Optimization_Agent SHALL reference the Performance_Skill, Algorithm_Skill, and Maintainability_Skill in its skills configuration.
3. THE Optimization_Agent SHALL include resource references to the project steering files (`file://.kiro/steering/**/*.md`) so that project conventions are available during analysis.
4. THE Optimization_Agent SHALL define a name field with the value `"optimization-agent"` to enable invocation via `kiro-cli chat --agent optimization-agent`.
5. WHEN the Optimization_Agent is invoked with `--no-interactive --trust-all-tools`, THE Optimization_Agent SHALL operate without requiring user input or tool approval prompts.

### Requirement 2: Performance Optimization Skill

**User Story:** As a developer, I want a skill that encodes Rust performance best practices, so that the optimization agent can detect inefficient patterns in my code.

#### Acceptance Criteria

1. THE Performance_Skill SHALL be defined as a SKILL.md file at `.kiro/skills/perf-optimizer/SKILL.md` with valid frontmatter (name, description, metadata fields).
2. THE Performance_Skill SHALL include a Reference_Guide at `references/rust-performance.md` covering iterator patterns, zero-copy techniques, allocation reduction, and `Cow<T>` usage.
3. THE Performance_Skill SHALL include a Reference_Guide at `references/memory-optimization.md` covering stack vs heap allocation, `Box`/`Rc`/`Arc` selection, and unnecessary clone detection.
4. THE Performance_Skill SHALL include a Reference_Guide at `references/concurrency-patterns.md` covering `tokio` async patterns, `Send`/`Sync` bounds, lock contention avoidance, and channel-based communication.
5. THE Performance_Skill SHALL include a Reference_Guide at `references/build-config.md` covering Cargo profile optimization, LTO settings, codegen-units tuning, and release build configuration.
6. THE Performance_Skill SHALL define detection rules for unnecessary `.clone()` calls, redundant allocations, inefficient string concatenation, and blocking operations in async contexts.

### Requirement 3: Algorithm and Data Structure Skill

**User Story:** As a developer, I want a skill that advises on algorithm and data structure choices, so that the optimization agent can suggest more efficient alternatives.

#### Acceptance Criteria

1. THE Algorithm_Skill SHALL be defined as a SKILL.md file at `.kiro/skills/algorithm-advisor/SKILL.md` with valid frontmatter (name, description, metadata fields).
2. THE Algorithm_Skill SHALL include a Reference_Guide at `references/data-structures.md` covering `HashMap` vs `BTreeMap`, `Vec` vs `VecDeque` vs `LinkedList`, `HashSet` vs `BTreeSet`, and domain-specific collection selection.
3. THE Algorithm_Skill SHALL include a Reference_Guide at `references/complexity-analysis.md` covering time and space complexity evaluation, amortized analysis, and common algorithmic anti-patterns.
4. THE Algorithm_Skill SHALL define guidance for identifying O(n²) patterns replaceable with O(n log n) or O(n) alternatives in collection processing.
5. THE Algorithm_Skill SHALL define guidance for evaluating iterator chain efficiency versus manual loop implementations.

### Requirement 4: Maintainability Review Skill

**User Story:** As a developer, I want a skill that checks code maintainability and adherence to SOLID principles, so that the optimization agent can flag long-term code health issues.

#### Acceptance Criteria

1. THE Maintainability_Skill SHALL be defined as a SKILL.md file at `.kiro/skills/maintainability-reviewer/SKILL.md` with valid frontmatter (name, description, metadata fields).
2. THE Maintainability_Skill SHALL include a Reference_Guide at `references/solid-principles.md` covering Single Responsibility, Open-Closed, Liskov Substitution, Interface Segregation, and Dependency Inversion applied to Rust traits and modules.
3. THE Maintainability_Skill SHALL include a Reference_Guide at `references/error-handling.md` covering `thiserror` vs `anyhow` usage, error propagation patterns, custom error type design, and context-rich error messages.
4. THE Maintainability_Skill SHALL include a Reference_Guide at `references/code-organization.md` covering module structure, visibility rules, API surface minimization, and documentation standards.
5. THE Maintainability_Skill SHALL define detection rules for functions exceeding 50 lines, modules with more than 10 public items, and deeply nested control flow (more than 3 levels).

### Requirement 5: IDE Hook for Automated Triggering

**User Story:** As a developer, I want IDE hooks that automatically trigger the optimization agent after key events, so that I receive optimization feedback without manual invocation.

#### Acceptance Criteria

1. THE Optimization_Hook SHALL be defined as a JSON file at `.kiro/hooks/optimization-agent-post-task.kiro.hook` conforming to the hook schema (name, version, description, when, then fields).
2. THE Optimization_Hook SHALL trigger on the `postTaskExecution` event type so that optimization analysis runs after each completed spec task.
3. THE Optimization_Hook SHALL execute the command `kiro-cli chat --no-interactive --trust-all-tools --agent optimization-agent` with a prompt instructing the agent to analyze recently changed Rust files.
4. WHEN the Optimization_Hook is triggered, THE Optimization_Hook SHALL pass a prompt to the agent instructing analysis of `backend/**/*.rs` and `frontend/**/*.rs` files modified in the current session.
5. THE Optimization_Hook SHALL include a `timeout` field set to 300 seconds to prevent indefinite execution of the background analysis.

### Requirement 6: Manual Trigger Hook

**User Story:** As a developer, I want to manually trigger the optimization agent on demand, so that I can request analysis at any point during development.

#### Acceptance Criteria

1. A second Optimization_Hook SHALL be defined at `.kiro/hooks/optimization-agent-manual.kiro.hook` with `when.type` set to `userTriggered`.
2. WHEN the developer manually triggers the hook, THE Optimization_Hook SHALL execute the Optimization_Agent with the same non-interactive command and analysis prompt as the post-task hook.
3. THE manual Optimization_Hook SHALL include a descriptive name field indicating it is for on-demand optimization analysis.

### Requirement 7: Agent Stop Hook

**User Story:** As a developer, I want the optimization agent to run when the main agent stops, so that a final optimization pass happens at the end of each work session.

#### Acceptance Criteria

1. A third Optimization_Hook SHALL be defined at `.kiro/hooks/optimization-agent-on-stop.kiro.hook` with `when.type` set to `agentStop`.
2. WHEN the main agent stops, THE Optimization_Hook SHALL execute the Optimization_Agent non-interactively to perform a final optimization analysis of all Rust files changed during the session.
3. THE agent-stop Optimization_Hook SHALL include a `timeout` field set to 300 seconds to prevent indefinite execution.

### Requirement 8: Optimization Report Output

**User Story:** As a developer, I want the optimization agent to produce structured, actionable reports, so that I can quickly understand and act on optimization findings.

#### Acceptance Criteria

1. THE Optimization_Agent prompt SHALL instruct the agent to produce an Optimization_Report with the following sections: Summary, Critical Issues (P0), Major Issues (P1), Minor Issues (P2), Informational (P3), and Positive Patterns Found.
2. WHEN the Optimization_Agent identifies an issue, THE Optimization_Report SHALL include for each finding: file path, line range, severity (P0-P3), category (performance, algorithm, maintainability), description, and suggested fix with code example.
3. THE Optimization_Agent prompt SHALL instruct the agent to categorize findings using these severity levels: P0 for correctness-affecting performance bugs, P1 for significant optimization opportunities, P2 for minor improvements, and P3 for informational best-practice suggestions.
4. THE Optimization_Agent prompt SHALL instruct the agent to limit the report to the top 20 findings sorted by severity to keep output actionable.

### Requirement 9: Skill Reference Guides Follow Project Conventions

**User Story:** As a developer, I want the optimization skills to respect the project's established patterns, so that suggestions align with the existing codebase conventions.

#### Acceptance Criteria

1. THE Performance_Skill reference guides SHALL include examples using `actix-web` async handlers, `SeaORM` query patterns, and `tokio` runtime conventions consistent with the project's backend stack.
2. THE Algorithm_Skill reference guides SHALL include examples relevant to property management domain operations such as contract overlap detection, payment aggregation, and tenant search filtering.
3. THE Maintainability_Skill reference guides SHALL reference the project's layered architecture (handlers → services → entities) and error handling patterns defined in `backend/src/errors.rs`.
4. EACH skill's SKILL.md frontmatter SHALL include `allowed-tools: Read, Grep, Glob` to restrict the agent to read-only file system operations during analysis.

### Requirement 10: Persistent Optimization Memory

**User Story:** As a developer, I want the optimization agent to remember findings and patterns across runs, so that it accumulates project-specific knowledge and avoids losing insights between sessions.

#### Acceptance Criteria

1. THE Optimization_Agent SHALL read the Optimization_Memory_File at `.kiro/optimization-memory.md` at the start of each run to load previously recorded findings, resolution statuses, and project-specific insights.
2. WHEN the Optimization_Agent completes an analysis run, THE Optimization_Agent SHALL append new findings, updated resolution statuses, and newly discovered project-specific patterns to the Optimization_Memory_File.
3. THE Optimization_Memory_File SHALL contain the following sections: Previously Identified Issues (with file path, description, date first seen, and resolution status), Recurring Patterns (patterns flagged more than once with occurrence count), and Project-Specific Insights (optimization knowledge specific to the codebase).
4. THE Optimization_Agent configuration SHALL include the Optimization_Memory_File as a resource reference (`file://.kiro/optimization-memory.md`) so that the file is loaded into agent context on each run.
5. THE Optimization_Agent prompt SHALL instruct the agent to read the Optimization_Memory_File before performing analysis and to update the Optimization_Memory_File after completing analysis.
6. IF the Optimization_Memory_File does not exist at the start of a run, THEN THE Optimization_Agent SHALL create the Optimization_Memory_File with empty sections and a header indicating the creation date.

### Requirement 11: Lessons Learned Integration

**User Story:** As a developer, I want the optimization agent to share significant discoveries with the team via the project's lessons-learned log, so that non-obvious solutions and performance insights are preserved for all developers and agents.

#### Acceptance Criteria

1. WHEN the Optimization_Agent discovers a non-obvious solution, a dependency gotcha, or a performance insight that affects architectural decisions, THE Optimization_Agent SHALL append an entry to the Lessons_Learned_File at the project root.
2. THE Optimization_Agent SHALL format each Lessons_Learned_File entry using the project convention: `### YYYY-MM-DD — <Topic Title>` followed by a brief description of the finding, what was observed, and the recommended action.
3. THE Optimization_Agent SHALL limit Lessons_Learned_File entries to one topic per entry and keep each entry concise and actionable.
4. WHEN the Optimization_Agent prepares a Lessons_Learned_File entry, THE Optimization_Agent SHALL search the existing Lessons_Learned_File content to verify the topic has not already been documented before appending.
5. THE Optimization_Agent SHALL include version numbers of relevant crates or tools in Lessons_Learned_File entries when applicable.
6. THE Optimization_Agent SHALL classify a finding as lesson-worthy only when the finding meets at least one of these criteria: the solution is non-obvious, the finding reveals unexpected dependency behavior, the finding represents a performance insight affecting architectural decisions, or the debugging effort to identify the finding was significant.

### Requirement 12: Finding Deduplication

**User Story:** As a developer, I want the optimization agent to avoid reporting the same issue repeatedly across runs, so that reports remain actionable and free of noise.

#### Acceptance Criteria

1. WHEN the Optimization_Agent identifies a finding during analysis, THE Optimization_Agent SHALL generate a Finding_Fingerprint based on the finding's file path, category, and description to uniquely identify the finding.
2. WHEN the Optimization_Agent generates a finding, THE Optimization_Agent SHALL compare the Finding_Fingerprint against previously identified issues recorded in the Optimization_Memory_File.
3. IF a finding's Finding_Fingerprint matches an unresolved issue in the Optimization_Memory_File, THEN THE Optimization_Agent SHALL suppress the finding from the current Optimization_Report and increment the occurrence count in the Optimization_Memory_File.
4. IF a finding's Finding_Fingerprint matches a previously resolved issue in the Optimization_Memory_File, THEN THE Optimization_Agent SHALL report the finding as a regression in the Optimization_Report with a reference to the previous resolution date.
5. THE Optimization_Agent SHALL include a Deduplicated Summary section at the end of the Optimization_Report listing the count of suppressed duplicate findings and their categories.
6. THE Optimization_Agent SHALL treat findings as matching when the file path and category are identical and the description similarity exceeds a reasonable threshold, allowing for minor line number shifts due to code changes.
