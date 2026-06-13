---
inclusion: fileMatch
fileMatchPattern: [".github/workflows/*.yml"]
---

# Workflow Retry Policy

All GitHub Actions workflow steps that can fail due to infrastructure flakiness must use `nick-fields/retry` (pinned to SHA `ad984534de44a9489a53aefd81eb77f87c70dc60`, v4.0.0).

## What to wrap

- Compilation and test steps (cargo build, cargo test, tarpaulin, trunk build, gradle): 30 min timeout, 2 attempts.
- Container image scans (trivy image): 20 min timeout, 2 attempts.
- Tool installs (curl + tar downloads): 10 min timeout, 3 attempts, 10s retry_wait_seconds.
- Security scans (cargo audit, cargo deny): 10 min timeout, 2 attempts.
- Deploy rollout waits (kubectl rollout status): 10 min timeout, 2 attempts.
- Health checks: 5 min timeout, 2 attempts.
- API calls (gh CLI, curl to external services): 3 min timeout, 3 attempts, 10s retry_wait_seconds.
- SonarQube scanner: 30 min timeout, 2 attempts, 30s retry_wait_seconds.

## What NOT to wrap

- Deterministic lint/format checks (rustfmt, clippy, detekt, spotless) — if they fail, the code has issues.
- `kubectl set image` or any mutating deploy command — retrying partial deploys is dangerous.
- Change detection (paths-filter) — lightweight, no network dependency.
- Checkout steps — handled by actions/checkout's own retry logic.

## Conventions

- Always pin to full commit SHA with version comment: `nick-fields/retry@ad984534de44a9489a53aefd81eb77f87c70dc60 # v4.0.0`.
- Preserve `tee` to temp files when error collection steps depend on the output.
- Use `exit ${PIPESTATUS[0]}` after piped commands to propagate the correct exit code.
- `working-directory` does not apply to retry action commands — use `cd` inside the command block instead.
- `env` goes at step level (sibling to `uses`), not inside `with`.
