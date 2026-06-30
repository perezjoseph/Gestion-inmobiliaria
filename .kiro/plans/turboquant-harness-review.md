# Turboquant Agent Harness Review

## FAIL

### Verification Results
- JSON validity: PASS (all 3 configs parse clean)
- Shell syntax: PASS (all hooks are valid bash)
- Exit code semantics: PASS (exit 2 = block, exit 1 = warn, exit 0 = pass)
- Prompt ↔ config alignment: FAIL (1 issue)
- Scope boundaries: FAIL (2 issues)
- Subagent configuration: FAIL (1 issue)
- Adversarial bypass vectors: FAIL (2 critical vectors)

### Issues

#### P0 (blocking)

1. **turboquant-sycl.json:97** — `"matcher": "subagent"` uses the config name, not the internal name. Per `built-in-tools.md`, the correct matcher value is `"use_subagent"`. The runtime emits `use_subagent` as the tool event name. This hook **never fires**, meaning the optimizer never sees the `[GATE] Verifier returned FAIL` annotation after spawning the verifier. The verifier gate is completely inert.
   - Fix: Change `"matcher": "subagent"` to `"matcher": "use_subagent"`.

2. **turboquant-sycl.json:87-89** (preToolUse push gate) — The TDD/recording gate blocks `git push` but does NOT require a passing verifier verdict. The optimizer can: commit a test, commit kernel code, update the log, then push — all without ever spawning the verifier. The `postToolUse` commit hook (line 101-103) only prints an advisory "[NEXT] Spawn verifier" with exit 0. Since issue #1 means the subagent postToolUse hook never fires either, there is **no mechanical enforcement** of verifier-before-push.
   - Fix: Add a verifier-ran check to the `preToolUse` push gate. For example, check for a sentinel file (e.g., `/tmp/turboquant-verifier-passed`) that the (fixed) `postToolUse` `use_subagent` hook creates only on PASS. Alternatively, `exit 2` in the commit postToolUse hook if no verifier verdict exists since the last kernel change.

#### P1 (must fix)

1. **turboquant-sycl.json:91-93** (preToolUse `fs_write`) — TDD check only matches `fs_write`. The optimizer also has `write` in its tools, which at runtime can emit `str_replace` for partial edits. Kernel code edited via `str_replace` bypasses the TDD warning entirely.
   - Fix: Add a second `preToolUse` hook with `"matcher": "str_replace"` applying the same TDD check for `*/ggml-sycl/*.cpp|*/ggml-sycl/*.hpp` paths.

2. **turboquant-release.json** (shell deniedCommands) — The release agent's denied list omits `git push`. Its system prompt explicitly instructs it to `git push origin sycl-support` as step 1. This means the release agent can push code that was never validated by the optimizer's TDD gate or verifier, since the release agent has none of those hooks. If the optimizer delegates prematurely (before its own push gate would pass), the release agent pushes uninspected code.
   - Fix: Either (a) remove `git push` from the release system prompt and deny it in config (the optimizer should always push), or (b) add TDD/log validation hooks to the release agent's preToolUse as well.

3. **turboquant-verifier.json** (shell deniedCommands) — Missing `git clean`, `rm` (without flags), `truncate`, `>` redirect. While the verifier lacks `write`, it still has `shell`. A plain `rm file.cpp` or `git clean -f` would succeed. The prompt claims "Read-only: no write tool, no git commit/push/reset, no kubectl" but `shell` can still mutate the filesystem.
   - Fix: Add `rm`, `git clean`, `mv`, `cp` to deniedCommands, or set `denyByDefault: true` with an `allowedCommands` list of read-only operations (`git diff`, `git log`, `git show`, `cat`, `grep`, `find`, `wc`).

#### P2 (should fix)

1. **turboquant-sycl.json:91-93** — The `fs_write` TDD hook exits with code `1` (warn) not `2` (block). This means the optimizer gets a warning but can still write kernel code without a committed test. The stronger gate (`exit 2`) is used for push but not for writes. Intentional? If TDD enforcement matters, kernel writes should also block.
   - Fix: Consider changing `exit 1` to `exit 2` if TDD is meant to be hard-enforced at write time, not just at push time.

2. **turboquant-release-system.md** — The verification step uses PowerShell (`Invoke-RestMethod`, `$body`, `$r`, `$text`), but hooks run via `bash -c`. The release agent's shell tool runs bash. If the agent follows the prompt literally and pastes the PowerShell snippet into a bash shell, it will fail. The prompt should use `curl` instead.
   - Fix: Replace the PowerShell verification code block with equivalent `curl`/`jq` commands, or note that the agent runs on a Windows host with PowerShell available via `pwsh`.

3. **turboquant-verifier.json** — No `mcpServers` isolation. The verifier has `context7` MCP server configured. While not a security issue (it's a documentation lookup service), the verifier's read-only contract should minimize external dependencies. If `context7` has write capabilities, this could be a scope escape.
   - Fix: Confirm context7 is read-only, or remove it from the verifier if not needed for its workflow.

4. **turboquant-sycl.json** (resources) — Individual file resources (14 entries) will break silently when files are renamed or added. Per `kiro-config-guide.md`: "Use glob patterns (`**/*.md`) — individual listings break when files added/removed."
   - Fix: Replace individual file listings with `"file://../../infra/llama-cpp-turboquant/ggml/src/ggml-sycl/**"` and similar globs.

#### P3 (nit)

1. **turboquant-sycl-system.md** — References `context7` for API signatures but the verifier also has it. No issue, just noting both agents can access the same MCP.

2. **OPTIMIZATION_LOG.md** — All baseline values are `TBD`. Not a config issue, but the agentSpawn hook reports "No log yet" only when the file is missing, not when it exists but is unpopulated. An agent could see the template and think baseline is captured.

### Summary

FAIL — The `"matcher": "subagent"` typo (should be `"use_subagent"`) means the verifier gate hook never fires, creating a silent bypass of the entire adversarial verification step. Combined with no mechanical verifier-before-push enforcement in the TDD gate, the optimizer can push unverified code. The `str_replace` gap and release-agent push permission add additional bypass vectors.
