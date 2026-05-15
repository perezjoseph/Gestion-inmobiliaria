# Implementation Plan: OVMS LLM Deployment

## Overview

Deploy OpenVINO Model Server as the dedicated LLM inference backend on the `inference` worker node's Intel Arc dGPU. The OCR service stays on the `coreos` control plane node using its iGPU. This involves extending the Intel GPU plugin, creating the OVMS manifest, pinning OCR to coreos, updating the backend endpoint, and wiring everything into the kustomization.

## Tasks

- [x] 1. Extend Intel GPU Plugin to inference node
  - [x] 1.1 Update `infra/k8s/intel-gpu-plugin.yml` DaemonSet scheduling
    - Replace `nodeSelector: kubernetes.io/hostname: coreos` with a nodeAffinity that matches both `coreos` and `inference` hostnames
    - Use `requiredDuringSchedulingIgnoredDuringExecution` with `operator: In, values: [coreos, inference]`
    - Verify existing plugin on `coreos` is not disrupted
    - _Requirements: 1.1, 1.2, 1.3_

- [x] 2. Create OVMS model configuration files
  - [x] 2.1 Create `infra/k8s/app/ovms/config.json` with mediapipe_config_list
    - Define the model name as `Qwen3-30B-A3B` with base_path `/models/Qwen3-30B-A3B`
    - Use empty `model_config_list` since we use MediaPipe graph mode
    - _Requirements: 2.4, 11.2_

  - [x] 2.2 Create `infra/k8s/app/ovms/graph.pbtxt` with LLMCalculator MediaPipe graph
    - Set `models_path` to `/models/Qwen3-30B-A3B/1/model`
    - Set `target_device` to `GPU`
    - Configure `enable_prefix_caching: true`, `max_num_batched_tokens: 4096`, `cache_size: 8`
    - Set `plugin_config` to `NUM_STREAMS 1`
    - _Requirements: 2.4, 11.2_

- [x] 3. Create OVMS Kubernetes manifest
  - [x] 3.1 Create `infra/k8s/app/ovms.yml` with PersistentVolumeClaim
    - Name: `ovms-models-pvc`, namespace: `realestate`
    - StorageClass: `local-path`, access mode: `ReadWriteOnce`
    - Request 25Gi storage capacity
    - _Requirements: 5.1, 5.2, 5.3_

  - [x] 3.2 Add OVMS Deployment to `infra/k8s/app/ovms.yml`
    - Image: `openvino/model_server:latest-gpu`
    - Single replica, `Recreate` strategy
    - NodeSelector: `kubernetes.io/hostname: inference`
    - Container port 8000
    - Args: `--config_path /models/config.json --port 8000`
    - Mount PVC at `/models` with `readOnly: true`
    - GPU resource: `gpu.intel.com/xe: "1"` in requests and limits
    - CPU: 500m request / 4000m limit
    - Memory: 16Gi request / 20Gi limit
    - Ephemeral storage: 128Mi request / 512Mi limit
    - `automountServiceAccountToken: false`
    - Security context: `runAsNonRoot: true`, `runAsGroup: 44`, `allowPrivilegeEscalation: false`
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 5.4, 9.1, 9.2, 9.3, 10.1, 10.2, 10.3_

  - [x] 3.3 Add health probes to OVMS Deployment
    - Readiness probe: HTTP GET `/v1/config` port 8000, initialDelaySeconds: 120, periodSeconds: 30, timeoutSeconds: 10
    - Liveness probe: HTTP GET `/v1/config` port 8000, initialDelaySeconds: 180, periodSeconds: 30, timeoutSeconds: 10
    - _Requirements: 4.1, 4.2, 4.3, 4.4_

  - [x] 3.4 Add OVMS Service to `infra/k8s/app/ovms.yml`
    - Name: `ovms`, namespace: `realestate`, type: ClusterIP
    - Selector: `app.kubernetes.io/name: ovms`
    - Port 8000 → targetPort 8000
    - _Requirements: 3.1, 3.2, 3.3_

- [x] 4. Pin OCR service to coreos node
  - [x] 4.1 Update `infra/k8s/app/ocr-service.yml` to add nodeSelector
    - Add `nodeSelector: kubernetes.io/hostname: coreos` to the pod spec
    - Keep existing `gpu.intel.com/i915: "1"` resource claim (uses coreos iGPU)
    - Keep `OPENVINO_DEVICE=GPU` (iGPU is the only Intel GPU on coreos)
    - _Requirements: 6.1, 6.2, 6.3_

- [x] 5. Checkpoint - Verify manifest structure
  - Ensure all YAML manifests are valid, ask the user if questions arise.

- [x] 6. Update backend and kustomization
  - [x] 6.1 Update `infra/k8s/app/backend.yml` environment variables
    - Change `OVMS_ENDPOINT` from `http://ocr-service:8000/v1` to `http://ovms:8000/v3`
    - Change `OVMS_CHAT_MODEL` from `qwen3.6` to `Qwen3-30B-A3B`
    - _Requirements: 7.1, 7.2_

  - [x] 6.2 Update `infra/k8s/app/kustomization.yml` to include OVMS resource
    - Add `ovms.yml` to the `resources` list after `ocr-service.yml` and before `baileys.yml`
    - _Requirements: 8.1, 8.2_

- [x] 7. Create model export Job manifest
  - [x] 7.1 Create `infra/k8s/app/ovms/export-job.yml` Kubernetes Job
    - Job runs `export_model.py` to export Qwen3-30B-A3B with INT4 quantization for GPU
    - NodeSelector: `kubernetes.io/hostname: inference`
    - Mount the `ovms-models-pvc` PVC as read-write
    - Include GPU resource claim for export (needs GPU for weight calibration)
    - Set `restartPolicy: Never` and `backoffLimit: 2`
    - On failure, report via Job status without corrupting existing files
    - _Requirements: 11.1, 11.2, 11.3, 11.4_

- [x] 8. Final checkpoint - Validate all manifests
  - Ensure all YAML files parse correctly and cross-references are consistent, ask the user if questions arise.

## Notes

- No property-based tests are applicable — this feature is purely IaC (Kubernetes manifests)
- Validation is done via smoke tests (YAML parsing, field assertions) and integration tests (cluster-level)
- The model export Job should be run manually before scaling up OVMS (Requirement 11.3)
- The Intel GPU plugin must be running on `inference` before OVMS can schedule (Requirement 1.2)
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1", "2.1", "2.2", "4.1"] },
    { "id": 1, "tasks": ["3.1", "6.1", "6.2"] },
    { "id": 2, "tasks": ["3.2", "3.3", "3.4"] },
    { "id": 3, "tasks": ["7.1"] }
  ]
}
```
