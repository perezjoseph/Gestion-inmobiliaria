# Chatbot Agent Inference Outage — Bugfix Design

## Overview

The agentic WhatsApp chatbot cannot generate replies in production because the vLLM inference service (`vllm-inference.realestate.svc.cluster.local:8000`) is completely unavailable. Two independent infrastructure failures prevent any vLLM pod from reaching a healthy `Running` state:

1. **Model download failure** — The vLLM container command passes the HuggingFace model ID `Qwen/Qwen3-Coder-30B-A3B-Instruct` directly to `vllm serve`, which attempts an online download at startup. Production network policies (`default-deny-egress`) block all internet egress from the vLLM pod, causing `ConnectionError: [Errno 101] Network is unreachable` and `CrashLoopBackOff`.
2. **GPU device unhealthy** — The Intel GPU device plugin (`intel/intel-gpu-plugin:0.35.0`) reports `gpu.intel.com/xe` devices as unhealthy, causing Kubernetes to reject pod admission with `UnexpectedAdmissionError`.

The fix restores inference availability by pre-provisioning the model into the existing PVC (`vllm-models-pvc`) and setting `HF_HUB_OFFLINE=1` so vLLM loads from the local cache without egress. The GPU health issue is resolved by restarting the device plugin after verifying `/dev/dri` device accessibility on the `inference` node.

## Glossary

- **Bug_Condition (C)**: Any chatbot reply request while the vLLM inference endpoint is unreachable — either because the model cannot be loaded without HuggingFace egress, or because GPU devices are reported unhealthy preventing pod admission.
- **Property (P)**: When the bug condition is eliminated, vLLM pods load the model from local storage, GPU devices are healthy, the pod reaches `Ready`, and inference requests return streamed completions.
- **Preservation**: All existing behavior unrelated to the inference outage must remain unchanged — config page rendering, prompt composition, error-handling paths for genuine failures, and non-vLLM AI features.
- **vLLM Deployment**: The `Deployment/vllm` in namespace `realestate` running `intel/vllm:0.17.0-xpu` on the `inference` node, serving at port 8000.
- **vllm-models-pvc**: A 25Gi `PersistentVolumeClaim` mounted at `/models` inside the vLLM container, currently empty (model was never successfully downloaded).
- **Intel GPU Device Plugin**: `DaemonSet/intel-gpu-plugin` in `kube-system`, managing `gpu.intel.com/xe` resource advertisements on nodes `coreos` and `inference`.
- **OvmsCompletionModel**: The Rust backend client (`backend/src/services/ovms_provider.rs`) that issues HTTP requests to the in-cluster vLLM endpoint.

## Bug Details

### Bug Condition

The bug manifests whenever a chatbot reply is requested while the vLLM inference service is unavailable. Two independent failure modes make the service unavailable:

1. The vLLM pod starts and runs `vllm serve Qwen/Qwen3-Coder-30B-A3B-Instruct` which triggers a model download from `huggingface.co`. The `default-deny-egress` network policy blocks all internet traffic from the vLLM pod (no egress rule exists for it), so the download fails and the pod crash-loops.
2. The Intel GPU device plugin reports `gpu.intel.com/xe` devices as unhealthy, so the kubelet cannot allocate the GPU resource requested by the vLLM deployment, and Kubernetes rejects pod admission entirely.

**Formal Specification:**
```
FUNCTION isBugCondition(X)
  INPUT: X of type ChatbotReplyRequest
  OUTPUT: boolean

  RETURN X.requestsAgentReply
     AND ( vllmModelUnavailableWithoutEgress()
        OR intelGpuDevicesUnhealthy()
        OR NOT vllmEndpointReachable() )
END FUNCTION
```

### Examples

- **Example 1**: Admin sends "¿Cuánto debo este mes?" in "Probar conversación" → `POST /api/v1/chatbot/test/stream` returns HTTP 500 with "Error del servidor (código 500)"; expected: HTTP 200 with streamed agent reply.
- **Example 2**: Tenant sends WhatsApp message "Quiero reportar una fuga" → no reply delivered; expected: agent generates and sends a reply through baileys.
- **Example 3**: `kubectl get pods -n realestate -l app.kubernetes.io/name=vllm` shows `CrashLoopBackOff` with logs containing `requests.exceptions.ConnectionError: ... [Errno 101] Network is unreachable`; expected: pod in `Running`/`Ready` state.
- **Example 4**: `kubectl describe pod` shows event `UnexpectedAdmissionError: Allocate failed due to no healthy devices present; cannot allocate unhealthy devices gpu.intel.com/xe`; expected: pod admitted and scheduled with GPU allocated.

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**
- The chatbot configuration page (`/configuracion/chatbot`) must continue to render correctly
- The backend's existing error-handling path for genuine inference failures (e.g., model overloaded, timeout) must continue to surface dismissible alerts to the UI
- Prompt composition (system prompt, persona, FAQs, policies, handoff keywords, guidance rules) must remain unchanged
- Mouse/touch interactions with the chatbot UI must work as before
- All other backend endpoints and AI features not dependent on vLLM must be unaffected
- Network policies for all other pods (backend, frontend, baileys, cloudflared, db) must remain unchanged

**Scope:**
All inputs that do NOT involve the vLLM inference availability should be completely unaffected by this fix. This includes:
- Configuration page rendering and chatbot CRUD operations
- WhatsApp message receipt and webhook delivery (baileys)
- Backend health checks, authentication, RBAC
- OCR service, other AI features not routed through vLLM
- Prometheus metrics scraping and Grafana dashboards

## Hypothesized Root Cause

Based on the bug description and infrastructure analysis, two independent root causes:

1. **Missing offline model provisioning**: The `vllm serve` command references the model by HuggingFace ID (`Qwen/Qwen3-Coder-30B-A3B-Instruct`). When `HF_HOME=/models` is set and the PVC is empty, vLLM's model loader falls back to downloading from `huggingface.co`. The network policy `default-deny-egress` blocks all egress from the vLLM pod (there is no egress rule for it unlike backend which has explicit SMTP egress). The `--download-dir /models` flag only controls where to cache, not whether to skip download.
   - **Fix**: Pre-populate the model files into `vllm-models-pvc` via a one-time Job with internet access (or a local copy), then set `HF_HUB_OFFLINE=1` environment variable in the Deployment so vLLM never attempts an online download.

2. **Intel GPU device plugin health reporting**: The device plugin version `0.35.0` with args `-shared-dev-num=2 -bypath=none` is reporting all `gpu.intel.com/xe` devices as unhealthy. This could be caused by:
   - A stale device plugin pod that lost contact with `/dev/dri` after a node reboot or driver update
   - The `-bypath=none` flag causing the plugin to enumerate devices incorrectly after a kernel/driver update
   - The plugin needing a restart to re-probe the device nodes in `/dev/dri`
   - **Fix**: Restart the `intel-gpu-plugin` DaemonSet pod on the `inference` node. If that doesn't resolve it, verify `/dev/dri/renderD128` exists and is accessible, check kernel driver (`i915` or `xe`) is loaded, and consider upgrading the plugin version.

## Correctness Properties

Property 1: Bug Condition — vLLM Inference Availability Restored

_For any_ chatbot reply request where the bug condition holds (vLLM was previously unavailable due to model download failure or GPU device unhealthy), the fixed infrastructure SHALL result in a healthy vLLM pod that loads the model from the pre-provisioned local PVC without requiring internet egress, the GPU device plugin SHALL report devices as healthy, and the inference endpoint SHALL return a streamed completion (HTTP 200) to the backend.

**Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**

Property 2: Preservation — Non-Inference Behavior Unchanged

_For any_ input that does NOT depend on the vLLM inference availability (config page rendering, prompt composition, other endpoints, error-handling paths for genuine failures, non-vLLM AI features), the fixed system SHALL produce exactly the same behavior as the original system, preserving all existing functionality.

**Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5**

## Fix Implementation

### Changes Required

Assuming our root cause analysis is correct:

**File**: `infra/k8s/app/shared/vllm.yml`

**Component**: vLLM Deployment environment variables

**Specific Changes**:

1. **Add `HF_HUB_OFFLINE=1` environment variable**: Add to the vLLM container env section so that the HuggingFace hub client never attempts network downloads. This forces vLLM to load exclusively from the local `/models` cache.
   ```yaml
   - name: HF_HUB_OFFLINE
     value: "1"
   ```

2. **Add `TRANSFORMERS_OFFLINE=1` environment variable**: Belt-and-suspenders to ensure the transformers library also stays offline.
   ```yaml
   - name: TRANSFORMERS_OFFLINE
     value: "1"
   ```

3. **Create a model-download Job** (`infra/k8s/jobs/download-model.yml`): A one-time Kubernetes Job that:
   - Runs a `huggingface/downloader` or `python:3.12-slim` image with `huggingface_hub` installed
   - Has an egress network policy allowing HTTPS to the internet
   - Mounts `vllm-models-pvc` at `/models`
   - Downloads `Qwen/Qwen3-Coder-30B-A3B-Instruct` into the HuggingFace cache layout under `/models`
   - Runs once to completion, then the PVC contains the model for all subsequent vLLM starts
   - After successful completion, the Job and its temporary egress policy can be deleted

4. **Restart Intel GPU device plugin**: Delete the `intel-gpu-plugin` pod on the `inference` node to force re-detection of `/dev/dri` devices:
   ```bash
   kubectl delete pod -n kube-system -l app.kubernetes.io/name=intel-gpu-plugin \
     --field-selector spec.nodeName=inference
   ```

5. **Verify GPU health after restart**: Confirm devices are re-registered as healthy:
   ```bash
   kubectl describe node inference | grep -A5 "Allocatable"
   # Should show gpu.intel.com/xe: 2
   ```

6. **Rollout restart vLLM deployment**: After model is provisioned and GPU is healthy:
   ```bash
   kubectl rollout restart deployment/vllm -n realestate
   ```

## Testing Strategy

### Validation Approach

The testing strategy follows a two-phase approach: first, confirm the bug exists by observing current cluster state, then verify each fix step restores health incrementally. Because this is an infrastructure bug (not a code logic bug), validation is primarily operational rather than unit-test-based.

### Exploratory Bug Condition Checking

**Goal**: Confirm the two root causes before applying fixes. Surface evidence that matches the hypothesized failures.

**Test Plan**: Run diagnostic commands against the production cluster to observe the current broken state.

**Test Cases**:
1. **Model download failure**: `kubectl logs -n realestate deployment/vllm --tail=50` — expect to see `ConnectionError: ... [Errno 101] Network is unreachable` and `CrashLoopBackOff` (confirms Root Cause 1)
2. **GPU admission rejection**: `kubectl describe pod -n realestate -l app.kubernetes.io/name=vllm` — expect to see `UnexpectedAdmissionError: Allocate failed due to no healthy devices present` in events (confirms Root Cause 2)
3. **Empty model PVC**: `kubectl exec` into a debug pod mounting `vllm-models-pvc` and verify `/models` is empty or has incomplete download artifacts
4. **GPU device plugin status**: `kubectl get pods -n kube-system -l app.kubernetes.io/name=intel-gpu-plugin` and `kubectl describe node inference | grep gpu.intel.com` — expect zero allocatable GPU resources

**Expected Counterexamples**:
- vLLM pod logs show network error attempting `huggingface.co` download
- Node `inference` reports `gpu.intel.com/xe: 0` in allocatable resources
- Backend logs show `ProviderError: Error conectando a OVMS stream`

### Fix Checking

**Goal**: Verify that for all inputs where the bug condition held, the fixed infrastructure now produces healthy inference responses.

**Pseudocode:**
```
FOR ALL X WHERE isBugCondition(X) DO
  // After fix: model is pre-provisioned, GPU is healthy, pod is Ready
  ASSERT vllmPodStatus() = "Running/Ready"
  ASSERT modelLoadedFromLocalPVC() = true
  ASSERT noEgressAttemptToHuggingFace() = true
  ASSERT gpuDevicesHealthy() = true
  result := POST /api/v1/chatbot/test/stream with test message
  ASSERT result.httpStatus = 200
  ASSERT result.streamsAgentReply = true
END FOR
```

### Preservation Checking

**Goal**: Verify that for all inputs where the bug condition does NOT hold, the system behaves identically after the fix.

**Pseudocode:**
```
FOR ALL X WHERE NOT isBugCondition(X) DO
  ASSERT F(X) = F'(X)
END FOR
```

**Testing Approach**: Manual smoke testing is appropriate for preservation because:
- The fix is purely infrastructure (env vars and model provisioning) — no backend code changes
- Network policies for all other pods are unchanged
- The vLLM Deployment only gains env vars; no ports, volumes, or resource requests change

**Test Plan**: After deploying the fix, exercise all non-inference paths to confirm no regression.

**Test Cases**:
1. **Config page preservation**: Load `/configuracion/chatbot` — verify UI renders, CRUD operations work
2. **Backend health preservation**: `GET /api/v1/health` returns 200
3. **Network policy preservation**: Verify backend can still reach db, baileys, ocr-service; frontend still serves static assets; cloudflared still tunnels traffic
4. **Error path preservation**: Temporarily scale vLLM to 0 replicas, send test message — verify backend still returns graceful error (HTTP 500 with dismissible alert), not a crash or hang
5. **Other AI features**: Verify OCR or any non-vLLM AI feature continues to operate

### Unit Tests

- Verify `OvmsCompletionModel` connection timeout handling (existing tests — no change expected)
- Verify `test_chat_stream` handler returns proper error structure when inference is unreachable (existing test)
- No new unit tests required since no application code is changed

### Property-Based Tests

- Generate random valid chatbot configurations and verify that after fix, inference requests succeed for all valid orgs with active chatbot config
- Generate random non-inference requests (config CRUD, page loads) and verify behavior is identical pre/post fix
- These are primarily integration-level property tests exercised against the running cluster

### Integration Tests

- End-to-end test: send message via "Probar conversación" and assert streamed reply is received
- End-to-end test: simulate WhatsApp inbound via baileys webhook and assert agent reply is generated
- Cluster health test: verify `kubectl get pods -n realestate -l app.kubernetes.io/name=vllm` shows `1/1 Running`
- GPU allocation test: verify `kubectl describe node inference` shows `gpu.intel.com/xe` with non-zero allocatable count
- Offline model loading test: verify vLLM pod logs show model loaded from `/models` with no network attempts
