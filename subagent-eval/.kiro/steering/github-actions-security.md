---
inclusion: fileMatch
fileMatchPattern: [".github/**"]
---

# GitHub Actions Security Best Practices

Rules derived from official GitHub documentation. Sources:
- https://docs.github.com/en/actions/reference/security/secure-use
- https://wellarchitected.github.com/library/application-security/recommendations/actions-security/

## Permissions
- Top-level `permissions: {}` on every workflow. Grant per-job only.
- Never use workflow-level permissions that apply to all jobs.

## Third-Party Actions
- Pin all third-party actions to full-length commit SHA. Add version comment on same line.
- Use `persist-credentials: false` on all `actions/checkout` steps.
- Prefer `actions/attest@<sha>` over `actions/attest-build-provenance` (deprecated wrapper).

## Secrets
- Never use `secrets: inherit`. Pass only the secrets each called workflow needs.
- Every `workflow_call` must declare its required secrets explicitly.
- Use `::add-mask::` for any dynamically generated sensitive value.

## Concurrency
- Use `cancel-in-progress: ${{ github.event_name == 'pull_request' }}` so PR runs cancel stale jobs but main-branch runs complete.

## Attestation (Container Images)
- Use `docker/build-push-action` with `push: true` and consume its `digest` output for attestation.
- Never extract digest via `docker inspect` after a manual `docker push` — causes race condition 404s.
- `subject-name` must be fully-qualified image name without tag.

## CODEOWNERS
- `.github/workflows/` and `.github/actions/` must be listed in CODEOWNERS requiring review.
