# Implementation Plan: CLI Optimization Agent

## Overview

Create all static configuration files for a non-interactive optimization agent that runs via `kiro-cli`. The artifacts are: one agent JSON config, three skill directories (each with SKILL.md and reference guides), three IDE hook files, and one memory file template. All content is declarative — JSON, Markdown, and YAML frontmatter. The shared analysis prompt is embedded directly in each hook's `then.command` field.

## Tasks

- [x] 1. Create agent configuration and memory file template
  - [x] 1.1 Create `.kiro/agents/optimization-agent.json`
    - Define `name` as `"optimization-agent"`
    - Set `resources` array: `file://.kiro/steering/**/*.md`, `file://.kiro/optimization-memory.md`, `file://lessons-learned.md`
    - Set `skills` array: `perf-optimizer`, `algorithm-advisor`, `maintainability-reviewer`
    - Set `mcpServers` to empty object `{}`
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 10.4_

  - [x] 1.2 Create `.kiro/optimization-memory.md` with empty template sections
    - Add header: `# Optimization Memory` with auto-maintained note and placeholder date
    - Add `## Previously Identified Issues` section with the table header (File, Category, Description, Fingerprint, First Seen, Occurrences, Status) and no data rows
    - Add `## Recurring Patterns` section with the table header (Pattern, Category, Occurrences, Files Affected) and no data rows
    - Add `## Project-Specific Insights` section with an empty bullet list placeholder
    - _Requirements: 10.1, 10.3, 10.6_

- [x] 2. Create perf-optimizer skill
  - [x] 2.1 Create `.kiro/skills/perf-optimizer/SKILL.md`
    - Add YAML frontmatter with `name: perf-optimizer`, `description` covering Rust performance anti-patterns (clones, allocations, string ops, blocking in async, iterators), `license: MIT`, `allowed-tools: Read, Grep, Glob`, and `metadata` block (author, version, domain, triggers, role, scope, output-format, related-skills)
    - Add core workflow section: scan for unnecessary `.clone()`, detect redundant heap allocations, find inefficient string concatenation, identify blocking ops in async contexts, check iterator patterns, evaluate `Cow<T>` opportunities
    - Add reference guide table pointing to the four reference files
    - _Requirements: 2.1, 2.6, 9.4_

  - [x] 2.2 Create `.kiro/skills/perf-optimizer/references/rust-performance.md`
    - Cover iterator adapter patterns (`.map().filter().collect()` vs manual loops), zero-copy techniques with `&str`/`&[u8]`, `Cow<T>` for conditional ownership, `into_iter()` vs `iter()` selection
    - Include examples using Actix-web handler patterns and SeaORM query results
    - _Requirements: 2.2, 9.1_

  - [x] 2.3 Create `.kiro/skills/perf-optimizer/references/memory-optimization.md`
    - Cover stack vs heap allocation guidance, `Box<T>` for large types, `Rc<T>`/`Arc<T>` for shared ownership, clone detection heuristics, `String::with_capacity` pre-allocation
    - _Requirements: 2.3_

  - [x] 2.4 Create `.kiro/skills/perf-optimizer/references/concurrency-patterns.md`
    - Cover `tokio::spawn` vs `spawn_blocking`, `tokio::sync::Mutex` vs `std::sync::Mutex` in async, channel selection (`mpsc`/`broadcast`/`watch`), `Send`/`Sync` bound troubleshooting, SeaORM connection pool sizing
    - Include examples using tokio runtime conventions consistent with the project
    - _Requirements: 2.4, 9.1_

  - [x] 2.5 Create `.kiro/skills/perf-optimizer/references/build-config.md`
    - Cover Cargo `[profile.release]` settings, LTO (`lto = true` vs `"thin"`), `codegen-units = 1`, `opt-level` selection, `strip = true` for binary size
    - _Requirements: 2.5_

- [x] 3. Create algorithm-advisor skill
  - [x] 3.1 Create `.kiro/skills/algorithm-advisor/SKILL.md`
    - Add YAML frontmatter with `name: algorithm-advisor`, `description` covering algorithm efficiency and data structure selection, `license: MIT`, `allowed-tools: Read, Grep, Glob`, and `metadata` block
    - Add core workflow: identify nested loops replaceable with hash lookups, evaluate collection type choices, check for O(n²) patterns, analyze iterator chain efficiency, flag unnecessary sorting
    - Add reference guide table pointing to the two reference files
    - _Requirements: 3.1, 3.4, 3.5, 9.4_

  - [x] 3.2 Create `.kiro/skills/algorithm-advisor/references/data-structures.md`
    - Cover `HashMap` vs `BTreeMap`, `Vec` vs `VecDeque`, `HashSet` vs `BTreeSet`, `SmallVec` for small collections
    - Include domain examples: contract date range storage, payment lookup by contrato_id, tenant search by cedula
    - _Requirements: 3.2, 9.2_

  - [x] 3.3 Create `.kiro/skills/algorithm-advisor/references/complexity-analysis.md`
    - Cover Big-O notation reference, amortized analysis for `Vec::push`, common anti-patterns (nested `.contains()`, repeated `.find()` on unsorted data), algorithmic alternatives for contract overlap detection
    - _Requirements: 3.3, 9.2_

- [x] 4. Create maintainability-reviewer skill
  - [x] 4.1 Create `.kiro/skills/maintainability-reviewer/SKILL.md`
    - Add YAML frontmatter with `name: maintainability-reviewer`, `description` covering SOLID principles, error handling, code organization, `license: MIT`, `allowed-tools: Read, Grep, Glob`, and `metadata` block
    - Add core workflow: check function length (>50 lines), check module public API surface (>10 public items), detect deep nesting (>3 levels), verify error handling follows `AppError` patterns, check `thiserror` vs `anyhow` consistency, evaluate trait design and module boundaries
    - Add reference guide table pointing to the three reference files
    - _Requirements: 4.1, 4.5, 9.4_

  - [x] 4.2 Create `.kiro/skills/maintainability-reviewer/references/solid-principles.md`
    - Cover SRP applied to Rust modules (handlers do HTTP, services do logic), OCP via trait objects and generics, LSP in trait hierarchies, ISP with fine-grained traits, DIP with trait-based dependency injection
    - Reference the project's handler → service → entity layering
    - _Requirements: 4.2, 9.3_

  - [x] 4.3 Create `.kiro/skills/maintainability-reviewer/references/error-handling.md`
    - Cover `thiserror` for library error types (like `AppError`), `anyhow` for application-level propagation, `.context()` for adding error context, mapping `sea_orm::DbErr` to `AppError`, error response format
    - Reference `backend/src/errors.rs` patterns
    - _Requirements: 4.3, 9.3_

  - [x] 4.4 Create `.kiro/skills/maintainability-reviewer/references/code-organization.md`
    - Cover module structure conventions, `pub` vs `pub(crate)` visibility, re-exports via `prelude.rs`, documentation standards, API surface minimization
    - _Requirements: 4.4_

- [x] 5. Checkpoint — Verify all skill artifacts exist
  - Verify all three SKILL.md files exist with valid frontmatter and `allowed-tools: Read, Grep, Glob`
  - Verify all 9 reference files exist in the correct directories (4 + 2 + 3)
  - Ensure all tests pass, ask the user if questions arise.

- [x] 6. Create IDE hook files with shared analysis prompt
  - [x] 6.1 Create `.kiro/hooks/optimization-agent-post-task.kiro.hook`
    - Set `enabled: true`, `name: "Optimization Agent — Post Task"`, `description`, `version: "1"`
    - Set `when.type` to `postTaskExecution`
    - Set `then.type` to `runCommand` with `command` containing the full `kiro-cli chat --no-interactive --trust-all-tools --agent optimization-agent --prompt "..."` invocation with the complete analysis prompt embedded
    - Set `then.timeout` to `300`
    - The embedded prompt must include all 7 steps: read memory, analyze Rust files, generate fingerprints, deduplicate, produce report (P0-P3 sections + Positive Patterns + Deduplicated Summary), update memory file, lessons-learned integration
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 8.1, 8.2, 8.3, 8.4, 10.1, 10.2, 10.5, 10.6, 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 12.1, 12.2, 12.3, 12.4, 12.5, 12.6_

  - [x] 6.2 Create `.kiro/hooks/optimization-agent-manual.kiro.hook`
    - Set `enabled: true`, `name: "Optimization Agent — On Demand"`, `description`, `version: "1"`
    - Set `when.type` to `userTriggered`
    - Set `then.type` to `runCommand` with the identical command and prompt as the post-task hook
    - Set `then.timeout` to `300`
    - _Requirements: 6.1, 6.2, 6.3_

  - [x] 6.3 Create `.kiro/hooks/optimization-agent-on-stop.kiro.hook`
    - Set `enabled: true`, `name: "Optimization Agent — On Agent Stop"`, `description`, `version: "1"`
    - Set `when.type` to `agentStop`
    - Set `then.type` to `runCommand` with the identical command and prompt as the post-task hook
    - Set `then.timeout` to `300`
    - _Requirements: 7.1, 7.2, 7.3_

- [x] 7. Final checkpoint — Verify all artifacts and cross-references
  - Verify `.kiro/agents/optimization-agent.json` is valid JSON and references all three skills
  - Verify all three hooks contain identical `then.command` values
  - Verify the embedded prompt references `backend/**/*.rs` and `frontend/**/*.rs`
  - Verify the embedded prompt specifies fingerprint format `{file_path}::{category}::{normalized_description}`
  - Verify the embedded prompt includes all report sections (Summary, P0, P1, P2, P3, Positive Patterns, Deduplicated Summary)
  - Verify the embedded prompt includes lessons-learned integration with all four lesson-worthy criteria
  - Verify `.kiro/optimization-memory.md` has all three required sections
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- No property-based tests apply — all artifacts are declarative configuration (JSON, Markdown), not executable code
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation of artifact existence and schema correctness
- All three hooks embed the same analysis prompt to ensure consistent behavior across trigger types
- The analysis prompt encodes all runtime logic (fingerprinting, deduplication, report structure, memory updates, lessons-learned integration) as natural language instructions for the LLM agent
