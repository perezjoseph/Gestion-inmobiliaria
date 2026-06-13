# Trivy Alert Triage Heuristics

## Overview

This document describes the complete classification logic for triaging Trivy alerts
in the `perezjoseph/Gestion-inmobiliaria` repository. The goal is to separate alerts
originating from **base images** (not directly our fault) from those in **our code or
dependencies** (actionable by the team).

## Repository Container Images

| Image | Dockerfile | Base Image | Scan Category |
|-------|-----------|------------|---------------|
| realestate-backend | `infra/docker/Dockerfile.backend` | `alpine:3.22` | `trivy-backend-image` |
| realestate-frontend | `infra/docker/Dockerfile.frontend` | `alpine:3.23` (+ `golang:1.26.3-alpine` caddy-builder) | `trivy-frontend-image` |
| baileys-service | `baileys-service/Dockerfile` | `node:24-alpine` | (if scanned) |
| ocr-service | `ocr-service/Dockerfile` | `python:3.12-slim` | (if scanned) |
| actions-runner | `infra/docker/Dockerfile.runner` | `ghcr.io/actions/actions-runner:2.334.0` | (if scanned) |

## Classification Categories

### 1. BASE_IMAGE_OS_PACKAGE

**Indicator:** The alert path is either:
- A bare image reference like `perezjoseph/realestate-backend` or `library/alpine` (no subpath, no file extension)
- A system library path like `usr/lib/`, `lib/`, `usr/share/` with `start_line = 1`

**What it means:** The vulnerability exists in an OS package (e.g., `libcrypto3`, `musl`, `zlib`) that ships with the base image. We did NOT install this package explicitly.

**Fix approach:**
- Bump the `FROM` tag to a newer patch version (e.g., `alpine:3.22` → `alpine:3.23`)
- Add `apk upgrade --no-cache` (Alpine) or `apt-get update && apt-get upgrade -y` (Debian) before installing app dependencies
- Rebuild and push

**Priority:** Lower — unless the CVE is actively exploited. Base image maintainers usually patch within days.

---

### 2. BASE_IMAGE_NPM_BUNDLED

**Indicator:** Path matches `usr/local/lib/node_modules/npm/**`

**What it means:** The CVE is in a transitive dependency bundled inside the `npm` binary that ships with the Node.js base image. Common culprits: `tar`, `minimatch`, `glob`, `cross-spawn`, `brace-expansion`, `ip-address`.

**Fix approach:**
- Bump the Node base image tag (already at `node:24-alpine` which has npm 11)
- Add `RUN npm install -g npm@latest` early in the Dockerfile (already done in baileys-service)
- Wait for upstream Node image rebuild if patched npm isn't released yet

**Priority:** Low-Medium — these are in the package manager itself, not in app code. Exploitability depends on whether the vulnerable npm subpackage is reachable during runtime (it usually isn't since npm only runs at build time).

---

### 3. OUR_DOCKERFILE_MISCONFIG

**Indicator:** Rule ID matches pattern `DS\d{4}` (Trivy misconfiguration scanner rules)

**What it means:** Our Dockerfile has a configuration weakness detected by Trivy's IaC scanner. Examples:
- `DS002`: RUN with sudo
- `DS005`: ADD instead of COPY
- `DS012`: Maintainer deprecated
- `DS026`: No HEALTHCHECK

**Fix approach:** Edit the Dockerfile directly. These are authoring issues in files we own.

**Priority:** Medium-High — these are entirely within our control.

---

### 4. OUR_DEPENDENCY

**Indicator:** Path matches one of:
- `app/node_modules/**` — npm packages we installed via `npm ci`
- `usr/local/lib/python3.12/site-packages/**` — Python packages from `requirements.txt`
- Go module paths or `/usr/bin/caddy` — Go deps in our caddy-builder stage
- Intel GPU package paths — packages we explicitly curl/dpkg in ocr-service

**What it means:** The vulnerability is in a package that WE chose to install. Even though it's inside a container, we have direct control over the version.

**Fix approach:**
- **Node (baileys-service):** Run `npm audit fix` or bump the vulnerable package in `package.json`
- **Python (ocr-service):** Bump version in `requirements.txt`
- **Go (frontend caddy-builder):** Update `go get` version pins in `Dockerfile.frontend`
- **Intel packages (ocr-service):** Bump download URLs to newer release

**Priority:** High — we own these dependency choices.

---

### 5. OUR_CODE

**Indicator:** Path under `backend/`, `frontend/`, `baileys-service/src/`, `ocr-service/*.py`

**What it means:** Trivy (or its IaC scanner) found a source-code-level issue. This is rare for Trivy (which focuses on packages), but possible for embedded secrets or hardcoded values.

**Fix approach:** Fix the source code directly.

**Priority:** High.

---

## Decision Tree (Flowchart)

```
Alert received from Trivy
│
├─ Rule ID matches DS\d{4}?
│  └─ YES → OUR_DOCKERFILE_MISCONFIG
│
├─ Path is bare image ref (owner/image, no subpath)?
│  └─ YES → BASE_IMAGE_OS_PACKAGE
│
├─ Path starts with usr/local/lib/node_modules/npm/?
│  └─ YES → BASE_IMAGE_NPM_BUNDLED
│
├─ Path starts with app/node_modules/?
│  └─ YES → OUR_DEPENDENCY (Node)
│
├─ Path starts with usr/local/lib/python*/site-packages/?
│  └─ YES → OUR_DEPENDENCY (Python)
│
├─ Path contains go/pkg/mod or usr/bin/caddy?
│  └─ YES → OUR_DEPENDENCY (Go/Caddy)
│
├─ Path contains intel-igc|libigdgmm|intel-opencl?
│  └─ YES → OUR_DEPENDENCY (Intel GPU)
│
├─ Path starts with usr/lib|lib/|usr/share AND start_line <= 1?
│  └─ YES → BASE_IMAGE_OS_PACKAGE
│
├─ Path under backend/|frontend/|baileys-service/src/|ocr-service/?
│  └─ YES → OUR_CODE
│
└─ Otherwise → UNCLASSIFIED (manual review)
```

## Action Summary Matrix

| Category | Who Fixes | How | Urgency |
|----------|-----------|-----|---------|
| BASE_IMAGE_OS_PACKAGE | DevOps (image rebuild) | Bump FROM tag or add OS upgrade | Low unless actively exploited |
| BASE_IMAGE_NPM_BUNDLED | DevOps (image rebuild) | Bump node image or `npm i -g npm@latest` | Low (build-time only) |
| OUR_DOCKERFILE_MISCONFIG | Dev team | Edit Dockerfile | Medium-High |
| OUR_DEPENDENCY | Dev team | Bump package version | High |
| OUR_CODE | Dev team | Fix source code | High |

## Dismissal Guidance

Alerts that qualify for dismissal as `won't_fix`:
- BASE_IMAGE_OS_PACKAGE with no available fix upstream and CVSS < 7.0
- BASE_IMAGE_NPM_BUNDLED where the vulnerable code path is unreachable at runtime

Alerts that qualify for dismissal as `false_positive`:
- Trivy reports a CVE in a package version that has already been patched (version mismatch in Trivy DB)
- The vulnerable function is not called/exposed in our usage

Never dismiss OUR_DEPENDENCY or OUR_CODE alerts without a documented justification.
