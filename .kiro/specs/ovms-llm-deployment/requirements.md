# Requirements Document

## Introduction

Deploy OpenVINO Model Server (OVMS) as the dedicated LLM inference backend for the realestate platform. OVMS serves the Qwen3-30B-A3B model (INT4 quantization) on the Intel Arc discrete GPU located on the `inference` worker node. The existing OCR service (PaddleOCR) remains on the `coreos` control plane node using its Intel iGPU. This enables GPU-accelerated LLM inference without contention between the two workloads.

## Glossary

- **OVMS**: OpenVINO Model Server — an inference server that serves models exported in OpenVINO IR format and exposes OpenAI-compatible HTTP endpoints.
- **Inference_Node**: The `inference` worker node with an Intel Arc discrete GPU and an AMD GPU (4GB VRAM, unused by this feature).
- **Control_Plane_Node**: The `coreos` control-plane node with an Intel integrated GPU (iGPU) already exposed via the Intel GPU plugin as `gpu.intel.com/i915`.
- **OCR_Service**: The PaddleOCR-based container that performs optical character recognition on document images, running on the control plane's iGPU.
- **Backend**: The Rust Actix-web API server that orchestrates business logic and calls downstream services.
- **Kustomization**: The kustomize manifest at `infra/k8s/app/kustomization.yml` that declares all application resources.
- **PV**: Kubernetes PersistentVolume — node-local storage provisioned on the `inference` node for model files.
- **PVC**: Kubernetes PersistentVolumeClaim — the claim that binds a pod to a PV.
- **Model_Export_Job**: A one-shot Kubernetes Job (or manual script execution) that runs `export_model.py` to export and quantize the Qwen3-30B-A3B model to the PV.
- **Intel_GPU_Plugin**: The Kubernetes device plugin (v0.31.1) that exposes `gpu.intel.com/i915` resources to the scheduler.

## Requirements

### Requirement 1: Intel GPU Plugin Extension

**User Story:** As a platform operator, I want the Intel GPU device plugin to run on the `inference` node, so that the Intel Arc dGPU is exposed as a schedulable Kubernetes resource.

#### Acceptance Criteria

1. THE Intel_GPU_Plugin DaemonSet SHALL be updated to schedule on both the `coreos` node and the `inference` node.
2. AFTER the plugin is running on the `inference` node, the node SHALL advertise `gpu.intel.com/xe` resources in its capacity and allocatable fields (the Intel Arc dGPU uses the xe kernel driver).
3. THE Intel_GPU_Plugin update SHALL NOT disrupt the existing plugin instance on the `coreos` node.

### Requirement 2: OVMS Deployment

**User Story:** As a platform operator, I want OVMS deployed as a Kubernetes Deployment in the `realestate` namespace on the `inference` node, so that the backend can perform LLM inference via a stable in-cluster service.

#### Acceptance Criteria

1. THE OVMS Deployment SHALL use the official upstream image `openvino/model_server:latest-gpu` without custom image builds.
2. THE OVMS Deployment SHALL run in the `realestate` namespace with a single replica and `Recreate` update strategy.
3. THE OVMS Deployment SHALL include a nodeSelector targeting `kubernetes.io/hostname: inference` to schedule on the inference node.
4. THE OVMS Deployment SHALL mount the model PVC at the path expected by the OVMS configuration.
5. THE OVMS Deployment SHALL request and limit exactly `gpu.intel.com/xe: "1"` to claim exclusive access to the Intel Arc dGPU (which uses the xe kernel driver).
6. THE OVMS Deployment SHALL set `automountServiceAccountToken: false` on the pod spec.
7. THE OVMS Deployment SHALL expose container port 8000 for the inference API.

### Requirement 3: OVMS Service

**User Story:** As a platform operator, I want a Kubernetes Service fronting OVMS, so that other pods can reach the inference endpoint via a stable DNS name.

#### Acceptance Criteria

1. THE OVMS Service SHALL be named `ovms` in the `realestate` namespace.
2. THE OVMS Service SHALL route traffic on port 8000 to the OVMS container port 8000.
3. THE OVMS Service SHALL select pods using the label `app.kubernetes.io/name: ovms`.

### Requirement 4: OVMS Health Probes

**User Story:** As a platform operator, I want Kubernetes health probes on OVMS, so that the scheduler restarts unhealthy pods and only routes traffic to ready instances.

#### Acceptance Criteria

1. THE OVMS Deployment SHALL define a readiness probe that performs an HTTP GET to `/v1/config` on port 8000.
2. THE OVMS Deployment SHALL configure the readiness probe with an initial delay of at least 120 seconds to allow model loading on GPU.
3. THE OVMS Deployment SHALL define a liveness probe that performs an HTTP GET to `/v1/config` on port 8000.
4. THE OVMS Deployment SHALL configure the liveness probe with an initial delay greater than the readiness probe initial delay.

### Requirement 5: Model Storage

**User Story:** As a platform operator, I want model files stored on a PersistentVolume local to the `inference` node, so that OVMS can load the large model without network transfer overhead.

#### Acceptance Criteria

1. THE PV SHALL use `local-path` storage class bound to the `inference` node.
2. THE PVC SHALL request sufficient storage capacity for the Qwen3-30B-A3B INT4 model files and graph configuration (minimum 20Gi).
3. THE PVC SHALL use `ReadWriteOnce` access mode.
4. THE OVMS Deployment SHALL mount the PVC as a read-only volume at runtime.

### Requirement 6: OCR Service Scheduling

**User Story:** As a platform operator, I want the OCR service to remain on the `coreos` control plane node using its iGPU, so that it does not compete with OVMS for the Arc dGPU on the inference node.

#### Acceptance Criteria

1. THE OCR_Service Deployment SHALL include a nodeSelector targeting `kubernetes.io/hostname: coreos` to ensure it schedules on the control plane node.
2. THE OCR_Service Deployment SHALL retain its `gpu.intel.com/i915: "1"` resource claim to use the control plane's iGPU.
3. THE OCR_Service Deployment SHALL keep `OPENVINO_DEVICE` set to `GPU` (the iGPU is the only Intel GPU on that node).

### Requirement 7: Backend Endpoint Configuration

**User Story:** As a platform operator, I want the backend to point at the new OVMS endpoint, so that LLM chat completions are served by the dedicated inference server.

#### Acceptance Criteria

1. THE Backend Deployment SHALL set the `OVMS_ENDPOINT` environment variable to `http://ovms:8000/v3`.
2. THE Backend Deployment SHALL set the `OVMS_CHAT_MODEL` environment variable to the model name matching the OVMS graph configuration.

### Requirement 8: Kustomization Integration

**User Story:** As a platform operator, I want the OVMS manifest included in the kustomization, so that `kubectl apply -k` deploys the full stack including the inference server.

#### Acceptance Criteria

1. THE Kustomization SHALL include the OVMS resource file in its `resources` list.
2. THE Kustomization SHALL list the OVMS resource after `ocr-service.yml` and before `baileys.yml` to reflect dependency order.

### Requirement 9: OVMS Resource Limits

**User Story:** As a platform operator, I want CPU and memory resource requests and limits on OVMS, so that the scheduler can make informed placement decisions and prevent resource starvation.

#### Acceptance Criteria

1. THE OVMS Deployment SHALL define CPU and memory resource requests appropriate for model serving (minimum 500m CPU, 16Gi memory).
2. THE OVMS Deployment SHALL define CPU and memory resource limits that cap usage (maximum 4000m CPU, 20Gi memory).
3. THE OVMS Deployment SHALL define ephemeral-storage requests and limits.

### Requirement 10: OVMS Security Context

**User Story:** As a platform operator, I want OVMS to run with a restricted security context, so that the container follows the principle of least privilege.

#### Acceptance Criteria

1. THE OVMS Deployment SHALL set `allowPrivilegeEscalation: false` on the container security context.
2. THE OVMS Deployment SHALL run as a non-root user via `runAsNonRoot: true`.
3. THE OVMS Deployment SHALL set `runAsGroup: 44` (video group) to access GPU device files.

### Requirement 11: Model Export Provisioning

**User Story:** As a platform operator, I want a documented model export process, so that the Qwen3-30B-A3B model can be prepared and placed on the PV before OVMS starts.

#### Acceptance Criteria

1. THE Model_Export_Job SHALL export the Qwen3-30B-A3B model with INT4 quantization targeting GPU execution.
2. THE Model_Export_Job SHALL write the exported model files and graph configuration to the PV mount path.
3. THE Model_Export_Job SHALL complete before the OVMS Deployment is scaled up.
4. IF the Model_Export_Job fails, THEN THE Model_Export_Job SHALL report the failure via Kubernetes Job status without corrupting existing model files on the PV.
