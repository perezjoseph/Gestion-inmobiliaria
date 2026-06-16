# Kiro CLI Built-in Tools Reference

Distilled from the official docs ([kiro.dev/docs/cli/reference/built-in-tools](https://kiro.dev/docs/cli/reference/built-in-tools/), page updated 2026-06-12). Content rephrased for compliance with licensing restrictions.

## Why this matters for harnesses

Hook `matcher` fields and `preToolUse`/`postToolUse` gating key off the **internal tool name** (the alias), not the friendly config name. Writing `matcher: "write"` never fires — the runtime emits `fs_write`. The table below is the source of truth for which name to use where.

- `tools` / `allowedTools` arrays: use the **config name** (`read`, `write`, `shell`, `aws`, `subagent`, ...) or the `@builtin` / `@server` sigils.
- `hooks.*.matcher`: use the **internal/alias name** (`fs_read`, `fs_write`, `execute_bash`, `use_aws`, `use_subagent`).
- `toolsSettings` keys: use the **config name** (`read`, `write`, `shell`, `glob`, `grep`, `aws`, `web_fetch`, `subagent`).

## Tool name → aliases → matcher

| Config name | Aliases (internal / matcher) | Purpose |
|-------------|------------------------------|---------|
| `read` | `fs_read`, `fsRead` | Read files, folders, images |
| `write` | `fs_write`, `fsWrite` | Create and edit files |
| `shell` | `execute_bash`, `execute_cmd` | Run a shell command |
| `aws` | `use_aws` | Make AWS CLI calls (service/operation/params) |
| `glob` | — | Fast file discovery by glob; respects `.gitignore` (prefer over `find`) |
| `grep` | — | Fast regex content search; respects `.gitignore` (prefer over `grep`/`rg`/`ag`) |
| `subagent` | `use_subagent` | Delegate to up to 4 parallel subagents with isolated context |
| `web_search` | — | Search the web |
| `web_fetch` | — | Fetch content from a URL (`selective` default / `truncated` / `full`) |
| `code` | — | Code intelligence: symbol search, LSP, pattern search/rewrite |
| `introspect` | — | Answer questions about Kiro CLI itself from built-in docs |
| `tool_search` | — | Find/load MCP tools on demand (read-only, auto-allowed) |
| `delegate` | — | Delegate to async background agents for long-running work |
| `knowledge` | — | (experimental) Store/retrieve info across sessions; semantic search |
| `thinking` | — | (experimental) Internal step-by-step reasoning |
| `todo` | — | (experimental) Track multi-step tasks |
| `goal` | — | Drives `/goal` iterative loops; used internally, not invoked directly |
| `session` | — | Temporarily override session-safe settings (list/get/set/reset) |
| `report` | — | Open a pre-filled GitHub issue template |

## Default permission behavior

- `report` is trusted by default.
- `read`, `grep`, `glob` are trusted in the current working directory; `code` symbol lookups/edits inside the workspace run without prompting.
- `shell`, `write`, `aws` prompt by default — narrow them with `toolsSettings` or list in `allowedTools`.
- `tool_search` is auto-allowed (read-only).
- `allowedTools` does **not** accept a `"*"` wildcard; list specific names or sigils.

## Config knobs worth knowing (for verification harnesses)

### shell (`toolsSettings.shell`)
- `allowedCommands` / `deniedCommands`: regex, anchored with `\A`...`\z`, **no look-around**. Deny is evaluated before allow.
- `autoAllowReadonly`: auto-approve read-only commands (does not restrict writes).
- `denyByDefault`: deny anything not explicitly allowed/auto-approved instead of prompting — the safe default for a CI agent.

### read / write / glob / grep
- `allowedPaths` / `deniedPaths`: gitignore-style globs; deny beats allow. (`glob`/`grep` also take `allowReadOnly`.)

### web_fetch (`toolsSettings.web_fetch`)
- `trusted` / `blocked`: regex URL patterns, anchored `^`...`$`; `blocked` wins; invalid `blocked` regex fails safe (denies all).

### subagent (`toolsSettings.subagent`)
- `availableAgents`: glob-restricted spawnable list. `trustedAgents`: run without prompts. Must add `subagent` to `tools` (or `@builtin`) in a custom agent.

### Shell side channels (wrapper scripts)
When the agent drives a shell command, Kiro exports two FIFOs:
- `AGENT_DISPLAY_OUT` — user-facing only (e.g. verbose build log); never enters agent context.
- `AGENT_CONTEXT_OUT` — surfaces in the tool result's `agent_notes`, so the agent sees it even when stdout is piped through `grep`/`tail`.

Both are only set when the agent runs the command; wrappers must fall back gracefully when empty (`[ -n "${AGENT_CONTEXT_OUT:-}" ]`). Useful for routing sensor summaries into agent context without bloating it.
