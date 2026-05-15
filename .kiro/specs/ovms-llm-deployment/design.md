# Design Document: OVMS LLM Deployment

## Overview

This design describes the Kubernetes infrastructure to deploy OpenVINO Model Server (OVMS) as the dedicated LLM inference backend for the realestate platform. OVMS serves the Qwen3-30B-A3B model (Mixture-of-Experts, INT4 quantization) on the Intel Arc discrete GPU located on the `inference` worker node. The existing OCR service (PaddleOCR) remains on the `coreos` control plane node using its Intel iGPU.

## Cluster Topology

| Node | Role | Hostname | GPU | RAM |
|------|------|----------|-----|-----|
| `coreos` | control-plane | coreos | Intel iGPU (exposed via `gpu.intel.com/i915`) | 32GB |
| `inference` | worker | inference | Intel Arc dGPU + AMD 4GB (unused) | — |
| `networkstorage` | worker | networkstorage | none | — |

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  inference node (worker)                                        │
│                                                                 │
│  ┌──────────────────────┐     ┌──────────────────────────┐     │
│  │  OVMS                │────▶│ Intel Arc dGPU            │     │
│  │  :8000               │     │ (gpu.intel.com/i915)      │     │
│  └──────────────────────┘     └──────────────────────────┘     │
│       │                                                         │
│  ┌────┴────────────┐                                            │
│  │ Model PV (local)│                                            │
│  │ 25Gi            │                                            │
│  └─────────────────┘                                            │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  coreos node (control-plane)                                    │
│                                                                 │
│  ┌──────────┐     ┌──────────────────────────┐                  │
│  │ Backend  │     │ Intel iGPU               │                  │
│  │ :8080    │     │ (gpu.intel.com/i915)     │                  │
│  └────┬─────┘     └──────────┬───────────────┘                  │
│       │                      │                                   │
│       │           ┌──────────┴───────────┐                       │
│       │           │  OCR Service :8000   │                       │
│       │           │  (PaddleOCR on iGPU) │                       │
│       │           └──────────────────────┘                       │
│       │                                                          │
│       └──── http://ovms:8000/v3/chat/completions ───────────────▶│
└─────────────────────────────────────────────────────────────────┘
```

### GPU Allocation Strategy

1. **OVMS** on `inference` node: claims `gpu.intel.com/xe: "1"` → gets the Intel Arc dGPU (xe KMD)
2. **OCR service** on `coreos` node: claims `gpu.intel.com/i915: "1"` → gets the Intel iGPU (unchanged from today, just pinned to coreos via nodeSelector)

The Intel GPU plugin DaemonSet must be extended to run on both nodes (currently only on `coreos`). This is done by changing the nodeSelector to a nodeAffinity that matches both hostnames, or by using a label-based selector.

### Request Flow

```
User → Backend (coreos)
         │
         ├─ POST http://ovms:8000/v3/chat/completions
         │   (cross-node, OpenAI-compatible, continuous batching)
         │
         └─ Response returned to caller
```

## Components

### 1. Intel GPU Plugin Update (`infra/k8s/intel-gpu-plugin.yml`)

Change the DaemonSet's `nodeSelector` from a single hostname to match both nodes:

**Option A** — nodeAffinity with multiple hostnames:
```yaml
affinity:
  nodeAffinity:
    requiredDuringSchedulingIgnoredDuringExecution:
      nodeSelectorTerms:
        - matchExpressions:
            - key: kubernetes.io/hostname
              operator: In
              values: ["coreos", "inference"]
```

**Option B** — label-based (add `gpu.intel.com/enabled: "true"` label to both nodes, use that as nodeSelector). Option A is simpler for two nodes.

### 2. OVMS Deployment (`infra/k8s/app/ovms.yml`)

A single-replica Deployment with `Recreate` strategy (only one pod can hold the GPU at a time).

**Container configuration:**
- Image: `openvino/model_server:latest-gpu`
- Port: 8000
- Command args: `--config_path /models/config.json --port 8000`
- Volume mount: PVC at `/models` (read-only at runtime)
- GPU: `gpu.intel.com/i915: "1"` in both requests and limits
- NodeSelector: `kubernetes.io/hostname: inference`
- Security: non-root, group 44 (video), no privilege escalation

**OVMS LLM serving mode:**
OVMS uses a `graph.pbtxt` MediaPipe graph configuration that defines the LLM engine with continuous batching and paged attention. The graph file lives on the PV alongside the model weights.

### 3. OVMS Service

A ClusterIP Service named `ovms` in the `realestate` namespace, routing port 8000 → 8000. Other pods reach it at `http://ovms:8000`.

### 4. Model PersistentVolume / PersistentVolumeClaim

Local storage on the `inference` node for the Qwen3-30B-A3B INT4 model files (~18GB). Uses `local-path` storage class with `ReadWriteOnce` access mode. The PVC requests 25Gi to accommodate model files, graph configuration, and tokenizer assets.

### 5. OCR Service Modifications (`infra/k8s/app/ocr-service.yml`)

- Add `nodeSelector: kubernetes.io/hostname: coreos` to pin OCR to the control plane
- Keep `gpu.intel.com/i915: "1"` resource claim (uses the iGPU on coreos)
- Keep `OPENVINO_DEVICE=GPU` (the iGPU is the only Intel GPU on that node)
- No other changes needed

### 6. Backend Modifications (`infra/k8s/app/backend.yml`)

- `OVMS_ENDPOINT`: `http://ocr-service:8000/v1` → `http://ovms:8000/v3`
- `OVMS_CHAT_MODEL`: `qwen3.6` → `Qwen3-30B-A3B`

### 7. Kustomization Update

Add `ovms.yml` to the resources list between `ocr-service.yml` and `baileys.yml`.

## Interfaces

### OVMS HTTP API (served on port 8000)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v3/chat/completions` | POST | OpenAI-compatible chat completions |
| `/v3/completions` | POST | OpenAI-compatible text completions |
| `/v1/config` | GET | Model configuration / health check |

**Chat Completions Request:**
```json
{
  "model": "Qwen3-30B-A3B",
  "messages": [
    {"role": "system", "content": "..."},
    {"role": "user", "content": "..."}
  ],
  "temperature": 0.7,
  "max_tokens": 1024
}
```

### Readiness Probe

```
GET /v1/config HTTP/1.1
Host: localhost:8000
```

Returns 200 when the model is loaded and ready to serve.

## Data Models

### PersistentVolume Layout

```
/models/
├── config.json              # OVMS model server config
└── Qwen3-30B-A3B/
    └── 1/
        ├── graph.pbtxt      # MediaPipe graph (LLM engine config)
        ├── tokenizer/       # Tokenizer files
        └── model/           # OpenVINO IR files (INT4 weights)
            ├── openvino_model.xml
            └── openvino_model.bin
```

### OVMS config.json

```json
{
  "model_config_list": [],
  "mediapipe_config_list": [
    {
      "name": "Qwen3-30B-A3B",
      "base_path": "/models/Qwen3-30B-A3B"
    }
  ]
}
```

### graph.pbtxt (LLM Engine Parameters)

```protobuf
input_stream: "HTTP_REQUEST_PAYLOAD:input"
output_stream: "HTTP_RESPONSE_PAYLOAD:output"

node {
  calculator: "LLMCalculator"
  input_stream: "HTTP_REQUEST_PAYLOAD:input"
  output_stream: "HTTP_RESPONSE_PAYLOAD:output"
  node_options: {
    [type.googleapis.com/mediapipe.LLMCalculatorOptions]: {
      models_path: "/models/Qwen3-30B-A3B/1/model"
      plugin_config: "NUM_STREAMS 1"
      target_device: "GPU"
      enable_prefix_caching: true
      max_num_batched_tokens: 4096
      cache_size: 8
    }
  }
}
```

## Resource Allocation

| Resource | OVMS (inference node) | OCR (coreos node) |
|----------|----------------------|-------------------|
| CPU request | 500m | 50m (unchanged) |
| CPU limit | 4000m | 2000m (unchanged) |
| Memory request | 16Gi | 1Gi (unchanged) |
| Memory limit | 20Gi | 4Gi (unchanged) |
| Ephemeral storage request | 128Mi | 64Mi (unchanged) |
| Ephemeral storage limit | 512Mi | 256Mi (unchanged) |
| GPU (k8s resource) | `gpu.intel.com/xe: "1"` | `gpu.intel.com/i915: "1"` (unchanged) |
| GPU (hardware) | Intel Arc dGPU | Intel iGPU |
| Node | `inference` | `coreos` |

## Error Handling

| Scenario | Behavior |
|----------|----------|
| OVMS pod not ready | Backend receives connection refused; Rig client returns error; `AiModule` maps to timeout message to user |
| Model loading (startup) | Readiness probe fails → Service doesn't route traffic → Backend gets no response → timeout handling applies |
| GPU OOM during inference | OVMS returns HTTP 500; backend logs error and returns user-friendly timeout message |
| OVMS crash loop | Liveness probe detects unresponsive pod → kubelet restarts container; `Recreate` strategy ensures clean GPU state |
| Model export failure | Kubernetes Job reports Failed status; existing model files on PV remain untouched |

## Deployment Sequence

1. Update Intel GPU plugin DaemonSet to run on both `coreos` and `inference` nodes
2. Verify `inference` node advertises `gpu.intel.com/i915: 1`
3. Apply OCR service changes (add nodeSelector for `coreos`)
4. Apply PV/PVC for model storage on `inference` node
5. Run model export (Job or manual `export_model.py` on node)
6. Apply OVMS Deployment + Service
7. Wait for OVMS readiness probe to pass
8. Apply backend changes (`OVMS_ENDPOINT`, `OVMS_CHAT_MODEL`)
9. Verify end-to-end: backend → OVMS → chat completion response

## Security Considerations

- OVMS runs as non-root with `runAsGroup: 44` (video) for GPU device access
- `allowPrivilegeEscalation: false` prevents container breakout
- `automountServiceAccountToken: false` prevents API server access
- Model PVC mounted read-only at runtime
- No external network access required — all communication is cluster-internal
- OVMS API has no authentication (cluster-internal only, not exposed via ingress)

## Correctness Properties

This feature is purely Infrastructure as Code (Kubernetes manifests). No property-based tests are applicable. Validation is done via:

- **Smoke tests**: Parse YAML manifests and assert expected field values
- **Example-based tests**: Cross-reference consistency (model name in backend matches OVMS config, probe ordering)
- **Integration tests**: End-to-end backend → OVMS → valid chat completion response (requires cluster)
