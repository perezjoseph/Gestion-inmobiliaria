# Quality Webhook — Architecture Diagrams

## 1. System Overview

```mermaid
graph TB
    subgraph "CI/CD (GitHub Actions)"
        GA[GitHub Actions Runner]
    end

    subgraph "SonarQube"
        SQ[SonarQube Server]
    end

    subgraph "Webhook Server (Windows Host)"
        SRV[HTTP Server<br/>0.0.0.0:9090]
        SEC[Security Layer<br/>IP filter + HMAC + Rate Limit]
        SRV --> SEC
    end

    subgraph "Processing (Background Threads)"
        CIF[/ci-failure handler/]
        CII[/ci-improve handler/]
        SQH[/sonarqube handler/]
        SFH[/sonar-fix handler/]
    end

    subgraph "Persistence"
        QUEUE[(SQLite Queue<br/>.webhook-queue.db)]
        HISTORY[(Fix History JSON<br/>.fix-history.json)]
        LANCE[(LanceDB<br/>.simplemem-data/)]
        FLAKY[(Flaky Tests JSON<br/>.flaky-tests.json)]
    end

    subgraph "WSL (Ubuntu 22.04)"
        KIRO[kiro-cli]
        CARGO[cargo test/clippy/fmt]
        GIT[git worktree operations]
    end

    GA -->|POST /ci-failure| SRV
    GA -->|POST /ci-improve| SRV
    SQ -->|POST /sonarqube| SRV
    GA -->|POST /sonar-fix| SRV

    SEC --> CIF
    SEC --> CII
    SEC --> SQH
    SEC --> SFH

    CIF -->|wsl_bash| KIRO
    CIF -->|wsl_bash| CARGO
    CIF -->|wsl_bash| GIT
    CII -->|wsl_bash| KIRO
    SFH -->|wsl_bash| KIRO

    CIF --> QUEUE
    CIF --> HISTORY
    CIF --> LANCE
    CIF --> FLAKY
```

## 2. CI Failure Fix Pipeline (Primary Flow)

```mermaid
flowchart TD
    A[POST /ci-failure] --> BUD{Budget exhausted?}
    BUD -->|Yes| ESC_BUD[Write pipeline escalation]
    BUD -->|No| BACK{Backoff needed?}
    BACK -->|Yes| WAIT[Sleep backoff period]
    WAIT --> DEP_CHK{Deployed during wait?}
    DEP_CHK -->|Yes| SKIP
    DEP_CHK -->|No| LOCK
    BACK -->|No| LOCK{Job lock available?}
    LOCK -->|No| ENQUEUE[Queue pending fix]
    LOCK -->|Yes| WT[Setup worktree]

    WT -->|Failed| FAIL[Record failure]
    WT -->|OK| HEAD_CHK{HEAD matches commit?}
    HEAD_CHK -->|No| RESET[Reset to origin/branch]
    HEAD_CHK -->|Yes| CHAIN
    RESET --> CHAIN{Auto-fix chain?}
    CHAIN -->|Yes| ROLLBACK[Rollback last auto-fix]
    CHAIN -->|No| DEDUP{Preflight dedup?}

    DEDUP -->|Rejected| DEEP[Deep research fix]
    DEDUP -->|Passed| CLASS[Classify error]

    CLASS --> CORR{Correlated failures?}
    CORR -->|Yes| BATCH[Hold & batch<br/>with correlation timer]
    CORR -->|No| FLAKY_CHK{All failures flaky?}

    BATCH -->|Timer expires| FLAKY_CHK
    FLAKY_CHK -->|Yes| SKIP[Skip — known flaky]
    FLAKY_CHK -->|No| REPRO[Reproduce locally]

    REPRO --> DECIDE[Build decision<br/>ranked strategies + feasibility]
    DECIDE --> MEM[Search semantic memory]
    MEM --> PROMPT[Build prompt<br/>+ error notes + memory + past attempts]
    PROMPT --> KIRO_RUN[run_kiro in worktree]

    KIRO_RUN -->|Timeout/Error| FAIL
    KIRO_RUN -->|Success| STAGE[git add -A]

    STAGE --> DIFF{Files changed?}
    DIFF -->|No| FAIL
    DIFF -->|Yes| GATES

    subgraph GATES [Quality Gates]
        direction TB
        G3A[Gate: Forbidden patterns<br/>no #ignore, no allow-unused]
        G3B[Gate: Security check<br/>no hardcoded secrets, no unsafe without SAFETY]
        G3C[Gate: Maintainability<br/>no unwrap/expect in prod, function length]
        G3E[Gate: Correctness<br/>no logic removals, invariant preservation]
        G4[Gate: Verification<br/>cargo test / clippy / fmt]
        G4P[Gate: PBT verification<br/>proptest suite]
        G5[Gate: Baseline<br/>no regressions vs main]

        G3A --> G3B --> G3C --> G3E --> G4 --> G4P --> G5
    end

    GATES -->|Any gate fails| DISCARD[discard_changes<br/>+ record failure + store memory]
    GATES -->|All pass| COMMIT[commit_and_push<br/>via worktrees module]

    COMMIT --> OUTCOME[Store fix outcome in memory]
    OUTCOME --> CLEANUP[Cleanup worktree]
    CLEANUP --> DONE[Record success + update tracker]
```

## 3. Retry & Escalation Strategy

```mermaid
flowchart TD
    R1[Round 1-3: Quick phase] -->|All fail| R2[Round 4-6: Deep phase]
    R2 -->|All fail| R3[Round 7+: Investigate phase]
    R3 -->|Fails| ESC[Write escalation<br/>to .kiro/escalations/]

    subgraph "Quick Phase"
        Q1[Standard prompt<br/>+ error context + memory search]
        Q2[Reproduction-informed<br/>parsed errors in prompt]
    end

    subgraph "Deep Phase"
        D1[Diagnosis via run_kiro<br/>analyze root cause]
        D2[Parse strategies from diagnosis]
        D3[Research-informed fix<br/>ranked strategy selection]
        D1 --> D2 --> D3
    end

    subgraph "Investigate Phase"
        I1[Deep research fix<br/>full analysis + planned fix]
        I2[GH CLI block<br/>wait for concurrent runs]
        I1 --> I2
    end

    R1 -.-> Q1
    R1 -.-> Q2
    R2 -.-> D1
    R3 -.-> I1

    subgraph "Decision Engine"
        FE[Feasibility check<br/>success rate history]
        SR[Strategy ranking<br/>past outcomes weighted]
        CO[Co-failure correlation<br/>hold for batch fix]
        FE --> SR --> CO
    end
```

## 4. Worktree Lifecycle

```mermaid
sequenceDiagram
    participant F as Fixer
    participant W as Worktrees module
    participant WSL as WSL/Git
    participant K as kiro-cli

    F->>W: setup_worktree(branch, commit)
    W->>W: _enforce_max_count() [max 3]
    W->>WSL: git fetch origin {branch} --no-tags
    W->>WSL: git worktree add .worktrees/fix-{branch}-{sha}
    alt Branch already checked out
        W->>WSL: git worktree add --detach (fallback)
        W->>WSL: git checkout -B {branch} origin/{branch}
    end
    W-->>F: (wsl_path, name)

    F->>WSL: run_kiro(prompt, cwd=wsl_path)
    WSL->>K: kiro chat --trust-all-tools --no-interactive
    K-->>WSL: edits files (does NOT commit)
    WSL-->>F: (success, output)

    F->>WSL: git add -A (in worktree)
    F->>WSL: git diff --cached (gates check staged diff)

    alt Gates pass
        F->>W: commit_and_push(wsl_path, branch, msg)
        W->>WSL: git commit -m "fix: ..."
        W->>WSL: git push origin {branch}
        alt Push rejected (branch moved)
            W->>WSL: git fetch + rebase + retry push
        end
    else Gates fail
        F->>W: discard_changes(wsl_path)
        W->>WSL: git reset HEAD -- . && checkout -- . && clean -fd
    end

    F->>W: cleanup_worktree(name)
    W->>WSL: git worktree remove --force
```

## 5. Module Dependency Map

```mermaid
graph LR
    server[server.py<br/>HTTP + routing + health] --> fixers[fixers.py<br/>Fix orchestration]
    server --> security[security.py<br/>HMAC + IP filter + rate limit]
    server --> tracker[tracker.py<br/>Pipeline state + lineage]
    server --> correlation[correlation.py<br/>Batch failures + cache health]
    server --> flaky[flaky.py<br/>Flaky test detection]
    server --> trends[trends.py<br/>Duration tracking + alerts]
    server --> queue[queue.py<br/>SQLite persistence queue]

    fixers --> runner[runner.py<br/>wsl_bash + run_kiro]
    fixers --> worktrees[worktrees.py<br/>Git worktree mgmt]
    fixers --> gates[gates.py<br/>Quality gates]
    fixers --> classifier[classifier.py<br/>Error classification]
    fixers --> reproducer[reproducer.py<br/>Local reproduction + parsing]
    fixers --> history[history.py<br/>Fix attempt records]
    fixers --> decisions[decisions.py<br/>Strategy ranking + feasibility]
    fixers --> memory[memory.py<br/>SimpleMem/LanceDB vectors]
    fixers --> vulns[vulns.py<br/>Vulnerability tracking]

    gates --> runner
    worktrees --> runner
    reproducer --> runner

    correlation --> history
    decisions --> history
    trends --> history
    vulns --> history

    runner --> config[config.py<br/>Paths + env + constraints]
    worktrees --> config
    gates --> config
    security --> config
    server --> config

```

## 6. Security & Request Flow

```mermaid
flowchart LR
    REQ[Incoming Request] --> IP{Source IP<br/>in ALLOWED_CLIENTS?}
    IP -->|No| R403[403 Forbidden]
    IP -->|Yes| RL{Rate limit<br/>30/min per IP?}
    RL -->|No| R429[429 Too Many]
    RL -->|Yes| CL{Content-Length<br/>≤ 512KB?}
    CL -->|No| R413[413 Too Large]
    CL -->|Yes| CT{Content-Type<br/>= JSON?}
    CT -->|No| R400[400 Bad Request]
    CT -->|Yes| HMAC{HMAC<br/>signature valid?}
    HMAC -->|No| R401[401 Unauthorized]
    HMAC -->|Yes| SANITIZE[Sanitize + validate fields]
    SANITIZE --> ROUTE{Route to handler}
    ROUTE --> RESPOND[200 OK immediately]
    RESPOND --> THREAD[Spawn background<br/>thread for processing]
```

## 7. Error Classification Tree

```mermaid
flowchart TD
    LOG[Error Log Text] --> DEP{Deploy patterns?<br/>health-check, compose}
    DEP -->|Yes| APP_BUG[app_bug / runner_environment]
    DEP -->|No| ENV{Runner env patterns?<br/>not found, permission denied}
    ENV -->|Yes| RUNNER[runner_environment]
    ENV -->|No| DEPN{Dependency patterns?<br/>RUSTSEC, GHSA, CVE, cargo-deny}
    DEPN -->|Yes| DEPENDENCY[dependency]
    DEPN -->|No| FLK{Flaky patterns?<br/>connection reset, broken pipe}
    FLK -->|Yes| FLAKY[flaky]
    FLK -->|No| PBT{Proptest patterns?<br/>proptest, shrunk to}
    PBT -->|Yes| PBT_F[pbt_failure]
    PBT -->|No| TST{test + failed?}
    TST -->|Yes| TEST[test_failure]
    TST -->|No| CQ{clippy/fmt?}
    CQ -->|Yes| CODE_Q[code_quality]
    CQ -->|No| BLD{Build patterns?<br/>error[E, linking, trunk}
    BLD -->|Yes| BUILD[build_failure]
    BLD -->|No| UNK[unknown]
```

## 8. Semantic Memory & Decision Engine

```mermaid
flowchart TD
    subgraph "Memory Layer (memory.py)"
        EMBED[Qwen3-Embedding-0.6B<br/>1024-d local embeddings]
        LANCE[(LanceDB<br/>file-based vector store)]
        EMBED --> LANCE
    end

    subgraph "Decision Engine (decisions.py)"
        STRAT[Strategy Records<br/>success/failure counts per job+class]
        FEAS[Feasibility Check<br/>success rate threshold]
        REPRO_P[Repro Profile<br/>steps to skip based on history]
        COFAIL[Co-Failure Pairs<br/>jobs that fail together]
        STRAT --> FEAS
    end

    FIX[fix_once_and_push] -->|search_relevant_context| LANCE
    FIX -->|build_decision| STRAT
    FIX -->|store_fix_attempt| LANCE
    FIX -->|record_strategy_outcome| STRAT
    FIX -->|store_fix_outcome| LANCE
```

## 9. Pipeline Tracker & Budget

```mermaid
stateDiagram-v2
    [*] --> Pending: failure registered
    Pending --> Fixing: job lock acquired
    Fixing --> Fixed: gates pass + push
    Fixing --> Failed: gates fail / kiro error
    Failed --> Fixing: retry (within budget)
    Failed --> Escalated: budget exhausted

    state "Budget Control" as BC {
        [*] --> Round1
        Round1 --> Round2: failure
        Round2 --> Round3: failure
        Round3 --> Exhausted: PIPELINE_BUDGET=10 reached
    }

    state "Backoff Schedule" as BS {
        [*] --> Immediate: rounds 1-2
        Immediate --> 2min: round 3
        2min --> 5min: round 4
        5min --> 10min: round 5+
    }
```

## 10. Queue & Replay

```mermaid
flowchart TD
    subgraph "SQLite Queue (queue.py)"
        ENQ[enqueue] --> DB[(webhook_queue table)]
        DB --> DEQ[dequeue]
        DEQ --> PROC[Processing]
        PROC -->|Success| DONE[mark_done]
        PROC -->|Error| FAILED[mark_failed<br/>retries < 3]
        FAILED -->|Stale > 30min| REQUEUE[requeue_stale]
        REQUEUE --> DB
        DB -->|> 7 days old| CLEANUP[cleanup_old]
    end

    subgraph "Server Replay"
        STARTUP[Server start] --> REPLAY[_replay_pending_queue]
        REPLAY --> DEQ
    end
```

## 11. Vulnerability Tracking

```mermaid
flowchart TD
    ERR[Error log with RUSTSEC/GHSA/CVE] --> EXTRACT[extract_vuln_info<br/>parse crate + advisory IDs]
    EXTRACT --> RECORD[record_vuln_failure<br/>append timestamp]
    RECORD --> CHECK{≥ 3 failures<br/>in 30 days?}
    CHECK -->|Yes| RECURRING[Mark as recurring<br/>surface in /health]
    CHECK -->|No| TRACK[Continue tracking]
```

## 12. Endpoint Summary

| Endpoint | Method | Handler | Purpose |
|---|---|---|---|
| `/health` | GET | `do_GET` | Health check + system stats (queue, memory, flaky, vulns, trends) |
| `/ci-failure` | POST | `_handle_ci_failure` | Fix CI failures via kiro-cli in worktrees |
| `/ci-improve` | POST | `_handle_ci_improve` | Optimize pipeline duration / fix focus areas |
| `/sonarqube` | POST | `_handle_sonarqube` | Receive SonarQube webhook (quality gate status) |
| `/sonar-fix` | POST | `_handle_sonar_fix` | Fix SonarQube issues in file groups |

## 13. Configuration Constants

| Constant | Value | Purpose |
|---|---|---|
| `PORT` | 9090 | HTTP server port |
| `BIND_ADDRESS` | 0.0.0.0 | Network binding (never change) |
| `MAX_CONCURRENT_FIXES` | 3 | Semaphore for parallel fix threads |
| `THREAD_POOL_SIZE` | 8 | Background thread pool |
| `PIPELINE_BUDGET` | 10 | Max fix rounds per pipeline lineage |
| `BACKOFF_SCHEDULE` | [0, 0, 120, 300, 600] | Seconds between retry rounds |
| `WORKTREE_MAX_COUNT` | 3 | Max simultaneous worktrees |
| `WORKTREE_MAX_AGE_H` | 4 | Auto-prune worktrees older than 4h |
| `DEDUP_WINDOW_MINUTES` | 60 | Dedup identical errors within window |
| `SONAR_PARALLEL_GROUPS` | 3 | Parallel SonarQube file group fixes |
| `KIRO_TIMEOUT` | 0 (unlimited) | kiro-cli process timeout |
| `MAX_LOG_BYTES` | 50MB | Log rotation threshold |
