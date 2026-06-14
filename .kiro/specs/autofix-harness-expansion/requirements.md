# Requirements Document

## Introduction

This feature expands the autofix CI remediation harness (`.kiro/agents/autofix.json` + `.kiro/shared/autofix-system.md`) by strengthening its three weakest subsystems and integrating with the existing Lifecycle (observability) track:

- **Verification** — add a Python (`ocr-service`) sensor suite, non-code validators (Dockerfile, Kubernetes, shell), and per-stack sensor-ran tracking so the termination gate verifies the correct sensor ran for the files actually modified.
- **State** — keep a within-run scratch notepad, add a structured compaction handoff, and add a persistent native knowledge base stored on an NFS-mounted volume that survives ephemeral runner pods.
- **Scope** — promote three currently-advisory prompt rules into mechanical controls: a suppression guard, a diff-budget guard (advisory by design), and expanded write denials for lockfiles and Python dependency manifests.
- **Lifecycle** — consume the trace/metrics pipeline already specified in `.kiro/plans/harness-observability.md` rather than duplicating it.

These requirements are derived from the approved design document and trace to proposed features F1–F10 and gaps GAP-1 through GAP-8. They preserve every existing harness protection (CODEOWNERS gating, retry steering, dependency steering, the git-block guard, and the max-2-block escape hatch).

## Glossary

- **Autofix_Harness**: The CI remediation agent defined by `.kiro/agents/autofix.json` and `.kiro/shared/autofix-system.md` that diagnoses build/deploy failures and applies minimal, verified fixes inside a verify-fix loop.
- **Subsystem**: One of the five harness subsystems — Instructions, State, Verification, Scope, Lifecycle.
- **Sensor**: A deterministic verification command (fmt, clippy, test, lint, validator) whose exit code is the only valid evidence of correctness.
- **Stack**: One language/service unit with its own sensor suite — Rust, TypeScript (baileys-service), Kotlin (android), or Python (ocr-service). Non-code file types (Dockerfile, Kubernetes manifests, shell) also classify to a stack for validation purposes.
- **Stack_Classifier**: The shared function that maps a modified file path to exactly one stack value (`rust`, `ts`, `kotlin`, `python`, `docker`, `k8s`, `shell`, or `none`), used by both the sensor-ran tracker and the stop gate.
- **Guard**: A `preToolUse` or `postToolUse` hook that mechanically blocks or flags an agent action.
- **Stop_Gate**: The `stop` hook that performs the three-layer termination check, including the per-stack sensor-ran verification and the max-2-block escape hatch.
- **Sensor_Ran_State**: The per-stack markers under `$RUNNER_TEMP/autofix-sensors-ran.d/` recording which stacks' sensors ran.
- **Modified_Files_State**: The list at `$RUNNER_TEMP/autofix-modified-files.txt` recording one modified file path per line.
- **Suppression_Guard**: The `preToolUse` guard that blocks any write increasing the count of suppression tokens (`#[allow(`, `@ts-ignore`, `@ts-nocheck`, `eslint-disable`, `@Suppress`, `# type: ignore`, `# noqa`) for the target file.
- **Diff_Budget_Guard**: The advisory `postToolUse` guard that emits CI annotations when the count of distinct modified files crosses soft/hard thresholds.
- **Within_Run_Scratch**: The ephemeral notepad `$RUNNER_TEMP/autofix-learnings.txt` that lives and dies with a single queue run and is never committed to git.
- **Compaction_Handoff**: The structured handoff block the agent writes when its context approaches the limit mid-fix.
- **Persistent_KB**: kiro-cli's native knowledge base, reached through the built-in `knowledge` tool, stored on an NFS-mounted volume at the kiro-cli data dir so it survives ephemeral runner pods.
- **Knowledge_Tool**: The built-in `knowledge` tool that provides search and store operations against the Persistent_KB for the headless agent.
- **AgentSpawn_Hook**: The `agentSpawn` hook that clears stale verification state and performs the graceful-degradation check for the Persistent_KB.
- **Diagnostic_Signature** (`diag_sig`): A normalized signature of a diagnostic — clippy lint name, Rust error code (e.g. `E0277`), failing test name, ruff rule code, hadolint rule, or a hash of the first error line when no code is present.
- **Memory_Key**: The `(artifact, diag_sig)` tag that keys each Persistent_KB entry, where `artifact` is the diagnostics artifact name.
- **Queue_Run**: One execution of the autofix queue in `kiro-autofix-trigger.yml`; may process many diagnostic artifacts and produce many commits.
- **Validation_Spike**: The prerequisite gate that confirms the Knowledge_Tool functions headless and determines the real KB store path before any infrastructure work begins.
- **GC_Job**: The out-of-band garbage-collection mechanism (e.g. a Kubernetes CronJob) that bounds the growth of the Persistent_KB.

## Requirements

### Requirement 1: Python sensor suite for ocr-service (F1, GAP-1)

**User Story:** As a harness maintainer, I want the Autofix_Harness to run Python sensors after editing `ocr-service` files, so that Python fixes are verified instead of escaping unchecked.

#### Acceptance Criteria

1. WHERE a fix modifies a file under `ocr-service/` matching `*.py`, THE Autofix_Harness SHALL define a Python sensor suite consisting of `ruff format --check`, `ruff check`, and `python -m pytest`, each run from the `ocr-service/` directory.
2. THE Autofix_Harness SHALL order the Python sensor suite as `ruff format --check`, then `ruff check`, then `python -m pytest`, following the Keep-Quality-Left priority.
3. THE Autofix_Harness SHALL document the Python sensor suite in both `autofix-system.md` and the `verify-fix-loop` skill sensor-selection table.
4. THE Autofix_Harness SHALL use the existing repository tooling (`ruff` configured at `.trunk/configs/ruff.toml`, and the `pytest`/`hypothesis` tests already in `ocr-service/`) without introducing a new runtime dependency.

### Requirement 2: Non-code validators (F2, GAP-2)

**User Story:** As a harness maintainer, I want validators for Dockerfiles, Kubernetes manifests, and shell scripts, so that non-code fixes are validated instead of being treated as "clean, no sensors".

#### Acceptance Criteria

1. WHERE a fix inspects a Dockerfile (matching `**/Dockerfile` or `*.Dockerfile`), THE Autofix_Harness SHALL provide `hadolint` as the validator for the `docker` stack.
2. WHERE a fix inspects a Kubernetes manifest under `infra/k8s/`, THE Autofix_Harness SHALL provide `kubeconform -strict -ignore-missing-schemas` as the validator for the `k8s` stack.
3. WHERE a fix inspects a shell script matching `*.sh`, THE Autofix_Harness SHALL provide `shellcheck` as the validator for the `shell` stack.
4. THE Autofix_Harness SHALL treat the non-code validators as validation-only Sensors that do not authorize writes to `infra/**`, which remains write-denied.
5. IF a required validator tool is absent from PATH after the retry-wrapped install step, THEN THE Autofix_Harness SHALL treat the affected stack as un-verified so the Stop_Gate blocks exit.

### Requirement 3: Per-stack sensor-ran tracking in the stop gate (F3, GAP-1, GAP-2)

**User Story:** As a harness maintainer, I want sensor-ran state tracked per stack, so that the Stop_Gate verifies the correct sensor ran for each modified stack rather than accepting one global flag.

#### Acceptance Criteria

1. WHEN a Sensor command runs, THE Autofix_Harness SHALL record a per-stack marker in `Sensor_Ran_State` corresponding to the stack of that Sensor command.
2. WHEN the agent attempts to stop AND `Modified_Files_State` contains at least one file whose stack (per the Stack_Classifier) is not `none` and whose corresponding marker is absent from `Sensor_Ran_State`, THE Stop_Gate SHALL block exit and return a reason naming each stack whose Sensor did not run.
3. WHEN the agent attempts to stop AND every modified file either classifies to `none` or has a matching marker in `Sensor_Ran_State`, THE Stop_Gate SHALL allow exit.
4. WHEN the Stop_Gate has already blocked exit twice in the current session, THE Stop_Gate SHALL allow exit, preserving the max-2-block escape hatch.
5. WHEN `Modified_Files_State` is empty, THE Stop_Gate SHALL allow exit.
6. WHEN the Stop_Gate allows exit, THE Stop_Gate SHALL remove `Modified_Files_State`, `Sensor_Ran_State`, and the block-count file.

### Requirement 4: Stack classifier (F3 supporting component)

**User Story:** As a harness maintainer, I want a single shared file-to-stack classifier, so that the sensor-ran tracker and the Stop_Gate agree on which Sensor applies to which file.

#### Acceptance Criteria

1. THE Stack_Classifier SHALL map any given file path to exactly one stack value from the set `rust`, `ts`, `kotlin`, `python`, `docker`, `k8s`, `shell`, `none`.
2. THE Stack_Classifier SHALL return `none` for files that legitimately require no Sensor, so the Stop_Gate does not block on them.
3. THE Autofix_Harness SHALL define the Stack_Classifier once and reference it from both the `postToolUse` sensor-ran tracker and the Stop_Gate.

### Requirement 5: Within-run scratch learnings (F4, GAP-3 within-run)

**User Story:** As a harness maintainer, I want a within-run notepad of attempted approaches, so that later artifacts in the same Queue_Run benefit from earlier attempts without repeating them.

#### Acceptance Criteria

1. WHEN a Queue_Run starts, THE Autofix_Harness SHALL reset Within_Run_Scratch to empty.
2. WHILE a Queue_Run is processing artifacts, THE Autofix_Harness SHALL allow short attempt notes to be appended to Within_Run_Scratch.
3. WHEN the agent processes an artifact after earlier artifacts in the same Queue_Run, THE Autofix_Harness SHALL make the notes appended during the earlier artifacts available to the agent.
4. THE Autofix_Harness SHALL keep Within_Run_Scratch out of git and SHALL NOT seed it from any git-tracked store.
5. THE Autofix_Harness SHALL hold no durable cross-run state in Within_Run_Scratch, delegating cross-run continuity to the Persistent_KB.

### Requirement 6: Structured compaction handoff (F5)

**User Story:** As a harness maintainer, I want the agent to write a structured handoff when its context approaches the limit, so that the next turn resumes without re-diagnosing.

#### Acceptance Criteria

1. WHERE the agent's context approaches the limit mid-fix, THE Autofix_Harness SHALL instruct the agent to write a Compaction_Handoff block before continuing.
2. THE Compaction_Handoff block SHALL capture the goal, constraints, progress (done/in-progress/blocked), key decisions, next steps, and cumulative modified files.
3. THE Autofix_Harness SHALL populate the cumulative-modified-files field of the Compaction_Handoff from the existing Modified_Files_State side-channel.

### Requirement 7: Suppression guard (F6, GAP-4)

**User Story:** As a security-conscious maintainer, I want net-new warning suppressions blocked mechanically, so that the agent fixes root causes instead of silencing security or quality lints.

#### Acceptance Criteria

1. WHEN the agent issues a `str_replace` write, IF the count of suppression tokens in the new string exceeds the count in the old string, THEN THE Suppression_Guard SHALL block the write and exit with code 2 and a reason directing the agent to fix the root cause.
2. WHEN the agent issues an `fs_write`, IF the count of suppression tokens in the new content exceeds the count in the current on-disk file (treating a non-existent file as count zero), THEN THE Suppression_Guard SHALL block the write and exit with code 2 and a reason directing the agent to fix the root cause.
3. WHEN a write leaves the suppression-token count for the target file unchanged, including a write that relocates a pre-existing suppression, THE Suppression_Guard SHALL allow the write.
4. THE Suppression_Guard SHALL recognize the suppression tokens `#[allow(`, `@ts-ignore`, `@ts-nocheck`, `eslint-disable`, `@Suppress`, `# type: ignore`, and `# noqa`.
5. THE Suppression_Guard SHALL enforce its decision as a hard `preToolUse` denial to which the max-2-block Stop_Gate escape hatch does not apply.

### Requirement 8: Diff-budget guard (F7, GAP-5)

**User Story:** As a harness maintainer, I want an advisory budget on the number of files a fix touches, so that the agent is nudged to keep fixes surgical.

#### Acceptance Criteria

1. WHEN the count of distinct files in Modified_Files_State reaches the soft threshold (default 8) but is below the hard threshold, THE Diff_Budget_Guard SHALL emit a `::notice::` CI annotation asking the agent to confirm each change traces to the diagnosed failure.
2. WHEN the count of distinct files in Modified_Files_State reaches the hard threshold (default 15), THE Diff_Budget_Guard SHALL emit a `::warning::` CI annotation instructing the agent to stop expanding scope and, if the fix genuinely needs that many files, to exit with `Status: PARTIAL`.
3. THE Diff_Budget_Guard SHALL be advisory only and SHALL NOT block any write or block Stop_Gate exit based on the file-count budget.
4. THE Autofix_Harness SHALL read the soft and hard thresholds from `KIRO_DIFF_SOFT` and `KIRO_DIFF_HARD`, defaulting to 8 and 15 respectively.

### Requirement 9: Expanded write denials for lockfiles and dependency manifests (F8, GAP-6)

**User Story:** As a maintainer enforcing the dependencies steering rule, I want lockfiles and Python dependency manifests write-protected, so that the agent cannot silently change dependency versions.

#### Acceptance Criteria

1. IF the agent attempts to write `baileys-service/package-lock.json`, THEN THE Autofix_Harness SHALL deny the write via `deniedPaths`.
2. IF the agent attempts to write `ocr-service/requirements.txt`, THEN THE Autofix_Harness SHALL deny the write via `deniedPaths`.
3. IF the agent attempts to write `android/gradle/libs.versions.toml`, THEN THE Autofix_Harness SHALL deny the write via `deniedPaths`.
4. THE Autofix_Harness SHALL retain the forward-looking `deniedPaths` entry `android/**/*.lockfile`, even though it matches no file today, so future Gradle lockfiles are denied from the start.
5. WHEN a fix genuinely requires a dependency change, THE Autofix_Harness SHALL exit with `Status: PARTIAL` and report the needed change for human action rather than editing a denied path.
6. THE Autofix_Harness SHALL retain all pre-existing `deniedPaths` entries unchanged when adding the new entries.

### Requirement 10: Persistent memory subsystem on NFS (F10, GAP-3 cross-run, GAP-8)

**User Story:** As a harness maintainer, I want durable cross-run memory stored outside the ephemeral runner pod, so that the agent does not re-derive the same root cause and re-attempt failed approaches on every CI invocation.

#### Acceptance Criteria

1. THE Autofix_Harness SHALL persist the Persistent_KB as kiro-cli's native knowledge base on an NFS-mounted volume at the kiro-cli data dir, so the knowledge store and its index survive destruction of an ephemeral runner pod.
2. THE Autofix_Harness SHALL store no memory in git and SHALL bake no memory into the container image.
3. THE Autofix_Harness SHALL grant the built-in `knowledge` tool by including `knowledge` in both the `tools` and `allowedTools` arrays of `autofix.json`, so the headless `--no-interactive` agent can use the Knowledge_Tool.
4. THE Autofix_Harness SHALL enable the experimental knowledge feature by setting `chat.enableKnowledge` to `true` and SHALL set `knowledge.indexType` to `Fast`.
5. THE Autofix_Harness SHALL persist the knowledge settings across ephemeral pods by baking them into the runner image or setting them idempotently at job start.
6. WHEN the agent begins a fix, THE Autofix_Harness SHALL search the Persistent_KB via the Knowledge_Tool for the current Memory_Key before attempting the fix.
7. IF the Persistent_KB records an approach for the current Memory_Key as `failed`, THEN THE Autofix_Harness SHALL avoid re-attempting that approach.
8. WHEN the Stop_Gate confirms a verified outcome, THE Autofix_Harness SHALL store a lesson in the Persistent_KB via the Knowledge_Tool, keyed by the Memory_Key, recording whether the approach worked or failed.
9. THE Autofix_Harness SHALL key each Persistent_KB entry by the Memory_Key, where `diag_sig` is the normalized Diagnostic_Signature, falling back to a hash of the first error line when no code is present.
10. THE Autofix_Harness SHALL partition the Persistent_KB per branch under `knowledge_bases/<branch>/`, deriving the branch segment from `github.event.workflow_run.head_branch` sanitized by replacing `/` and any non-`[A-Za-z0-9._-]` character with `_`.
11. THE Autofix_Harness SHALL reuse the existing per-branch concurrency group `kiro-autofix-${{ head_branch }}` and SHALL NOT introduce a new concurrency group.
12. THE Autofix_Harness SHALL bound the growth of the Persistent_KB out-of-band via a GC_Job that trims entries by an age cap and a size cap.

### Requirement 11: Persistent-memory infrastructure delivery constraints (F10)

**User Story:** As an infrastructure owner, I want the persistent-memory infra changes gated and human-applied, so that high-risk changes follow CODEOWNERS review and never bypass the spike gate.

#### Acceptance Criteria

1. THE Autofix_Harness SHALL require the Validation_Spike to pass before any change to `infra/` or any helm change for the Persistent_KB.
2. WHEN the Validation_Spike runs, THE Validation_Spike SHALL confirm that the granted Knowledge_Tool functions in a headless `--no-interactive` invocation (search returns results and store persists) and SHALL determine the real KB store path on the runner image.
3. IF the Validation_Spike shows the granted Knowledge_Tool does not function headless, THEN THE Autofix_Harness SHALL redesign or drop F10 and SHALL NOT proceed to the NFS mount or any helm change.
4. THE Autofix_Harness SHALL deliver the `runner-scale-set-values.yml` NFS-volume delta as a human-applied, CODEOWNERS-reviewed change, since `infra/**` is write-denied to the agent.
5. THE Autofix_Harness SHALL mount the `kiro-memory` NFS volume into the runner container at the kiro-cli data dir and SHALL extend the `init-nfs-permissions` initContainer to make the new mount writable, reusing the existing cargo-cache NFS server and path convention.

### Requirement 12: Graceful degradation and spawn-time state handling (F10)

**User Story:** As a harness operator, I want the agent to run without memory when the NFS store is unavailable, so that a NAS outage slows learning instead of breaking autofix.

#### Acceptance Criteria

1. WHEN the agent spawns, THE AgentSpawn_Hook SHALL clear stale verification state, including the per-stack Sensor_Ran_State directory and the block-count file, and SHALL reset Within_Run_Scratch to empty.
2. WHEN the agent spawns AND the Persistent_KB directory is present on the NFS mount, THE AgentSpawn_Hook SHALL announce that memory is available and already indexed.
3. IF the Persistent_KB directory is absent when the agent spawns, THEN THE AgentSpawn_Hook SHALL announce that the agent proceeds without memory and THE Autofix_Harness SHALL continue the run as a soft degradation rather than failing the job.
4. THE AgentSpawn_Hook SHALL NOT rebuild or re-index the Persistent_KB from source, since the NFS-mounted store is already present and indexed at session start.

### Requirement 13: Lifecycle trace and metrics integration (F9, GAP-7)

**User Story:** As a harness maintainer, I want the expansion to feed the existing observability pipeline, so that verification metrics are accurate without duplicating the trace design.

#### Acceptance Criteria

1. THE Autofix_Harness SHALL delegate all trace, metric, and eval persistence to `.kiro/plans/harness-observability.md` and SHALL NOT specify a new trace-emission design.
2. THE Autofix_Harness SHALL expose the per-stack Sensor_Ran_State as the `harness.sensors_ran` signal so the observability pipeline can compute `verification_gap` per stack.
3. THE Autofix_Harness SHALL NOT recreate a git-tracked learnings JSONL to satisfy the prior observability Phase 7 assumption; cross-run memory SHALL be the Persistent_KB only.

### Requirement 14: Preservation of existing controls and security posture

**User Story:** As a security-conscious maintainer, I want the expansion to add controls without weakening any existing protection, so that the harness never loses ground.

#### Acceptance Criteria

1. THE Autofix_Harness SHALL retain every existing `deniedPaths` entry, the git-block `preToolUse` guard, the auto-format `postToolUse` behavior, and the max-2-block escape hatch unchanged.
2. THE Autofix_Harness SHALL parse all hook input as untrusted data using safe extraction and quoted expansions, and SHALL NOT execute agent-provided strings.
3. THE Autofix_Harness SHALL record in Within_Run_Scratch and Persistent_KB entries only artifact names, Diagnostic_Signatures, file paths, verdicts, and short approach summaries, and SHALL NOT record diagnostic file contents, tokens, or environment values.
4. THE Autofix_Harness SHALL require human, CODEOWNERS-reviewed application for changes under `infra/`, `.github/workflows/`, and `.github/actions/`, and SHALL keep these paths write-denied to the agent.
