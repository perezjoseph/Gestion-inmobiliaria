# Kubernetes Multi-Environment Setup

Two isolated environments on the same k3s cluster with shared GPU-bound services.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  k3s cluster                                                    │
│                                                                 │
│  ┌─────────────────────────────────┐                            │
│  │ namespace: realestate (PROD)    │                            │
│  │  backend (2+ replicas, HPA)     │                            │
│  │  frontend (Caddy)               │                            │
│  │  postgres (isolated data)       │                            │
│  │  baileys                        │                            │
│  │  cloudflared ──► internet       │                            │
│  │  ocr-service (shared, GPU)      │                            │
│  │  ovms (shared, GPU)             │                            │
│  │  fdroid-repo                    │                            │
│  └─────────────────────────────────┘                            │
│                                                                 │
│  ┌─────────────────────────────────┐                            │
│  │ namespace: realestate-dev (DEV) │                            │
│  │  backend (1 replica)            │                            │
│  │  frontend (Caddy)               │                            │
│  │  postgres (isolated data)       │                            │
│  │  baileys                        │◄── gestion.local (LAN)     │
│  └─────────────────────────────────┘                            │
│                                                                 │
│  Traefik (kube-system) ─► gestion.local → dev frontend          │
│  Cloudflare Tunnel ─► gestion.myhomeva.us → prod frontend       │
└─────────────────────────────────────────────────────────────────┘
```

## Shared Resources

OCR service, OVMS, and F-Droid repo live in the `realestate` namespace and are accessed cross-namespace by both environments. These are GPU-bound singletons that shouldn't be duplicated.

Both `dev` and `prod` backends reference them via FQDN:
- `http://ocr-service.realestate.svc.cluster.local:8000`
- `http://ovms.realestate.svc.cluster.local:8000/v3`

## Deployment

### 1. Shared resources (once)

```bash
kubectl apply -k infra/k8s/app/shared/
```

### 2. Dev environment

```bash
# Create secrets (copy and fill in values)
cp infra/k8s/app/overlays/dev/secrets.example.yml infra/k8s/app/overlays/dev/secrets.yml
# Edit secrets.yml with real values, then:
kubectl apply -f infra/k8s/app/overlays/dev/secrets.yml
kubectl apply -k infra/k8s/app/overlays/dev/
```

Access: https://gestion.local (requires DNS or /etc/hosts pointing to the node IP)

### 3. Prod environment

```bash
# Set up Cloudflare Tunnel first:
cloudflared tunnel login
cloudflared tunnel create realestate-prod
cloudflared tunnel route dns realestate-prod gestion.myhomeva.us

# Create secrets (copy and fill in values)
cp infra/k8s/app/overlays/prod/secrets.example.yml infra/k8s/app/overlays/prod/secrets.yml
# Edit secrets.yml — include the tunnel credentials JSON
kubectl apply -f infra/k8s/app/overlays/prod/secrets.yml
kubectl apply -k infra/k8s/app/overlays/prod/
```

Access: https://gestion.myhomeva.us (public, via Cloudflare)

### 4. F-Droid repo (shared)

Still accessible at http://fdroid.local via the existing Traefik IngressRoute in `shared/`.

## Data Isolation

- Each environment has its own PostgreSQL instance with separate PVCs and credentials.
- WhatsApp sessions (baileys) are per-environment with different encryption keys.
- Uploads PVCs are per-environment.
- Backups run per-environment (prod only by default in base).

## Differences: Dev vs Prod

| Aspect | Dev | Prod |
|--------|-----|------|
| Namespace | `realestate-dev` | `realestate` |
| Backend replicas | 1 | 2-5 (HPA) |
| Access | `gestion.local` (LAN) | `gestion.myhomeva.us` (internet) |
| Ingress | Traefik IngressRoute | Cloudflare Tunnel |
| SMTP | not configured | mailcow |
| BCRD API | not configured | secret |
| PDB/HPA | no | yes |
| ENVIRONMENT var | `development` | `production` |
