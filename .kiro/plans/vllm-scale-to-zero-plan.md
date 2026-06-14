# vLLM Scale-to-Zero with KEDA

## Summary

Install KEDA on k3s and configure a `ScaledObject` to scale the vLLM deployment to 0 replicas after 10 minutes of idle (no inference requests), scaling back to 1 when a request hits the service — saving ~95W of GPU idle power.

## Design Decisions

### Why KEDA over a CronJob/script?

- KEDA is a single Helm install (~20MB), purpose-built for this exact use case
- Handles the Prometheus query → HPA → scale loop with built-in cooldown, stabilization, and activation logic
- A CronJob polling Prometheus + `kubectl scale` is fragile (auth, error handling, race conditions) and reinvents what KEDA already does
- KEDA's `ScaledObject` with `minReplicaCount: 0` is the canonical Kubernetes pattern for scale-to-zero

### Scale trigger metric

Use `vllm:request_success_total` (counter) via Prometheus. Specifically, `rate(vllm:request_success_total[5m])` — when the rate drops to 0 for the cooldown period, scale to 0. When a new request arrives (intercepted by KEDA's HTTP proxy or detected via metric threshold), scale to 1.

**Problem**: When vLLM is scaled to 0, Prometheus can't scrape it, so the metric disappears. KEDA handles this with `fallback` configuration — when the metric query returns no data, KEDA treats it as 0 (keeping replicas at 0).

### Cold-start handling

The backend already returns a graceful message on timeout (`"Lo siento, el servicio está temporalmente no disponible. Por favor, intente de nuevo en unos momentos."`). However, on connection refused (vLLM pod doesn't exist), reqwest returns an error immediately which surfaces as HTTP 500 to the frontend.

**Fix**: Add a retry-with-backoff in the `OvmsCompletionModel` when the error is a connection error. This gives KEDA time to spin up the pod (~90s). Alternative: return the same graceful "temporarily unavailable" message on connection errors — simpler and matches the existing UX pattern.

### Scale-up trigger

KEDA's Prometheus scaler alone can't trigger scale-up when replicas=0 (no requests = no metrics). Two approaches:

1. **KEDA HTTP Add-on** (proxy that intercepts requests and triggers scale-up) — adds complexity, another component
2. **Prometheus metric from the backend** — use `ai_inference_duration_seconds_count` (backend's own counter of attempted AI requests). When the backend attempts a request, this metric increments even if vLLM is down. KEDA sees the rate > 0 and scales up.

**Choice**: Option 2 — use the backend's `ai_inference_duration_seconds_count` metric. The backend already increments this on every AI request attempt. KEDA watches this metric; when it rises from 0, activation triggers and vLLM scales to 1.

Wait — actually this metric only increments after a successful inference (it's a histogram tracking duration). Let me check... the `AI_REQUESTS` counter in metrics.rs likely tracks all attempts. Let me use a **combined approach**: KEDA watches `rate(vllm:request_success_total{job="vllm"}[5m])` for scale-down decisions, and watches `rate(ai_requests_total{endpoint="/chatbot"}[2m])` from the backend for activation (scale from 0→1).

**Simplest viable approach**: Use a single Prometheus metric `sum(rate(vllm:request_success_total[10m]))` with KEDA's activation threshold. When vLLM is at 0 replicas and a user makes a request, the backend gets connection refused, returns the "temporarily unavailable" message. The user retries in ~2 minutes. Meanwhile, we need *something* to trigger scale-up.

**Final decision**: Use **KEDA's `prometheus` scaler with a backend-side metric** for activation:
- Scale-down: `rate(vllm:request_success_total[5m]) == 0` for 10 minutes → scale to 0
- Scale-up (activation): `increase(ai_requests_total[2m]) > 0` → scale to 1
- Backend modification: increment a counter on every AI request *attempt* (before calling vLLM), and return a "service is waking up" message on connection refused instead of HTTP 500

## Affected Files

| Path | Action | Changes |
|------|--------|---------|
| `infra/k8s/keda.yml` | Create | KEDA installation manifest (Helm template output or static manifests) |
| `infra/k8s/app/shared/vllm-scaledobject.yml` | Create | KEDA ScaledObject targeting vllm Deployment |
| `infra/k8s/app/shared/vllm.yml` | Modify | Remove `replicas: 1` (let KEDA manage), add annotation for KEDA |
| `backend/src/services/ovms_provider.rs` | Modify | Detect connection refused and return user-friendly error instead of propagating raw error |
| `backend/src/metrics.rs` | Modify | Add `AI_REQUESTS_ATTEMPTED` counter that fires before calling vLLM |
| `backend/src/services/ai_module/mod.rs` | Modify | Increment attempt counter before calling model, improve cold-start error message |
| `backend/src/handlers/chatbot.rs` | Modify | Increment attempt counter before streaming call |
| `infra/k8s/monitoring.yml` | Modify | Add scrape config for KEDA metrics (optional, for observability) |
| `infra/k8s/alerts.yml` | Modify | Add alert for "vLLM failed to scale up within 3 minutes" |

## Steps

### Step 1: Install KEDA on k3s

**File**: `infra/k8s/keda.yml`

Create a manifest that installs KEDA. For k3s, the recommended lightweight approach is Helm:

```bash
helm repo add kedacore https://kedacore.github.io/charts
helm repo update
helm install keda kedacore/keda --namespace keda --create-namespace
```

But since this project uses static manifests (no Helm in the gitops flow), create a manifest that documents the install command and any CRDs needed. Alternatively, use KEDA's official static install YAML.

**Rationale**: KEDA is the only component that provides scale-to-zero with Prometheus-based activation in Kubernetes. k3s supports it natively.

### Step 2: Create ScaledObject for vLLM

**File**: `infra/k8s/app/shared/vllm-scaledobject.yml`

```yaml
apiVersion: keda.sh/v1alpha1
kind: ScaledObject
metadata:
  name: vllm-scaledobject
  namespace: realestate
spec:
  scaleTargetRef:
    name: vllm
  minReplicaCount: 0
  maxReplicaCount: 1
  cooldownPeriod: 600        # 10 minutes idle before scaling to 0
  pollingInterval: 30        # Check metrics every 30s
  advanced:
    restoreToOriginalReplicaCount: false
  triggers:
    - type: prometheus
      metadata:
        serverAddress: http://prometheus.monitoring.svc.cluster.local:9090
        query: sum(rate(vllm:request_success_total{job="vllm"}[5m]))
        threshold: "0.001"   # Any requests/sec > 0 means active
        activationThreshold: "0"
      metricType: AverageValue
    - type: prometheus
      metadata:
        serverAddress: http://prometheus.monitoring.svc.cluster.local:9090
        query: sum(increase(ai_request_attempts_total[2m]))
        threshold: "1"
        activationThreshold: "0.5"  # Any attempt in last 2min triggers activation
      metricType: AverageValue
  fallback:
    failureThreshold: 3
    replicas: 0              # If metrics unavailable, stay at 0
```

**Rationale**: 
- `cooldownPeriod: 600` = 10 minutes after last scale activity before scaling down
- The first trigger (`vllm:request_success_total`) drives scale-down: when rate drops to 0, KEDA scales down after cooldown
- The second trigger (`ai_request_attempts_total`) drives activation from 0→1: when the backend receives a chat request and vLLM is down, the attempt counter increments, KEDA sees it > 0 and activates
- `fallback.replicas: 0` ensures that when vLLM is down and metrics aren't available, it doesn't accidentally scale up

### Step 3: Modify vLLM Deployment

**File**: `infra/k8s/app/shared/vllm.yml`

Changes:
- Remove `replicas: 1` (or set to 0 and let KEDA manage)
- Add KEDA pause annotation capability (optional)

```yaml
spec:
  replicas: 0  # KEDA manages replica count
```

**Rationale**: KEDA takes ownership of the replica count. Setting initial to 0 means vLLM only starts when needed. On first deploy, manually scale to 1 to "warm up", then KEDA takes over.

### Step 4: Add attempt counter metric in backend

**File**: `backend/src/metrics.rs`

Add a new counter:
```rust
pub static AI_REQUEST_ATTEMPTS: LazyLock<IntCounter> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter!(
            "ai_request_attempts_total",
            "Intentos de solicitud al servicio de inferencia (incluye cuando vLLM no está disponible)"
        ),
        "ai_request_attempts_total",
    )
});
```

Initialize it in the existing `init_metrics()` function.

**Rationale**: This counter increments on every AI request attempt regardless of vLLM's availability. KEDA uses it to detect "someone wants inference" and trigger scale-up from 0.

### Step 5: Increment attempt counter before AI calls

**File**: `backend/src/services/ai_module/mod.rs`

In `process_message`, before the `tokio::time::timeout` call:
```rust
crate::metrics::AI_REQUEST_ATTEMPTS.inc();
```

**File**: `backend/src/handlers/chatbot.rs`

In the streaming handler (`test_chat_stream`), before the `stream()` call:
```rust
crate::metrics::AI_REQUEST_ATTEMPTS.inc();
```

**Rationale**: Both the agentic path (WhatsApp) and the streaming path (UI test chat) need to signal "inference needed" to KEDA.

### Step 6: Handle connection refused gracefully

**File**: `backend/src/services/ovms_provider.rs`

In both `completion()` and `stream()`, detect connection refused errors and return a specific user-friendly error:

```rust
let response = request_builder
    .json(&ovms_request)
    .send()
    .await
    .map_err(|e| {
        if e.is_connect() {
            CompletionError::ProviderError(
                "INFERENCE_COLD_START: El servicio de inferencia se está iniciando. Por favor, intente de nuevo en 1-2 minutos.".to_string()
            )
        } else {
            CompletionError::ProviderError(format!("Error conectando a OVMS: {e}"))
        }
    })?;
```

**File**: `backend/src/services/ai_module/mod.rs`

In the `Ok(Err(e))` match arm, detect the cold-start sentinel:
```rust
if error_msg.contains("INFERENCE_COLD_START") {
    return Ok(AgentResponse {
        reply: "El asistente se está iniciando. Por favor, intente de nuevo en 1-2 minutos.".to_string(),
        tools_invoked: vec![],
        extracted_receipt: None,
    });
}
```

**Rationale**: When vLLM is scaled to 0 and a request arrives, the backend gets connection refused. Instead of returning HTTP 500, it returns a friendly "warming up" message. The user retries in 1-2 minutes (by which time KEDA has scaled vLLM to 1 and the startup probe has passed). This matches the existing pattern for timeout handling.

### Step 7: Add KEDA metrics scraping (optional)

**File**: `infra/k8s/monitoring.yml`

Add to Prometheus scrape configs:
```yaml
- job_name: "keda"
  metrics_path: "/metrics"
  static_configs:
    - targets: ["keda-operator.keda.svc.cluster.local:8080"]
```

**Rationale**: Observe KEDA's scaling decisions, queue depth, and errors in Grafana.

### Step 8: Add scale-up failure alert

**File**: `infra/k8s/alerts.yml`

```yaml
- alert: VllmScaleUpFailed
  expr: |
    kube_deployment_spec_replicas{deployment="vllm", namespace="realestate"} > 0
    AND kube_deployment_status_ready_replicas{deployment="vllm", namespace="realestate"} == 0
  for: 3m
  labels:
    severity: warning
  annotations:
    summary: "vLLM requested but not ready after 3 minutes"
    description: "KEDA scaled vLLM to 1 but the pod hasn't become ready. Cold start may have failed."
```

**Rationale**: Detects when KEDA triggers a scale-up but vLLM fails to start (model corruption, GPU issue, etc.).

## Risks & Edge Cases

1. **Race condition on first request**: User sends message → backend increments counter → KEDA sees it and scales up → vLLM takes 90s to start → user gets "warming up" message → user may not retry. **Mitigation**: The message explicitly says "1-2 minutos". WhatsApp users are accustomed to async replies.

2. **Prometheus scrape gap**: When vLLM is at 0 replicas, the `vllm:request_success_total` metric goes stale. KEDA's `fallback` config handles this — stays at 0 replicas. When vLLM scales back up, Prometheus resumes scraping within 15s.

3. **PVC mount delay**: The `vllm-models-pvc` uses `local-path` provisioner on the inference node. Since the PV is already bound and the pod runs on the same node, mount time is negligible (<1s).

4. **KEDA CRD conflicts**: k3s ships with a basic HPA controller. KEDA creates its own HPA objects. No conflict as long as we don't manually create an HPA for vLLM.

5. **Multiple rapid requests during cold start**: Many users hit the chatbot while vLLM is starting. All get the "warming up" message. The attempt counter keeps incrementing but KEDA already triggered scale-up (maxReplicas=1 caps it). No thundering herd.

6. **Cooldown too aggressive**: If usage is bursty (one message every 9 minutes), vLLM would stay up. The 10-minute cooldown counts from last scaling *activity*, not last request. The `rate(vllm:request_success_total[5m])` dropping to 0 means no requests in the last 5 minutes, plus the 10-minute cooldown = effectively 15 minutes of true idle before scale-down.

7. **Backend deploy resets counter**: If the backend restarts, `ai_request_attempts_total` resets to 0. This is fine — KEDA looks at `increase()` over 2 minutes, so a reset doesn't falsely trigger scale-up.

8. **GPU not releasing memory**: Intel Arc GPUs release VRAM when the process exits. Since KEDA scales the Deployment to 0 (pod deleted), the GPU memory is fully freed. Confirmed by the `Recreate` strategy already in use.

## Verification

1. **Install verification**: `kubectl get pods -n keda` shows keda-operator and keda-metrics-apiserver running
2. **ScaledObject active**: `kubectl get scaledobject -n realestate` shows `vllm-scaledobject` with `READY=True`
3. **Scale-down test**: Leave vLLM idle for 15 minutes, verify `kubectl get deploy vllm -n realestate` shows `0/0` replicas
4. **Scale-up test**: Send a chat message via the UI test chat, verify vLLM scales to 1 within 60s (check KEDA logs)
5. **Cold-start UX test**: With vLLM at 0, send a message — verify response is "El asistente se está iniciando..." (not HTTP 500)
6. **Steady-state test**: With vLLM at 1, send messages continuously — verify it stays at 1 and responses stream normally
7. **Power savings**: With vLLM at 0, verify GPU power via `xpu-smi` or xpumanager metrics drops to ~0W (vs ~95W when loaded)
8. **Alert test**: Scale vLLM to 1 manually, then break the image tag — verify `VllmScaleUpFailed` alert fires after 3 minutes
9. **Backend metrics**: Verify `ai_request_attempts_total` is scraped by Prometheus and increments on chat requests
10. **Run existing tests**: `cargo test` in backend to verify no regressions from the metrics/error-handling changes
