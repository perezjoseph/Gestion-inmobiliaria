# Evaluation Plan for Subagent Orchestration System

## 1. Evaluation Requirements

- **User Input:** `"Evaluate the subagent orchestration system focusing on correct agent selection, DAG dependency structure, and final output quality across multi-stage tasks"`
- **Revised (trace phase):** Evaluate routing accuracy, response quality, and faithfulness of the default kiro-cli orchestrator across all 11 specialist domains.

---

## 2. Agent Analysis

| **Attribute**         | **Details**                                                                                                                                                   |
| :-------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Agent Name**        | Kiro CLI Default Agent (orchestrator)                                                                                                                         |
| **Purpose**           | Routes prompts to the correct specialist based on description matching, returns domain-expert responses.                                                      |
| **Core Capabilities** | Prompt routing to 11 specialists, project-aware responses using steering/skills, headless CLI execution                                                       |
| **Input**             | Natural language prompt (string) via `kiro-cli chat --no-interactive --trust-all-tools`                                                                       |
| **Output**            | Specialist-quality response to stdout (exit 0 = success)                                                                                                      |
| **Agent Framework**   | Kiro CLI (custom, TypeScript runtime)                                                                                                                         |
| **Technology Stack**  | kiro-cli binary, Claude models, 11 specialist agent configs (`.kiro/agents/*.json`), steering files, skills                                                   |

**Specialist Agents (11):**

| Agent | Domain | Tools |
|-------|--------|-------|
| code-planner | Architecture planning | read, write, web, @context7 |
| code-reviewer | Code quality verification | read, write, shell |
| database-specialist | PostgreSQL/SeaORM | read, write, shell |
| frontend-designer | Leptos/Tailwind UI | read, write, shell |
| infra-ops | K8s, CI/CD, Docker | read, write, shell, web |
| kotlin-coder | Android/Compose | read, write, shell |
| librarian | Documentation research | read, web, @context7 |
| ocr-ml-engineer | OCR/OpenVINO ML | read, write, shell |
| rust-coder | Rust backend/frontend | read, write, shell |
| security-auditor | Security analysis | read, write, shell |
| test-engineer | Testing/QA | read, write, shell |

**Observability Status:**

- **Tracing:** External instrumentation via `opentelemetry-sdk` wrapping subprocess calls
- **Span Attributes:** `traceloop.entity.name`, `traceloop.entity.input`, `traceloop.entity.output`, `eval.test_case_id`, `eval.expected_agent`, `eval.duration_seconds`
- **Export:** OTLP HTTP → otelcol-contrib (file exporter) → `eval/otel-traces.jsonl` → `trace-processor.py` → `eval/traces/*.json`

---

## 3. Evaluation Metrics

### Routing Accuracy

- **Evaluation Area:** Agent selection correctness
- **Description:** Does the response demonstrate domain expertise of the expected specialist? Measured by checking whether the output contains domain-specific signals (framework names, patterns, conventions) matching the expected agent.
- **Method:** LLM-as-Judge (GEval)
- **Criteria:** Given the prompt and expected specialist, the response should exhibit expertise specific to that domain (e.g., rust-coder responses use AppError/SeaORM patterns, not generic advice).

### Response Quality

- **Evaluation Area:** Actionability and correctness
- **Description:** Is the response actionable, technically correct, and project-specific? Penalizes generic advice, rewards concrete file paths, code, and domain patterns.
- **Method:** LLM-as-Judge (GEval)
- **Criteria:** (1) Actionable — provides concrete steps, code, or artifacts. (2) Correct — technically sound for the project stack. (3) Project-specific — references real project patterns, not generic boilerplate.

### Faithfulness

- **Evaluation Area:** Grounding in project context
- **Description:** Does the response reference real project patterns (AppError, SeaORM entities, DOP/USD currency, Actix-web extractors) without hallucinating non-existent APIs, files, or conventions?
- **Method:** LLM-as-Judge (GEval)
- **Criteria:** References to code patterns, file paths, and conventions should be verifiable against the project's steering files and actual codebase. Penalize fabricated entity names, wrong framework APIs, or invented file paths.

---

## 4. Test Data

- **Coverage:** 25 test cases across 11 specialist domains (2-3 per domain)
- **Domains:** planning, database, rust-backend, review, frontend, infrastructure, android, research, ocr-ml, security, testing
- **Format:** JSONL with fields: `id`, `input`, `expected_agent`, `domain`, `expected_signals`
- **File:** `eval/test-cases.jsonl`

---

## 5. Evaluation Implementation Design

### 5.1 Code Structure

```
eval/
├── eval-plan.md         # This plan
├── agent_runner.py      # OTel-instrumented kiro-cli runner
├── test-cases.jsonl     # 25 test cases (2-3 per domain)
├── metrics.py           # Routing Accuracy + Response Quality + Faithfulness
├── run_evaluation.py    # Load traces → evaluate → save results
├── config.yaml          # Model, thresholds, paths
├── otel-traces.jsonl    # Raw OTLP spans (generated at runtime)
├── results/             # Evaluation output
│   └── agent_runs.json  # Raw CLI outputs
└── traces/              # Processed per-trace JSON (from trace-processor.py)
    └── <traceId>.json
```

### 5.2 Technical Stack

| **Component**            | **Selection**                                                     |
| :----------------------- | :---------------------------------------------------------------- |
| **Language/Version**     | Python 3.11                                                       |
| **Tracing**             | `opentelemetry-sdk` + `opentelemetry-exporter-otlp-proto-http`    |
| **OTEL Infrastructure**  | otelcol-contrib (file exporter → JSONL)                           |
| **Evaluation**          | DeepEval (GEval for LLM-as-Judge)                                 |
| **LLM Service**          | LiteLLM                                                           |
| **LLM Provider**         | kiro-cli (eval-judge agent, no tools)                             |
| **LLM Model**            | claude-opus-4-6 via eval-judge agent                              |
| **Agent Integration**    | `kiro-cli chat --no-interactive --trust-all-tools`                |
| **Results Storage**      | JSON files in `eval/results/`                                     |

### 5.3 Dependencies

```
opentelemetry-sdk
opentelemetry-exporter-otlp-proto-http
deepeval
litellm
pyyaml
```

---

## 6. Progress Tracking

### 6.1 User Requirements Log

| **Timestamp**       | **Source**      | **Requirement**                                                                         |
| :------------------ | :-------------- | :-------------------------------------------------------------------------------------- |
| 2026-06-13 18:47    | `/evalkit.plan` | Evaluate subagent orchestration: agent selection, DAG structure, output quality          |
| 2026-06-13 18:50    | `/evalkit.data` | Generate test cases                                                                     |
| 2026-06-13 19:04    | `/evalkit.trace` | Revised: external OTel tracing, 2-3 prompts per domain, routing/quality/faithfulness metrics |
| 2026-06-13 21:03    | `/evalkit.report` | Generate evaluation report with recommendations                                              |

### 6.2 Evaluation Progress

| **Timestamp**       | **Component**          | **Status**    | **Notes**                                                           |
| :------------------ | :--------------------- | :------------ | :------------------------------------------------------------------ |
| 2026-06-13 18:47    | Evaluation Plan        | Completed     | Initial plan on branch `002-eval-pipeline`                          |
| 2026-06-13 18:50    | Test Data              | Completed     | 25 test cases in `eval/test-cases.jsonl`                            |
| 2026-06-13 19:04    | Trace Instrumentation  | Completed     | `agent_runner.py` with external OTel SDK tracing                    |
| 2026-06-13 19:10    | Agent Execution        | Completed     | 25/25 passed (exit=0), 25 trace files in `eval/traces/`             |
| 2026-06-13 20:33    | Metrics Implementation | Completed     | 3 GEval metrics with eval-judge agent (no tools, JSON-only)         |
| 2026-06-13 20:33    | Evaluation Run         | Completed     | Routing: 0.732 PASS, Quality: 0.628 FAIL, Faithfulness: 0.712 PASS  |
| 2026-06-13 20:33    | Documentation          | Completed     | `eval/README.md` with usage instructions                            |
| 2026-06-13 21:03    | Evaluation Report      | Completed     | `eval/eval-report.md` — 5 prioritized action items                  |
