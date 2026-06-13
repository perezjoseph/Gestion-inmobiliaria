---
inclusion: fileMatch
fileMatchPattern: [".kiro/steering/**/*.md", "AGENTS.md", "~/.kiro/steering/**/*.md"]
---

# Rules for Editing Rules

## No Duplication Across Layers
- Before adding a rule, check if it already exists in `AGENTS.md` (always on) or global steering (always on).
- Rules in always-on files must not be repeated in fileMatch or conditional files.
- If a rule applies universally, it goes in `AGENTS.md`. If it's domain-specific, it goes in the matching steering file. Never both.

## Placement
- `AGENTS.md`: agent behavior (loop, verification, escalation, memory, guardrails). Project-agnostic.
- Global steering (`~/.kiro/steering/`): cross-project preferences (intent classification, conventions, agent system).
- Workspace steering (`.kiro/steering/`): project-specific rules scoped by fileMatch to the files they protect.

## Budget
- Always-on files (AGENTS.md + global + `inclusion: always`): keep under 1,500 words combined.
- Conditional files: no hard limit, but keep each file focused. One domain per file.

## Maintenance
- When moving a rule between files, delete it from the source. Never leave copies.
- When a rule becomes obsolete, remove it. Stale rules degrade agent behavior.
- After editing any steering file, verify no duplication was introduced across always-on files.
