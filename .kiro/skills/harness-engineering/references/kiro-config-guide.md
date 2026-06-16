# Kiro Agent Config — Complete Guide

Detailed field guidance, gotchas, and validation workflow for `.kiro/agents/*.json` files.

## Correct Config Template

```json
{
  "name": "autofix",
  "prompt": "file://../prompts/autofix-system.md",
  "tools": ["*"],
  "allowedTools": ["read", "write", "shell", "@builtin"],
  "resources": [
    "file://../../AGENTS.md",
    "file://../../.kiro/steering/**/*.md",
    "skill://../../.kiro/skills/**/SKILL.md"
  ],
  "hooks": {
    "postToolUse": [
      { "matcher": "fs_write", "command": "cargo fmt --all" }
    ]
  },
  "model": "claude-sonnet-4.5"
}
```

## Field-by-Field Guidance

**`prompt`** — Must be a `file://` URI. Path resolves **relative to config file's directory** (e.g., config at `.kiro/agents/autofix.json` + `file://../prompts/autofix-system.md` = `.kiro/prompts/autofix-system.md`). Common mistake: workspace-root-relative paths like `file://.kiro/prompts/...` which resolves to `.kiro/agents/.kiro/prompts/...`.

**`tools`** vs **`allowedTools`** — Separate concerns. `tools` declares what categories agent can see. `allowedTools` controls which are pre-approved (no confirmation). `allowedTools` does NOT support `"*"` — list specific patterns or `"@builtin"`.

**`resources`** — `file://` for documents, `skill://` for skills. Paths resolve relative to config file (same as `prompt`). Use glob patterns (`**/*.md`) — individual listings break when files added/removed.

**`hooks.postToolUse[].matcher`** — Field name is `matcher` (not `match`, `trigger`, `on`). Value must be **internal tool name**: `fs_write`, `fs_read`, `execute_bash`, `str_replace`. NOT simplified category names. This is the #1 runtime failure passing schema validation.

**`hooks` trigger keys** — Valid: `agentSpawn`, `userPromptSubmit`, `preToolUse`, `postToolUse`, `stop`. NOT `postToolWrite`, `afterWrite`, `onFileChange`.

## Gotchas (Runtime Failures That Pass Validation)

Ordered by frequency. Schema validator catches structure but not runtime correctness.

### 1. Hook matcher field name and value (most common)

```json
// ❌ WRONG — passes validation, fails at runtime
{ "match": "write", "command": "cargo fmt" }
{ "on": "fs_write", "command": "cargo fmt" }
{ "matcher": "write", "command": "cargo fmt" }

// ✅ CORRECT
{ "matcher": "fs_write", "command": "cargo fmt --all" }
```

Internal tool name mapping:
| Category (tools field) | Internal name (matcher value) |
|------------------------|-------------------------------|
| `write` | `fs_write` |
| `read` | `fs_read` |
| `shell` | `execute_bash` |
| `edit` | `str_replace` |

### 2. file:// path resolution (second most common)

```
Config location: .kiro/agents/autofix.json
Prompt location: .kiro/prompts/autofix-system.md

// ❌ resolves to .kiro/agents/.kiro/prompts/autofix-system.md
"prompt": "file://.kiro/prompts/autofix-system.md"

// ❌ resolves to .kiro/agents/prompts/autofix-system.md
"prompt": "file://./prompts/autofix-system.md"

// ✅ resolves to .kiro/prompts/autofix-system.md
"prompt": "file://../prompts/autofix-system.md"
```

### 3. allowedTools wildcard

```json
// ❌ "*" not supported for allowedTools
"allowedTools": ["*"]

// ✅ list specific patterns
"allowedTools": ["read", "write", "shell", "@builtin"]
```

### 4. Missing system prompt env var documentation

If workflow passes env vars (KIRO_DIAGNOSTICS_FILE, KIRO_COMMIT_MSG_FILE, etc.), system prompt MUST document them. Agent has no other way to know they exist. Manifests as agent ignoring vars and hallucinating workflow.

### 5. Headless mode requirements

- `--no-interactive` required for CI (kiro-cli expects TTY without it, hangs)
- `KIRO_API_KEY` must be set (missing = cryptic auth error, not "missing key")
- `--trust-all-tools` required if agent writes files without confirmation

### 6. skill:// progressive loading

`skill://` loads only metadata (frontmatter) at startup. Full body loads on demand when agent determines relevance. Vague descriptions → skill never activates. Make descriptions specific and slightly pushy.

## Validation Workflow

After creating/editing any `.kiro/agents/*.json`:

1. Run validator:
   ```bash
   node .kiro/skills/harness-engineering/scripts/validate-kiro-agent.js .kiro/agents/autofix.json
   ```
2. Fix structural errors if validation fails.
3. **After validation passes**, manually verify Gotchas #1, #2, #4, #6 — validator cannot catch these.
4. Confirm system prompt file exists at resolved path.
5. Confirm all resource files/globs resolve to existing paths.
