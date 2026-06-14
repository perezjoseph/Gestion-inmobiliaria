# Agent Evals

Structured evaluation framework for Kiro IDE sub-agents. The agents exist to **isolate context** from the main orchestrator — each specialist handles domain work in its own context window so the orchestrator stays lean. These evals verify that specialists produce correct output given their isolated context.

## What We're Testing

1. **Correctness** — Does the specialist produce domain-correct output given its system prompt + steering + skills?
2. **Convention adherence** — Does it follow project patterns (snake_case, SeaORM, AppError, etc.) without hallucinating?
3. **Contamination** — Does it stay in its lane (no infra references from rust-coder, no SeaORM from infra-ops)?

## How It Works

1. **Test cases** in `cases/` define prompts + expected signals per agent
2. **Runner** invokes the specialist agent with the prompt and context files
3. **Grader** scores output against assertions and checks for contamination
4. **Aggregator** computes per-agent pass rates

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
        │   ├── output.md
        │   └── grading.json
        └── benchmark.json
```

## Running Evals

In Kiro chat, say:

> Run agent evals for rust-coder

The orchestrator will:
1. Read the test cases from `cases/<agent>.json`
2. For each case, invoke the specialist agent with the prompt and context files
3. Grade the output against assertions and check for contamination
4. Aggregate into `runs/<timestamp>/benchmark.json`
5. Generate a markdown report

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
  "agent": "rust-coder",
  "assertions": [
    { "text": "Uses AppError or maps to specific error variants", "passed": true, "evidence": "Response shows AppError::NotFound and AppError::Validation usage" },
    { "text": "Does not introduce unwrap()", "passed": true, "evidence": "No unwrap() calls in proposed code" }
  ],
  "score": 1.0,
  "contamination": []
}
```

## Interpreting Results

- **pass_rate > 0.9** — agent is working well for this task type
- **pass_rate < 0.7** — system prompt or steering may need tuning
- **contamination > 0** — agent is leaking knowledge from outside its domain (skills loading wrong context?)
- **Specific assertion failures** point to gaps in steering or system prompt instructions
