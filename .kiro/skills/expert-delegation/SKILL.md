---
name: expert-delegation
description: >
  Structured workflow for solving complex multi-part problems by delegating specialized tasks
  to expert sub-agents. Use when the user asks to fix multiple bugs, implement several features,
  refactor across files, or any task that benefits from breaking work into expert-level subtasks
  with research, implementation, review, and gap-fixing phases. Triggers on phrases like
  "fix all the problems", "use experts", "delegate to agents", "expert workflow",
  "research then implement", "sequential fixes with review", or when a task has 3+ distinct
  parts that each need focused attention. Also use when the user explicitly asks for a
  research-implement-review loop or wants sub-agents to handle specialized work.
---

# Expert Delegation Workflow

A structured approach to solving complex, multi-part problems by breaking them into
specialized tasks, delegating each to a focused expert sub-agent, then reviewing and
closing gaps. This workflow maximizes quality by ensuring each subtask gets dedicated
attention from a purpose-built expert.

## When to Use

- Fixing multiple bugs across different files or modules
- Implementing several related features that touch different parts of the codebase
- Any task with 3+ distinct subtasks that each require focused expertise
- When the user explicitly asks for expert delegation or research-then-implement patterns
- Cross-cutting changes that span backend, frontend, CI/CD, or infrastructure

## Core Principles

1. **Sequential execution** — fix one thing at a time, verify before moving on
2. **Research before action** — understand the code patterns before writing changes
3. **Specialized experts** — each sub-agent gets a narrow, well-defined task with full context
4. **Verify after every change** — run diagnostics and tests after each fix
5. **Independent review** — a separate reviewer expert catches what the implementer missed
6. **Close all gaps** — fix everything the reviewer finds before declaring done

## The Workflow

### Phase 1: Investigation

Before any implementation, understand the full scope of the problem.

1. **Gather context** — use `context-gatherer` sub-agent or read files directly to understand
   the codebase structure, patterns, and conventions
2. **Identify all problems** — list every issue with a clear description of what's wrong and why
3. **Prioritize** — order fixes by dependency (fix foundations before things that depend on them)
   and by impact (most dangerous bugs first)
4. **Present the plan** — show the user what you found and your fix order before starting

### Phase 2: Sequential Fix Cycle

For each fix, execute this three-step cycle:

#### Step 1: Research

Before creating the expert, gather the specific knowledge it needs:

- Read the exact code that needs changing (functions, classes, modules)
- Identify the patterns used in surrounding code (naming, error handling, logging style)
- Find all callers and callees of functions being modified
- Check for related tests, configs, or documentation that may need updates
- Look up library documentation if the fix involves external dependencies

The goal is to arm the expert with everything it needs to make surgical, correct changes
on the first attempt.

#### Step 2: Create and Execute Expert

Spawn a `general-task-execution` sub-agent with a highly specific prompt:

**Expert prompt structure:**
```
You are a [domain] expert. [Fix/Implement] [specific thing].

## Problem
[Clear description of what's wrong and why]

## What to implement
[Exact changes needed, with code snippets showing before/after]
[File paths, function names, line numbers when possible]

## IMPORTANT RULES:
- Use `strReplace` for surgical edits — don't rewrite whole files
- Match existing code style exactly
- Do NOT modify any unrelated code
- [Domain-specific constraints]
```

**Key principles for expert prompts:**
- Be extremely specific — tell the expert exactly what to change, not just what's wrong
- Include code snippets showing the current state and desired state
- Provide file paths and function names so the expert doesn't waste time searching
- List constraints explicitly (what NOT to do is as important as what to do)
- Include context files via `contextFiles` parameter so the expert can read them

#### Step 3: Verify

After each expert completes:

1. **Run diagnostics** — `getDiagnostics` on all modified files
2. **Run tests** — execute the project's test suite (or relevant subset)
3. **Spot-check** — grep for the changes to confirm they landed correctly
4. **Check for regressions** — ensure pre-existing tests still pass

If verification fails, either fix inline or create another expert to address the issue
before moving to the next fix.

### Phase 3: Review

After ALL fixes are applied and individually verified, create a **reviewer expert**:

```
You are a senior code reviewer. Perform a FINAL review of all changes made.

## Changes Made
[List every fix with a one-line summary]

## Verification Checklist
[For each fix, specific things to verify:]
1. Are all callers of modified functions updated?
2. Are there edge cases that could cause failures?
3. Are imports complete?
4. Is backward compatibility maintained?
5. [Domain-specific checks]

Search the codebase for all callers of modified functions using grep.
Report any gaps found.

Output format:
## Gap 1: [description]
File: [path]
Issue: [what's wrong]
Fix needed: [what to do]

If no gaps: "NO GAPS FOUND"
```

The reviewer should:
- Search for ALL callers of every modified function
- Check that signature changes are propagated everywhere
- Verify no imports are missing
- Look for edge cases in new code
- Confirm style consistency

### Phase 4: Fix Gaps

For each gap the reviewer found:
1. Apply the fix (directly or via another expert if complex)
2. Verify the fix with diagnostics and tests
3. If the gap fix was substantial, consider running the reviewer again

### Phase 5: Final Verification

1. Run the full test suite one last time
2. Run diagnostics on all modified files
3. Compile/build if applicable
4. Summarize all changes in a clear table for the user

## Expert Prompt Templates

### Bug Fix Expert
```
You are a [language/framework] expert. Fix [specific bug] in [file].

## Problem
[What's broken and why]

## Fix
[Exact changes with before/after code]

## Rules:
- Use strReplace for surgical edits
- Match existing code style
- Do NOT modify unrelated code
- Verify with [test command]
```

### Feature Implementation Expert
```
You are a [domain] expert. Implement [feature] in [file(s)].

## Requirements
[What the feature should do]

## Implementation Plan
1. [Step with file and function]
2. [Step with file and function]

## Rules:
- Follow existing patterns in [reference file]
- Do NOT add unnecessary abstractions
- Include error handling matching [existing pattern]
```

### Code Reviewer Expert
```
You are a senior code reviewer specializing in [domain].
Review the following changes for gaps, missed edge cases, and correctness.

## Changes Made
[Numbered list of all changes]

## Checklist
[Specific verification items per change]

Search the codebase and report PASS/FAIL for each item.
```

## Anti-Patterns to Avoid

- **Don't skip research** — experts without context produce generic, wrong code
- **Don't batch too many fixes into one expert** — each expert should have ONE clear task
- **Don't skip verification between fixes** — a broken fix compounds into later fixes
- **Don't skip the review phase** — implementers have blind spots; reviewers catch them
- **Don't ignore reviewer gaps** — every gap is a potential production bug
- **Don't rewrite files** — use surgical `strReplace` edits, not full file rewrites
