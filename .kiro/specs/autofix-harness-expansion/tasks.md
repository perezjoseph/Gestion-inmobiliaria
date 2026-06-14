# Implementation Plan: Autofix Harness Expansion

## Overview

This plan converts the approved design into incremental, verifiable coding steps. Work is sequenced by the design's ratchet order: the lower-risk, fully editable Verification (F1–F3) and Scope (F6–F8) controls land first, followed by within-run State (F4/F5), the non-code validators (F2), and finally the highest-infra-cost feature (F10 persistent NFS knowledge base), which is gated behind a hard validation spike.

Implementation surfaces:

- `.kiro/agents/autofix.json` — agent config: `deniedPaths`, `preToolUse`/`postToolUse`/`stop`/`agentSpawn` hook bodies (bash) with their `matcher` fields, and the `knowledge` tool grant.
- `.kiro/shared/autofix-system.md` — system prompt: sensor tables, handoff block, KB advisory.
- `.kiro/skills/verify-fix-loop/SKILL.md` — sensor-selection table rows.
- `.github/workflows/kiro-autofix-trigger.yml` (and sibling autofix workflows) — within-run scratch reset, per-branch KB export.
- runner image (`ghcr.io/perezjoseph/realestate-runner`) — validator tools and knowledge settings.
- `infra/k8s/arc-v2/runner-scale-set-values.yml` + a GC CronJob manifest — **human-applied, CODEOWNERS-reviewed** (the agent is write-denied on `infra/**`).

Constraints respected throughout: reuse existing tooling (`ruff` at `.trunk/configs/ruff.toml`, `pytest`/`hypothesis` already in `ocr-service/`) with no new runtime dependency; treat all hook input as untrusted (safe `jq` extraction, quoted expansions, no `eval`); preserve every existing control (existing `deniedPaths`, git-block guard, auto-format, max-2-block escape hatch); and delegate all trace/metrics work to `.kiro/plans/harness-observability.md` (F9) rather than duplicating it.

## Tasks

- [ ] 1. Shared stack classifier (F3 supporting component)
  - [x] 1.1 Create the shared `classify_stack`/`sensor_pattern` bash function
    - Add a single sourced script (e.g. `.kiro/shared/autofix-stack-classify.sh`) mapping a file path to exactly one of `rust | ts | kotlin | python | docker | k8s | shell | none`, mirroring the `classify()` switch in the design
    - Return `none` for files that legitimately need no sensor (e.g. Markdown), so the stop gate does not block on them
    - Expose it as the single source of truth referenced by both the `postToolUse` sensor-ran tracker and the `stop` gate
    - _Requirements: 4.1, 4.2, 4.3_

  - [~] 1.2 Write property test for the stack classifier
    - **Property (Classifier totality): `classify_stack(f)` returns exactly one stack for any path, and `none` only for paths with no sensor suite**
    - Use a table-driven `hypothesis` generator under `ocr-service/` (or a shell corpus) over representative and adversarial paths
    - **Validates: Requirements 4.1, 4.2**

- [x] 2. Python sensor suite for ocr-service (F1)
  - [x] 2.1 Document the Python sensor suite in `autofix-system.md`
    - Add the Verify-section commands run from `ocr-service/`: `ruff format --check`, then `ruff check`, then `python -m pytest`, in Keep-Quality-Left order
    - State reuse of existing `.trunk/configs/ruff.toml` and the existing `pytest`/`hypothesis` tests; introduce no new runtime dependency
    - _Requirements: 1.1, 1.2, 1.4_

  - [x] 2.2 Add the Python row to the `verify-fix-loop` SKILL sensor-selection table
    - Mirror the same three commands and ordering so the skill and system prompt agree
    - _Requirements: 1.3_

- [ ] 3. Per-stack sensor-ran tracking and stop gate (F3)
  - [~] 3.1 Replace the sensor-ran tracker, stop gate, and formatter in `autofix.json`
    - Replace the `postToolUse` sensor-ran tracker so each sensor command touches a per-stack marker under `$RUNNER_TEMP/autofix-sensors-ran.d/` (rust/ts/kotlin/python/docker/k8s/shell)
    - Replace the `stop` gate to source the classifier and block exit when any modified file's stack has no matching marker, naming each missing stack; allow exit when all modified files are `none` or have markers, when `Modified_Files_State` is empty, or after 2 blocks; on clean exit remove `autofix-modified-files.txt`, `autofix-sensors-ran.d/`, and the block-count file
    - Extend the `postToolUse` formatter with the `*/ocr-service/*.py) ruff format` case
    - Retain each hook's `matcher` field (`execute_bash` for the tracker, `fs_write`/`str_replace` for the formatter) so hooks bind correctly
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 4.3_

  - [~] 3.2 Write property tests for the stop gate
    - **Property 1: Sensor coverage completeness — exit allowed implies `sensor-ran[s]` set for every modified stack `s` (or block count = 2)**
    - **Property 2: No silent stack — a modified Python file with only a Rust sensor run implies the gate blocks**
    - Pipe crafted `Modified_Files_State` + marker-dir states into the hook and assert `{"decision":"block"}` vs clean exit and cleanup
    - **Validates: Requirements 3.2, 3.3, 3.4, 3.5, 3.6**

  - [~] 3.3 Write unit tests for the sensor-ran tracker
    - Pipe representative `tool_input.command` JSON (cargo, npm, gradlew, pytest/ruff, hadolint, kubeconform, shellcheck) and assert the correct per-stack marker is created
    - _Requirements: 3.1_

- [ ] 4. Expanded write denials for lockfiles and dependency manifests (F8)
  - [~] 4.1 Add the new `deniedPaths` entries to `autofix.json`
    - Append `baileys-service/package-lock.json`, `ocr-service/requirements.txt`, `android/gradle/libs.versions.toml`, and the forward-looking `android/**/*.lockfile`
    - Retain every pre-existing `deniedPaths` entry unchanged
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.6, 14.1_

  - [~] 4.2 Write property test for dependency immutability
    - **Property 4: Dependency immutability — any write targeting a lockfile or pinned-dependency manifest is denied**
    - Assert each new and pre-existing denied path is matched; assert a non-denied source path is allowed
    - **Validates: Requirements 9.1, 9.2, 9.3, 9.5, 9.6**

- [ ] 5. Suppression guard (F6)
  - [~] 5.1 Add the `preToolUse` suppression guards to `autofix.json`
    - Add `str_replace` and `fs_write` matcher hooks that count suppression tokens (`#[allow(`, `@ts-ignore`, `@ts-nocheck`, `eslint-disable`, `@Suppress`, `# type: ignore`, `# noqa`) and `exit 2` only when the new count exceeds the old (treating a non-existent file as 0)
    - Parse input via `jq -r ... // empty` with quoted expansions; never `eval` agent strings
    - Document that this hard `preToolUse` denial is exempt from the max-2-block escape hatch
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

  - [~] 5.2 Write property test for the suppression guard
    - **Property 3: Suppression impossibility — a write that increases the suppression-token count is blocked (exit 2); a write that leaves the count unchanged, including relocating an existing suppression, is allowed**
    - Cover both `str_replace` (newStr vs oldStr) and `fs_write` (text vs on-disk file) paths, including the count-neutral relocate case
    - **Validates: Requirements 7.1, 7.2, 7.3**

- [ ] 6. Diff-budget guard (F7, advisory)
  - [~] 6.1 Add the advisory `postToolUse` diff-budget guard to `autofix.json`
    - Count distinct files in `Modified_Files_State`; emit `::notice::` at the soft threshold and `::warning::` at the hard threshold instructing the agent to narrow scope or exit `Status: PARTIAL`
    - Read thresholds from `KIRO_DIFF_SOFT` (default 8) and `KIRO_DIFF_HARD` (default 15); block no write and gate no exit
    - _Requirements: 8.1, 8.2, 8.3, 8.4_

  - [~] 6.2 Write unit tests for the diff-budget thresholds
    - Assert no annotation below soft, `::notice::` at soft, `::warning::` at hard, and that exit code is always 0 (advisory only)
    - _Requirements: 8.1, 8.2, 8.3, 8.4_

- [~] 7. Checkpoint — Verification and Scope controls
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 8. Within-run scratch learnings (F4)
  - [x] 8.1 Keep the queue-start scratch reset in the autofix trigger workflow
    - Confirm/keep `: > "$LEARNINGS_FILE"` (`$RUNNER_TEMP/autofix-learnings.txt`) at queue start; ensure there is no seed-from-git step and no commit-of-memory step
    - Allow short attempt notes to be appended during the run so later artifacts in the same Queue_Run benefit from earlier ones; record only artifact names, diagnostic signatures, file paths, verdicts, and short summaries
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 13.3, 14.3_

  - [~] 8.2 Write test for within-run scratch behavior
    - **Property 5: Within-run memory — notes appended while processing earlier artifacts are visible when processing a later artifact in the same run; the file is ephemeral and never git-seeded**
    - **Validates: Requirements 5.1, 5.2, 5.3, 5.4**

- [ ] 9. Structured compaction handoff (F5)
  - [~] 9.1 Add the "Handoff on Compaction" section to `autofix-system.md`
    - Instruct the agent to write a handoff block (goal, constraints, progress done/in-progress/blocked, decisions, next steps, cumulative modified files) when context approaches the limit
    - Populate the FILES field from the existing `Modified_Files_State` side-channel
    - _Requirements: 6.1, 6.2, 6.3_

- [ ] 10. Non-code validators (F2)
  - [~] 10.1 Add the non-code validator tier to `autofix-system.md`
    - Document `hadolint` (docker), `kubeconform -strict -ignore-missing-schemas` (k8s), `shellcheck` (shell) as validation-only sensors
    - State they do not authorize writes to `infra/**`, which remains write-denied; if a required tool is absent from PATH after the retry-wrapped install, the stack is treated as un-verified so the stop gate blocks
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [~] 10.2 Add the non-code validator row to the `verify-fix-loop` SKILL table
    - Mirror the three validators and their stacks
    - _Requirements: 2.1, 2.2, 2.3_

  - [~] 10.3 Add validator tools to the runner image with pinned versions
    - Add `hadolint`, `kubeconform`, and `shellcheck` to `ghcr.io/perezjoseph/realestate-runner` (pin exact versions; ensure `ruff` is also on PATH), fetched via the retry-wrapped install pattern
    - _Requirements: 2.5, 1.4_

  - [~] 10.4 Write tests for non-code stack classification and tracking
    - Assert the classifier maps Dockerfile/`infra/k8s` manifests/`*.sh` to `docker`/`k8s`/`shell`, and the tracker creates the matching markers for `hadolint`/`kubeconform`/`shellcheck` commands
    - _Requirements: 2.1, 2.2, 2.3, 3.1_

- [ ] 11. F10 validation spike — HARD GATE
  - [~] 11.1 Grant the `knowledge` tool in `autofix.json`
    - Add `"knowledge"` to BOTH the `tools` and `allowedTools` arrays so the headless `--no-interactive` agent can use it
    - _Requirements: 10.3_

  - [~] 11.2 Author and run the headless knowledge spike
    - Write a spike script that runs `kiro-cli chat --agent autofix --no-interactive --trust-all-tools` on the runner image and confirms (1) a knowledge `search` returns results and (2) a knowledge `store` persists; record the result
    - Determine the real KB store path on the runner image (do not trust the documented path blindly)
    - Gate decision: if the tool does not function headless, redesign or drop F10 and do NOT proceed to any `infra/` or helm work
    - _Requirements: 11.1, 11.2, 11.3_

- [~] 12. Checkpoint — F10 spike gate
  - Ensure all tests pass, ask the user if questions arise. Do not start tasks 13–14 unless the spike (11.2) confirmed the knowledge tool functions headless and the real KB path is known.

- [ ] 13. Persistent memory — agent, settings, and workflow (F10)
  - [~] 13.1 Update the `agentSpawn` hook in `autofix.json`
    - Clear stale verification state (the F3 per-stack `autofix-sensors-ran.d/` dir and the block-count file) and reset the within-run scratch to empty
    - Announce memory available + already indexed when the NFS KB dir is present; announce proceeding WITHOUT memory (soft degradation) when absent; never rebuild or re-index from source
    - _Requirements: 12.1, 12.2, 12.3, 12.4_

  - [~] 13.2 Add the per-branch KB export step to the autofix workflows
    - Export a sanitized branch segment from `github.event.workflow_run.head_branch` (replace `/` and any non-`[A-Za-z0-9._-]` char with `_`) so the KB points at `knowledge_bases/<branch>/`
    - Reuse the existing `kiro-autofix-${{ head_branch }}` concurrency group; add no new concurrency group
    - _Requirements: 10.10, 10.11_

  - [~] 13.3 Persist knowledge settings on the runner image
    - Bake `chat.enableKnowledge true` and `knowledge.indexType Fast` into the runner image (or set them idempotently at job start)
    - _Requirements: 10.4, 10.5_

  - [~] 13.4 Add the F10 KB advisory to `autofix-system.md`
    - Instruct the agent to search the Persistent_KB via the `knowledge` tool for the current `(artifact, diag_sig)` Memory_Key before a fix, avoid approaches recorded as `failed`, and store a lesson keyed by the Memory_Key after the stop gate confirms a verified outcome
    - Specify `diag_sig` normalization (lint name, error code, test name, ruff/hadolint rule, or hash of first error line) and that entries record only safe metadata, never file contents/tokens/env values
    - _Requirements: 10.6, 10.7, 10.8, 10.9, 14.3_

  - [~] 13.5 Write test for graceful degradation
    - **Property 12: Graceful degradation — when the NFS KB dir is absent, `agentSpawn` announces no-memory and the run continues (soft, not hard, failure); state is still cleared**
    - **Validates: Requirements 12.1, 12.3**

- [ ] 14. Persistent memory — infrastructure delta (F10, human-applied / CODEOWNERS)
  - [~] 14.1 Author the proposed NFS-volume delta to `runner-scale-set-values.yml`
    - Add the `kiro-memory` NFS volume (server `192.168.88.22`, path under `/volume1/docker/k3s-cache/`), mount it in the `runner` container at the kiro-cli data dir, and extend the `init-nfs-permissions` initContainer to make the mount writable, reusing the existing cargo-cache pattern
    - Mark as human-applied and CODEOWNERS-reviewed; `infra/**` is write-denied to the autofix agent and the `helm upgrade` application is performed by a human
    - _Requirements: 10.1, 10.2, 11.4, 11.5_

  - [~] 14.2 Author the GC CronJob manifest
    - Create a Kubernetes CronJob that mounts the same `kiro-memory` NFS path and trims the store by an age cap and a size cap (oldest-first eviction); human-applied/CODEOWNERS
    - _Requirements: 10.12_

  - [~] 14.3 Write test for the GC policy
    - **Property 7: Bounded memory store — after a GC run the store is trimmed to the configured age and/or size cap**
    - Exercise the prune logic against a synthetic store fixture and assert age/size caps are enforced
    - **Validates: Requirements 10.12**

- [ ] 15. Lifecycle integration and control preservation (F9, R13, R14)
  - [~] 15.1 Expose the per-stack sensor-ran state as the `harness.sensors_ran` signal
    - Surface `Sensor_Ran_State` so the observability pipeline can compute `verification_gap` per stack; delegate all trace/metric/eval persistence to `.kiro/plans/harness-observability.md` and add no new trace design; do not recreate a git-tracked learnings JSONL
    - _Requirements: 13.1, 13.2, 13.3_

  - [~] 15.2 Verify security posture of all new hooks and denials
    - Confirm every hook parses input with safe `jq` extraction and quoted expansions and executes no agent-provided strings; confirm scratch/KB entries record only safe metadata; confirm `infra/`, `.github/workflows/`, and `.github/actions/` remain write-denied and CODEOWNERS-gated
    - _Requirements: 14.2, 14.3, 14.4_

  - [~] 15.3 Write regression test for preserved existing controls
    - **Property 6: No regression — every pre-existing `deniedPaths` entry, the git-block guard, the auto-format behavior, and the max-2-block escape hatch are preserved unchanged**
    - **Validates: Requirements 14.1**

- [~] 16. Final checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional test sub-tasks and can be skipped for a faster MVP; core implementation sub-tasks are never optional.
- Each task references specific requirement sub-clauses for traceability; property sub-tasks name the design property and the requirements they validate.
- The design has a Correctness Properties section, so property-based tests are included. The classifier property uses `hypothesis` (already in `ocr-service/`); hook properties are exercised by piping crafted tool-input JSON and asserting exit code / stderr / decision JSON.
- F10 (tasks 13–14) is gated: do not begin until the validation spike (11.2) passes. If the spike fails, redesign or drop F10 — no `infra/` or helm change proceeds.
- The `infra/` delta and GC CronJob (task 14) are authored as proposed changes but applied by a human under CODEOWNERS review, because `infra/**` is write-denied to the agent.
- F9 (lifecycle/trace) defers entirely to `.kiro/plans/harness-observability.md`; this plan only exposes the `harness.sensors_ran` signal and reconciles the removed git-tracked learnings JSONL.

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1", "2.1", "2.2", "8.1"] },
    { "id": 1, "tasks": ["3.1", "1.2", "9.1", "10.2"] },
    { "id": 2, "tasks": ["4.1", "3.2", "3.3", "10.1", "10.3"] },
    { "id": 3, "tasks": ["5.1", "4.2", "10.4"] },
    { "id": 4, "tasks": ["6.1", "5.2", "8.2"] },
    { "id": 5, "tasks": ["11.1", "6.2"] },
    { "id": 6, "tasks": ["11.2"] },
    { "id": 7, "tasks": ["13.1", "13.2", "13.3", "13.4"] },
    { "id": 8, "tasks": ["13.5", "14.1", "14.2"] },
    { "id": 9, "tasks": ["14.3", "15.1", "15.2"] },
    { "id": 10, "tasks": ["15.3"] }
  ]
}
```
