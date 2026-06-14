# Observability Metrics Expansion — Implementation Plan

## Summary

Expand Prometheus metrics coverage across the realestate stack in five risk-ordered phases: unlock the already-configured Tempo remote-write, scrape Traefik, add a least-privilege postgres-exporter, and instrument the baileys and ocr services — each with a matching provisioned Grafana dashboard.

## Grounding (verified against source)

Every claim below was read from the actual files:

- `infra/k8s/monitoring.yml` — Prometheus Deployment `prometheus` (image `docker.io/prom/prometheus:v3.4.0`) runs with only `--config.file`, `--storage.tsdb.path`, `--storage.tsdb.retention.time=30d`. No remote-write receiver, no exemplar feature flag. Static `scrape_configs` live in ConfigMap `prometheus-config`. Grafana mounts each dashboard JSON from its own ConfigMap (`grafana-dashboard-*`) via subPath; provider config in ConfigMap `grafana-dashboard-providers` points at `/var/lib/grafana/dashboards/{infrastructure,application}`.
- `infra/k8s/tempo.yml` — `metrics_generator` already runs processors `[service-graphs, span-metrics]` and remote-writes to `http://prometheus.monitoring.svc.cluster.local:9090/api/v1/write` with `send_exemplars: true`. This is the silent 404 source: Prometheus has no `/api/v1/write` receiver enabled.
- `infra/k8s/traefik-config.yml` — k3s `HelmChartConfig` named `traefik` in `kube-system`, currently only sets image/nodeSelector/tolerations. No metrics block.
- `infra/k8s/app/base/postgres.yml` — Deployment `db`, image `postgres:16`, single container `postgres`, creds from Secret `realestate-db-secret`. `readOnlyRootFilesystem: true`, `runAsUser: 999`.
- `infra/k8s/app/base/baileys.yml` — Deployment `baileys`, container port 3100, Service `baileys` (realestate ns). `src/index.ts` is an Express 5 app; `/health` is unauthenticated, `app.use('/sessions', authMiddleware)` gates session routes only. Connection state available via `getConnectionCounts()` in `src/session-manager.ts`. Reconnect / logout transitions live in `src/session-manager.ts` (`scheduleReconnect`, `handleConnectionClose`) and `src/reconnect.ts`. `package.json` deps are exact-pinned.
- `infra/k8s/app/shared/ocr-service.yml` — Deployment `ocr-service` (realestate ns), container port 8000, Service `ocr-service`. `main.py` is FastAPI `app = FastAPI(title="OCR Service")`; OCR work routed through `ocr_engine.OpenVINOOCREngine.predict()`. `requirements.txt` is exact-pinned.
- `infra/k8s/app/base/network-policy.yml` — realestate ns is default-deny ingress + egress. Prometheus scrape of `backend` is allowed via `allow-backend-ingress` (monitoring ns → 8080); vllm via `allow-vllm-metrics-ingress` (monitoring ns → 8000). There is **no** ingress policy for `ocr-service` and the baileys ingress policy allows **only** backend → 3100.
- `infra/k8s/monitoring-network-policy.yml` — `allow-prometheus-ingress` already permits Tempo → 9090 (so remote-write is not blocked by netpol once the receiver is on).
- `backend/migrations/m20260510_000002_create_whatsapp_auth_tables.rs` — the canonical pattern for creating a dedicated PG role via `execute_unprepared` `DO $$ ... CREATE ROLE ... $$` guarded by `pg_roles` existence check, plus `GRANT`/`REVOKE` in `up`/`down`. `mod.rs` registers each migration in both the `pub mod` list and the `migrations()` vec.
- `.kiro/powers/grafana/mcp.json` — the grafana MCP server's `autoApprove` list is read-only (`search_dashboards`, `get_dashboard_*`, `query_prometheus`, etc.); `update_dashboard` is **not** auto-approved and the service-account token is a placeholder.

## Dashboard delivery decision — provisioned ConfigMap JSON (GitOps)

Use **provisioned ConfigMap JSON committed to the repo**, not the grafana MCP `update_dashboard` tool.

Rationale: this repo is GitOps — dashboards are version-controlled JSON under `infra/k8s/dashboards/{application,infrastructure}/`, mounted into Grafana via per-dashboard ConfigMaps declared in `monitoring.yml`, and Grafana auto-reloads provisioned dashboards on ConfigMap change (per `infra/k8s/dashboards/README.md`). The MCP server here is configured read-only (`update_dashboard` not in `autoApprove`, placeholder token), and any dashboard created through the API would live only in Grafana's PVC — invisible to git and wiped on a clean reprovision. MCP read tools (`query_prometheus`, `list_prometheus_metric_names`) are still the right way to **verify** metrics exist after each phase.

Each new dashboard therefore requires four mechanical edits (same as README "Adding a New Dashboard"):
1. JSON file in the correct subfolder.
2. `volume` + `volumeMount` (subPath) in the Grafana Deployment in `monitoring.yml`.
3. A `grafana-dashboard-<name>` ConfigMap (created via the `kubectl create configmap ... --dry-run=client -o yaml | kubectl apply -f -` idiom).
4. Re-apply `monitoring.yml` to attach the new mount (this restarts the Grafana pod).

---

## Phase 1 — Prometheus remote-write receiver + exemplars + Tracing/Service RED dashboard

**Risk:** Low. Value: High (unblocks metrics Tempo already emits). Do first.

### Affected files

| Path | Action | Change |
|---|---|---|
| `infra/k8s/monitoring.yml` | modify | Add two args to the `prometheus` container; add Grafana volume + volumeMount for the new dashboard |
| `infra/k8s/dashboards/application/grafana-tracing-service-red-dashboard.json` | create | New dashboard using `traces_spanmetrics_*` / `traces_service_graph_*` |

### Steps

1. **`infra/k8s/monitoring.yml`** — in the `prometheus` Deployment `args`, append:
   ```yaml
           - "--web.enable-remote-write-receiver"
           - "--enable-feature=exemplar-storage"
   ```
   Why: `metrics_generator` in `tempo.yml` already remote-writes spanmetrics/service-graph series with exemplars; the receiver flag opens `/api/v1/write`, the feature flag enables exemplar TSDB storage so `send_exemplars: true` lands instead of being dropped.

2. **`infra/k8s/dashboards/application/grafana-tracing-service-red-dashboard.json`** — create dashboard (datasource templated `${datasource}` → Prometheus uid `PBFA97CFB590B2093`, matching the existing database dashboard convention). Panels:
   - Request rate: `sum by (service) (rate(traces_spanmetrics_calls_total[5m]))`
   - Error rate: `sum by (service) (rate(traces_spanmetrics_calls_total{status_code="STATUS_CODE_ERROR"}[5m]))`
   - Latency p50/p95/p99: `histogram_quantile(0.95, sum by (le, service) (rate(traces_spanmetrics_latency_bucket[5m])))`
   - Service graph throughput/errors: `traces_service_graph_request_total`, `traces_service_graph_request_failed_total`
   - Enable exemplars on the latency panels (`"exemplar": true` in the target) so trace links surface in Grafana.

   Note: exact metric names depend on Tempo's generator config; the canonical names for Tempo 2.7.2 are `traces_spanmetrics_calls_total`, `traces_spanmetrics_latency_bucket`, `traces_service_graph_request_total`. Confirm the live names in step Verify before finalizing panel queries.

3. **`infra/k8s/monitoring.yml`** (Grafana) — add volume + volumeMount:
   ```yaml
   # volumeMounts (application dashboards block)
   - name: dashboard-tracing-service-red
     mountPath: /var/lib/grafana/dashboards/application/tracing-service-red-dashboard.json
     subPath: tracing-service-red-dashboard.json
   # volumes
   - name: dashboard-tracing-service-red
     configMap:
       name: grafana-dashboard-tracing-service-red
   ```

### Commands

```powershell
kubectl apply -f infra/k8s/monitoring.yml

# Dashboard ConfigMap (run from infra/k8s/dashboards/)
kubectl create configmap grafana-dashboard-tracing-service-red `
  --namespace monitoring `
  --from-file=tracing-service-red-dashboard.json=application/grafana-tracing-service-red-dashboard.json `
  --dry-run=client -o yaml | kubectl apply -f -

kubectl rollout status deploy/prometheus -n monitoring
kubectl rollout status deploy/grafana -n monitoring
```

### Verify

```powershell
# Receiver flag is live
kubectl -n monitoring exec deploy/prometheus -- wget -qO- http://localhost:9090/api/v1/status/flags | findstr remote-write

# Tempo no longer 404s on remote-write
kubectl -n monitoring logs deploy/tempo | findstr /api/v1/write   # expect no 404 after restart

# Metrics arriving (PromQL via port-forward or grafana MCP query_prometheus)
kubectl -n monitoring port-forward svc/prometheus 9090:9090
#   then: curl "http://localhost:9090/api/v1/query?query=traces_spanmetrics_calls_total"
#   expect non-empty result vector; also confirm traces_service_graph_request_total exists
```
Generate traffic first (hit backend endpoints) so the generator has spans to aggregate. Confirm exemplars: query `traces_spanmetrics_latency_bucket` and check `/api/v1/query_exemplars`.

### Rollback

Remove the two args and the dashboard volume/volumeMount from `monitoring.yml`, `kubectl apply -f infra/k8s/monitoring.yml`, and `kubectl delete configmap grafana-dashboard-tracing-service-red -n monitoring`. Tempo returns to silently 404-ing (its prior state) — no data loss, the generator's WAL just isn't consumed. Reversible.

---

## Phase 2 — Traefik built-in Prometheus metrics + edge/ingress dashboard

**Risk:** Low/Medium (touches the k3s-managed ingress controller; a malformed HelmChartConfig can disrupt ingress). Value: High.

### Affected files

| Path | Action | Change |
|---|---|---|
| `infra/k8s/traefik-config.yml` | modify | Add `metrics.prometheus` on a dedicated `metrics` entryPoint (port 9100), expose it |
| `infra/k8s/monitoring.yml` | modify | Add scrape job `traefik` to `prometheus-config`; add Grafana volume/mount for dashboard |
| `infra/k8s/dashboards/infrastructure/grafana-traefik-edge-dashboard.json` | create | Edge/ingress dashboard |

### Steps

1. **`infra/k8s/traefik-config.yml`** — extend `valuesContent` (Traefik Helm chart values):
   ```yaml
       metrics:
         prometheus:
           entryPoint: metrics
           addEntryPointsLabels: true
           addServicesLabels: true
           addRoutersLabels: true
       ports:
         metrics:
           port: 9100
           expose:
             default: true
           exposedPort: 9100
           protocol: TCP
   ```
   Why: the bundled Traefik chart exposes Prometheus metrics when `metrics.prometheus` is set and the `metrics` entryPoint/port is declared and exposed. `addServicesLabels`/`addRoutersLabels` enrich the RED metrics per router/service. Port 9100 is requested per the task. Note: 9100 collides with node-exporter's host port, but Traefik's is a ClusterIP Service port inside kube-system, not a hostPort — no conflict on the node. Confirm in Verify.

2. **`infra/k8s/monitoring.yml`** (`prometheus-config`) — add a static scrape job. The k3s Traefik Service is `traefik.kube-system`; once the metrics port is exposed it is reachable at `:9100/metrics`:
   ```yaml
         - job_name: "traefik"
           metrics_path: "/metrics"
           static_configs:
             - targets: ["traefik.kube-system.svc.cluster.local:9100"]
   ```
   Verify the exposed Service port name/number after the Helm values apply; if the chart publishes a separate `traefik-metrics` Service, point the target there instead.

3. **Dashboard** `grafana-traefik-edge-dashboard.json` (Infrastructure folder). Panels from Traefik metrics:
   - Requests/s by entrypoint: `sum by (entrypoint) (rate(traefik_entrypoint_requests_total[5m]))`
   - 5xx rate by service: `sum by (service) (rate(traefik_service_requests_total{code=~"5.."}[5m]))`
   - p95 request duration: `histogram_quantile(0.95, sum by (le, entrypoint) (rate(traefik_entrypoint_request_duration_seconds_bucket[5m])))`
   - Open connections: `traefik_entrypoint_open_connections`

4. **`infra/k8s/monitoring.yml`** (Grafana) — add `dashboard-traefik-edge` volume + volumeMount under the infrastructure block (subPath `traefik-edge-dashboard.json`).

### Commands

```powershell
kubectl apply -f infra/k8s/traefik-config.yml
# k3s Helm controller re-reconciles the traefik HelmChart; watch the job:
kubectl -n kube-system get pods -l app.kubernetes.io/name=traefik -w

kubectl apply -f infra/k8s/monitoring.yml   # scrape job + dashboard mount

kubectl create configmap grafana-dashboard-traefik-edge `
  --namespace monitoring `
  --from-file=traefik-edge-dashboard.json=infrastructure/grafana-traefik-edge-dashboard.json `
  --dry-run=client -o yaml | kubectl apply -f -

kubectl rollout restart deploy/prometheus -n monitoring
```

### Verify

```powershell
# Metrics endpoint answers inside the cluster
kubectl -n monitoring exec deploy/prometheus -- wget -qO- http://traefik.kube-system.svc.cluster.local:9100/metrics | findstr traefik_entrypoint_requests_total

# Target is UP
#   port-forward prometheus, then query up{job="traefik"} == 1
# Confirm ingress still works (regression check):
curl http://grafana.local   # still routes through web entryPoint
```

### Rollback

Revert `traefik-config.yml` to the original (image/nodeSelector/tolerations only) and `kubectl apply` — the Helm controller reconciles back, restoring the prior Traefik without the metrics entrypoint. Remove the `traefik` scrape job and dashboard mount from `monitoring.yml`, re-apply, and delete the dashboard ConfigMap. Reversible; the only risk window is the Traefik pod restart during reconcile.

---

## Phase 3 — postgres-exporter (least-privilege) + Database dashboard extension

**Risk:** Medium. Touches the database deployment and adds a DB role. Value: High.

Security constraints honored: dedicated `pg_monitor` role (never app user / superuser), DSN only via K8s Secret, role password never committed.

### Affected files

| Path | Action | Change |
|---|---|---|
| `backend/migrations/mXXXXXXXX_000001_create_metrics_exporter_role.rs` | create | Creates `metrics_exporter` LOGIN role, `GRANT pg_monitor` |
| `backend/migrations/mod.rs` | modify | Register the new migration (pub mod + migrations() vec) |
| `infra/k8s/app/base/postgres.yml` | modify | Add `postgres-exporter` sidecar container + port 9187; add ingress allowance reference |
| `infra/k8s/app/base/network-policy.yml` | modify | Allow monitoring ns → db pod on 9187 |
| `infra/k8s/monitoring.yml` | modify | Add scrape job `postgres-exporter`; extend Database dashboard mount (already mounted) |
| `infra/k8s/dashboards/application/grafana-database-dashboard.json` | modify | Add exporter-backed panels |
| (out of band) Secret `postgres-exporter-secret` | create | `DATA_SOURCE_NAME` — created via kubectl, not committed |

### Steps

1. **Migration** `backend/migrations/m{nextdate}_000001_create_metrics_exporter_role.rs` — follow the exact pattern from `m20260510_000002_create_whatsapp_auth_tables.rs` (`execute_unprepared`, `pg_roles` guard). `up`:
   ```sql
   DO $$ BEGIN
     IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'metrics_exporter') THEN
       CREATE ROLE metrics_exporter LOGIN;
     END IF;
   END $$;
   GRANT pg_monitor TO metrics_exporter;
   GRANT CONNECT ON DATABASE realestate TO metrics_exporter;
   ```
   `down`: `REVOKE pg_monitor FROM metrics_exporter;` then `DROP ROLE IF EXISTS metrics_exporter;`
   Why `pg_monitor`: PG16 built-in role granting read of `pg_stat_*` views without table data access — exactly least-privilege for an exporter. No `SELECT` on business tables is granted.
   The role password is **not** set in the migration (no secret in git). It is set out of band (step 6) and must match the Secret.

2. **`backend/migrations/mod.rs`** — add `pub mod m{nextdate}_000001_create_metrics_exporter_role;` in the module list and `Box::new(m{nextdate}_000001_create_metrics_exporter_role::Migration),` at the end of the `migrations()` vec.

3. **`infra/k8s/app/base/postgres.yml`** — add a sidecar container to the `db` pod (sidecar chosen over separate Deployment so it shares the pod and reaches Postgres over localhost; keeps blast radius on one pod):
   ```yaml
   - name: postgres-exporter
     image: quay.io/prometheuscommunity/postgres-exporter:v0.19.1
     ports:
       - containerPort: 9187
         name: metrics
     env:
       - name: DATA_SOURCE_NAME
         valueFrom:
           secretKeyRef:
             name: postgres-exporter-secret
             key: data-source-name
     resources:
       requests: { cpu: 10m, memory: 32Mi, ephemeral-storage: 16Mi }
       limits:   { cpu: 100m, memory: 128Mi, ephemeral-storage: 64Mi }
     securityContext:
       runAsNonRoot: true
       runAsUser: 65534
       allowPrivilegeEscalation: false
       readOnlyRootFilesystem: true
       capabilities: { drop: ["ALL"] }
   ```
   Add a `Service` for the exporter or add a named port to the existing `db` Service. Recommended: keep `db` Service for 5432 only and add a separate `Service` `db-metrics` (selector `app.kubernetes.io/name: db`, port 9187) so scrape config is unambiguous and the 5432 ingress rules stay narrow.

4. **`infra/k8s/app/base/network-policy.yml`** — add ingress allowance so monitoring can scrape the exporter (the db pod is default-deny; `allow-db-ingress` only opens 5432):
   ```yaml
   apiVersion: networking.k8s.io/v1
   kind: NetworkPolicy
   metadata:
     name: allow-db-metrics-ingress
   spec:
     podSelector:
       matchLabels:
         app.kubernetes.io/name: db
     policyTypes: [Ingress]
     ingress:
       - from:
           - namespaceSelector:
               matchLabels:
                 kubernetes.io/metadata.name: monitoring
         ports:
           - port: 9187
             protocol: TCP
   ```
   This does not widen 5432 access — the exporter role is the only path to data and it has read-only stat access.

5. **`infra/k8s/monitoring.yml`** (`prometheus-config`) — add:
   ```yaml
         - job_name: "postgres-exporter"
           static_configs:
             - targets: ["db-metrics.realestate.svc.cluster.local:9187"]
               labels:
                 environment: "production"
   ```

6. **Out-of-band Secret + role password** (operator-run, not committed). Generate a strong password, create the Secret, then set the role password to match. The DSN uses `sslmode=disable` only because traffic stays on the pod loopback:
   ```powershell
   $pw = -join ((48..57)+(65..90)+(97..122) | Get-Random -Count 32 | % {[char]$_})
   kubectl -n realestate create secret generic postgres-exporter-secret `
     --from-literal=data-source-name="postgresql://metrics_exporter:$pw@localhost:5432/realestate?sslmode=disable" `
     --dry-run=client -o yaml | kubectl apply -f -
   # Set the role password to match (psql in the db pod; value piped, not echoed into git):
   kubectl -n realestate exec -i deploy/db -- psql -U realestate -d realestate -v p="$pw" -c "ALTER ROLE metrics_exporter LOGIN PASSWORD :'p';"
   ```
   The password exists only in the Secret and the role; never in a manifest or migration.

7. **Dashboard** `grafana-database-dashboard.json` (already mounted) — add panels:
   - Connections by state: `pg_stat_activity_count` / `sum by (state) (pg_stat_activity_count)`
   - Commit/rollback rate: `rate(pg_stat_database_xact_commit{datname="realestate"}[5m])`, `..._xact_rollback`
   - Cache hit ratio: `pg_stat_database_blks_hit / (pg_stat_database_blks_hit + pg_stat_database_blks_read)`
   - Deadlocks: `rate(pg_stat_database_deadlocks{datname="realestate"}[5m])`
   - DB size: `pg_database_size_bytes{datname="realestate"}`

### Commands

```powershell
# 1. Run migration (backend applies migrations on boot, or via the backend's migrate path)
cd backend; cargo build; cd ..
#   migration runs on backend rollout; confirm role exists (step Verify)

# 2. Create the exporter secret + set role password (step 6 above)

# 3. Apply manifests
kubectl apply -f infra/k8s/app/base/postgres.yml
kubectl apply -f infra/k8s/app/base/network-policy.yml
kubectl apply -f infra/k8s/monitoring.yml

# 4. Refresh Database dashboard ConfigMap
kubectl create configmap grafana-dashboard-database `
  --namespace monitoring `
  --from-file=database-dashboard.json=application/grafana-database-dashboard.json `
  --dry-run=client -o yaml | kubectl apply -f -

kubectl rollout restart deploy/prometheus -n monitoring
```

### Verify

```powershell
# Role has pg_monitor and NO table privileges
kubectl -n realestate exec deploy/db -- psql -U realestate -d realestate -c "\du metrics_exporter"
kubectl -n realestate exec deploy/db -- psql -U realestate -d realestate -c "SELECT has_table_privilege('metrics_exporter','contratos','SELECT');"  # expect f

# Exporter answers
kubectl -n realestate exec deploy/db -c postgres-exporter -- wget -qO- http://localhost:9187/metrics | findstr pg_up   # pg_up 1

# Prometheus target UP and metric present
#   query: up{job="postgres-exporter"} == 1 ; pg_stat_database_xact_commit{datname="realestate"}
```

### Rollback

Remove the sidecar + `db-metrics` Service from `postgres.yml`, the `allow-db-metrics-ingress` policy, and the scrape job; re-apply. Run the migration `down` (drops role) or `cargo run` rollback path. Delete `postgres-exporter-secret`. Revert the database dashboard JSON and refresh its ConfigMap. Sidecar removal triggers one `db` pod restart (Recreate strategy → brief DB downtime) — schedule accordingly; this is the main risk.

---

## Phase 4 — Instrument baileys-service (prom-client) + WhatsApp dashboard

**Risk:** Medium (application code + new dependency in the WhatsApp sidecar). Value: Medium/High.

`/metrics` stays cluster-internal — baileys has no IngressRoute and is reachable only by backend + (new) monitoring within the cluster. The endpoint is unauthenticated like `/health`; it must never be added to a public Traefik route.

### Affected files

| Path | Action | Change |
|---|---|---|
| `baileys-service/package.json` | modify | Add `"prom-client": "15.1.3"` (exact) to dependencies |
| `baileys-service/src/metrics.ts` | create | Registry + custom metric definitions and helpers |
| `baileys-service/src/index.ts` | modify | Register default metrics, add `GET /metrics` (no auth, before `/sessions` guard) |
| `baileys-service/src/session-manager.ts` | modify | Update gauges/counters on connection + message events |
| `infra/k8s/app/base/network-policy.yml` | modify | Allow monitoring ns → baileys 3100 |
| `infra/k8s/monitoring.yml` | modify | Add scrape job `baileys`; Grafana mount for dashboard |
| `infra/k8s/dashboards/application/grafana-whatsapp-dashboard.json` | create | WhatsApp dashboard |

### Steps

1. **`baileys-service/package.json`** — add `"prom-client": "15.1.3"` under `dependencies` (exact pin per dependencies steering). Justification: prom-client is the de-facto Node Prometheus client; no existing dep covers metric exposition. Run `npm install` to refresh the lockfile.

2. **`baileys-service/src/metrics.ts`** (new) — create a single `Registry`, call `collectDefaultMetrics({ register })`, and define:
   - `whatsapp_connection_up` — `Gauge`, labels `[realm]` (1 connected, 0 otherwise)
   - `whatsapp_messages_total` — `Counter`, labels `[realm, direction, status]` (`direction` = inbound|outbound)
   - `whatsapp_reconnects_total` — `Counter`, labels `[realm]`
   - `whatsapp_session_restore_total` — `Counter`, labels `[result]` (success|failure)

   Export the registry and typed helper functions (`setConnectionUp`, `incMessages`, `incReconnect`, `incSessionRestore`). The `realm` label = WhatsApp auth `realm_id` per the baileys-sessions steering. Bound cardinality: only realms that actually connect emit series (one per org).

3. **`baileys-service/src/index.ts`** — import the registry; add the route **above** `app.use('/sessions', authMiddleware)` so it is not gated by the session token but remains internal:
   ```ts
   app.get('/metrics', async (_req, res) => {
     res.set('Content-Type', registry.contentType);
     res.end(await registry.metrics());
   });
   ```

4. **`baileys-service/src/session-manager.ts`** — wire metrics into existing transitions (no behavior change):
   - In `handleConnectionUpdate`: on `connection === 'open'` → `setConnectionUp(realmId, 1)`; on close/logout in `handleConnectionClose` → `setConnectionUp(realmId, 0)`.
   - In `scheduleReconnect`: `incReconnect(realmId)` when a reconnect is actually scheduled.
   - In `forwardToBackend` (inbound accepted) → `incMessages(realmId, 'inbound', 'ok')`; in `sendMessage` → `incMessages(realmId, 'outbound', 'ok')` on success / `'error'` in catch.
   - In `restoreSessions`: `incSessionRestore('success')` / `incSessionRestore('failure')` per realm in the existing try/catch.

5. **`infra/k8s/app/base/network-policy.yml`** — extend `allow-baileys-ingress` to add a monitoring-ns source on 3100 (keep the existing backend source):
   ```yaml
       - from:
           - namespaceSelector:
               matchLabels:
                 kubernetes.io/metadata.name: monitoring
         ports:
           - port: 3100
             protocol: TCP
   ```

6. **`infra/k8s/monitoring.yml`** (`prometheus-config`):
   ```yaml
         - job_name: "baileys"
           metrics_path: "/metrics"
           static_configs:
             - targets: ["baileys.realestate.svc.cluster.local:3100"]
               labels:
                 environment: "production"
   ```
   Plus Grafana `dashboard-whatsapp` volume/volumeMount (application, subPath `whatsapp-dashboard.json`).

7. **Dashboard** `grafana-whatsapp-dashboard.json` panels: connected realms `sum(whatsapp_connection_up)`; per-realm up/down state-timeline; message throughput `sum by (direction) (rate(whatsapp_messages_total[5m]))`; error ratio by status; reconnect rate `rate(whatsapp_reconnects_total[5m])`; session restore success/failure.

### Commands

```powershell
cd baileys-service
npm install            # adds prom-client@15.1.3, updates package-lock.json
npm run build
npm test               # vitest — ensure no regressions
cd ..

# build + push image, then:
kubectl apply -f infra/k8s/app/base/network-policy.yml
kubectl apply -f infra/k8s/app/base/baileys.yml      # if pod spec unchanged, rollout restart
kubectl rollout restart deploy/baileys -n realestate
kubectl apply -f infra/k8s/monitoring.yml

kubectl create configmap grafana-dashboard-whatsapp `
  --namespace monitoring `
  --from-file=whatsapp-dashboard.json=application/grafana-whatsapp-dashboard.json `
  --dry-run=client -o yaml | kubectl apply -f -
kubectl rollout restart deploy/prometheus -n monitoring
```

### Verify

```powershell
# Endpoint internal-only and serving
kubectl -n realestate exec deploy/baileys -- wget -qO- http://localhost:3100/metrics | findstr whatsapp_connection_up
# Confirm NO public route exposes it:
#   review infra/k8s for any IngressRoute targeting baileys -> there must be none
# Prometheus: up{job="baileys"} == 1 ; whatsapp_messages_total present after a message flows
```

### Rollback

Revert `index.ts`, `session-manager.ts`, delete `metrics.ts`, drop the dep from `package.json`, rebuild/redeploy the image. Remove the netpol monitoring source, scrape job, dashboard mount; re-apply; delete dashboard ConfigMap. Fully reversible; metrics are additive and the route removal returns baileys to its prior surface.

---

## Phase 5 — Instrument ocr-service (prometheus-fastapi-instrumentator) + OCR dashboard

**Risk:** Medium (application code + dependency; `/metrics` on same port 8000 must stay internal). Value: Medium.

`/metrics` stays cluster-internal: ocr-service has no IngressRoute. Note the netpol gap below — ocr currently has **no** explicit ingress policy, so this phase must add one for both backend and monitoring.

### Affected files

| Path | Action | Change |
|---|---|---|
| `ocr-service/requirements.txt` | modify | Add `prometheus-fastapi-instrumentator==8.0.0` (exact) |
| `ocr-service/main.py` | modify | Instrument app, expose `/metrics`, define custom counters/histograms |
| `ocr-service/ocr_engine.py` | modify | Time OpenVINO inference; expose a hook the app records |
| `infra/k8s/app/base/network-policy.yml` | modify | Add `allow-ocr-service-ingress` (backend + monitoring → 8000) |
| `infra/k8s/monitoring.yml` | modify | Add scrape job `ocr-service`; Grafana mount for dashboard |
| `infra/k8s/dashboards/application/grafana-ocr-dashboard.json` | create | OCR dashboard |

### Steps

1. **`ocr-service/requirements.txt`** — add under web framework: `prometheus-fastapi-instrumentator==8.0.0`. Justification: purpose-built FastAPI ASGI middleware for Prometheus exposition (auto request/latency metrics + `/metrics`); no existing dep covers it. Exact-pin per steering.

2. **`ocr-service/main.py`** — after `app = FastAPI(...)`:
   ```python
   from prometheus_fastapi_instrumentator import Instrumentator
   from prometheus_client import Counter, Histogram

   ocr_documents_total = Counter("ocr_documents_total", "OCR documents processed", ["doc_type"])
   ocr_duration_seconds = Histogram("ocr_duration_seconds", "End-to-end OCR request seconds")
   ocr_inference_seconds = Histogram("ocr_inference_seconds", "OpenVINO inference seconds", ["stage"])

   Instrumentator().instrument(app).expose(app, endpoint="/metrics", include_in_schema=False)
   ```
   In `ocr_extract`: wrap the handler body in `with ocr_duration_seconds.time():` and after classification call `ocr_documents_total.labels(doc_type=doc_type).inc()`. Keep custom metric names exactly as the task specifies (`ocr_documents_total{doc_type}`, `ocr_duration_seconds`).

3. **`ocr-service/ocr_engine.py`** — record OpenVINO inference time around the two `infer()` calls. Add `ocr_inference_seconds.labels(stage="detection")` / `stage="recognition"` timing in `_detect` and `_recognize_batch`. Import the histogram from `main` would create a cycle; instead define the inference histogram in a tiny shared `metrics.py` (new, optional) or pass a timing callback. Simplest non-cyclic approach: define `ocr_inference_seconds` in a new `ocr-service/metrics.py` and import it from both `main.py` and `ocr_engine.py`.

   Adjusted file list: add `ocr-service/metrics.py` (create) holding the three custom metric objects; `main.py` and `ocr_engine.py` both import from it. This avoids the import cycle and keeps definitions single-source.

4. **`infra/k8s/app/base/network-policy.yml`** — add (ocr has no ingress policy today, so backend traffic relies on default behavior; make it explicit and add monitoring):
   ```yaml
   apiVersion: networking.k8s.io/v1
   kind: NetworkPolicy
   metadata:
     name: allow-ocr-service-ingress
   spec:
     podSelector:
       matchLabels:
         app.kubernetes.io/name: ocr-service
     policyTypes: [Ingress]
     ingress:
       - from:
           - podSelector:
               matchLabels:
                 app.kubernetes.io/name: backend
           - namespaceSelector:
               matchLabels:
                 kubernetes.io/metadata.name: monitoring
         ports:
           - port: 8000
             protocol: TCP
   ```
   Verify first whether an ocr ingress policy exists elsewhere (none found in `network-policy.yml`); if backend→ocr currently works without one, adding this explicit policy must preserve backend access — which it does.

5. **`infra/k8s/monitoring.yml`** (`prometheus-config`):
   ```yaml
         - job_name: "ocr-service"
           metrics_path: "/metrics"
           static_configs:
             - targets: ["ocr-service.realestate.svc.cluster.local:8000"]
               labels:
                 environment: "production"
   ```
   Plus Grafana `dashboard-ocr` volume/volumeMount (application, subPath `ocr-dashboard.json`).

6. **Dashboard** `grafana-ocr-dashboard.json`: documents by type `sum by (doc_type) (rate(ocr_documents_total[5m]))`; request latency p95 `histogram_quantile(0.95, sum by (le) (rate(ocr_duration_seconds_bucket[5m])))`; inference time by stage `histogram_quantile(0.95, sum by (le, stage) (rate(ocr_inference_seconds_bucket[5m])))`; request rate + error rate from the auto `http_request_*` metrics the instrumentator emits.

### Commands

```powershell
cd ocr-service
pip install -r requirements.txt
python -m pytest        # ensure instrumentation doesn't break existing tests
cd ..

# build + push image, then:
kubectl apply -f infra/k8s/app/base/network-policy.yml
kubectl rollout restart deploy/ocr-service -n realestate
kubectl apply -f infra/k8s/monitoring.yml

kubectl create configmap grafana-dashboard-ocr `
  --namespace monitoring `
  --from-file=ocr-dashboard.json=application/grafana-ocr-dashboard.json `
  --dry-run=client -o yaml | kubectl apply -f -
kubectl rollout restart deploy/prometheus -n monitoring
```

### Verify

```powershell
kubectl -n realestate exec deploy/ocr-service -- wget -qO- http://localhost:8000/metrics | findstr ocr_documents_total
# Prometheus: up{job="ocr-service"} == 1 ; after one /ocr/extract call, ocr_documents_total and ocr_inference_seconds_bucket present
# Confirm no IngressRoute targets ocr-service (internal only)
```

### Rollback

Revert `main.py`/`ocr_engine.py`, delete `metrics.py`, drop the dep from `requirements.txt`, rebuild/redeploy. Remove `allow-ocr-service-ingress` (only if it did not pre-exist and backend access is otherwise satisfied — verify before removing so backend isn't cut off), the scrape job, and dashboard mount; delete dashboard ConfigMap. Reversible.

---

## Cross-cutting risks & edge cases

- **Prometheus restart (Phases 1–5 each touch `prometheus-config`).** A bad YAML edit fails config load; `prometheus` may keep running the old config or crashloop. Validate with `promtool check config` (exec in pod) before relying on a scrape. Batch scrape-job edits where possible to limit restarts.
- **Exemplar storage memory.** `--enable-feature=exemplar-storage` uses an in-memory circular buffer (default ~100k). Negligible at this scale; watch `prometheus_tsdb_exemplar_*` if memory pressure appears (limits are 1536Mi).
- **Cardinality.** spanmetrics label explosion (per-route) and `whatsapp_messages_total{realm,direction,status}` are the main risks. Realms are per-org (low). Keep `status` to a small enum (ok/error), not raw error strings.
- **`/metrics` exposure.** baileys (3100) and ocr (8000) serve metrics on the same port as their app. They have no IngressRoute, so they stay cluster-internal — but never add one, and never put `/metrics` behind the public Traefik `web`/`websecure` entryPoints. The netpol additions only add the monitoring namespace as a scrape source.
- **postgres-exporter secret/role drift.** If the Secret DSN password and the `metrics_exporter` role password diverge, `pg_up` stays 0. They are set together in Phase 3 step 6; rotate both together.
- **Port 9100 reuse (Traefik metrics vs node-exporter hostPort).** Different scopes (ClusterIP Service vs node hostPort) — no real conflict, but verify the Traefik metrics Service/port name after Helm reconcile before trusting the scrape target.
- **Dashboard datasource uid.** New dashboards must template `${datasource}` defaulting to Prometheus uid `PBFA97CFB590B2093` (or Tempo uid `tempo` where trace links are used), matching existing dashboards, or panels render "datasource not found".

## End-to-end verification (after all phases)

1. `up == 0` check: `count(up == 0)` should be 0 for the new jobs `traefik`, `postgres-exporter`, `baileys`, `ocr-service`.
2. Metric existence (via grafana MCP `list_prometheus_metric_names` / `query_prometheus`, which are auto-approved): `traces_spanmetrics_calls_total`, `traefik_entrypoint_requests_total`, `pg_stat_database_xact_commit`, `whatsapp_connection_up`, `ocr_documents_total`.
3. All six new/extended dashboards load with live data in their Grafana folders.
4. Regression: `grafana.local` ingress still serves; backend→ocr and backend→baileys still function (netpol changes are additive).
```
