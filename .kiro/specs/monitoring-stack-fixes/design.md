# Monitoring Stack Fixes Bugfix Design

## Overview

The Kubernetes monitoring stack in `infra/k8s/` has five operational defects that collectively undermine the reliability guarantees it is supposed to provide. This design formalizes the bug conditions, identifies root causes from the actual manifest files, and plans minimal, targeted fixes for each defect while preserving all existing monitoring behavior.

The fix strategy is additive: deploy a missing component (kube-state-metrics), add configuration blocks (Loki retention), adjust resource limits (Prometheus memory), add new alert rules (self-monitoring), and replace hardcoded IPs with service discovery for in-cluster nodes while keeping the external node's static config.

## Glossary

- **Bug_Condition (C)**: The set of conditions under which the monitoring stack fails to behave correctly — alerts not firing, storage filling up, OOM kills, missing self-monitoring, or scraping failure on IP change
- **Property (P)**: The desired behavior when each bug condition is triggered — alerts evaluate, retention enforced, no OOM, self-monitoring fires, scraping survives IP changes
- **Preservation**: All existing alert rules, scrape targets, Alertmanager routing, Loki ingestion, and node-exporter metrics collection that must remain unchanged
- **kube-state-metrics (KSM)**: A Kubernetes add-on that generates metrics about the state of Kubernetes objects (pods, jobs, deployments)
- **Compactor**: Loki component responsible for compacting index files and enforcing retention policies
- **dns_sd_configs**: Prometheus service discovery mechanism that resolves DNS SRV or A records to discover scrape targets
- **networkstorage**: External node (192.168.88.22) outside the Kubernetes cluster that runs node-exporter

## Bug Details

### Bug Condition

The monitoring stack fails when any of five independent conditions hold. Each condition corresponds to a distinct infrastructure gap in the current manifests.

**Formal Specification:**
```
FUNCTION isBugCondition(input)
  INPUT: input of type MonitoringStackState
  OUTPUT: boolean

  RETURN (
    // Defect 1: KSM-dependent alerts never evaluate
    (input.alertRule.referencesMetric IN [
      'kube_pod_container_status_restarts_total',
      'kube_pod_status_ready',
      'kube_job_status_failed',
      'kube_job_status_completion_time'
    ] AND NOT kubeStateMetricsDeployed(input.namespace))

    OR

    // Defect 2: Loki has no retention, PVC fills up
    (input.lokiDataAge > 0 AND NOT lokiRetentionConfigured(input.lokiConfig))

    OR

    // Defect 3: Prometheus OOMs under load
    (input.prometheusRetention == '30d'
     AND input.scrapeJobCount >= 5
     AND input.prometheusMemoryLimit <= 512Mi)

    OR

    // Defect 4: No self-monitoring alerts exist
    (input.condition IN ['prometheus_down', 'target_down', 'gpu_overheating']
     AND NOT selfMonitoringAlertExists(input.condition))

    OR

    // Defect 5: Node IP changes break scraping
    (input.nodeIPChanged == true
     AND scrapeConfigUsesHardcodedIP(input.job, input.nodeType)
     AND input.nodeType IN ['coreos', 'inference'])
  )
END FUNCTION
```

### Examples

- **Defect 1**: Alert `PodRestarts` references `kube_pod_container_status_restarts_total` but kube-state-metrics is not deployed → alert never evaluates, pod restart storms go undetected
- **Defect 2**: Loki ingests 500MB/day of logs with no retention → after ~20 days the 10Gi PVC is full, Loki stops ingesting
- **Defect 3**: Prometheus with 30-day retention and 5 scrape jobs grows to ~800MB RSS → 512Mi limit triggers OOM kill, monitoring goes offline
- **Defect 4**: Prometheus itself crashes (OOM) → no alert fires because there is no self-monitoring rule; the outage is only discovered manually
- **Defect 5**: DHCP reassigns `coreos` node from 192.168.88.112 to 192.168.88.120 → Prometheus scrapes the old IP indefinitely, all node metrics for that host disappear

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**
- All existing alert rules (HighErrorRate, HighLatencyP95, DatabaseConnectionPoolExhausted, DatabaseDown, DiskUsageHigh, DiskUsageCritical, HighMemoryUsage, BackupJobFailed, BackupJobMissing, SLO burn rates, HighOverduePayments, AIInferenceDown, LoginBruteForce) continue to fire with the same expressions, thresholds, and severity labels
- Loki continues to ingest log entries within the 7-day `reject_old_samples_max_age` window normally
- Prometheus continues to scrape xpumanager, backend, backend-dev, and vllm targets at 15s intervals with the same labels and metrics paths
- Alertmanager continues to route alerts through the existing routing tree (kiro-autofix for critical production, default for others) with the same inhibit rules
- Node-exporter continues to collect the same node-level metrics with node labels (coreos, inference, networkstorage) on port 9100
- The networkstorage node (192.168.88.22, external to cluster) continues to be scraped via static_configs since it cannot be discovered via Kubernetes service discovery

**Scope:**
All inputs that do NOT match any of the five bug conditions should be completely unaffected by this fix. This includes:
- Existing scrape configs for xpumanager, backend, backend-dev, vllm
- Existing Grafana datasources and dashboards
- Existing Loki ingestion pipeline and Alloy DaemonSet configuration
- Existing Alertmanager configuration and routing
- Prometheus storage path, retention time, and scrape intervals

## Hypothesized Root Cause

Based on analysis of the actual manifest files:

1. **Missing kube-state-metrics deployment** (Defect 1): The `alerts.yml` ConfigMap references `kube_*` metrics in PodRestarts, PodNotReady, BackupJobFailed, and BackupJobMissing rules, but no kube-state-metrics Deployment/Service/RBAC exists anywhere in the `infra/k8s/` directory. Without the metrics source, Prometheus has no data points to evaluate these rules against.

2. **Missing Loki compactor retention config** (Defect 2): The `logging.yml` ConfigMap defines a `compactor` section with only `working_directory` set. The `retention_enabled: true` and `retention_period` fields are absent. Without these, the compactor runs but never deletes old chunks.

3. **Undersized Prometheus memory limit** (Defect 3): The Prometheus Deployment in `monitoring.yml` sets `limits.memory: 512Mi`. With 30-day retention (`--storage.tsdb.retention.time=30d`) and 5 active scrape jobs, the TSDB head block and WAL easily exceed this limit during compaction or query load.

4. **No self-monitoring alert rules** (Defect 4): The `alerts.yml` ConfigMap has groups for backend, database, infrastructure, backups, slo, and business — but none for monitoring infrastructure itself. No rules use `absent()`, the `up` meta-metric for all jobs, or xpumanager temperature metrics.

5. **Hardcoded node IPs in scrape config** (Defect 5): The `node-exporter` job in `monitoring.yml` uses `static_configs` with literal IPs (192.168.88.112, 192.168.88.115, 192.168.88.22). A headless Service `node-exporter` already exists in the monitoring namespace (defined at the bottom of `monitoring.yml`), but Prometheus doesn't use it. The external networkstorage node (192.168.88.22) must remain as a static target since it's outside the cluster.

## Correctness Properties

Property 1: Bug Condition - KSM-Dependent Alerts Evaluate

_For any_ alert rule that references kube-state-metrics metrics (kube_pod_container_status_restarts_total, kube_pod_status_ready, kube_job_status_failed, kube_job_status_completion_time), the fixed monitoring stack SHALL have kube-state-metrics deployed and serving those metrics in the monitoring namespace, so that Prometheus can evaluate and fire those alerts.

**Validates: Requirements 2.1**

Property 2: Bug Condition - Loki Retention Prevents PVC Exhaustion

_For any_ Loki deployment storing log data over time, the fixed configuration SHALL enforce a 14-day retention period via the compactor with `retention_enabled: true`, automatically deleting chunks older than 14 days.

**Validates: Requirements 2.2**

Property 3: Bug Condition - Prometheus Memory Sufficient for Workload

_For any_ Prometheus deployment running with 30-day retention and 5+ scrape jobs, the fixed deployment SHALL have a memory limit of at least 1Gi to prevent OOM kills under realistic cardinality.

**Validates: Requirements 2.3**

Property 4: Bug Condition - Self-Monitoring Alerts Exist

_For any_ condition where Prometheus is down, a scrape target is unreachable, or GPU temperature exceeds safe range, the fixed alert rules SHALL include dedicated PrometheusDown, TargetDown, and GpuOverheating rules that fire for those conditions.

**Validates: Requirements 2.4**

Property 5: Bug Condition - Node-Exporter Service Discovery

_For any_ in-cluster node running node-exporter, the fixed scrape config SHALL use dns_sd_configs against the headless Service (node-exporter.monitoring.svc.cluster.local) instead of hardcoded IPs, while maintaining a separate static_configs entry for the external networkstorage node.

**Validates: Requirements 2.5**

Property 6: Preservation - Existing Alerts Unchanged

_For any_ existing alert rule (backend, database, infrastructure, backups, slo, business groups), the fixed configuration SHALL preserve the exact same expressions, thresholds, `for` durations, and severity labels.

**Validates: Requirements 3.1**

Property 7: Preservation - Existing Scrape Targets Unchanged

_For any_ existing scrape target (xpumanager, backend, backend-dev, vllm), the fixed configuration SHALL preserve the same intervals, labels, and metrics paths. The networkstorage node SHALL continue to be scraped at 192.168.88.22:9100 with the node="networkstorage" label.

**Validates: Requirements 3.3, 3.5, 3.6**

Property 8: Preservation - Loki Ingestion Unchanged

_For any_ log entry within the 7-day reject_old_samples_max_age window, the fixed Loki configuration SHALL continue to ingest and store it normally without rejection.

**Validates: Requirements 3.2**

## Fix Implementation

### Changes Required

**File**: `infra/k8s/kube-state-metrics.yml` (NEW)

**Specific Changes**:
1. **Deploy kube-state-metrics**: Create a new manifest with Deployment, Service, ServiceAccount, ClusterRole, and ClusterRoleBinding in the monitoring namespace. Use the official `registry.k8s.io/kube-state-metrics/kube-state-metrics:v2.15.0` image. Expose metrics on port 8080. Add a Prometheus scrape job for it.

---

**File**: `infra/k8s/monitoring.yml`

**Function**: Prometheus ConfigMap and Deployment

**Specific Changes**:
2. **Increase Prometheus memory limit**: Change `limits.memory` from `512Mi` to `1536Mi` and `requests.memory` from `256Mi` to `512Mi` in the Prometheus container spec.

3. **Replace hardcoded node-exporter IPs with service discovery**: Replace the `static_configs` block for in-cluster nodes (coreos, inference) with a `dns_sd_configs` entry pointing to `node-exporter.monitoring.svc.cluster.local` on port 9100. Keep a separate `static_configs` entry for networkstorage (192.168.88.22:9100) since it's external.

4. **Add kube-state-metrics scrape job**: Add a new `scrape_configs` entry for job `kube-state-metrics` targeting `kube-state-metrics.monitoring.svc.cluster.local:8080`.

---

**File**: `infra/k8s/logging.yml`

**Function**: Loki ConfigMap

**Specific Changes**:
5. **Enable compactor retention**: Add `retention_enabled: true` and `delete_request_store: filesystem` to the `compactor` section. Add `retention_period: 336h` (14 days) to the `limits_config` section.

---

**File**: `infra/k8s/alerts.yml`

**Function**: Prometheus alerts ConfigMap

**Specific Changes**:
6. **Add self-monitoring alert group**: Add a new `monitoring` group with three rules:
   - `PrometheusDown`: `absent(up{job="prometheus"})` or use `up{job="prometheus"} == 0` — fires when Prometheus self-scrape fails (requires adding a prometheus self-scrape job)
   - `TargetDown`: `up == 0` for any job, for 5m — fires when any scrape target is unreachable
   - `GpuOverheating`: Expression using xpumanager temperature metrics exceeding threshold (e.g., `xpum_gpu_temperature > 85`)

## Testing Strategy

### Validation Approach

The testing strategy follows a two-phase approach: first, surface counterexamples that demonstrate the bugs on unfixed manifests, then verify the fix works correctly and preserves existing behavior. Since these are infrastructure (YAML) changes rather than application code, testing involves manifest validation, deployment verification, and PromQL expression validation.

### Exploratory Bug Condition Checking

**Goal**: Surface counterexamples that demonstrate the bugs BEFORE implementing the fix. Confirm or refute the root cause analysis.

**Test Plan**: Validate the current manifests against the expected conditions. Check for missing resources, missing config fields, and incorrect values.

**Test Cases**:
1. **KSM Missing Test**: Scan all YAML files in `infra/k8s/` for a kube-state-metrics Deployment — confirm it does not exist (will demonstrate defect 1)
2. **Loki Retention Test**: Parse the Loki ConfigMap and check for `retention_enabled` in compactor section — confirm it is absent (will demonstrate defect 2)
3. **Prometheus Memory Test**: Parse Prometheus Deployment and check memory limit — confirm it is 512Mi (will demonstrate defect 3)
4. **Self-Monitoring Test**: Parse alerts ConfigMap and check for PrometheusDown/TargetDown/GpuOverheating rules — confirm they don't exist (will demonstrate defect 4)
5. **Hardcoded IP Test**: Parse Prometheus ConfigMap node-exporter job and check for hardcoded IPs — confirm 192.168.88.112 and 192.168.88.115 are present (will demonstrate defect 5)

**Expected Counterexamples**:
- No kube-state-metrics manifest exists anywhere in the repository
- Loki compactor section has no retention fields
- Prometheus memory limit is exactly 512Mi
- No self-monitoring alert rules exist
- Node-exporter scrape uses literal IP addresses for in-cluster nodes

### Fix Checking

**Goal**: Verify that for all inputs where the bug condition holds, the fixed manifests produce the expected behavior.

**Pseudocode:**
```
FOR ALL manifest WHERE isBugCondition(manifest) DO
  result := validateFixedManifest(manifest)
  ASSERT expectedBehavior(result)
END FOR
```

**Specific checks after fix:**
- kube-state-metrics Deployment exists in monitoring namespace with correct image and port
- Loki config contains `retention_enabled: true` and `retention_period: 336h`
- Prometheus memory limit >= 1Gi
- Alerts ConfigMap contains PrometheusDown, TargetDown, and GpuOverheating rules
- Node-exporter scrape uses dns_sd_configs for in-cluster nodes
- Networkstorage (192.168.88.22) still present as static_configs

### Preservation Checking

**Goal**: Verify that for all inputs where the bug condition does NOT hold, the fixed manifests produce the same result as the original.

**Pseudocode:**
```
FOR ALL manifest WHERE NOT isBugCondition(manifest) DO
  ASSERT originalManifest(manifest) = fixedManifest(manifest)
END FOR
```

**Testing Approach**: Diff-based validation ensures only intended changes are made. For YAML manifests, this means comparing the original and fixed versions to ensure no unintended modifications to existing resources.

**Test Plan**: Compare original and fixed manifests to verify no unintended changes to existing resources.

**Test Cases**:
1. **Existing Alerts Preservation**: Diff alerts.yml and verify all existing rules (backend, database, infrastructure, backups, slo, business groups) have identical expressions, thresholds, and labels
2. **Existing Scrape Configs Preservation**: Diff monitoring.yml and verify xpumanager, backend, backend-dev, vllm scrape configs are unchanged
3. **Loki Ingestion Preservation**: Verify `reject_old_samples_max_age: 168h` and other limits_config values remain unchanged
4. **Alertmanager Preservation**: Verify alertmanager.yml is not modified at all
5. **Networkstorage Static Config**: Verify 192.168.88.22:9100 with node="networkstorage" label remains as a static_configs entry

### Unit Tests

- Validate kube-state-metrics manifest has correct RBAC permissions (list/watch pods, jobs, deployments, etc.)
- Validate Loki retention config syntax is correct for Loki 3.7.0
- Validate Prometheus resource requests < limits
- Validate PromQL syntax of new self-monitoring alert expressions
- Validate dns_sd_configs syntax for node-exporter discovery

### Property-Based Tests

- Generate random subsets of the 5 fixes and verify each fix is independent (no ordering dependency)
- Generate random node counts and verify dns_sd_configs correctly discovers all in-cluster node-exporter instances
- Generate random time-series cardinality values and verify 1536Mi is sufficient for the configured retention

### Integration Tests

- Deploy fixed manifests to cluster and verify kube-state-metrics endpoints respond on port 8080
- Deploy fixed manifests and verify Prometheus successfully scrapes kube-state-metrics (check `up{job="kube-state-metrics"} == 1`)
- Deploy fixed manifests and verify Prometheus discovers node-exporter via DNS (check targets in Prometheus UI)
- Deploy fixed manifests and wait for Loki compactor to run, verify retention is enforced
- Deploy fixed manifests and verify all self-monitoring alerts are in "inactive" state (not firing on healthy cluster)
