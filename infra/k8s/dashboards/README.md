# Grafana Dashboards

Organized by Grafana folder:

```
dashboards/
├── infrastructure/    → "Infrastructure" folder in Grafana
│   ├── grafana-xpum-dashboard.json
│   ├── grafana-node-exporter-dashboard.json
│   ├── grafana-vllm-dashboard.json
│   └── grafana-logs-dashboard.json
└── application/       → "Application" folder in Grafana
    ├── grafana-backend-api-dashboard.json
    └── grafana-slo-dashboard.json
```

## Applying Dashboard Changes

After editing a dashboard JSON, recreate its ConfigMap:

```bash
# Application dashboards
kubectl create configmap grafana-dashboard-backend-api \
  --namespace monitoring \
  --from-file=backend-api-dashboard.json=application/grafana-backend-api-dashboard.json \
  --dry-run=client -o yaml | kubectl apply -f -

kubectl create configmap grafana-dashboard-slo \
  --namespace monitoring \
  --from-file=slo-dashboard.json=application/grafana-slo-dashboard.json \
  --dry-run=client -o yaml | kubectl apply -f -

# Infrastructure dashboards
kubectl create configmap grafana-dashboard-xpum \
  --namespace monitoring \
  --from-file=xpum-dashboard.json=infrastructure/grafana-xpum-dashboard.json \
  --dry-run=client -o yaml | kubectl apply -f -

kubectl create configmap grafana-dashboard-node \
  --namespace monitoring \
  --from-file=node-exporter-dashboard.json=infrastructure/grafana-node-exporter-dashboard.json \
  --dry-run=client -o yaml | kubectl apply -f -

kubectl create configmap grafana-dashboard-vllm \
  --namespace monitoring \
  --from-file=vllm-dashboard.json=infrastructure/grafana-vllm-dashboard.json \
  --dry-run=client -o yaml | kubectl apply -f -

kubectl create configmap grafana-dashboard-logs \
  --namespace monitoring \
  --from-file=logs-dashboard.json=infrastructure/grafana-logs-dashboard.json \
  --dry-run=client -o yaml | kubectl apply -f -
```

Grafana auto-reloads provisioned dashboards on ConfigMap change (no pod restart needed).

## Adding a New Dashboard

1. Create the JSON file in the appropriate subfolder
2. Add a volume + volumeMount in `monitoring.yml` (Grafana deployment)
3. Create the ConfigMap (see commands above)
4. Apply `monitoring.yml` to pick up the new mount
