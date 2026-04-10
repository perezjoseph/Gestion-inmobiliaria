---
inclusion: always
---

# Lessons Learned Rules

## When to Document
- Always append an entry to `lessons-learned.md` at the project root after discovering:
  - Non-obvious solutions or workarounds
  - Gotchas, pitfalls, or unexpected behaviors in dependencies
  - Important configuration discoveries
  - Performance insights that affect architectural decisions
  - Debugging sessions that took significant effort to resolve

## Format
- Always use this format: `### YYYY-MM-DD — <Topic Title>` followed by a brief description of the lesson, what was tried, what worked, and why.

## Rules
- Always keep entries concise and actionable.
- Always limit to one topic per entry.
- Never duplicate existing entries — always search first.
- Always include version numbers of relevant crates/tools when applicable.
