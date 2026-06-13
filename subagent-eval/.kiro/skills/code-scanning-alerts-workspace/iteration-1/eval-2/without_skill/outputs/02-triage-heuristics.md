# Trivy Alert Triage: Base Image vs Our Code

## Overview

This document defines the classification logic for determining whether a Trivy
alert originates from a **base image** (OS packages, upstream binaries) or from
**our code** (application dependencies, Dockerfiles, IaC configs we author).

---

## Alert Sources in This Project

| SARIF Category | What Trivy Scans | Typical Findings |
|---|---|---|
| `trivy-backend-image` | Full container image `realestate-backend` | OS packages + app binary |
| `trivy-frontend-image` | Full container image `realestate-frontend` | OS packages + Caddy binary + static assets |
| `trivy-iac` | `infra/docker/` filesystem | Dockerfile misconfigurations |

---

## Base Images Used

| Image | Used In | Base For |
|---|---|---|
| `alpine:3.22` | Dockerfile.backend (runtime) | Backend production image |
| `alpine:3.23` | Dockerfile.frontend (runtime + prebuilt) | Frontend production image |
| `rust:1.88-alpine` | Dockerfile.backend (compile stage) | Build-only, not in final image |
| `rust:1.88-bookworm` | Dockerfile.frontend (compile stage) | Build-only, not in final image |
| `golang:1.26.3-alpine` | Dockerfile.frontend (caddy-builder) | Build-only, not in final image |

---

## Classification Heuristics

### Heuristic 1: Package Class (Primary Signal)

The most reliable signal is the **package type** reported in the Trivy finding.

| Package Type / Class | Classification | Rationale |
|---|---|---|
| `os` (apk, deb, rpm) | **Base Image** | Installed by `apk add` in base or our layer |
| `lang-go` (Go binary) | **Depends** — see Heuristic 2 | Caddy is built by us |
| `lang-rust` | **Our Code** | Compiled into our binary |
| `lang-python` | **Our Code** (if present) | Our pip dependencies |
| `lang-node` | **Our Code** (if present) | Our npm dependencies |

### Heuristic 2: File Path Analysis (Secondary Signal)

The `location.physicalLocation.artifactLocation.uri` in SARIF (or `path` in the
alert instance) tells us where the vulnerable component lives.

| Path Pattern | Classification | Rationale |
|---|---|---|
| `/usr/lib/`, `/lib/`, `/etc/` | **Base Image** | Alpine system paths |
| `/usr/share/`, `/var/` | **Base Image** | OS-managed locations |
| `/usr/local/bin/realestate-backend` | **Our Code** | Our compiled binary |
| `/usr/bin/caddy` | **Our Code** | We build Caddy from source with pinned deps |
| `/srv/` | **Our Code** | Our frontend static assets |
| `/app/` | **Our Code** | Application directory |
| `/etc/caddy/Caddyfile` | **Our Code** | Our configuration |

### Heuristic 3: CVE / Package Name Lookup

Cross-reference the vulnerable package name:

| Package Name Pattern | Classification |
|---|---|
| `alpine-baselayout`, `busybox`, `musl`, `zlib`, `libretls`, `ssl_client` | **Base Image** |
| `ca-certificates`, `libssl3`, `libpq`, `libcrypto3` | **Base Image** (installed explicitly but from Alpine repos) |
| `mailcap`, `curl`, `wget` | **Base Image** (utility packages) |
| Any Go module (`github.com/...`, `golang.org/...`) | **Our Code** — we build Caddy |
| Any Rust crate | **Our Code** — compiled into backend binary |

### Heuristic 4: SARIF Category Shortcut

| Category | Default Classification | Override Condition |
|---|---|---|
| `trivy-iac` | **Our Code** (always) | Dockerfile misconfigs are ours to fix |
| `trivy-backend-image` | Check pkg type | OS pkgs → Base Image; binary vulns → Our Code |
| `trivy-frontend-image` | Check pkg type | OS pkgs → Base Image; Go modules → Our Code |

### Heuristic 5: Layer Attribution (Advanced)

If using `trivy image --format json` with layer metadata:

- Layers from `alpine:3.22` / `alpine:3.23` base → **Base Image**
- Layers from our `RUN apk add ...` → **Gray area** (we chose to install it, but the vuln is upstream)
- Layers from `COPY --from=source` or `COPY --from=caddy-builder` → **Our Code**

---

## Decision Tree

```
START
  │
  ├─ Category == "trivy-iac"?
  │     └─ YES → OUR CODE (Dockerfile/IaC misconfig)
  │
  ├─ Package class == "os"?
  │     ├─ Package in explicit `apk add` list?
  │     │     ├─ YES → BASE IMAGE (upstream vuln, but note: we can pin/remove)
  │     │     └─ NO  → BASE IMAGE (came with alpine:3.2x)
  │     └─ (still base image either way, but flag if we explicitly install it)
  │
  ├─ Package class == "lang-go"?
  │     ├─ Path contains `/usr/bin/caddy`?
  │     │     └─ YES → OUR CODE (we build Caddy, we pin its deps)
  │     └─ NO  → BASE IMAGE (unlikely, but possible from Go toolchain)
  │
  ├─ Package class == "lang-rust" or path == `/usr/local/bin/realestate-backend`?
  │     └─ YES → OUR CODE (our Rust dependencies)
  │
  ├─ Path starts with `/srv/` or `/app/` or `/etc/caddy/`?
  │     └─ YES → OUR CODE
  │
  └─ DEFAULT → BASE IMAGE (conservative: if unclear, it's likely from Alpine)
```

---

## Remediation Guidance by Classification

### Base Image Alerts

1. **Upgrade base image tag** — bump `alpine:3.22` → latest patch or next minor
2. **Wait for upstream fix** — if no patched Alpine package exists yet
3. **Dismiss with note** — if the CVE is not exploitable in our context (no network exposure to vulnerable path)
4. **Pin specific package version** — `apk add libssl3=3.x.y-r0` if a fix is available before next Alpine release

### Our Code Alerts

1. **Update Cargo.lock** — `cargo update -p <crate>` for Rust dependency vulns
2. **Update Caddy dependency pins** — modify `go get` lines in Dockerfile.frontend
3. **Fix Dockerfile misconfiguration** — for trivy-iac findings (e.g., missing USER, exposed secrets)
4. **Update application code** — if the vulnerability is in how we use a library

---

## Packages We Explicitly Install (Gray Area)

These are installed by us via `apk add` but are upstream Alpine packages.
A vulnerability here is still "base image" in origin but we have agency to remove
or pin them.

### Backend Runtime
- `ca-certificates`, `libssl3`, `libpq`, `curl`

### Frontend Runtime
- `ca-certificates`, `mailcap`, `wget`

**Recommendation**: Classify as **Base Image** for triage priority, but track
separately as "removable base deps" — if we don't need `curl`/`wget` at runtime,
removing them eliminates the attack surface entirely.
