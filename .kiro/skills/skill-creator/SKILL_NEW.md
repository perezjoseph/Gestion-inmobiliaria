---
name: skill-creator
description: Create new skills, modify and improve existing skills, and measure skill performance. Use when users want to create a skill from scratch, edit, or optimize an existing skill, run evals to test a skill, benchmark skill performance with variance analysis, or optimize a skill's description for better triggering accuracy.
---

# Skill Creator (Kiro IDE Edition)

A skill for creating new skills and iteratively improving them inside Kiro IDE.

At a high level, the process of creating a skill goes like this:

- Decide what you want the skill to do and roughly how it should do it
- Write a draft of the skill
- Create a few test prompts and run the skill on them via sub-agents
- Help the user evaluate the results both qualitatively and quantitatively
  - While the runs happen in the background, draft some quantitative evals if there aren't any. Then explain them to the user
  - Use the `eval-viewer/generate_review.py` script to show the user the results
- Rewrite the skill based on feedback from the user's evaluation of the results
- Repeat until you're satisfied
- Expand the test set and try again at larger scale

Your job when using this skill is to figure out where the user is in this process and then jump in and help them progress through these stages.

## Kiro IDE Environment

This skill runs inside Kiro IDE. Key differences from Claude Code:

- **Skills location**: `.kiro/skills/<skill-name>/SKILL.md`
- **Triggering**: Kiro activates skills via `discloseContext` based on the skill's `name` and `description` fields in YAML frontmatter
- **Sub-agents**: Use `invoke_sub_agent` with named agents (`general-task-execution`, `context-gatherer`, etc.)
- **Platform**: Windows (PowerShell/cmd). Use `start` to open files in browser. Use `;` as command separator.
- **No `claude` CLI for trigger evals**: The `run_eval.py` and `run_loop.py` scripts depend on `claude -p`. For Kiro, trigger evaluation is done manually (see "Description Optimization" section).
- **Viewer**: `generate_review.py` works — use `--static <output_path>` to write a standalone HTML file, then `start <path>` to open it.

---

## Creating a skill

### Capture Intent

Start by understanding the user's intent. The current conversation might already contain a workflow the user wants to capture. If so, extract answers from the conversation history first.

1. What should this skill enable the agent to do?
2. When should this skill trigger? (what user phrases/contexts)
3. What's the expected output format?
4. Should we set up test cases to verify the skill works?

### Interview and Research

Proactively ask questions about edge cases, input/output formats, example files, success criteria, and dependencies. Wait to write test prompts until you've got this part ironed out.

### Write the SKILL.md

Based on the user interview, fill in these components:

- **name**: Skill identifier (kebab-case, max 64 chars)
- **description**: When to trigger, what it does. This is the primary triggering mechanism — include both what the skill does AND specific contexts for when to use it. Make the description a little "pushy" to combat undertriggering.
- **the rest of the skill**

### Skill Writing Guide

#### Anatomy of a Skill

```text
skill-name/
├── SKILL.md (required)
│   ├── YAML frontmatter (name, description required)
│   └── Markdown instructions
└── Bundled Resources (optional)
    ├── scripts/    - Executable code for deterministic/repetitive tasks
    ├── references/ - Docs loaded into context as needed
    └── assets/     - Files used in output (templates, icons, fonts)
```

#### Progressive Disclosure

Skills use a three-level loading system:

1. **Metadata** (name + description) - Always in context (~100 words)
2. **SKILL.md body** - In context whenever skill triggers (<500 lines ideal)
3. **Bundled resources** - As needed (unlimited, scripts can execute without loading)

**Key patterns:**

- Keep SKILL.md under 500 lines; if approaching this limit, add hierarchy with clear pointers
- Reference files clearly from SKILL.md with guidance on when to read them
- For large reference files (>300 lines), include a table of contents

**Domain organization**: When a skill supports multiple domains/frameworks, organize by variant:

```text
cloud-deploy/
├── SKILL.md (workflow + selection)
└── references/
    ├── aws.md
    ├── gcp.md
    └── azure.md
```

The agent reads only the relevant reference file.

#### Writing Patterns

Prefer using the imperative form in instructions.

**Defining output formats:**

```markdown
## Report structure
ALWAYS use this exact template:
# [Title]
## Executive summary
## Key findings
## Recommendations
```

**Examples pattern:**

```markdown
## Commit message format
**Example 1:**
Input: Added user authentication with JWT tokens
Output: feat(auth): implement JWT-based authentication
```

### Writing Style

Try to explain to the model why things are important in lieu of heavy-handed musty MUSTs. Use theory of mind and try to make the skill general and not super-narrow to specific examples.

### Test Cases

After writing the skill draft, come up with 2-3 realistic test prompts — the kind of thing a real user would actually say. Share them with the user and ask for confirmation.

Save test cases to `evals/evals.json`. Don't write assertions yet — just the prompts.

```json
{
  "skill_name": "example-skill",
  "evals": [
    {
      "id": 1,
      "prompt": "User's task prompt",
      "expected_output": "Description of expected result",
      "files": []
    }
  ]
}
```

See `references/schemas.md` for the full schema.

## Running and evaluating test cases

This section is one continuous sequence — don't stop partway through.

Put results in `<skill-name>-workspace/` as a sibling to the skill directory. Within the workspace, organize results by iteration (`iteration-1/`, `iteration-2/`, etc.) and within that, each test case gets a directory (`eval-0/`, `eval-1/`, etc.).

### Step 1: Spawn all runs (with-skill AND baseline) in the same turn

For each test case, spawn two sub-agents in the same turn — one with the skill, one without. Launch everything at once.

Use `invoke_sub_agent` with `general-task-execution` agent for each run.

**With-skill run prompt:**

```text
Execute this task. First, read the SKILL.md at <path-to-skill>/SKILL.md and follow its instructions.

Task: <eval prompt>
Input files: <eval files if any, or "none">

Save all outputs to: <workspace>/iteration-<N>/eval-<ID>/with_skill/outputs/
```

**Baseline run** (same task, but no skill):

- **Creating a new skill**: no skill at all. Same prompt, no skill path, save to `without_skill/outputs/`.
- **Improving an existing skill**: the old version. Snapshot the skill first, then point the baseline sub-agent at the snapshot. Save to `old_skill/outputs/`.

Write an `eval_metadata.json` for each test case:

```json
{
  "eval_id": 0,
  "eval_name": "descriptive-name-here",
  "prompt": "The user's task prompt",
  "assertions": []
}
```

### Step 2: While runs are in progress, draft assertions

Draft quantitative assertions for each test case and explain them to the user. Good assertions are objectively verifiable and have descriptive names.

Update the `eval_metadata.json` files and `evals/evals.json` with the assertions once drafted.

### Step 3: As runs complete, capture timing data

Save timing data to `timing.json` in the run directory:

```json
{
  "total_duration_seconds": 23.3
}
```

### Step 4: Grade, aggregate, and launch the viewer

Once all runs are done:

1. **Grade each run** — spawn a grader sub-agent (or grade inline) that reads `agents/grader.md` and evaluates each assertion against the outputs. Save results to `grading.json`. The expectations array must use the fields `text`, `passed`, and `evidence`.

2. **Aggregate into benchmark** — run the aggregation script from the skill-creator directory:

   ```powershell
   python -m scripts.aggregate_benchmark <workspace>/iteration-N --skill-name <name>
   ```

   This produces `benchmark.json` and `benchmark.md`. See `references/schemas.md` for the exact schema.

3. **Do an analyst pass** — read the benchmark data and surface patterns. See `agents/analyzer.md` for what to look for.

4. **Launch the viewer:**

   ```powershell
   python <skill-creator-path>/eval-viewer/generate_review.py <workspace>/iteration-N --skill-name "my-skill" --benchmark <workspace>/iteration-N/benchmark.json --static <workspace>/iteration-N/review.html
   start <workspace>/iteration-N/review.html
   ```

   For iteration 2+, also pass `--previous-workspace <workspace>/iteration-<N-1>`.

5. **Tell the user**: "I've opened the results in your browser. There are two tabs — 'Outputs' lets you click through each test case and leave feedback, 'Benchmark' shows the quantitative comparison. When you're done, come back here and let me know."

### What the user sees in the viewer

The "Outputs" tab shows one test case at a time:

- **Prompt**: the task that was given
- **Output**: the files the skill produced, rendered inline where possible
- **Previous Output** (iteration 2+): collapsed section showing last iteration's output
- **Formal Grades** (if grading was run): collapsed section showing assertion pass/fail
- **Feedback**: a textbox that auto-saves as they type

The "Benchmark" tab shows the stats summary: pass rates, timing, and token usage for each configuration.

### Step 5: Read the feedback

When the user tells you they're done, read `feedback.json`:

```json
{
  "reviews": [
    {"run_id": "eval-0-with_skill", "feedback": "the chart is missing axis labels", "timestamp": "..."},
    {"run_id": "eval-1-with_skill", "feedback": "", "timestamp": "..."}
  ],
  "status": "complete"
}
```

Empty feedback means the user thought it was fine. Focus improvements on test cases with specific complaints.

---

## Improving the skill

### How to think about improvements

1. **Generalize from the feedback.** We're trying to create skills that work across many prompts. Rather than put in fiddly overfitty changes, try branching out and using different metaphors or patterns.

2. **Keep the prompt lean.** Remove things that aren't pulling their weight. Read the transcripts, not just the final outputs.

3. **Explain the why.** Try hard to explain the **why** behind everything you're asking the model to do. If you find yourself writing ALWAYS or NEVER in all caps, reframe and explain the reasoning instead.

4. **Look for repeated work across test cases.** If all test cases resulted in the sub-agent writing a similar script, that's a signal the skill should bundle that script in `scripts/`.

### The iteration loop

After improving the skill:

1. Apply your improvements to the skill
2. Rerun all test cases into a new `iteration-<N+1>/` directory, including baseline runs
3. Launch the reviewer with `--previous-workspace` pointing at the previous iteration
4. Wait for the user to review and tell you they're done
5. Read the new feedback, improve again, repeat

Keep going until:

- The user says they're happy
- The feedback is all empty (everything looks good)
- You're not making meaningful progress

---

## Advanced: Blind comparison

For situations where you want a more rigorous comparison between two versions of a skill, there's a blind comparison system. Read `agents/comparator.md` and `agents/analyzer.md` for the details.

This is optional and most users won't need it.

---

## Description Optimization (Kiro-adapted)

The description field in SKILL.md frontmatter is the primary mechanism that determines whether Kiro activates a skill.

### How skill triggering works in Kiro

Skills appear in Kiro's "Available Items" list with their `name` and `description`. When the user sends a message, Kiro decides whether to call `discloseContext` to activate a skill based on keyword/intent matching against the description.

Key insight: Kiro's triggering is keyword-and-intent based. The description should contain the key terms a user would naturally use when they need this skill.

### Step 1: Generate trigger eval queries

Create 20 eval queries — a mix of should-trigger and should-not-trigger. Save as JSON:

```json
[
  {"query": "the user prompt", "should_trigger": true},
  {"query": "another prompt", "should_trigger": false}
]
```

For the **should-trigger** queries (8-10): different phrasings of the same intent, some formal, some casual.

For the **should-not-trigger** queries (8-10): near-misses that share keywords but actually need something different.

### Step 2: Review with user

Present the eval set to the user for review. Show the queries in a readable format and ask them to confirm, edit, or add entries.

### Step 3: Manual triggering assessment

Since Kiro doesn't have a `claude -p` equivalent for automated trigger testing, evaluate triggering manually:

1. For each query, assess whether the skill's current description contains enough keyword overlap and intent signal that Kiro would activate it
2. Check for false-positive risk: would the description cause activation on the should-not-trigger queries?
3. Look for gaps: are there should-trigger queries whose key terms don't appear anywhere in the description?

Based on this analysis, propose description improvements that:

- Add missing trigger keywords
- Remove or qualify terms that cause false positives
- Keep the description under 1024 characters
- Maintain clarity about what the skill actually does

### Step 4: Apply the result

Update the skill's SKILL.md frontmatter description. Show the user before/after and explain what changed and why.

---

## Reference files

The agents/ directory contains instructions for specialized sub-agents:

- `agents/grader.md` — How to evaluate assertions against outputs
- `agents/comparator.md` — How to do blind A/B comparison between two outputs
- `agents/analyzer.md` — How to analyze why one version beat another

The references/ directory has additional documentation:

- `references/schemas.md` — JSON structures for evals.json, grading.json, etc.

---

## Core loop summary

- Figure out what the skill is about
- Draft or edit the skill
- Run the skill on test prompts via sub-agents
- With the user, evaluate the outputs:
  - Create benchmark.json and run `eval-viewer/generate_review.py` to help the user review them
  - Run quantitative evals
- Repeat until you and the user are satisfied
