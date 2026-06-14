# Plan: Harness Observability & Eval Pipeline

Fix the autofix harness observability gaps using the existing Grafana stack. Dual-sink: OTel spans → Tempo (live obs/metrics/alerting) + structured trace file → evals (offline scoring/baselines/trend).

## Problem (gaps found in current autofix)

| Gap | Today | Impact |
|-----|-------|--------|
| G1 No persisted trace | session JSONL + side-channels die in `RUNNER_TEMP` at teardown; zero `upload-artifact` in autofix workflows | can't reconstruct what the agent did |
| G2 No structured metrics | data in commit msgs + side-channels, never extracted | VCR/verification_gap uncomputed |
| G3 No trend store | each run independent | regression/entropy invisible |
| G4 Outcome/escape not captured | held/failed exists in workflow_run chain, never recorded against the fix | escape rate unknown |
| G5 verification_gap uncomputed | `Status: CLEAN` never joined to sensors_ran + held | the key CI-only metric is lost |
| G6 Learnings reset per queue run | `: > LEARNINGS_FILE` each run | cross-run learning lost |

## Target architecture

```
kiro JSONL + harness side-channels (RUNNER_TEMP)
  └─ adapter (otel-cli / script), runs in autofix job before teardown
       ├─ OTel spans (gen_ai.* + harness.*) → tempo.monitoring.svc.cluster.local:4317
       │     → span-metrics → Prometheus → Alertmanager (regression alerts)
       └─ structured trace file (OTel GenAI shape) → upload-artifact + append evals/autofix-history.jsonl
             → offline eval scoring, baselines, trend
```

Platform: existing Tempo. No new install. Defer Phoenix/Langfuse.
Format: OTel GenAI semconv (`gen_ai.*`) + `harness.*` namespace.

## Phases (each independently shippable, ratchet-style)

### Phase 0 — Prerequisite: NetworkPolicy (infra, blocks everything)
- **File:** `infra/k8s/arc-v2/allow-arc-runner-egress.yml` (or new policy)
- Allow `arc-v2` ns → `monitoring` ns TCP 4317 (Tempo OTLP grpc).
- **Verify:** from a runner pod, `nc -zv tempo.monitoring.svc.cluster.local 4317` succeeds.
- **Risk:** infra change, but additive egress allowance. Low blast radius.
- **Done when:** span reaches Tempo from an in-cluster test pod.

### Phase 1 — Persist the trace (closes G1)
- **File:** `kiro-autofix-trigger.yml` — add a `collect-harness-trace` step after the queue loop, `if: always()`.
- Collect into a bundle dir: per-artifact commit msgs, `autofix-learnings.txt`, `autofix-*.txt` side-channels, and `~/.kiro/sessions/cli/<id>.jsonl` for the run's sessions.
- `actions/upload-artifact@<pinned-sha>` (match the SHA already used in build-and-test.yml: `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a`).
- **No retry wrap** (upload-artifact has own retry); the step is deterministic collection.
- **Done when:** every autofix run leaves a downloadable trace bundle.

### Phase 2 — Compute structured metrics (closes G2, G5)
- **File:** new `scripts/harness-metrics.{sh,py}` invoked by the collect step.
- Parse per-artifact: `verdict` (CLEAN/PARTIAL/NO_FIX from commit-msg Status + has_changes), `iterations` (from msg), `sensors_ran` (`test -s autofix-sensors-ran.txt`), `agent_exit`, `files`.
- Emit one run record: `{run_id, ts, branch, queue_run, model, artifacts:[{name, verdict, iterations, sensors_ran, files, fix_sha}]}`.
- `verification_gap` per artifact = `verdict==CLEAN AND !sensors_ran` (held added in Phase 4).
- Write to `$GITHUB_STEP_SUMMARY` (human view) + the bundle.
- **Done when:** each run produces a parseable metric record, no git-log scraping needed.

### Phase 3 — Emit spans to Tempo (closes live obs; enables alerting)
- **Tool:** `otel-cli` installed in the runner image (`infra/docker/Dockerfile.runner`) or fetched (retry-wrapped tool install per policy: 10min, 3 attempts, 10s).
- Span hierarchy: run span → per-artifact span → (optional) tool-call child spans parsed from session JSONL ToolResults.
- Attributes (obtainable only): `gen_ai.operation.name=invoke_agent`, `gen_ai.agent.name=autofix`, `gen_ai.request.model` (from the `--model` flag, not the trace), `gen_ai.tool.name` (from ToolResults), tool success/failure; `harness.verdict`, `harness.iterations`, `harness.sensors_ran`, `harness.verification_gap`.
- **Blocked (do not attempt):** `gen_ai.usage.*` token counts, per-call latency, error rate — verified absent from JSONL/hooks; needs native OTLP (kiro issue #6319).
- Export OTLP grpc → `tempo.monitoring.svc.cluster.local:4317`.
- **Done when:** autofix run appears as a trace in Grafana; span-metrics show in Prometheus.

### Phase 4 — Capture the held/escape signal (closes G4, completes G5)
- autofix is `workflow_run`-triggered by CI completion. The fix's CI re-run conclusion = held/escaped.
- **Approach:** the metric record carries `fix_sha`. A small correlation step (in the NEXT autofix trigger, or a tiny scheduled job) reads the CI conclusion for `fix_sha` on that branch → sets `held` on the prior record.
- Recompute `verification_gap = verdict==CLEAN AND (!sensors_ran OR !held)` once `held` is known.
- **Done when:** escape rate is queryable.

### Phase 5 — Trend store + regression alert (closes G3)
- Append each run record to `evals/autofix-history.jsonl` (working branch — travels with the fix; simpler than a metrics branch).
- Commit it alongside the fix in the existing commit step (already pushes; add the file to `git add`).
- **Alerting:** add a Prometheus rule on the span-metric for `harness.verification_gap` rate (rolling) → `infra/k8s/alerts.yml` → Alertmanager. No separate trend job needed for alerting; the JSONL is for offline baselines.
- **Done when:** rolling VCR / verification-gap visible in Grafana; alert fires on regression.

### Phase 6 — Eval framework consumption (closes the loop)
- The structured trace file (OTel GenAI shape) is the eval input — NOT Tempo.
- Wire `.kiro/evals/` (existing) or Agent-EvalKit: skip `/evalkit.trace` (no framework to instrument), feed the trace file to `/evalkit.eval` + `/evalkit.report`.
- Pre-merge gate (later): PR touching `.kiro/agents/` or `.kiro/steering/` runs the golden set, compares to committed baseline in `evals/`, blocks on regression.
- **Done when:** an eval run scores a harness change against a baseline.

### Phase 7 (optional) — Persist learnings across queue runs (closes G6)
- Append learnings to a committed `evals/autofix-learnings.jsonl` instead of wiping `RUNNER_TEMP` file each run.
- Feed prior learnings to the agent via `KIRO_LEARNINGS_FILE` at queue start.
- **Done when:** queue run N+1 sees run N's structured learnings, not just commit diffs.

## Sequencing

Phase 0 blocks 3. Phases 1→2→5 are the minimum viable trend (git-native, no Tempo). Phase 3 adds live obs. Phase 4 completes the headline metric. 1+2 alone close the worst gaps (G1, G2, G5-partial) in one step and are the recommended first PR.

## Constraints (steering)

- `.github/workflows/` + `.github/actions/` are CODEOWNERS-protected — show diffs, get review.
- `permissions: {}` top-level; metrics step needs `contents: write` only if committing the JSONL.
- Pin all actions to full SHA + version comment.
- Tool installs (otel-cli fetch) → retry-wrap (10min, 3 attempts, 10s). Deterministic parsing → no wrap.
- Don't push to main for metrics; ride the existing fix commit/push.
- NetworkPolicy change is additive egress; verify from a test pod before relying on it.

## Files touched

| Phase | File | Type |
|-------|------|------|
| 0 | `infra/k8s/arc-v2/allow-arc-runner-egress.yml` | infra (CODEOWNERS) |
| 1,2,5 | `.github/workflows/kiro-autofix-trigger.yml` | CI (CODEOWNERS) |
| 2 | `scripts/harness-metrics.{sh,py}` | new script |
| 3 | `infra/docker/Dockerfile.runner` | infra |
| 5 | `infra/k8s/alerts.yml`, `evals/autofix-history.jsonl` | infra + data |
| 6 | `.kiro/evals/` wiring | eval config |

## Verification per phase

Each phase ends with a concrete check (listed inline). Overall done = an autofix run produces: a trace in Tempo, RED metrics in Prometheus, a metric record in `evals/autofix-history.jsonl`, a trace bundle artifact, and (after a CI re-run) a `held` outcome — and a regression in verification_gap fires an Alertmanager alert.
