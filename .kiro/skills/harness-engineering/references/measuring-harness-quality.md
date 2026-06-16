# Measuring Harness Quality

Two layers. Behavioral is ground truth and expensive. Structural is a cheap proxy. Structural **predicts**, behavioral **confirms**. Never claim causality from structural alone — ablation tells you which component is most valuable, only failure attribution finds the bottleneck.

## Layer 1 — Behavioral (Ground Truth)

The only honest measure: does the harness convert capability into reliable outcomes? Requires real task runs.

### Metric Suite

| Metric | Definition | Good | Bad |
|--------|-----------|------|-----|
| VCR (Verified Completion Rate) | verified tasks / activated tasks | → 1.0 | < 0.6 |
| Verification gap | claimed-done minus actually-done, over N tasks | → 0 | wide |
| Rework rate | tasks needing redo / total | low | high |
| Rebuild cost | wall-clock for fresh session to reach executable state | < 3 min | 15–60 min |
| Iterations to pass | mean verify-fix loops per task | low, bounded | unbounded |
| Escape rate | defects passing harness but failing later | → 0 | high |
| Cost per verified task | $ or tokens / verified task | low | high |

Reference deltas from literature (same model, harness change only): WIP=1 → +37% completion; dedicated init → +31%; structured feature lists → +45%; clean-state over 12 weeks → build 68%→97%, startup 60min→9min; missing observability → 30–50% session time wasted on diagnosis.

### Measurement Protocol (Controlled Variable Exclusion)

1. **Fix the model.** Same model across all runs.
2. **Pick a representative task set.** N ≥ 5, ideally 10+. For autofix: a set of known CI failures.
3. **Baseline.** Run with full harness, record every metric.
4. **Ablate.** Remove ONE component, re-run, measure delta. Repeat per component.
5. **Rank.** Largest delta = highest marginal value for the current task.
6. **Find the bottleneck.** Ablation ranks value, not bottleneck. Also attribute each failure to one of the five layers (task spec / context / environment / verification / state). The layer with the most failures is the bottleneck.

### Per-Agent Behavioral Test (autofix example)

| Step | Measure |
|------|---------|
| Run against N known CI failures | CLEAN-status rate = clean exits / N |
| Count verify-fix loops per task | mean iterations, compare to budget |
| Re-run CI on the produced fix | escape rate = fixes that fail re-run / N |
| Diff size per fix | scope discipline (minimal vs sprawling) |

## Layer 2 — Structural (Proxy)

Mechanical rubric over the agent JSON + system prompt. Score each item 0 (absent) / 1 (partial) / 2 (solid). Items marked [cfg] are deterministically checkable from JSON; [prompt] need reading the system prompt.

### Rubric

**Instructions** (max 8)
- prompt present, uses `file://` not inline bloat [cfg]
- prompt declares role + explicit NOT-do list [prompt]
- resources load AGENTS.md + steering via glob [cfg]
- skills loaded via `skill://` for progressive disclosure [cfg]

**State / Memory** (max 4)
- reads prior-attempt / history (env var or file in prompt) [prompt]
- produces structured output artifact + exit codes [prompt]

**Verification** (max 6)
- postToolUse sensors present [cfg]
- stop-gate blocks exit when files modified but sensors didn't run [cfg]
- sensors cover every file type the agent can write [cfg+prompt]

**Scope / Bounded Variety** (max 8)
- `tools` minimal, not `"*"` [cfg]
- `allowedTools` scoped, no blanket wildcard [cfg]
- `toolsSettings.write.deniedPaths` set for dangerous paths [cfg]
- `preToolUse` blocks dangerous ops (git, rm -rf) [cfg]

**Lifecycle** (max 4)
- `agentSpawn` clears stale state [cfg]
- clean-state / exit contract documented in prompt [prompt]

### Scoring

Sum / 30, normalize to %. Map to maturity:

| Score | Maturity |
|-------|----------|
| < 50% | Level 1–2 (instructions only, weak verification) |
| 50–75% | Level 3 (scoped + verified) |
| 75–90% | Level 4 (continuous) |
| > 90% | Level 5 (autonomous) |

### Honesty Caveat

A high structural score is a **prediction** that behavioral metrics will be good. If structural is high but VCR is low, the rubric is mis-weighted — re-weight it (the rubric is itself subject to the ratchet). Never report a structural score as proof of effectiveness.

## The Three-Question Gate (Quick Check)

Before formal measurement, every harness must pass:

1. **Does it move the number?** Component with zero metric delta = decoration or debt. Remove it.
2. **Can the agent bypass it?** Reliance on agent choosing to comply = suggestion, not control. Must be mechanical.
3. **Does it survive the agent?** Erodes over sessions (pattern-copying, hidden unfinished work, doc drift) = not good. Anti-entropy must be built in.

## Collecting Traces from kiro-cli (Prerequisite for Behavioral Metrics)

You can't compute VCR, iterations, or escape rate without traces. kiro-cli persists traces automatically — no OTel instrumentation needed. Two complementary streams.

### Stream 1 — Conversation trace (automatic)

Every session writes `~/.kiro/sessions/cli/<sessionId>.jsonl`. Each line: `{version, kind, data}`.

| kind | Captures |
|------|----------|
| `Prompt` | task/user input + timestamp |
| `AssistantMessage` | model output |
| `ToolResults` | tool calls + results |

This is the full execution trace — what evalkit's trace step consumes. Companion files: `.json` (metadata), `.history`. v2 sessions record the parent session ID (subagent traceability).

Index sessions: `kiro-cli chat --list-sessions --format json` → `sessionId`, `source`, `title`, `updatedAt`, `messageCount`.

### Stream 2 — Harness trace (you emit via hooks)

What the harness *enforced* (not what the model said). The autofix agent already does this — hooks append to side-channel files + a structured commit message (iterations, sensors PASS/FAIL, status). Generalize into a standard trace-emitter hook set on any agent you want to track:

| Hook | Emits |
|------|-------|
| `agentSpawn` | `run_start` (run-id, agent, model, task, ts) |
| `preToolUse` | `tool_call` (tool, input-hash) |
| `postToolUse` | `tool_result` + sensor-ran flag |
| `stop` | `run_end` (verdict, iterations, status) |

All append JSONL to `$TRACE_DIR/<run-id>.harness.jsonl`.

### Which stream feeds which metric

| Metric | Stream |
|--------|--------|
| VCR, verification gap, iterations-to-pass | Harness (`run_end`, sensor flags) |
| Escape rate, tool-call analysis, rebuild cost | Conversation (`.jsonl`) |
| Cost per task | blocked — needs token counts (issue #6319); approximate via wall-clock only |

### Correlating a headless run to its trace

```powershell
kiro-cli chat --agent autofix --no-interactive "task..."
$id = (kiro-cli chat --list-sessions --format json | ConvertFrom-Json)[0].sessions[0].sessionId
Get-Content "$env:USERPROFILE\.kiro\sessions\cli\$id.jsonl"
```

Cleaner than racing `--list-sessions`: have the `agentSpawn` hook write the run-id + start metadata to the harness trace, then match on timestamp.

### evalkit integration

Agent-EvalKit's `/evalkit.trace` instruments agent **source code** for supported frameworks (Strands, LangGraph, CrewAI) — it injects OpenTelemetry into a Python SDK agent. It does NOT apply to a kiro-cli harness: there's no framework to detect, no agent source to instrument, and kiro-cli already writes the trace as session `.jsonl`. Verified: kiro-cli `settings list` exposes no telemetry/otel/export key, and no OTLP exporter env is honored — the JSONL session file is the trace mechanism.

Use evalkit with a kiro harness like this:

| Phase | kiro-cli harness |
|-------|------------------|
| `/evalkit.plan` | use — design metrics |
| `/evalkit.data` | use, or reuse `.kiro/evals/cases/` |
| `/evalkit.trace` | SKIP — no framework; trace already exists |
| `/evalkit.run_agent` | replace with `kiro-cli chat --no-interactive`; run writes `<sessionId>.jsonl` |
| `/evalkit.eval` | use — point at the session `.jsonl` |
| `/evalkit.report` | use — code-level recommendations |

Map kiro's JSONL records (`Prompt` / `AssistantMessage` / `ToolResults`) to the structured-trace shape the eval phase expects. Use evalkit for LLM-as-judge scoring over the conversation; use the harness JSONL (hook-emitted) for deterministic metrics. Never let LLM-judge scores stand alone — pair with the deterministic harness stream (success silent, failure verbose).

## Measuring from the GitHub Runner (CI)

The autofix harness runs headless in CI (`kiro-autofix.yml`, `kiro-autofix-runtime.yml` on `arc-runner-dind`). The runner is ephemeral — traces must be collected and persisted before teardown. The runner is also the only place with ground truth: whether the fix actually held in the real environment on the real failure.

### Runner trace sources (already produced, no new instrumentation)

| Source | Path / output |
|--------|---------------|
| Output contract | `${RUNNER_TEMP}/kiro-commit-message.txt` (Status, Iteration n/max, sensor PASS/FAIL) |
| Harness trace | `${RUNNER_TEMP}/autofix-{modified-files,sensors-ran,stop-blocks}.txt` |
| Conversation trace | `~/.kiro/sessions/cli/<sessionId>.jsonl` |
| Exit code | `steps.kiro.outputs.agent_exit` |
| Attempt count | `steps.attempts.outputs.count` (consecutive kiro-bot commits) |
| Change flag | `steps.changes.outputs.has_changes` |
| Re-verification | subsequent build/test result — did the fix HOLD (escape signal) |

The last four are runner-only. Local measurement can't tell you if a fix actually worked.

### In-job metrics (compute in a step after the kiro step)

```
verdict          = CLEAN | PARTIAL | NO_FIX
iterations       = n/max from commit msg
sensors_ran      = test -s autofix-sensors-ran.txt
held             = re-verification passed
verification_gap = verdict==CLEAN AND (!sensors_ran OR !held)
```

`verification_gap` is the headline CI metric — the agent claimed done but the runner proves otherwise. Only CI can measure it.

### Three persistence sinks

1. Raw bundle → `actions/upload-artifact@<sha>` (pinned). Per-run debug.
2. Trend store → append one JSONL line to `evals/history.jsonl` on a dedicated `harness-metrics` orphan branch (not main). The trajectory.
3. Per-run → `$GITHUB_STEP_SUMMARY` markdown.

### Trend / regression

Scheduled workflow (mirror `drift-detection.yml`) reads `harness-metrics` history → rolling VCR, verification-gap rate, escape rate → flags drift.

### CI constraints (steering)

- `permissions: {}` top-level; metrics step needs only `contents: write` if pushing to `harness-metrics`.
- Pin `upload-artifact` to full SHA.
- The push to `harness-metrics` is network/API → wrap in `nick-fields/retry` (3 min, 3 attempts). Metric computation is deterministic → do not wrap.
- `.github/workflows/` is CODEOWNERS-protected.

## Trace Platform Decision — Dual-Sink (this repo)

One source, two consumers. The adapter that reads kiro's session `.jsonl` + harness side-channel files fans the same data out two ways:

```
kiro JSONL + harness side-channels  (single source)
        │
        ├──▶ OTel spans (gen_ai.* semconv)  ──▶ Tempo (existing, monitoring ns, OTLP :4317)
        │         → live obs, span-metrics → Prometheus → Alertmanager regression alerts
        │
        └──▶ structured trace file (OTel GenAI shape)  ──▶ eval framework
                  → offline scoring (VCR, faithfulness, verification_gap), baselines, trend
```

### Why dual-sink, not Tempo-as-eval-source

- Tempo is a trace **store**, not an eval framework. Evals consume trace files (or OTLP ingest), not TraceQL queries.
- Tempo retention is 336h and may sample under load — wrong source-of-truth for evals, which need complete, durable traces (baselines span months). The eval trace file is the durable source (committed/artifact), independent of Tempo.
- Querying Tempo + converting spans → eval shape is a lossy extra hop. Emit both from the adapter instead.

### Platform: use existing Grafana Tempo

No new install. Push OTLP to `tempo.monitoring.svc.cluster.local:4317` from the in-cluster `arc-runner-dind`. Tempo's span-metrics + service-graphs generators already remote_write to Prometheus, so harness spans become RED metrics automatically → alertable via existing Alertmanager. `harness.verification_gap` as a span attribute → Prometheus metric → regression alert, no separate trend store needed for alerting.

### Format: OTel GenAI semantic conventions

Emit `gen_ai.*` for the obtainable model/tool layer (`gen_ai.operation.name`, `gen_ai.agent.name`, `gen_ai.tool.name`) + a `harness.*` namespace for CI-only signals (`harness.verdict`, `harness.iterations`, `harness.sensors_ran`, `harness.held`, `harness.verification_gap`). This format is portable across OTel-native eval/obs tools (OpenInference, Phoenix, Langfuse) and convertible to DeepEval/evalkit test-case shape — the eval source isn't locked to one framework.

Do NOT plan on `gen_ai.usage.*` (token counts) or per-call latency — they are not obtainable today (see capability matrix below). The high-value signals are the `harness.*` ones, which don't depend on runtime internals.

### Defer Phoenix / Langfuse

Those are OTLP-native eval platforms where spans ARE the eval substrate (one system for trace + eval). Adding one on top of Tempo only pays off when prompt-level inspection or first-class eval-score storage becomes a recurring need. Per the ratchet: defer until a failure demands it. Tempo + files + GenAI format covers traces, metrics, alerting, and offline evals today.

### kiro-cli has no native OTLP — verified

Binary inspection of `kiro-cli.exe` (June 2026): zero occurrences of `opentelemetry`, `otlp`, or `otel_exporter`. kiro-cli does NOT honor `OTEL_EXPORTER_OTLP_ENDPOINT` and emits no OpenTelemetry. Its `telemetry` strings are internal AWS usage analytics (the enterprise dashboard); its `tracing`/`span` strings are the Rust `tracing` crate for internal `-v` logging. There is no env-var shortcut — an adapter is required.

The session `.jsonl` is the richest trace source. The sqlite `conversations_v2` table holds the same data as whole-conversation blobs — coarser, not better. Verbose `-v` output is unstructured log lines, not spans.

### Capability matrix — what's obtainable vs blocked (verified June 2026)

Verified against real session JSONL (`AssistantMessage.data` = `message_id` + `content` only; `ToolResults.data` = `message_id` + `content` + `results`) and confirmed by Kiro issue #6319 (open feature request for native OTLP).

| Signal | Obtainable | Source |
|--------|-----------|--------|
| Prompt text + timestamp | yes | JSONL `Prompt.data` + `meta.timestamp` |
| Assistant response text | yes | JSONL `AssistantMessage.data.content` |
| Tool name + purpose | yes | JSONL `ToolResults.results.<id>.tool` |
| Tool success/failure | yes | `ToolResults...status` / `result.Error\|Ok` |
| Tool count + sequence | yes | JSONL order |
| Run-level wall-clock | yes | hooks (agentSpawn→stop) or file mtime |
| Model | yes (indirect) | known at invocation (`--model` flag), not in trace |
| `harness.*` (verdict, iterations, sensors_ran, held) | yes | output contract + hooks |
| **Token usage (input/output)** | **NO** | runtime-internal; not in JSONL, hooks, or any surface |
| **Per-LLM-call latency** | **NO** | no per-call timing exposed |
| **Per-tool duration** | **NO** | no duration field in `ToolResults` |
| **LLM error rate** | **NO** | runtime-internal |

The blocked signals require native OTLP export, tracked in **kiro issue #6319** (open, `keep-open`). Until it ships, plan traces around the obtainable column — and lean on `harness.*`, which is independent of runtime internals and is where the CI-only value (verification_gap, held) lives anyway. Re-evaluate when #6319 lands: it would replace the adapter with a single `OTEL_EXPORTER_OTLP_ENDPOINT` env var and unlock token/latency/error signals.

### Adapter design — two concerns, two mechanisms

Separate enforcement from trace reconstruction:

| Concern | Mechanism | Why |
|---------|-----------|-----|
| Harness enforcement (sensors_ran flag, stop-gate) | hooks — minimal | fire at lifecycle moments, cheap; see `assets/trace-hooks.json` |
| The trace (tool calls, model I/O, span hierarchy + timing) | post-run converter reading the session `.jsonl` | the JSONL already has the complete ordered timestamped event stream — one pass builds a correct parent/child span tree |

Do NOT reconstruct the trace from per-tool hooks — isolated spans with no clean parent linkage and fragile inline `jq` parsing. Read the JSONL after the run instead.

The converter writes the eval trace file AND emits OTLP spans (dual-sink). Run it in the autofix job before teardown. Network gotcha: `arc-v2` ns → `monitoring` ns port 4317 needs an explicit NetworkPolicy allowance or spans drop silently.
