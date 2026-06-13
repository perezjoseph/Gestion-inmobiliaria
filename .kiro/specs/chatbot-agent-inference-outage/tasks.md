# Implementation Plan

## Overview

Restore chatbot agent inference by fixing the two independent infrastructure failures (model download blocked by egress policy, GPU devices unhealthy) using the bug condition methodology: explore the bug, preserve existing behavior, implement the fix, validate.

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
      "description": "Add offline environment variables to vLLM Deployment manifest",
      "tasks": ["3.1"]
    },
    {
      "wave": 3,
      "description": "Create model-download Job and restart GPU device plugin (parallelizable)",
      "tasks": ["3.2", "3.3"]
    },
    {
      "wave": 4,
      "description": "Rollout restart vLLM deployment after model provisioned and GPU healthy",
      "tasks": ["3.4"]
    },
    {
      "wave": 5,
      "description": "Verify bug condition exploration test now passes",
      "tasks": ["3.5"]
    },
    {
      "wave": 6,
      "description": "Verify preservation tests still pass",
      "tasks": ["3.6"]
    },
    {
      "wave": 7,
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
  - **GOAL**: Surface counterexamples that demonstrate the two independent failure modes (model download failure + GPU device unhealthy)
  - **Scoped PBT Approach**: Scope the property to the concrete failing cases: (1) vLLM pod status is not Running/Ready, (2) inference endpoint returns non-200 for any test message
  - Write a shell-based property test script that:
    - Checks `kubectl get pods -n realestate -l app.kubernetes.io/name=vllm` reports at least one pod in `Running`/`Ready` state
    - Checks `kubectl describe node inference | grep "gpu.intel.com/xe"` shows non-zero allocatable GPU devices
    - Checks vLLM pod logs do NOT contain `ConnectionError` or `Network is unreachable` referencing `huggingface.co`
    - Sends a test `POST /api/v1/chatbot/test/stream` request and asserts HTTP 200 with a streamed agent reply
  - The test assertions match the Expected Behavior Properties from design: `vllmPodHealthy() = true AND intelGpuDevicesHealthy() = true AND modelLoadedWithoutHuggingFaceEgress() = true AND result.httpStatus = 200`
  - Run test on UNFIXED infrastructure
  - **EXPECTED OUTCOME**: Test FAILS (this is correct — it proves the bug exists: pods in CrashLoopBackOff, GPU unhealthy, endpoint unreachable)
  - Document counterexamples found:
    - vLLM pod logs show `requests.exceptions.ConnectionError: ... [Errno 101] Network is unreachable` attempting `huggingface.co`
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

- [ ] 3. Fix for vLLM inference outage — restore chatbot agent reply generation

  - [x] 3.1 Add offline environment variables to vLLM Deployment manifest
    - Edit `infra/k8s/app/shared/vllm.yml`
    - Add `HF_HUB_OFFLINE=1` env var to the vLLM container spec to prevent HuggingFace Hub from attempting online downloads
    - Add `TRANSFORMERS_OFFLINE=1` env var as belt-and-suspenders to ensure transformers library also stays offline
    - _Bug_Condition: isBugCondition(X) where vllmModelUnavailableWithoutEgress() — pod crashes attempting download from huggingface.co_
    - _Expected_Behavior: vLLM loads model exclusively from local /models PVC cache without network egress_
    - _Preservation: No changes to ports, volumes, resource requests, or network policies for other pods_
    - _Requirements: 2.3_

  - [x] 3.2 Create model-download Kubernetes Job
    - Create `infra/k8s/jobs/download-model.yml`
    - Job uses `python:3.12-slim` image with `huggingface_hub` pip-installed
    - Mounts `vllm-models-pvc` at `/models`
    - Downloads `Intel/Qwen3-30B-A3B-Instruct-2507-int4-AutoRound` into HuggingFace cache layout under `/models`
    - Include a temporary egress NetworkPolicy allowing HTTPS to the internet for the Job pod only
    - Job runs once to completion; after success the PVC contains the full model for all subsequent vLLM starts
    - Apply the Job: `kubectl apply -f infra/k8s/jobs/download-model.yml`
    - Wait for Job completion: `kubectl wait --for=condition=complete job/download-model -n realestate --timeout=1800s`
    - Verify model files exist: `kubectl exec` into a debug pod mounting `vllm-models-pvc` and confirm model directory is populated
    - After success, delete the temporary egress NetworkPolicy
    - _Bug_Condition: isBugCondition(X) where vllmModelUnavailableWithoutEgress() — PVC was empty, no model to load offline_
    - _Expected_Behavior: modelLoadedWithoutHuggingFaceEgress() = true; model served from /models PVC_
    - _Preservation: Temporary egress policy scoped only to download Job; removed after completion_
    - _Requirements: 2.3_

  - [x] 3.3 Restart Intel GPU device plugin on inference node
    - Delete the `intel-gpu-plugin` pod on the `inference` node to force re-detection of `/dev/dri` devices:
      ```
      kubectl delete pod -n kube-system -l app.kubernetes.io/name=intel-gpu-plugin --field-selector spec.nodeName=inference
      ```
    - Wait for replacement pod to become Ready
    - Verify GPU health: `kubectl describe node inference | grep -A5 "Allocatable"` — should show `gpu.intel.com/xe: 2`
    - If devices still unhealthy: verify `/dev/dri/renderD128` exists on the node, check `xe` or `i915` kernel driver is loaded, consider plugin version upgrade
    - _Bug_Condition: isBugCondition(X) where intelGpuDevicesUnhealthy() — pod admission rejected_
    - _Expected_Behavior: intelGpuDevicesHealthy() = true; gpu.intel.com/xe shows non-zero allocatable_
    - _Preservation: Only restarts the specific plugin pod on inference node; no changes to other nodes or DaemonSet spec_
    - _Requirements: 2.4_

  - [-] 3.4 Rollout restart vLLM deployment
    - After model is provisioned (3.2) and GPU is healthy (3.3), apply the updated manifest and restart:
      ```
      kubectl apply -f infra/k8s/app/shared/vllm.yml
      kubectl rollout restart deployment/vllm -n realestate
      ```
    - Wait for rollout: `kubectl rollout status deployment/vllm -n realestate --timeout=300s`
    - Verify pod reaches `Running`/`Ready`: `kubectl get pods -n realestate -l app.kubernetes.io/name=vllm`
    - Verify pod logs show model loaded from `/models` with no network download attempts
    - _Bug_Condition: Both root causes resolved — pod should now start successfully_
    - _Expected_Behavior: vllmPodHealthy() = true; pod Running/Ready, serving on port 8000_
    - _Preservation: No changes to other deployments in the namespace_
    - _Requirements: 2.1, 2.2, 2.3, 2.4_

  - [~] 3.5 Verify bug condition exploration test now passes
    - **Property 1: Expected Behavior** — vLLM Inference Restored
    - **IMPORTANT**: Re-run the SAME test from task 1 — do NOT write a new test
    - The test from task 1 encodes the expected behavior: healthy pod, healthy GPU, model loaded offline, inference returns HTTP 200
    - When this test passes, it confirms the expected behavior is satisfied
    - Run bug condition exploration test from step 1
    - **EXPECTED OUTCOME**: Test PASSES (confirms bug is fixed)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [~] 3.6 Verify preservation tests still pass
    - **Property 2: Preservation** — Non-Inference Behavior Unchanged
    - **IMPORTANT**: Re-run the SAME tests from task 2 — do NOT write new tests
    - Run preservation property tests from step 2
    - **EXPECTED OUTCOME**: Tests PASS (confirms no regressions)
    - Confirm config page, health endpoint, chatbot CRUD, error-handling path, and other AI features still work
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [~] 4. Checkpoint — Ensure all tests pass
  - Run full bug condition test (Property 1) — must PASS
  - Run full preservation test suite (Property 2) — must PASS
  - End-to-end validation: send message via "Probar conversación" panel and verify streamed agent reply is received (HTTP 200)
  - End-to-end validation: confirm WhatsApp inbound path generates agent reply via baileys
  - Cluster health: `kubectl get pods -n realestate` shows all pods healthy
  - GPU allocation: `kubectl describe node inference` shows `gpu.intel.com/xe` with expected allocatable count
  - Ensure all tests pass, ask the user if questions arise.
