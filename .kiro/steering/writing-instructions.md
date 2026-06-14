---
inclusion: fileMatch
fileMatchPattern: [".kiro/steering/**/*.md", "AGENTS.md", ".kiro/skills/**/*.md", ".kiro/agents/*.json", ".kiro/shared/*.md"]
---

# Writing Instruction Docs for LLMs

How to write steering, skills, AGENTS.md, and agent prompts so models actually follow them. Placement, budget, and de-duplication live in `steering-rules.md` — this file is craft, not bookkeeping.

## Front-load critical rules

Models attend most to the top and bottom of a doc, least to the middle (U-shaped attention). A rule buried mid-document is the rule that gets ignored.

- Put the most important constraint first or last, never in the middle.
- Order rules by importance, not by the date you added them.

## One constraint per rule

Models reliably satisfy about three concurrent constraints in a single instruction. A fourth or fifth drops compliance.

- Good: three separate bullets, each one directive.
- Bad: "Use a friendly tone, format as a numbered list, keep under 200 words, and ask a follow-up." (four constraints, one breath)

## Keep the rule count low

Compliance falls off a cliff as rules pile up, and models drop rules silently rather than flag them. Fewer enforced rules beat many ignored ones.

- Every rule must trace to a real failure. No speculative rules.
- Delete a rule when a stronger model makes it redundant.

## Phrase as positive directives

"Do Y" lands harder than "do not do X." Telling a model to ignore something can distract it.

- Prefer "Validate input at the service layer" over "Don't trust input."
- When you must forbid, also state the correct action.

## Be specific

Vague instructions regress twice as often across model and prompt changes. Name what, when, and how.

- Bad: "Handle errors gracefully."
- Good: "On a missing record, return 404 with a Spanish error message."

## Steer intent, don't script steps

State the goal and constraints; let the model choose the path. Rigid step-by-step scripts break when the codebase shifts.

- Exception: genuinely fragile operations (migrations, release steps) get exact commands. Match specificity to fragility.

## Show, don't just tell

One input/output example teaches style better than a paragraph of description. Add an example wherever output shape matters.

## Format for the reader

- This project runs on Claude: structure complex prompts with XML-style tags (`<rules>`, `<context>`), which Claude is tuned to parse.
- Use headings as section labels, numbered lists for sequence, bullets for parallel items, code blocks for literal commands and schemas.
- Use one term for one concept throughout. Do not alternate synonyms.
- Prefer plain Markdown and JSON over compact or novel formats; models parse familiar formats more reliably.

## No time-sensitive content

Dated instructions rot silently. Put superseded guidance in a collapsed `<details>` "old patterns" block, not inline.
