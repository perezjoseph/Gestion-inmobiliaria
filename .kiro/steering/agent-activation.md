---
inclusion: manual
---

# Agent & Skill Activation Patterns

Patterns for improving how the orchestrator selects specialist agents and how agents activate their loaded skills in Kiro's `.kiro/agents/` system.

## When to Use

- Agent routing is inconsistent (wrong specialist selected)
- Skills are loaded but not reflected in responses
- Responses are too generic despite specialist knowledge being available
- Adding a new specialist agent and want it to trigger correctly
- Tuning an existing agent's description for better delegation

## Kiro Agent File Format

Agents live in `.kiro/agents/*.md` with YAML frontmatter:

```markdown
---
name: agent-name
description: "Concise routing description — the orchestrator reads this to decide delegation. Include trigger phrases, domain boundaries, and negative boundaries."
tools: ["read", "write", "shell"]
---

System prompt body goes here. This is what the agent sees when activated.
```

### Frontmatter Fields

| Field | Purpose |
|-------|---------|
| `name` | Identifier used in `invokeSubAgent` calls |
| `description` | **The routing signal** — orchestrator pattern-matches on this to delegate |
| `tools` | Tool categories the agent can use: `read`, `write`, `shell`, `web` |

### The Description Is Everything for Routing

The orchestrator reads agent descriptions and decides which specialist handles a prompt. The description is not documentation — it's a **routing trigger**.

## Description Engineering Rules

### Rule 1: Lead with unique domain identifiers

```
BAD:  "Implements code and handles tasks related to the frontend"
GOOD: "Any Leptos component, UI layout, Tailwind styling, or visual design task — even though Leptos is Rust. Handles responsive layout, accessibility, and component architecture."
```

### Rule 2: Include negative boundaries

```yaml
description: "Rust backend implementation: Actix-web handlers, SeaORM queries, AppError. Does NOT handle UI components, visual design, or Tailwind — those go to frontend-designer."
```

### Rule 3: List explicit trigger phrases

The orchestrator pattern-matches on keywords. Include the exact phrases users type:

```yaml
description: "Code quality gate. Delegate here when asked to: review code, verify implementation, check for issues, run tests and report, give a PASS/FAIL verdict, or classify issues by P0-P3 severity."
```

### Rule 4: Specify output format expectations

```yaml
description: "...Outputs a structured plan to .kiro/plans/ with affected files table, numbered steps, and verification commands."
```

### Rule 5: Claim exclusive ownership of a domain

```yaml
description: "This is the ONLY agent for Android and Kotlin work. Delegate here for ANY Android implementation..."
```

## Routing Failure Patterns

| Pattern | Cause | Fix |
|---------|-------|-----|
| Review prompts handled inline | Orchestrator thinks it can review itself | Add "Do NOT review code yourself — delegate to this agent" in description |
| Leptos UI → rust-coder | Both are Rust | Frontend-designer description must say "even though Leptos is Rust" |
| "Plan the implementation" → handled inline | Ambiguous | Add ".kiro/plans/" and "affected files table" as code-planner triggers |
| Kotlin Compose UI → frontend-designer | Both do UI | Kotlin-coder must lead with "Android", "Kotlin", "Jetpack" and say "NOT for Leptos/web UI" |
| Simple file edit → specialist | Overkill delegation | Orchestrator should handle trivial edits directly |

## System Prompt Body Patterns

Structure agent bodies for consistent behavior:

```markdown
---
name: my-agent
description: "..."
tools: ["read", "write", "shell"]
---

You are the {role}. You {primary action}.

## Output Expectations
What the response must look like.

## Constraints
Hard rules the agent must never violate.

## Process
Numbered steps the agent follows every time.

## Response Style
Tone and format guidance.
```

### Body Best Practices

- Start with a one-line identity statement: "You are the X. You do Y."
- Define output format explicitly — agents produce inconsistent output without it
- List constraints as hard "never" rules, not suggestions
- Define a numbered process so the agent doesn't skip steps
- Reference project patterns: "Use the project's `AppError` pattern", "Match existing code in affected files"

## Skill Activation Patterns

Skills live in `.kiro/skills/{name}/SKILL.md` and activate based on their frontmatter description.

### How Skills Load

1. **Metadata phase**: Skill name + description visible at startup
2. **Activation phase**: Full skill body loaded when prompt matches skill description keywords
3. **Application phase**: Agent uses skill patterns in response

### Skill Description as Trigger Condition

```yaml
---
name: verify-fix-loop
description: "Use when implementing code changes that need verification. Provides the fmt→clippy→test loop pattern, error interpretation, and fix strategies. Activates on: implement, fix, refactor, build error, test failure."
---
```

Key elements:
- **"Use when..."** — tells the system the activation condition
- **Action verbs** — "implementing", "fixing", "refactoring"
- **Signal words** — "build error", "test failure", "clippy warning"

### Why Skills Don't Activate

| Cause | Evidence | Fix |
|-------|----------|-----|
| Description doesn't match user phrasing | Skill says "deployment" but user says "push to prod" | Add synonyms to description |
| Too many skills compete | Multiple skill descriptions match the prompt | Narrow descriptions with "Use when... NOT when..." |
| Agent prompt too minimal | 1-line system prompt doesn't reference skills | Use full multi-paragraph prompts that reference skill patterns |
| Description too vague | "Helps with code" matches everything | Be specific: "Use when running cargo clippy fails and you need fix strategies" |

## Preventing Domain Contamination

- Each agent owns a clear domain with negative boundaries
- Skills are scoped to relevant agents — don't dump all skills globally
- Agent descriptions explicitly exclude neighboring domains
- When domains overlap (e.g., Leptos is both Rust and UI), one agent claims it explicitly

## Quick Checklist

When an agent isn't routing correctly:
- [ ] Does its `description` frontmatter contain the exact words users type?
- [ ] Is there overlap with another agent's domain? Add negative boundaries.
- [ ] Does the description start with unique identifiers (not generic "handles tasks")?
- [ ] Is there an explicit "ONLY agent for X" or "NOT for Y" claim?

When skills aren't activating:
- [ ] Does the skill description use "Use when..." + action verbs + signal words?
- [ ] Are the trigger keywords things users actually say?
- [ ] Is the skill description narrow enough to avoid false matches?

When adding a new agent:
- [ ] Create `.kiro/agents/{name}.md` with proper frontmatter
- [ ] Set `tools` to minimum needed (don't give `write` to read-only agents)
- [ ] Add negative boundaries excluding neighboring agent domains
- [ ] Test by phrasing the delegation request how a user would phrase it
