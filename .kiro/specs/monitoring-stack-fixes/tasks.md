# Implementation Plan

## Overview

Fix five operational defects in the Kubernetes monitoring stack: deploy kube-state-metrics, add Loki compactor retention, increase Prometheus memory limit, add self-monitoring alert rules, and replace hardcoded node IPs with dns_sd_configs for in-cluster nodes.

## Task Dependency Graph

```json
{
  "waves": [
    { "tasks": ["1", "2"] },
    { "tasks": ["3.1", "3.2", "3.3", "3.4", "3.5", "3.6"] },
    { "tasks": ["3.7", "3.8"] },
    { "tasks": ["4"] }
  ]
}
```

## Notes

- This is a Kubernetes infrastructure fix involving YAML manifests, not application code.
- Tests validate manifest structure and content rather than runtime behavior.
- The networkstorage node (192.168.88.22) is external to the cluster and must remain as a static target.
- All five defects are independent — fixes can be applied in any order within wave 2.

## Tasks

- [x] 1. Write bug condition exploration test
  - **Property 1: Bug Condition** - Monitoring Stack Infrastructure Defects
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bugs exist
  - **DO NOT attempt to fix the test or the code when it fails**
  - **NOTE**: This test encodes the expected behavior - it will validate the fix when it passes after implementation
  - **GOAL**: Surface counterexamples that demonstrate all five defects exist in the current manifests
  - **Scoped PBT Approach**: Scope the property to concrete failing cases for each defect:
    - Defect 1: Assert a kube-state-metrics Deployment exists in `infra/k8s/` → will fail (no KSM manifest)
    - Defect 2: Assert Loki config contains `retention_enabled: true` in compactor section → will fail (field absent)
    - Defect 3: Assert Prometheus memory limit >= 1Gi (1073741824 bytes) → will fail (currently 512Mi)
    - Defect 4: Assert alerts.yml contains PrometheusDown, TargetDown, and GpuOverheating rules → will fail (no self-monitoring group)
    - Defect 5: Assert node-exporter job uses `dns_sd_configs` for in-cluster nodes → will fail (uses hardcoded IPs 192.168.88.112, 192.168.88.115)
  - Write a manifest validation test (shell script or Python) that parses YAML and checks all five conditions
  - Run test on UNFIXED manifests
  - **EXPECTED OUTCOME**: Test FAILS (this is correct - it proves the bugs exist)
  - Document counterexamples: KSM not deployed, no retention config, 512Mi limit, no self-monitoring alerts, hardcoded IPs
  - Mark task complete when test is written, run, and failure is documented
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [x] 2. Write preservation property tests (BEFORE implementing fix)
  - **Property 2: Preservation** - Existing Monitoring Configuration Unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe: All existing alert rules in alerts.yml (backend, database, infrastructure, backups, slo, business groups) have specific expressions, thresholds, for durations, and severity labels
  - Observe: Prometheus scrape configs for xpumanager, backend, backend-dev, vllm targets exist with 15s interval and specific labels
  - Observe: Loki limits_config has `reject_old_samples_max_age: 168h`, `max_query_series: 500`, `allow_structured_metadata: true`
  - Observe: Node-exporter networkstorage target (192.168.88.22:9100) with `node: "networkstorage"` label exists
  - Observe: Alertmanager routing at `alertmanager.monitoring.svc.cluster.local:9093` is referenced in Prometheus config
  - Write preservation test that captures snapshots of all existing alert rules (expressions, thresholds, labels), scrape configs (targets, intervals, paths), Loki ingestion settings, and networkstorage static entry
  - Assert existing alert groups contain exactly: HighErrorRate, HighLatencyP95, PodRestarts, PodNotReady, DatabaseConnectionPoolExhausted, DatabaseDown, DiskUsageHigh, DiskUsageCritical, HighMemoryUsage, BackupJobFailed, BackupJobMissing, SLOAvailabilityBurnRateCritical, SLOAvailabilityBurnRateHigh, SLOLatencyBurnRateCritical, HighOverduePayments, AIInferenceDown, LoginBruteForce
  - Assert xpumanager, backend, backend-dev, vllm scrape jobs exist with unchanged targets and labels
  - Assert networkstorage (192.168.88.22:9100) remains as a static_configs target with node="networkstorage"
  - Verify test passes on UNFIXED manifests
  - **EXPECTED OUTCOME**: Tests PASS (this confirms baseline behavior to preserve)
  - Mark task complete when tests are written, run, and passing on unfixed code
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [x] 3. Fix for monitoring stack infrastructure defects

  - [x] 3.1 Create kube-state-metrics manifest
    - Create new file `infra/k8s/kube-state-metrics.yml`
    - Include ServiceAccount, ClusterRole (list/watch on pods, jobs, deployments, nodes, namespaces, replicasets, statefulsets), ClusterRoleBinding, Deployment (registry.k8s.io/kube-state-metrics/kube-state-metrics:v2.15.0, port 8080), and Service in monitoring namespace
    - _Bug_Condition: kubeStateMetricsDeployed(namespace) is false_
    - _Expected_Behavior: KSM deployed in monitoring namespace, serving metrics on port 8080_
    - _Preservation: No existing manifests modified_
    - _Requirements: 2.1_

  - [x] 3.2 Add kube-state-metrics scrape job to Prometheus config
    - Add new `scrape_configs` entry in `infra/k8s/monitoring.yml` ConfigMap for job `kube-state-metrics` targeting `kube-state-metrics.monitoring.svc.cluster.local:8080`
    - _Bug_Condition: alerts reference kube_* metrics but no scrape job collects them_
    - _Expected_Behavior: Prometheus scrapes KSM and evaluates KSM-dependent alerts_
    - _Preservation: Existing scrape_configs (xpumanager, node-exporter, backend, backend-dev, vllm) unchanged_
    - _Requirements: 2.1_

  - [x] 3.3 Add Loki compactor retention configuration
    - In `infra/k8s/logging.yml`, add `retention_enabled: true` and `delete_request_store: filesystem` to the `compactor` section
    - Add `retention_period: 336h` to the `limits_config` section
    - _Bug_Condition: lokiRetentionConfigured(config) is false — compactor has no retention fields_
    - _Expected_Behavior: Loki enforces 14-day (336h) retention, auto-deleting old chunks_
    - _Preservation: reject_old_samples_max_age: 168h, max_query_series: 500, and all other limits_config values unchanged_
    - _Requirements: 2.2_

  - [x] 3.4 Increase Prometheus memory limit
    - In `infra/k8s/monitoring.yml`, change Prometheus container `resources.limits.memory` from `512Mi` to `1536Mi` and `resources.requests.memory` from `256Mi` to `512Mi`
    - _Bug_Condition: prometheusMemoryLimit <= 512Mi with 30d retention and 5+ scrape jobs_
    - _Expected_Behavior: Memory limit 1536Mi prevents OOM kills under realistic cardinality_
    - _Preservation: CPU limits/requests, storage path, retention time, all other Prometheus settings unchanged_
    - _Requirements: 2.3_

  - [x] 3.5 Add self-monitoring alert rules
    - In `infra/k8s/alerts.yml`, add a new `monitoring` group with three rules:
    - `PrometheusDown`: expr `absent(up{job="prometheus"})` for 5m, severity critical
    - `TargetDown`: expr `up == 0` for 5m, severity critical
    - `GpuOverheating`: expr `xpum_gpu_temperature > 85` for 5m, severity critical
    - _Bug_Condition: selfMonitoringAlertExists(condition) is false for all three conditions_
    - _Expected_Behavior: Dedicated alerts fire for Prometheus down, target down, GPU overheating_
    - _Preservation: All existing alert groups (backend, database, infrastructure, backups, slo, business) unchanged_
    - _Requirements: 2.4_

  - [x] 3.6 Replace hardcoded node IPs with dns_sd_configs for in-cluster nodes
    - In `infra/k8s/monitoring.yml`, replace the node-exporter job's static_configs entries for coreos (192.168.88.112) and inference (192.168.88.115) with a `dns_sd_configs` entry pointing to `node-exporter.monitoring.svc.cluster.local` port 9100
    - Keep a separate static_configs entry for networkstorage (192.168.88.22:9100) with `node: "networkstorage"` label since it's external to the cluster
    - _Bug_Condition: scrapeConfigUsesHardcodedIP(job, nodeType) for in-cluster nodes (coreos, inference)_
    - _Expected_Behavior: dns_sd_configs resolves node-exporter headless Service for in-cluster nodes; networkstorage remains static_
    - _Preservation: networkstorage target (192.168.88.22:9100) with node="networkstorage" label preserved as static_configs_
    - _Requirements: 2.5_

  - [x] 3.7 Verify bug condition exploration test now passes
    - **Property 1: Expected Behavior** - Monitoring Stack Defects Resolved
    - **IMPORTANT**: Re-run the SAME test from task 1 - do NOT write a new test
    - The test from task 1 encodes the expected behavior for all five defects
    - When this test passes, it confirms: KSM deployed, retention configured, memory sufficient, self-monitoring alerts exist, dns_sd_configs used
    - Run bug condition exploration test from step 1
    - **EXPECTED OUTCOME**: Test PASSES (confirms all five bugs are fixed)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [x] 3.8 Verify preservation tests still pass
    - **Property 2: Preservation** - Existing Monitoring Configuration Unchanged
    - **IMPORTANT**: Re-run the SAME tests from task 2 - do NOT write new tests
    - Run preservation property tests from step 2
    - **EXPECTED OUTCOME**: Tests PASS (confirms no regressions)
    - Confirm all existing alert rules, scrape configs, Loki settings, and networkstorage target are unchanged
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [x] 4. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.
  - Verify kube-state-metrics manifest is valid YAML with correct structure
  - Verify monitoring.yml changes are syntactically correct
  - Verify logging.yml changes are syntactically correct
  - Verify alerts.yml changes are syntactically correct
  - Confirm exploration test (task 1) passes on fixed manifests
  - Confirm preservation test (task 2) passes on fixed manifests
