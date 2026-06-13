---
name: harness-engineering
description: >
  Use when building or modifying kiro-cli custom agent JSON configs, designing CI
  autofix workflows, structuring verify-fix loops, configuring feedforward/feedback
  controls, or setting up postToolUse hooks for coding agents. Also use when asked
  about agent harness patterns, generator/evaluator separation, context resets vs
  compaction, or Ashby's Law applied to agent variety reduction — even if the user
  doesn't explicitly say "harness engineering."
license: MIT
compatibility: Requires Node.js 18+ for scripts/validate-kiro-agent.js
metadata:
  author: perezjoseph
  version: "1.3"
---

# Harness Engineering for Coding Agents

Patterns for designing agent harnesses — particularly kiro-cli custom agent configurations and CI autofix workflows. The primary value here is preventing subtle runtime failures that pass schema validation but break in production.

## Available Scripts

- **`scripts/validate-kiro-agent.js`** — Validates a kiro-cli agent JSON config against the schema. Run after every config edit. Note: the validator checks structure but cannot catch all runtime issues (see Gotchas).

## References (load on demand)

- Read `references/anthropic-harness-patterns.md` for the generator/evaluator pattern, context resets, sprint contracts, or evaluator tuning.
- Read `references/fowler-harness-engineering.md` for the feedforward/feedback framework, computational vs inferential controls, Ashby's Law, or the steering loop.
- Read `references/kiro-agent-schema.md` for the complete kiro-cli custom agent JSON schema, field details, hook triggers, or resource URI formats.

## Deliverables Checklist

When creating or modifying an agent harness, always produce a **complete package** — not isolated artifacts. An agent config without its system prompt is like a function signature without a body.

| Task | Required Deliverables |
|------|----------------------|
| Create agent config | Config JSON + system prompt file + validation result |
| Add postToolUse hook | Updated config + test that hook fires correctly |
| Design verify-fix loop | System prompt section + sensor commands + exit code documentation |
| Refactor workflow to use agent | Workflow step + agent config + system prompt (all three) |
| Add feedforward resources | Updated config resources array + the resource files themselves |

The reason for complete packages: context resets mean each agent invocation starts fresh. If the system prompt doesn't exist or doesn't document the env vars the workflow passes, the agent will hallucinate behavior. Every piece must be consistent with every other piece.

## Kiro Agent Config — Correct vs Incorrect

The template below shows the correct way to write a config. Pay attention to the annotations — each one addresses a common mistake that passes validation but fails at runtime.

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

### Field-by-field guidance

**`prompt`** — Must be a `file://` URI. The path resolves **relative to the config file's directory** (e.g., if config is at `.kiro/agents/autofix.json`, then `file://../prompts/autofix-system.md` resolves to `.kiro/prompts/autofix-system.md`). A common mistake is using workspace-root-relative paths like `file://.kiro/prompts/...` which would look for `.kiro/agents/.kiro/prompts/...`.

**`tools`** vs **`allowedTools`** — These are separate concerns. `tools` declares what tool categories the agent can see. `allowedTools` controls which tools are pre-approved (no confirmation needed). The `allowedTools` field does NOT support `"*"` — you must list specific patterns or use `"@builtin"`.

**`resources`** — Use `file://` for documents and `skill://` for skills. Paths resolve relative to the config file location (same as `prompt`). Use glob patterns (`**/*.md`) for maintainability — listing files individually is brittle and breaks when files are added/removed.

**`hooks.postToolUse[].matcher`** — The field name is `matcher` (not `match`, not `trigger`, not `on`). The value must be an **internal tool name**: `fs_write`, `fs_read`, `execute_bash`, `str_replace`. NOT the simplified category names from the `tools` field (`write`, `read`, `shell`). This is the #1 runtime failure that passes schema validation.

**`hooks` trigger keys** — Valid triggers: `agentSpawn`, `userPromptSubmit`, `preToolUse`, `postToolUse`, `stop`. Not `postToolWrite`, not `afterWrite`, not `onFileChange`.

## Gotchas (Runtime Failures That Pass Validation)

These are ordered by frequency of occurrence. The schema validator catches structural issues but cannot verify runtime correctness.

### 1. Hook matcher field name and value (most common)

```json
// ❌ WRONG — passes validation, fails at runtime
{ "match": "write", "command": "cargo fmt" }
{ "on": "fs_write", "command": "cargo fmt" }
{ "matcher": "write", "command": "cargo fmt" }

// ✅ CORRECT
{ "matcher": "fs_write", "command": "cargo fmt --all" }
```

The validator only checks that a `command` field exists in each hook object. It does not validate the matcher field name or value. At runtime, kiro-cli looks specifically for `matcher` with an internal tool name.

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

// ❌ WRONG — resolves to .kiro/agents/.kiro/prompts/autofix-system.md
"prompt": "file://.kiro/prompts/autofix-system.md"

// ❌ WRONG — resolves to .kiro/agents/prompts/autofix-system.md
"prompt": "file://./prompts/autofix-system.md"

// ✅ CORRECT — resolves to .kiro/prompts/autofix-system.md
"prompt": "file://../prompts/autofix-system.md"
```

The validator checks if the resolved file exists on disk. If you create the file at the wrong path to make validation pass, the agent will load the wrong file at runtime.

### 3. allowedTools wildcard

```json
// ❌ WRONG — "*" is not supported for allowedTools
"allowedTools": ["*"]

// ✅ CORRECT — list specific patterns
"allowedTools": ["read", "write", "shell", "@builtin"]
```

### 4. Missing system prompt env var documentation

If your workflow passes env vars (KIRO_DIAGNOSTICS_FILE, KIRO_COMMIT_MSG_FILE, etc.) to the agent, the system prompt MUST document them. The agent has no other way to know they exist. This isn't caught by any validator — it manifests as the agent ignoring the env vars and hallucinating its own workflow.

### 5. Headless mode requirements

- `--no-interactive` flag is required for CI. Without it, kiro-cli expects a TTY and hangs.
- `KIRO_API_KEY` env var must be set. Missing it produces a cryptic auth error, not "missing key".
- `--trust-all-tools` is required if the agent needs to write files without confirmation prompts.

### 6. skill:// progressive loading

`skill://` resources only load metadata (name + description from YAML frontmatter) at startup. The full SKILL.md body loads on demand when the agent determines relevance. If the description is vague, the skill never activates. Make descriptions specific and slightly pushy.

## Core Patterns (Quick Reference)

These patterns inform design decisions. For deep dives, read the reference files.

| Pattern | One-liner | When it matters |
|---------|-----------|-----------------|
| Generator/Evaluator | Agent fixes, sensors verify. Never self-evaluate. | Always — every harness needs external sensors |
| Feedforward + Feedback | Steering before, sensors after. Both required. | Always — one without the other fails |
| Keep Quality Left | Cheapest sensors first: format → lint → compile → test | Verify-fix loops, CI pipelines |
| Context Resets | Fresh invocation per artifact, structured handoffs via env vars | Multi-artifact queues |
| Constrain Variety | Pin to topologies with pre-defined sensor suites | Polyglot workspaces |
| Steering Loop | Recurring issue → improve harness, not individual fixes | Post-incident improvement |

## Sensor Command Reference

When defining feedback sensors for verify-fix loops, use per-package targeting with `--locked` for reproducible CI builds. Workspace-wide commands mask which package has the issue, making feedback less actionable for the agent.

```bash
# ✅ CORRECT — per-package, --locked, actionable error locations
cargo fmt --all -- --check
cargo clippy --locked -p realestate-backend -- -D warnings
cargo clippy --locked --target wasm32-unknown-unknown -p realestate-frontend -- -D warnings
cargo test --locked -p realestate-backend --no-fail-fast

# ❌ WRONG — workspace-wide hides which package failed
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# TypeScript sensors — always use strict mode flags
cd baileys-service && npm run build          # tsc type-check
cd baileys-service && npx eslint . --max-warnings 0   # lint (--max-warnings 0 = fail on any warning)
cd baileys-service && npm test               # vitest
```

Why `--locked`: ensures the exact dependency versions from `Cargo.lock` are used. Without it, cargo may update dependencies during the build, introducing non-determinism in CI.

Why per-package (`-p`): when clippy reports errors, the agent sees exactly which crate is broken. Workspace-wide output mixes errors from multiple crates, making it harder for the agent to prioritize and fix.

Why `--max-warnings 0` on eslint: without this flag, eslint exits 0 even when warnings exist. The sensor would report "pass" while lint issues remain — a false negative that defeats the purpose of the feedback loop.

## Validation Workflow

After creating or editing any `.kiro/agents/*.json` file:

1. Run the validator:
   ```bash
   node .kiro/skills/harness-engineering/scripts/validate-kiro-agent.js .kiro/agents/autofix.json
   ```
2. If validation fails, fix the structural errors.
3. **After validation passes**, manually verify the Gotchas above — the validator cannot catch runtime issues #1, #2, #4, or #6.
4. Confirm the system prompt file exists at the resolved path.
5. Confirm all resource files/globs resolve to existing paths.

## Anti-Patterns

1. **Inline prompts in shell scripts** — Use `file://` URIs. Shell interpolation of untrusted content causes injection.
2. **Self-evaluation without sensors** — The agent will say "looks good" even when broken. Always run computational checks.
3. **Suppressing warnings** (`#[allow(...)]`, `// @ts-ignore`) — Fix the root cause.
4. **Incremental patching on repeated failures** — Same error after 2 attempts means the approach is wrong.
5. **Running all sensors regardless of changes** — Activate selectively by file type.
6. **Producing isolated artifacts** — A workflow step without its companion agent config and system prompt is incomplete. Always deliver the full package.
7. **Listing resource files individually** — Use glob patterns (`**/*.md`). Individual listings break when files are added/removed and create maintenance burden.
