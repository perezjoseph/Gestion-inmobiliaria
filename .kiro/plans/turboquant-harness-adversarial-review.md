# Turboquant Multi-Agent Harness — Adversarial First-Run Review

## FAIL

The harness will not complete its first autonomous iteration. Multiple independent failures guarantee this.

---

## Critical Blockers (will crash first run)

### B1. Verifier sentinel hook never fires — wrong matcher name

**File**: `turboquant-sycl.json:103`  
**Matcher value**: `"invoke_sub_agent"`  
**Correct value**: `"use_subagent"` (per `built-in-tools.md`)

The postToolUse hook that creates `/tmp/turboquant-verifier-passed` uses the wrong internal tool name. The hook NEVER fires. The push gate (line 90) checks for this sentinel file and blocks with "Verifier not run." The optimizer is permanently deadlocked: it can never push because the sentinel is never created, regardless of what the verifier actually returns.

**Impact**: Loop halts after first optimization attempt. Cannot push. Cannot proceed.  
**Previous review**: Identified this same class of bug (was `"subagent"`, now `"invoke_sub_agent"`). Still wrong after the "fix."

---

### B2. Context window exhaustion on agent spawn

**Resource load at spawn**:

| File | Size |
|------|------|
| ggml-sycl.cpp | 244 KB / 5375 lines |
| test-backend-ops.cpp | 416 KB / 8752 lines |
| ggml-common.h | 137 KB / 1874 lines |
| mmvq.cpp | 131 KB / 2366 lines |
| common.hpp | 34 KB / 864 lines |
| turbo_quant.hpp | 32 KB / 609 lines |
| Others (7 files) | ~50 KB combined |
| **TOTAL** | **~1,044 KB / ~20,420 lines** |

Plus 3 skill files (~15 KB), the system prompt (~5 KB), and the agentSpawn hook output. That's ~1 MB of raw text loaded before the agent makes its first tool call.

Claude's context window is 200K tokens. At ~4 chars/token, 1 MB = ~250K tokens. The resources alone **exceed the context window**. The agent either fails to spawn, gets a truncated context (missing critical files), or is compacted immediately — losing the system prompt and instructions.

**Impact**: Agent cannot function. Even if it spawns, it has no room for reasoning or tool calls.

---

### B3. oneAPI version mismatch — optimizer targets 2025.3, CI builds with 2025.1

**System prompt**: "Compiler: icpx (oneAPI 2025.3)"  
**release-sycl.yml container**: `intel/oneapi-basekit:2025.1.0-0-devel-ubuntu24.04`  
**Upstream build-sycl.yml**: Uses `ONEAPI_INSTALLER_VERSION: "2025.3.3"`

The optimizer is told it has access to oneAPI 2025.3 features (joint_matrix, newer intrinsics). Any code it writes using 2025.3-specific APIs will compile fine in documentation but fail in the actual CI container which runs 2025.1.0. The release agent will report "build failed" with cryptic template instantiation errors, and the optimizer will chase phantom bugs for multiple iterations.

**Impact**: Build failures on every iteration that uses 2025.3 features. 20-minute feedback loop per failed attempt.

---

### B4. No llama-bench in the pod — baseline measurement impossible

The optimizer's first task is "measure baseline with llama-bench." But:

1. **release-sycl.yml** only builds `--target llama-server`. `llama-bench` is not built or packaged.
2. **llama-cpp.yml** (k8s manifest) downloads only `llama-server-sycl-intel-arc.tar.gz` from the release. No bench binary.
3. The pod has no `llama-bench` binary. The optimizer cannot profile.

The optimizer's workflow starts with PROFILE. Without profiling, it cannot identify bottlenecks, cannot justify optimizations, and the recording gate blocks push (no before/after tok/s to record).

**Impact**: First workflow step fails. The optimizer either halts or skips profiling and violates its own rules.

---

### B5. No fast-fail — 20-minute blind build cycles

The release-sycl workflow takes ~20 minutes (full cmake configure + build in a container). There is:
- No local syntax check before push
- No `icpx -fsyntax-only` step
- No incremental build cache (container is ephemeral)
- No pre-push compilation gate in the hooks

A single typo costs 20 minutes. The optimizer will discover this on iteration 1 when it introduces its first change. If the agent is aggressive (multiple changes per session), it burns 60-100 minutes on failed builds before getting signal.

**Impact**: Massive time waste per iteration. A 10-iteration optimization loop takes days instead of hours.

---

## Serious Issues (will cause failures within first few iterations)

### S1. TDD gate logic bug — always passes on existing upstream commits

The push gate checks `git log --oneline -5 --diff-filter=AM -- tests/` and greps for `test|bench|assert`. The current last 5 commits touching tests/ are from upstream merges ("chat : implement minicpm5 parser", "jinja: add --dump-prog for debugging", etc.). None contain "test", "bench", or "assert."

When the optimizer pushes with a new test commit, the check uses the last 5 commits touching tests/ in the ENTIRE history — mixing the optimizer's commits with upstream noise. If the optimizer's test commit is not in the last 5, the gate fails spuriously. If it IS in the last 5 but another commit pushes it out (e.g., the merge brought many commits touching tests/), the gate fails.

The check should scope to `origin/sycl-support..HEAD` not the global log.

---

### S2. Verifier ground truth fetches from wrong branch

**Verifier system prompt** fetches reference from:  
`https://raw.githubusercontent.com/TheTom/llama-cpp-turboquant/feature/turboquant-kv-cache/ggml/src/ggml-turbo-quant.c`

But the local fork's turbo files have been modified (git status shows turbo_quant.hpp, turbo_dequant.hpp, turbo_fattn.hpp all modified in working tree). The verifier compares the optimizer's SYCL kernel against the CPU reference implementation. If the CPU reference in the fork has already diverged from TheTom's upstream (additional bug fixes, different packing order), the verifier will flag correct code as wrong or approve incorrect code.

---

### S3. /tmp sentinel file does not persist across sessions

`/tmp/turboquant-verifier-passed` lives in the WSL filesystem. When:
- The kiro session is restarted (context compaction triggers new session)
- The machine reboots
- WSL is terminated (`wsl --shutdown`)

...the sentinel file disappears. If the optimizer gets a verifier PASS in one session, then context compacts before push, the new session cannot push because the sentinel is gone. The optimizer must re-run the verifier even though nothing changed.

Worse: `/tmp` in WSL is separate from `/tmp` in Windows. The `bash -c` hooks run in WSL. If kiro ever switches to a non-WSL bash (Git Bash), the paths diverge and the sentinel becomes invisible.

---

### S4. Release agent uses PowerShell syntax in system prompt but runs bash via shell tool

The release agent's verification steps are written in PowerShell (`$body = ...`, `Invoke-RestMethod`, `$r.choices[0]`). But:
- The agent's shell tool runs `execute_bash` (per built-in-tools.md, `shell` -> `execute_bash`)
- On this Windows+WSL system, `execute_bash` invokes WSL bash
- If the agent pastes the PowerShell snippets verbatim into bash, they fail immediately

The agent might be smart enough to translate, but the prompt strongly suggests copy-paste execution. First verification attempt will produce syntax errors.

---

### S5. Working tree has uncommitted changes — agentSpawn hook reports "0 unpushed"

The agentSpawn hook only checks `git log origin/sycl-support..HEAD`. Currently this returns 0 (HEAD and origin are identical). But the working tree has 939 insertions across 3 turbo files that are NOT committed.

The hook tells the agent "Clean state. Start next iteration." But the actual state is NOT clean — there are substantial uncommitted kernel changes. The agent may overwrite these changes or be confused about the codebase state.

---

## Design Issues (will bite within 3-5 iterations)

### D1. Optimization ceiling — wrong bottleneck

The model architecture is:
- 75% DeltaNet layers (linear attention, no KV cache, no turbo kernels fire)
- MoE with 256 experts, 512-wide FFN (tiny matmuls per expert, dominated by gather/scatter overhead)
- Only 10 full-attention layers with 2 KV heads each

The turbo kernels only fire on KV cache operations in the 10 full-attention layers. The optimizer's allowedPaths scope is `ggml-sycl/` files. If profiling reveals the bottleneck is in:
- Expert routing/scatter (likely in `ggml-backend-sycl` or generic GGML)
- Linear attention recurrence (DeltaNet kernel, possibly not in ggml-sycl)
- Memory-bound expert weight loading (608 GB/s ceiling, no kernel fix possible)

...the optimizer has nothing to optimize within its scope. The loop stalls permanently with "no actionable bottleneck."

---

### D2. preToolUse TDD check on fs_write uses exit 1 (warn) not exit 2 (block)

The kernel write TDD check (lines 93-96) exits with code 1, which is a WARNING not a BLOCK. The agent can still write kernel code without a committed test. Only the push gate (exit 2) actually blocks. This means TDD enforcement is advisory, not mechanical, during the write phase.

---

## Summary

The harness has **5 independent blockers** that guarantee first-run failure:

1. Wrong hook matcher (`invoke_sub_agent` should be `use_subagent`) — verifier gate is dead
2. ~1 MB of resources exceeds context window — agent cannot spawn usefully  
3. oneAPI 2025.1 in CI vs 2025.3 in prompt — builds will fail on modern APIs
4. No llama-bench binary available — cannot profile (first workflow step)
5. No fast-fail — 20 min per mistake with no local compilation check

Fix priority: #1 (one-character fix), #2 (remove ggml-common.h, test-backend-ops.cpp, ggml-sycl.cpp from resources — agent should `read` them on demand), #3 (update release-sycl.yml to use oneapi-basekit:2025.3.3), #4 (add llama-bench to build targets and release tarball), #5 (add pre-push `icpx -fsyntax-only` on changed files).
