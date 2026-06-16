# Kiro CLI Custom Agent Configuration Schema

Distilled from the official Kiro CLI documentation (kiro.dev/docs/cli/custom-agents/configuration-reference).

## File Locations

- **Local agents (project-specific):** `.kiro/agents/<name>.json`
- **Global agents (user-wide):** `~/.kiro/agents/<name>.json`
- Local takes precedence over global when names collide.

## Complete Schema

```json
{
  "name": "string (optional, derived from filename)",
  "description": "string (human-readable purpose)",
  "prompt": "string | file:// URI",
  "model": "string (model ID, e.g. claude-sonnet-4)",
  "tools": ["array of tool references"],
  "allowedTools": ["array of patterns for auto-approved tools"],
  "toolAliases": { "original": "alias" },
  "toolsSettings": { "tool_name": { "setting": "value" } },
  "resources": ["array of file:// or skill:// URIs, or knowledgeBase objects"],
  "hooks": {
    "agentSpawn": [{ "command": "string" }],
    "userPromptSubmit": [{ "command": "string" }],
    "preToolUse": [{ "matcher": "tool_name", "command": "string" }],
    "postToolUse": [{ "matcher": "tool_name", "command": "string" }],
    "stop": [{ "command": "string" }]
  },
  "mcpServers": {
    "server_name": {
      "command": "string (required)",
      "args": ["array"],
      "env": { "KEY": "VALUE" },
      "timeout": 120000
    }
  },
  "includeMcpJson": true,
  "keyboardShortcut": "ctrl+a",
  "welcomeMessage": "string"
}
```

## Field Details

### prompt

High-level context for the agent. Supports inline text or file URI:

- Inline: `"You are an expert Rust developer"`
- File: `"file://./prompts/system.md"` (resolved relative to agent config file)

### tools

What tools the agent can potentially use:

- `"*"` — All available tools (built-in + MCP)
- `"@builtin"` — All built-in tools
- `"@server_name"` — All tools from a specific MCP server
- `"@server_name/tool_name"` — Specific MCP tool
- `"read"`, `"write"`, `"shell"` — Specific built-in tools

### allowedTools

Tools that run without prompting. Supports glob patterns:

- Exact: `"read"`, `"@git/git_status"`
- Server-level: `"@fetch"` (all tools from fetch server)
- Wildcards: `"@server/read_*"`, `"@git-*/*"`
- `*` matches any sequence, `?` matches one character

Does NOT support `"*"` wildcard for allowing all tools.

### resources

Files and skills available to the agent:

- `"file://README.md"` — Loaded into context at startup
- `"file://.kiro/steering/**/*.md"` — Glob patterns supported
- `"skill://.kiro/skills/**/SKILL.md"` — Progressive loading (metadata at startup, full on demand)

Knowledge base resources (object form):

```json
{
  "type": "knowledgeBase",
  "source": "file://./docs",
  "name": "ProjectDocs",
  "description": "Project documentation",
  "indexType": "best",
  "autoUpdate": true
}
```

### hooks

Commands run at specific trigger points:

| Trigger            | When it fires                    | Can block? |
| ------------------ | -------------------------------- | ---------- |
| agentSpawn         | Agent is initialized             | No         |
| userPromptSubmit   | User submits a message           | No         |
| preToolUse         | Before a tool executes           | Yes        |
| postToolUse        | After a tool executes            | No         |
| stop               | Agent finishes responding        | No         |

Each hook has:
- `command` (required): Shell command to execute
- `matcher` (optional, for preToolUse/postToolUse): Pattern matching internal tool names (`fs_read`, `fs_write`, `execute_bash`, `use_aws`)

### mcpServers

MCP servers the agent can access:

- `command` (required): Command to start the server
- `args` (optional): Arguments array
- `env` (optional): Environment variables
- `timeout` (optional): Request timeout in ms (default 120000)
- `oauth` (optional, for HTTP-based servers):
  - `clientId`: Pre-registered OAuth client ID (for services like Slack/GitHub that don't support DCR)
  - `redirectUri`: Custom redirect URI (e.g., "127.0.0.1:7778")
  - `oauthScopes`: Array of scopes to request

HTTP MCP server example:
```json
{
  "mcpServers": {
    "github": {
      "type": "http",
      "url": "https://api.github.com/mcp",
      "oauth": {
        "clientId": "your-app-client-id",
        "redirectUri": "127.0.0.1:8080",
        "oauthScopes": ["repo", "user"]
      }
    }
  }
}
```

### toolsSettings (Built-in Tool Options)

```json
{
  "toolsSettings": {
    "write": {
      "allowedPaths": ["src/**", "tests/**", "Cargo.toml"]
    },
    "shell": {
      "allowedCommands": ["git status", "git fetch"],
      "deniedCommands": ["git commit .*", "git push .*"],
      "autoAllowReadonly": true
    }
  }
}
```

### Security: Write Tool Permissions

When write tools are enabled (`write`, `shell`, MCP tools with write access):
- Agent can modify ALL files under `~/.kiro` (skills, steering, MCP configs, other agents)
- No isolation between skills — any skill's context readable/modifiable
- Skills can't execute code alone, but if `shell` is allowed, agent CAN execute commands from any loaded skill

Mitigations:
- Only enable specific write tools needed
- Use `toolsSettings.write.allowedPaths` to restrict paths
- Use `preToolUse` hooks to audit/block sensitive operations
- Review skills before installing from untrusted sources

### Headless Mode (CI Usage)

For non-interactive CI usage:

```bash
kiro-cli --agent autofix chat --no-interactive --trust-all-tools "prompt here"
```

- `--no-interactive`: No user input possible
- `--trust-all-tools`: Auto-approve all tool calls
- `--trust-tools=read,grep`: Auto-approve specific categories only
- `KIRO_API_KEY` env var required for authentication

### Creating Agents

From within a Kiro CLI session:
```
/agent create backend-specialist -D "Backend coding specialist" -m code-analysis
```

Options:
- `--directory workspace|global|./path` — where to save
- `--from existing-agent` — template to base on (implies --manual)
- `--description "..."` — AI-assisted mode description
- `--mcp-server server-name` — include MCP server (repeatable)
- `--manual` — editor-based instead of AI generation

From terminal:
```bash
kiro-cli agent create backend-specialist
```

## Skill File Format

Skills use YAML frontmatter + markdown body:

```markdown
---
name: my-skill-name
description: Clear description of what this does and when to use it
---

# Instructions

The markdown body contains full instructions loaded on demand.
```

Only `name` and `description` are required in frontmatter. The description determines when the skill activates — include trigger keywords and contexts.

## Multi-Agent Workflows in Kiro

Kiro-cli has native subagent orchestration via the `subagent` tool. An orchestrator agent delegates to subagents that run in isolated context and report back. This is real runtime orchestration — not just single-shot delegation.

### Native Capabilities (runtime)

| Capability | Detail |
|-----------|--------|
| Delegation | Orchestrator must have `subagent` in its `tools` array (or `@builtin`). Spawn by name: "Use the backend agent to..." |
| Parallel execution | Up to **4 subagents at once** |
| Task graph (DAG) | Main agent plans full DAG upfront — independent tasks run parallel, dependents wait. Immutable once execution starts. |
| Review loops | A stage loops back to an earlier stage on a trigger. `target` (stage to re-run) + `trigger` (text like `NEEDS_CHANGES`, min 4 chars) + `max_iterations` (1–10). No self-loops, no mutual loops. This is generator/evaluator, native. |
| Permission scoping | `toolsSettings.subagent.availableAgents` (glob-restricted spawnable list) + `trustedAgents` (run without prompts) |
| Result aggregation | Subagent calls built-in `summary` tool to return findings to parent |
| Monitor | Ctrl+G — live per-subagent status, no main-chat interruption |
| Traceability | Subagent sessions record the spawning session's ID |

### Orchestrator Config Example

```json
{
  "name": "orchestrator",
  "description": "Coordinates specialized subagents",
  "tools": ["fs_read", "subagent"],
  "toolsSettings": {
    "subagent": {
      "availableAgents": ["player", "coach", "tester", "docs-*"],
      "trustedAgents": ["player", "coach"]
    }
  }
}
```

Restrict a subagent's tools in ITS OWN config (`allowedTools`), not the parent's — each subagent runs with its own configuration.

### Worked Example: Player/Coach (community)

Generator/evaluator implemented with three custom agents (Ricardo Sueiras, beachgeek.co.uk):

```
.kiro/agents/{orch,player,coach}.json
.kiro/shared/{ORCH,PLAYER,COACH}.md   # per-role system prompts
```

- `orch` spawns `player` (does the work) → `player` invokes `coach` (reviews) → feedback loops back
- Eval criteria given ONLY to coach (scoped context)
- Shared learnings file across subagents (filesystem handoff)
- Different models for player vs coach
- Maps cleanly to native review loops: coach emits `NEEDS_CHANGES`, player re-runs

Observed sharp edges (real failure history):
- **Cost:** one player/coach cycle ≈ 4.3 credits vs <1 for one-shot. Multi-agent isn't free — justify it.
- **Hanging:** subagents that start a server/service stall waiting; leftover processes cause port conflicts. Mitigate with steering guidance.
- **One-shot bypass:** orchestrator sometimes ignores the pipeline and does it all in one shot. Enforce decomposition via prompt + scope.
- **Files in wrong places:** workflow not always followed; pin output paths in steering.

### Still Filesystem/Workflow Layer (not subagent runtime)

| Need | Layer |
|------|-------|
| Memory across separate kiro-cli runs (autofix queue `KIRO_LEARNINGS_FILE`) | Filesystem, between sessions |
| CI sequencing, gating, retries | GitHub Actions (`.github/workflows/kiro-autofix-*.yml`) |
| Conflict-free parallel work on shared code | Git branch/worktree per task |

### Genuinely Not Available

- Peer-to-peer subagent messaging — subagents report to parent via `summary`, they don't message each other
- Mid-execution task-graph mutation — the DAG is planned upfront and frozen
- Unbounded recursive spawning — keep delegation shallow or context cost explodes

Behavior-derivation still applies: name what the coordination delivers (conflict-free parallelism, scoped review, work distribution) and use the layer that supports it — subagent runtime for in-session orchestration, workflow/filesystem for cross-session.

