# Kiro Autofix Workflow

End-to-end flow for the autofix system. CI/Semgrep workflows complete, the
trigger workflow discovers per-job diagnostics artifacts, and a single
sequential queue produces one focused commit per failing job — pushed once
at the end.

## Overall flow

```mermaid
flowchart TD
    Start([Developer pushes commit]) --> CI[CI workflow runs]
    Start --> Semgrep[Semgrep workflow runs]

    CI --> CIJobs{Jobs<br/>complete}
    Semgrep --> SemgrepJobs{Jobs<br/>complete}

    CIJobs -->|All pass| Done([CI green ✅])
    CIJobs -->|≥1 fails| UploadCI[Each failing job uploads<br/>diag-* artifact via<br/>collect-diagnostics action]
    SemgrepJobs -->|≥1 fails| UploadSem[diag-semgrep-* uploaded]

    UploadCI --> Trigger
    UploadSem --> Trigger

    Trigger[workflow_run trigger fires<br/>kiro-autofix-trigger.yml]
    Trigger --> BuildQueue[autofix-build-queue job<br/>runs for ALL branches]
    Trigger --> MainCheck{Branch<br/>== main<br/>AND push?}

    MainCheck -->|No| NoDeployFix([Deploy autofix<br/>not applicable])
    MainCheck -->|Yes| DeployPrecheck{Deploy artifact<br/>present?}
    DeployPrecheck -->|Yes| DeployFix[kiro-autofix-deploy<br/>single artifact]
    DeployPrecheck -->|No| DeploySkip([Deploy autofix skipped<br/>CI failed before deploy stage])

    BuildQueue --> Checkout[Checkout branch<br/>fetch-depth: 0<br/>persist-credentials: true]
    Checkout --> CountAttempts{Prior kiro-bot<br/>commits at HEAD<br/>≥ MAX_ATTEMPTS?}

    CountAttempts -->|Yes| Notify[Comment on PR:<br/>manual intervention<br/>required]
    Notify --> Fail([Job fails])

    CountAttempts -->|No| Setup[Rust setup +<br/>verify LSP servers on PATH]
    Setup --> ListArtifacts[gh api lists<br/>diag-* artifacts<br/>priority-sorted]

    ListArtifacts --> HasArtifacts{Artifacts<br/>found?}
    HasArtifacts -->|No| Skip([No autofix needed])
    HasArtifacts -->|Yes| Download[Download all<br/>diag-* artifacts<br/>from CI run]

    Download --> Loop[Process queue:<br/>see next diagram]
    Loop --> CumulativeVerify{cargo fmt +<br/>clippy still<br/>passing?}

    CumulativeVerify -->|Yes| Push
    CumulativeVerify -->|No| Fixup[Kiro fixup pass:<br/>one extra commit<br/>resolves cross-cutting issues]
    Fixup --> Push

    Push[git push with retry:<br/>fetch + rebase if<br/>origin advanced]
    Push -->|Success| NewCI[CI re-runs<br/>on new HEAD]
    Push -->|Fails after retries| PushFail[Comment on PR:<br/>push failed]

    NewCI --> CIJobs

    DeployFix --> KubeFix[kubectl logs/events<br/>fed to Kiro<br/>direct push to main]

    classDef success fill:#d4edda,stroke:#28a745
    classDef failure fill:#f8d7da,stroke:#dc3545
    classDef process fill:#cce5ff,stroke:#004085
    classDef decision fill:#fff3cd,stroke:#856404

    class Done,Skip,DeploySkip,NoDeployFix success
    class Fail,Notify,PushFail failure
    class CI,Semgrep,UploadCI,UploadSem,Trigger,Checkout,Setup,ListArtifacts,Download,Loop,Fixup,Push,NewCI,DeployFix,KubeFix,BuildQueue process
    class CIJobs,SemgrepJobs,MainCheck,CountAttempts,HasArtifacts,CumulativeVerify,DeployPrecheck decision
```

## Sequential queue (the loop body)

```mermaid
flowchart TD
    Start([Begin queue<br/>N artifacts priority-sorted]) --> Pick[Pick next artifact<br/>build > lint > container ><br/>security > semgrep > codeql]

    Pick --> Empty{context.txt<br/>empty?}
    Empty -->|Yes| Next1[Skip artifact]
    Empty -->|No| BuildPrompt[Write prompt to file<br/>git history + single<br/>artifact diagnostics]

    BuildPrompt --> Kiro[kiro-cli chat<br/>--no-interactive<br/>via stdin]
    Kiro --> Diff{git diff<br/>has changes?}

    Diff -->|No| Next2[Skip — Kiro made<br/>no edits]
    Diff -->|Yes| Commit[git commit -F<br/>commit-message-artifact.txt<br/>HEAD advances]

    Commit --> Inc[COMMITS_MADE++]
    Inc --> More{More<br/>artifacts?}
    Next1 --> More
    Next2 --> More

    More -->|Yes| Pick
    More -->|No| Done([Queue complete<br/>N commits local<br/>not yet pushed])

    classDef accept fill:#d4edda,stroke:#28a745
    classDef skip fill:#e2e3e5,stroke:#6c757d
    classDef action fill:#cce5ff,stroke:#004085
    classDef decision fill:#fff3cd,stroke:#856404

    class Done accept
    class Next1,Next2 skip
    class Pick,BuildPrompt,Kiro,Commit,Inc action
    class Empty,Diff,More decision
```

## Diagnostics artifact production

```mermaid
flowchart LR
    subgraph CI["CI / Semgrep workflows"]
        Backend[Backend job<br/>clippy + tests + build]
        Frontend[Frontend job<br/>clippy + tests + build]
        Lint[Lint jobs<br/>fmt, baileys-tsc]
        Container[Container jobs<br/>backend, frontend,<br/>baileys, ocr]
        Security[Security jobs<br/>cargo-deny, trivy-iac,<br/>gitleaks]
        SemgrepJob[Semgrep jobs<br/>ci, community]
        CodeQL[CodeQL jobs<br/>rust, js, python, kotlin]
    end

    subgraph Composite["collect-diagnostics composite action"]
        Build[Build context.txt<br/>job-name + step-outcomes<br/>+ tail of output files]
        Upload[Upload as<br/>diag-{suffix} artifact]
    end

    Backend -.->|on failure| Composite
    Frontend -.->|on failure| Composite
    Lint -.->|on failure| Composite
    Container -.->|on failure| Composite
    Security -.->|on failure| Composite
    SemgrepJob -.->|on failure| Composite
    CodeQL -.->|on failure| Composite

    Build --> Upload
    Upload --> Storage[(diag-build-backend<br/>diag-build-frontend<br/>diag-lint-fmt<br/>diag-container-baileys<br/>diag-security-cargo-deny<br/>diag-semgrep-ci<br/>diag-codeql-rust<br/>...)]

    Storage --> Discovery[Trigger workflow<br/>discovers via gh api]
```

## Loop protection

```mermaid
flowchart TD
    Start([Queue produces N commits<br/>+ optional fixup]) --> Push[git push]
    Push --> NewCI[New CI run starts]
    NewCI --> Result{All<br/>passing?}

    Result -->|Yes| Green([CI green — done])
    Result -->|No, different jobs fail| NewQueue[New autofix-trigger run<br/>queues behind current<br/>concurrency: false]
    Result -->|No, same jobs fail| Loop[Counter increments]

    NewQueue --> CheckCount{Cumulative<br/>kiro-bot<br/>commits ≥ 5?}
    Loop --> CheckCount

    CheckCount -->|No| ProcessAgain[Process queue<br/>again]
    CheckCount -->|Yes| HumanCall[Comment on PR<br/>halt autofix]

    ProcessAgain --> Push

    classDef stop fill:#f8d7da,stroke:#dc3545
    classDef ok fill:#d4edda,stroke:#28a745
    classDef proc fill:#cce5ff,stroke:#004085

    class Green ok
    class HumanCall stop
    class Push,NewCI,NewQueue,ProcessAgain,Loop proc
```

## Key files

- `.github/actions/collect-diagnostics/action.yml` — composite action used by every job to upload `diag-*` artifacts on failure
- `.github/workflows/kiro-autofix-trigger.yml` — the `workflow_run` listener that runs the sequential queue
- `.github/workflows/kiro-autofix.yml` — reusable workflow for the deploy stage (single artifact, no queue)
