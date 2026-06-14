# Running Agent Evals (Orchestrator Instructions)

When the user says "run agent evals for <agent>" or "eval <agent>", follow this process:

## Step 1: Load Test Cases

Read the eval cases from `.kiro/evals/cases/<agent>.json`.

## Step 2: Create Run Directory

Create a timestamped run directory: `.kiro/evals/runs/<YYYY-MM-DD-HHmmss>/`

## Step 3: For Each Eval Case

### 3a. Run the Specialist

Invoke the specialist sub-agent with the eval prompt:

```
invoke_sub_agent:
  name: <agent>  (e.g. "rust-coder")
  prompt: <eval prompt from the case>
  contextFiles: <context_files from the case, with full paths>
```

Save the response to `runs/<timestamp>/<agent>-<eval-id>/output.md`

### 3b. Grade the Output

Construct a grading prompt and grade the output (inline or via general-task-execution):

```
You are an evaluation judge. Grade this agent output against the assertions below.
Return ONLY a JSON object with this structure:
{
  "assertions": [
    { "text": "<assertion text>", "passed": true/false, "evidence": "<brief reason>" }
  ],
  "contamination": ["<any contaminant terms found in the output>"],
  "score": <float 0.0-1.0, weighted pass rate>
}

ASSERTIONS:
<list assertions from the eval case>

CONTAMINANTS (should NOT appear in output):
<list contaminants from the eval case>

AGENT OUTPUT:
<the agent's response>
```

### 3c. Save Grading

Write `runs/<timestamp>/<agent>-<eval-id>/grading.json`:

```json
{
  "eval_id": "<id>",
  "eval_name": "<name>",
  "agent": "<agent>",
  "score": 0.85,
  "assertions": [...],
  "contamination": []
}
```

## Step 4: Aggregate

Run: `python .kiro/evals/scripts/aggregate.py .kiro/evals/runs/<timestamp>/`

## Step 5: Report

Run: `python .kiro/evals/scripts/report.py .kiro/evals/runs/<timestamp>/benchmark.json`

Read and present the report to the user.

## Notes

- Run multiple eval cases in parallel when possible (they're independent)
- If an agent errors out, record score 0.0 with evidence "agent error: <message>"
- The grading prompt must be self-contained — the judge has no access to the codebase
- Weight handling: score = sum(passed * weight) / sum(all weights)
