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
        SRV[HTTP Server<br/>0.0.0.0:port]
        SEC[Security Layer<br/>IP filter + HMAC + Rate Limit]
        SRV --> SEC
    end

    subgraph "Processing (Background Threads)"
        CIF[/ci-failure handler/]
        CII[/ci-improve handler/]
        SQH[/sonarqube handler/]
        SFH[/sonar-fix handler/]
    end

    subgraph "WSL (Ubuntu)"
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
```

## 2. CI Failure Fix Pipeline (Primary Flow)

```mermaid
flowchart TD
    A[POST /ci-failure] --> B{Budget exhausted?}
    B -->|Yes| SKIP[Skip — log error]
    B -->|No| C{Already fixing<br/>this job?}
    C -->|Yes| SKIP
    C -->|No| D[Classify error]

    D --> E{Correlated<br/>failures?}
    E -->|Yes| F[Hold & batch<br/>with timer]
    E -->|No| G[fix_once_and_push]

    F -->|Timer expires or<br/>correlated arrives| G

    G --> H[Setup worktree<br/>git fetch + worktree add]
    H --> I{Flaky test?}
    I -->|Yes| SKIP2[Skip — known flaky]
    I -->|No| J[Reproduce locally]

    J --> K[Build prompt<br/>+ error notes + past attempts]
    K --> L[run_kiro in worktree]

    L --> M{kiro-cli<br/>success?}
    M -->|No| FAIL[Record failure<br/>+ cleanup worktree]
    M -->|Yes| N[git add -A]

    N --> O{Files changed?}
    O -->|No| FAIL
    O -->|Yes| GATES

    subgraph GATES [Quality Gates]
        direction TB
        G3A[Gate 3a: Forbidden patterns<br/>no #ignore, no allow-unused]
        G3B[Gate 3b: Security check<br/>no hardcoded secrets]
        G3C[Gate 3c: Maintainability<br/>no unwrap/expect in prod]
        G3E[Gate 3e: Correctness<br/>no logic removals]
        G4[Gate 4: Verification<br/>cargo test / clippy]
        G5[Gate 5: Baseline<br/>no regressions]

        G3A --> G3B --> G3C --> G3E --> G4 --> G5
    end

    GATES -->|Any gate fails| DISCARD[discard_changes<br/>+ record failure]
    GATES -->|All pass| COMMIT[commit_and_push<br/>to branch]

    COMMIT --> DONE[Record success<br/>+ cleanup worktree]
```

## 3. Retry & Escalation Strategy

```mermaid
flowchart TD
    R1[Round 1-3: Quick phase] -->|All fail| R2[Round 4-6: Deep phase]
    R2 -->|All fail| R3[Round 7+: Investigate phase]
    R3 -->|Fails| ESC[Write escalation<br/>to .kiro/escalations/]

    subgraph "Quick Phase"
        Q1[Standard prompt<br/>+ error context]
    end

    subgraph "Deep Phase"
        D1[Diagnosis first<br/>via run_kiro]
        D2[Research-informed<br/>fix prompt]
        D1 --> D2
    end

    subgraph "Investigate Phase"
        I1[Deep research fix<br/>full analysis + planned fix]
    end

    R1 -.-> Q1
    R2 -.-> D1
    R3 -.-> I1
```

## 4. Worktree Lifecycle

```mermaid
sequenceDiagram
    participant F as Fixer
    participant W as Worktrees module
    participant WSL as WSL/Git
    participant K as kiro-cli

    F->>W: setup_worktree(branch, commit)
    W->>WSL: git fetch origin {branch}
    W->>WSL: git worktree add .worktrees/fix-{branch}-{sha}
    W-->>F: (wsl_path, name)

    F->>WSL: run_kiro(prompt, cwd=wsl_path)
    WSL->>K: kiro chat --trust-all-tools (in worktree dir)
    K-->>WSL: edits files
    WSL-->>F: (success, output)

    F->>WSL: git add -A (in worktree)
    F->>WSL: git diff --cached (gates check staged diff)

    alt Gates pass
        F->>W: commit_and_push(wsl_path, branch, msg)
        W->>WSL: git commit -m "fix: ..."
        W->>WSL: git push origin {branch}
    else Gates fail
        F->>W: discard_changes(wsl_path)
        W->>WSL: git checkout -- . && git clean -fd
    end

    F->>W: cleanup_worktree(name)
    W->>WSL: git worktree remove --force
```

## 5. Module Dependency Map

```mermaid
graph LR
    server[server.py<br/>HTTP + routing] --> fixers[fixers.py<br/>Fix orchestration]
    server --> security[security.py<br/>HMAC + IP filter]
    server --> tracker[tracker.py<br/>Pipeline state]
    server --> correlation[correlation.py<br/>Batch failures]
    server --> flaky[flaky.py<br/>Flaky detection]
    server --> trends[trends.py<br/>Duration tracking]

    fixers --> runner[runner.py<br/>wsl_bash + run_kiro]
    fixers --> worktrees[worktrees.py<br/>Git worktree mgmt]
    fixers --> gates[gates.py<br/>Quality gates]
    fixers --> classifier[classifier.py<br/>Error classification]
    fixers --> reproducer[reproducer.py<br/>Local reproduction]
    fixers --> history[history.py<br/>Fix attempt records]
    fixers --> decisions[decisions.py<br/>Strategy ranking]
    fixers --> memory[memory.py<br/>SimpleMem/LanceDB]

    gates --> runner
    worktrees --> runner
    reproducer --> runner

    runner --> config[config.py<br/>Paths + env vars]
    worktrees --> config
    gates --> config
```

## 6. Security & Request Flow

```mermaid
flowchart LR
    REQ[Incoming Request] --> IP{Source IP<br/>in allowlist?}
    IP -->|No| R403[403 Forbidden]
    IP -->|Yes| RL{Rate limit<br/>OK?}
    RL -->|No| R429[429 Too Many]
    RL -->|Yes| CL{Content-Length<br/>valid?}
    CL -->|No| R400[400 Bad Request]
    CL -->|Yes| CT{Content-Type<br/>= JSON?}
    CT -->|No| R400
    CT -->|Yes| SZ{Payload<br/>< max size?}
    SZ -->|No| R413[413 Too Large]
    SZ -->|Yes| HMAC{HMAC<br/>signature valid?}
    HMAC -->|No| R401[401 Unauthorized]
    HMAC -->|Yes| PARSE[Parse JSON]
    PARSE --> ROUTE{Route to<br/>handler}
    ROUTE --> RESPOND[200 OK immediately]
    RESPOND --> THREAD[Spawn background<br/>thread for processing]
```

## 7. Error Classification Tree

```mermaid
flowchart TD
    LOG[Error Log Text] --> DEP{Deploy patterns?}
    DEP -->|Yes| DEPLOY[deploy]
    DEP -->|No| ENV{Runner env<br/>patterns?}
    ENV -->|Yes| RUNNER[runner_environment]
    ENV -->|No| DEPN{Dependency<br/>patterns?}
    DEPN -->|Yes| DEPENDENCY[dependency]
    DEPN -->|No| FLK{Flaky<br/>patterns?}
    FLK -->|Yes| FLAKY[flaky]
    FLK -->|No| PBT{Proptest<br/>patterns?}
    PBT -->|Yes| PBT_F[pbt_failure]
    PBT -->|No| TST{test + failed?}
    TST -->|Yes| TEST[test_failure]
    TST -->|No| CQ{clippy/fmt?}
    CQ -->|Yes| CODE_Q[code_quality]
    CQ -->|No| BLD{Build<br/>patterns?}
    BLD -->|Yes| BUILD[build_failure]
    BLD -->|No| UNK[unknown]
```
