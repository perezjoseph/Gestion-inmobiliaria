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

### Headless Mode (CI Usage)

For non-interactive CI usage:

```bash
kiro-cli --agent autofix chat --no-interactive --trust-all-tools "prompt here"
```

- `--no-interactive`: No user input possible
- `--trust-all-tools`: Auto-approve all tool calls
- `--trust-tools=read,grep`: Auto-approve specific categories only
- `KIRO_API_KEY` env var required for authentication

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
