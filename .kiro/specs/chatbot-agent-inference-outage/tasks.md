# Implementation Plan

## Overview

Restore chatbot agent inference by fixing the two remaining infrastructure failures preventing the vLLM pod from becoming healthy:
1. **Model PVC is empty** — The `vllm-models-pvc` at `/models` has no model files; since `HF_HUB_OFFLINE=1` is set, vLLM cannot download the model and crashes.
2. **GPU device plugin unhealthy** — `intel-gpu-plugin` reports `gpu.intel.com/xe` devices as unhealthy on the `inference` node, blocking pod admission.

### Current Infrastructure State (already applied)

- Image: `intel/llm-scaler-vllm:0.14.0-b8.3.1` (Intel's optimized vLLM for Arc Pro GPUs)
- Entrypoint: `source /opt/intel/oneapi/setvars.sh --force` before `vllm serve`
- Offline env vars: `HF_HUB_OFFLINE=1` and `TRANSFORMERS_OFFLINE=1` already set
- Quantization: `--quantization sym_int4` with `VLLM_QUANTIZE_Q40_LIB=/usr/local/lib/python3.12/dist-packages/vllm_int4_for_multi_arc.so`
- Model: `Qwen/Qwen3.6-27B` (HuggingFace model ID passed to `vllm serve`)
- PVC: `vllm-models-pvc` mounted at `/models` with `HF_HOME=/models`

## Task Dependency Graph

```json
{
  "waves": [
    {
      "wave": 1,
      "description": "Write bug condition exploration test and preservation tests on unfixed infrastructure",
      "tasks": ["1", "2"]
    },
    {
      "wave": 2,
      "description": "Create model download Job and restart GPU plugin (parallelizable; 3.1 already done)",
      "tasks": ["3.1", "3.2", "3.3"]
    },
    {
      "wave": 3,
      "description": "Rollout restart vLLM deployment after model provisioned and GPU healthy",
      "tasks": ["3.4"]
    },
    {
      "wave": 4,
      "description": "Verify bug condition test passes and preservation tests still pass",
      "tasks": ["3.5", "3.6"]
    },
    {
      "wave": 5,
      "description": "Final validation checkpoint",
      "tasks": ["4"]
    }
  ]
}
```

## Tasks

- [x] 1. Write bug condition exploration test
  - **Property 1: Bug Condition** — vLLM Inference Unavailability
  - **CRITICAL**: This test MUST FAIL on unfixed infrastructure — failure confirms the bug exists
  - **DO NOT attempt to fix the test or the infrastructure when it fails**
  - **NOTE**: This test encodes the expected behavior — it will validate the fix when it passes after implementation
  - **GOAL**: Surface counterexamples that demonstrate the two independent failure modes (empty model PVC + GPU device unhealthy)
  - **Scoped PBT Approach**: Scope the property to the concrete failing cases: (1) vLLM pod status is not Running/Ready, (2) inference endpoint returns non-200 for any test message
  - Write a shell-based property test script that:
    - Checks `kubectl get pods -n realestate -l app.kubernetes.io/name=vllm` reports at least one pod in `Running`/`Ready` state
    - Checks `kubectl describe node inference | grep "gpu.intel.com/xe"` shows non-zero allocatable GPU devices
    - Checks vLLM pod logs do NOT contain `ConnectionError` or `Network is unreachable` or model-not-found errors
    - Sends a test `POST /api/v1/chatbot/test/stream` request and asserts HTTP 200 with a streamed agent reply
  - The test assertions match the Expected Behavior Properties from design: `vllmPodHealthy() = true AND intelGpuDevicesHealthy() = true AND modelLoadedWithoutHuggingFaceEgress() = true AND result.httpStatus = 200`
  - Run test on UNFIXED infrastructure
  - **EXPECTED OUTCOME**: Test FAILS (this is correct — it proves the bug exists: pods in CrashLoopBackOff due to empty PVC, GPU unhealthy, endpoint unreachable)
  - Document counterexamples found:
    - vLLM pod crashes because `/models` PVC is empty and `HF_HUB_OFFLINE=1` prevents downloading
    - `kubectl describe pod` shows `UnexpectedAdmissionError: Allocate failed due to no healthy devices present`
    - Node `inference` reports `gpu.intel.com/xe: 0` allocatable
    - `POST /api/v1/chatbot/test/stream` returns HTTP 500 with `ProviderError: Error conectando a OVMS stream`
  - Mark task complete when test is written, run, and failure is documented
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [x] 2. Write preservation property tests (BEFORE implementing fix)
  - **Property 2: Preservation** — Non-Inference Behavior Unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe behavior on UNFIXED infrastructure for non-buggy inputs (inputs that do NOT depend on vLLM availability):
    - Observe: `GET /configuracion/chatbot` renders configuration page correctly (HTTP 200)
    - Observe: `GET /api/v1/health` returns HTTP 200 (backend health)
    - Observe: Backend can reach database, baileys, ocr-service via K8s service DNS
    - Observe: Frontend static assets served correctly via Caddy
    - Observe: Chatbot configuration CRUD operations work (create/read/update chatbot config)
    - Observe: Network policies for non-vLLM pods are intact (backend egress to SMTP still works)
  - Write property-based test script that asserts: for all non-inference paths, behavior is identical to current observed behavior
    - Config page rendering returns expected HTML/JSON
    - Backend health endpoint returns 200
    - Chatbot config CRUD works without error
    - Error-handling path for genuine inference failures still surfaces dismissible alert (scale vLLM to 0, send test message, assert graceful HTTP 500 error response)
  - Verify tests PASS on UNFIXED infrastructure (these paths work today despite inference being broken)
  - **EXPECTED OUTCOME**: Tests PASS (this confirms baseline behavior to preserve)
  - Mark task complete when tests are written, run, and passing on unfixed infrastructure
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 3. Fix for vLLM inference outage — restore chatbot agent reply generation

  - [x] 3.1 Update model ID in vLLM Deployment manifest
    - Edit `infra/k8s/app/shared/vllm.yml`
    - Manifest references `Qwen/Qwen3-Coder-30B-A3B-Instruct` in the `vllm serve` command
    - The `intel/llm-scaler-vllm:0.14.0-b8.3.1` image supports `Qwen/Qwen3-Coder-30B-A3B-Instruct` with `--quantization sym_int4` (runtime quantization via bundled `vllm_int4_for_multi_arc.so`)
    - Uses `--tool-call-parser qwen3_coder` for native agentic tool calling support
    - Context window set to 8192 tokens (sufficient for chatbot with system prompt + FAQs + conversation history)
    - All other container settings remain unchanged (oneAPI setvars, offline env vars, etc.)
    - _Bug_Condition: isBugCondition(X) where vllmModelUnavailableWithoutEgress() — manifest referenced wrong model ID_
    - _Expected_Behavior: vLLM loads Qwen/Qwen3-Coder-30B-A3B-Instruct from local /models PVC cache with sym_int4 quantization_
    - _Preservation: No changes to ports, volumes, resource requests, env vars, or network policies_
    - _Requirements: 2.3_

  - [x] 3.2 Create model-download Kubernetes Job to populate PVC
    - Create `infra/k8s/jobs/download-model.yml`
    - Job uses `python:3.12-slim` image with `huggingface_hub` pip-installed
    - Mounts `vllm-models-pvc` at `/models`
    - Sets `HF_HOME=/models` so files land in the HuggingFace cache layout
    - Downloads `Qwen/Qwen3-Coder-30B-A3B-Instruct` (~17GB in safetensors format)
    - Include a temporary egress NetworkPolicy allowing HTTPS to the internet for the Job pod only
    - Job runs on the `inference` node (same node where the PVC is bound via `local-path` StorageClass)
    - Job runs once to completion; after success the PVC contains the full model for all subsequent vLLM starts
    - Apply the Job: `kubectl apply -f infra/k8s/jobs/download-model.yml`
    - Wait for Job completion: `kubectl wait --for=condition=complete job/download-model -n realestate --timeout=1800s`
    - Verify model files exist: `kubectl exec` into a debug pod mounting `vllm-models-pvc` and confirm `models--Qwen--Qwen3-Coder-30B-A3B-Instruct` directory is populated under `/models/hub/`
    - After success, delete the temporary egress NetworkPolicy
    - _Bug_Condition: isBugCondition(X) where vllmModelUnavailableWithoutEgress() — PVC was empty, no model to load offline_
    - _Expected_Behavior: modelLoadedWithoutHuggingFaceEgress() = true; Qwen/Qwen3-Coder-30B-A3B-Instruct served from /models PVC_
    - _Preservation: Temporary egress policy scoped only to download Job; removed after completion_
    - _Requirements: 2.3_

  - [x] 3.3 Restart Intel GPU device plugin on inference node
    - Delete the `intel-gpu-plugin` pod on the `inference` node to force re-detection of `/dev/dri` devices:
      ```
      kubectl delete pod -n kube-system -l app.kubernetes.io/name=intel-gpu-plugin --field-selector spec.nodeName=inference
      ```
    - Wait for replacement pod to become Ready
    - Verify GPU health: `kubectl describe node inference | grep -A5 "Allocatable"` — should show `gpu.intel.com/xe: 2`
    - If devices still unhealthy: SSH to inference node and verify `/dev/dri/renderD128` exists, check `xe` kernel driver is loaded (`lsmod | grep xe`), consider plugin version upgrade
    - _Bug_Condition: isBugCondition(X) where intelGpuDevicesUnhealthy() — pod admission rejected_
    - _Expected_Behavior: intelGpuDevicesHealthy() = true; gpu.intel.com/xe shows non-zero allocatable_
    - _Preservation: Only restarts the specific plugin pod on inference node; no changes to other nodes or DaemonSet spec_
    - _Requirements: 2.4_

  - [x] 3.4 Rollout restart vLLM deployment
    - After model is provisioned (3.2) and GPU is healthy (3.3), apply the updated manifest and restart:
      ```
      kubectl apply -f infra/k8s/app/shared/vllm.yml
      kubectl rollout restart deployment/vllm -n realestate
      ```
    - Wait for rollout: `kubectl rollout status deployment/vllm -n realestate --timeout=600s`
    - Verify pod reaches `Running`/`Ready`: `kubectl get pods -n realestate -l app.kubernetes.io/name=vllm`
    - Verify pod logs show:
      - `source /opt/intel/oneapi/setvars.sh --force` succeeded
      - Model `Qwen/Qwen3.6-27B` loaded from `/models` with no network download attempts
      - `sym_int4` quantization applied via `vllm_int4_for_multi_arc.so`
      - Server listening on `0.0.0.0:8000`
    - _Bug_Condition: Both root causes resolved — pod should now start successfully_
    - _Expected_Behavior: vllmPodHealthy() = true; pod Running/Ready, serving on port 8000_
    - _Preservation: No changes to other deployments in the namespace_
    - _Requirements: 2.1, 2.2, 2.3, 2.4_

  - [x] 3.5 Verify bug condition exploration test now passes
    - **Property 1: Expected Behavior** — vLLM Inference Restored
    - **IMPORTANT**: Re-run the SAME test from task 1 — do NOT write a new test
    - The test from task 1 encodes the expected behavior: healthy pod, healthy GPU, model loaded offline, inference returns HTTP 200
    - When this test passes, it confirms the expected behavior is satisfied
    - Run bug condition exploration test from step 1
    - **EXPECTED OUTCOME**: Test PASSES (confirms bug is fixed)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [x] 3.6 Verify preservation tests still pass
    - **Property 2: Preservation** — Non-Inference Behavior Unchanged
    - **IMPORTANT**: Re-run the SAME tests from task 2 — do NOT write new tests
    - Run preservation property tests from step 2
    - **EXPECTED OUTCOME**: Tests PASS (confirms no regressions)
    - Confirm config page, health endpoint, chatbot CRUD, error-handling path, and other AI features still work
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 4. Checkpoint — Ensure all tests pass
  - Run full bug condition test (Property 1) — must PASS
  - Run full preservation test suite (Property 2) — must PASS
  - End-to-end validation: send message via "Probar conversación" panel and verify streamed agent reply is received (HTTP 200)
  - End-to-end validation: confirm WhatsApp inbound path generates agent reply via baileys
  - Cluster health: `kubectl get pods -n realestate` shows all pods healthy
  - GPU allocation: `kubectl describe node inference` shows `gpu.intel.com/xe` with expected allocatable count
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- The `intel/llm-scaler-vllm:0.14.0-b8.3.1` image is from [intel/llm-scaler](https://github.com/intel/llm-scaler) — Intel's optimized vLLM build for Arc Pro B60/B70 GPUs
- The image bundles oneAPI, SYCL runtime, and the `vllm_int4_for_multi_arc.so` quantization library
- `Qwen/Qwen3.6-27B` is a dense 27B parameter model — fits in the 12GB Arc Pro B60 VRAM with sym_int4 quantization and `--language-model-only` (skips vision encoder)
- The download Job must run on the `inference` node because `vllm-models-pvc` uses `local-path` StorageClass (node-local storage)
- Startup probe allows up to 30 minutes (60s initial + 30s×60 attempts) for model loading and quantization
