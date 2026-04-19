# Kubernetes Manifests (k3s)

Manifests for deploying the application and SonarQube to a k3s cluster.

## Prerequisites

- k3s cluster with Traefik ingress (default)
- `kubectl` configured to access the cluster
- GHCR image pull secret in the `realestate` namespace
- `vm.max_map_count >= 262144` on the node (for SonarQube/Elasticsearch)

## Structure

```
k8s/
├── namespace.yml              # realestate + sonarqube namespaces
├── app/
│   ├── kustomization.yml      # kustomize entry point
│   ├── secret.yml             # secret templates (DO NOT commit real values)
│   ├── ghcr-secret.yml        # instructions for GHCR pull secret
│   ├── postgres.yml           # PostgreSQL deployment + PVC
│   ├── backend.yml            # Backend API deployment + uploads PVC
│   ├── frontend.yml           # Frontend (Caddy) deployment
│   ├── ocr-service.yml        # OCR service deployment
│   └── ingress.yml            # Traefik IngressRoute
└── sonarqube/
    ├── kustomization.yml      # kustomize entry point
    ├── secret.yml             # secret template
    ├── postgres.yml           # SonarQube PostgreSQL
    └── sonarqube.yml          # SonarQube server + IngressRoute
```

## Initial Setup

```bash
# 1. Create namespaces
kubectl apply -f k8s/namespace.yml

# 2. Create GHCR pull secret
kubectl create secret docker-registry ghcr-login \
  --namespace realestate \
  --docker-server=ghcr.io \
  --docker-username=<github-user> \
  --docker-password=<github-pat>

# Patch default service account to use it
kubectl patch serviceaccount default -n realestate \
  -p '{"imagePullSecrets": [{"name": "ghcr-login"}]}'

# 3. Create application secrets
kubectl create secret generic realestate-db-secret \
  --namespace realestate \
  --from-literal=username=realestate \
  --from-literal=password=<db-password>

kubectl create secret generic realestate-db-url \
  --namespace realestate \
  --from-literal=DATABASE_URL=postgresql://realestate:<db-password>@db:5432/realestate

kubectl create secret generic realestate-app-secret \
  --namespace realestate \
  --from-literal=jwt-secret=<jwt-secret>

# 4. Deploy application
kubectl apply -k k8s/app/

# 5. Create SonarQube secrets
kubectl create secret generic sonarqube-db-secret \
  --namespace sonarqube \
  --from-literal=username=sonar \
  --from-literal=password=<sonar-db-password>

# 6. Set vm.max_map_count (required for SonarQube)
sudo sysctl -w vm.max_map_count=262144
echo "vm.max_map_count=262144" | sudo tee /etc/sysctl.d/99-sonarqube.conf

# 7. Deploy SonarQube
kubectl apply -k k8s/sonarqube/
```

## CI/CD

The deploy workflow (`deploy.yml`) runs automatically after container images are built and attested. It:

1. Verifies image attestations via `gh attestation verify`
2. Updates secrets via `kubectl create secret --dry-run=client | kubectl apply`
3. Applies manifests via `kubectl apply -k k8s/app/`
4. Sets image digests via `kubectl set image`
5. Waits for rollout via `kubectl rollout status`
6. Runs a health check against the backend `/health` endpoint
7. Rolls back on failure via `kubectl rollout undo`

## Storage

All PVCs use `local-path` (k3s default). Data lives on the node's local disk. For multi-node clusters, consider switching to Longhorn or an NFS provisioner.
