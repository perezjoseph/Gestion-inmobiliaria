# Agent Evals

Structured evaluation framework for Kiro IDE sub-agents. Measures whether specialist agents outperform a general-purpose agent on domain-specific tasks.

## How It Works

1. **Test cases** in `cases/` define prompts + expected signals per agent
2. **Runner** spawns each case through the **specialist agent** AND the **general-task-execution** baseline
3. **Grader** (eval-judge) scores each output against assertions
4. **Aggregator** computes pass rates and deltas (specialist vs general)

## Comparison Model

| Variant | What it is | invoke_sub_agent name |
|---------|-----------|------------------------|
| **specialist** | Domain agent with system prompt, tools, steering, and skills | e.g. `rust-coder`, `infra-ops`, `frontend-designer` |
| **baseline** | Generic agent with all tools but no domain specialization | `general-task-execution` |

Both are invoked via `invoke_sub_agent` with the same prompt. The eval answers: "Does the specialist agent produce better results than a generic agent on domain tasks?"

## Directory Structure

```
evals/
├── README.md
├── cases/
│   ├── rust-coder.json
│   ├── frontend-designer.json
│   ├── infra-ops.json
│   ├── database-specialist.json
│   └── ...
├── scripts/
│   ├── aggregate.py        # Aggregates grading results into benchmark.json
│   └── report.py           # Generates markdown report from benchmark.json
└── runs/                   # gitignored — ephemeral eval outputs
    └── <timestamp>/
        ├── <agent>-<eval-id>/
        │   ├── specialist/output.md
        │   ├── baseline/output.md
        │   ├── grading.json
        │   └── metadata.json
        └── benchmark.json
```

## Running Evals

In Kiro chat, say:

> Run agent evals for rust-coder

The orchestrator will:
1. Read the test cases from `cases/<agent>.json`
2. For each case, invoke the specialist agent with the prompt
3. Invoke `general-task-execution` with the same prompt (baseline)
4. Grade both outputs using eval-judge
5. Aggregate into `runs/<timestamp>/benchmark.json`
6. Generate a markdown report

## Test Case Format

```json
{
  "agent": "rust-coder",
  "evals": [
    {
      "id": "rc-01",
      "name": "error-handling-refactor",
      "prompt": "Refactor the create_propiedad handler to use proper AppError variants instead of generic 500s",
      "assertions": [
        { "text": "Uses AppError or maps to specific error variants", "weight": 2 },
        { "text": "Preserves existing functionality", "weight": 1 },
        { "text": "Does not introduce unwrap() or expect() in handler code", "weight": 1 }
      ],
      "contaminants": ["Kubernetes", "Playwright", "Jetpack Compose"],
      "context_files": ["backend/src/handlers/propiedades.rs"]
    }
  ]
}
```

## Grading Schema

Each assertion is graded pass/fail with evidence:

```json
{
  "eval_id": "rc-01",
  "variant": "specialist",
  "assertions": [
    { "text": "Uses AppError or maps to specific error variants", "passed": true, "evidence": "Response shows AppError::NotFound and AppError::Validation usage" },
    { "text": "Does not introduce unwrap()", "passed": true, "evidence": "No unwrap() calls in proposed code" }
  ],
  "score": 1.0,
  "contamination": []
}
```

## Interpreting Results

- **pass_rate delta** (specialist - baseline) > 0.1 means the specialist adds clear value
- **pass_rate delta** ≈ 0 means the specialist isn't outperforming a generic agent (skills/steering may not be activating)
- **pass_rate delta** < 0 means the specialist is worse — investigate system prompt or skill conflicts
- **contamination > 0** means the agent mentioned domains it shouldn't know about
- **Individual assertion failures** point to specific skill or steering gaps
