# Bugfix Requirements Document

## Introduction

The Kubernetes monitoring stack (`infra/k8s/`) has five operational defects identified during an infrastructure audit. These issues collectively mean: critical alerts silently never fire (kube-state-metrics missing), log storage will eventually fill up (no Loki retention), Prometheus will OOM under realistic cardinality (memory limit too low), monitoring infrastructure failures go undetected (no self-monitoring alerts), and node-exporter scraping is fragile (hardcoded IPs instead of service discovery). Together, these undermine the reliability guarantees the monitoring stack is supposed to provide.

## Bug Analysis

### Current Behavior (Defect)

1.1 WHEN alert rules reference kube-state-metrics metrics (kube_pod_container_status_restarts_total, kube_pod_status_ready, kube_job_status_failed, kube_job_status_completion_time) THEN the system never evaluates these alerts because kube-state-metrics is not deployed, causing PodRestarts, PodNotReady, BackupJobFailed, and BackupJobMissing alerts to remain permanently inactive

1.2 WHEN Loki ingests log data over time THEN the system accumulates old log chunks indefinitely on the 10Gi PVC because no compactor retention is configured, eventually exhausting disk space

1.3 WHEN Prometheus scrapes 5 jobs with 30-day retention and time-series cardinality grows THEN the system OOM-kills Prometheus because the memory limit is set to 512Mi, which is insufficient for the configured retention and scrape volume

1.4 WHEN Prometheus itself becomes unavailable (OOM, crash, misconfiguration) or a scrape target goes down (up == 0) or GPU temperature exceeds safe thresholds THEN the system generates no alert because no self-monitoring rules exist for these conditions

1.5 WHEN cluster node IPs change (DHCP reassignment, node rebuild, network reconfiguration) THEN the system silently stops scraping node-exporter because the Prometheus scrape config uses hardcoded IP addresses (192.168.88.112, 192.168.88.115, 192.168.88.22) instead of the in-cluster headless service

### Expected Behavior (Correct)

2.1 WHEN alert rules reference kube-state-metrics metrics THEN the system SHALL have kube-state-metrics deployed in the monitoring namespace, providing the required metrics so that PodRestarts, PodNotReady, BackupJobFailed, and BackupJobMissing alerts evaluate and fire correctly

2.2 WHEN Loki stores log data THEN the system SHALL enforce a retention period (14 days) via the compactor with retention_enabled: true, automatically deleting log chunks older than the retention window to prevent PVC exhaustion

2.3 WHEN Prometheus runs with 30-day retention and 5 scrape jobs THEN the system SHALL have a memory limit of at least 1Gi to accommodate realistic time-series cardinality without OOM kills

2.4 WHEN Prometheus is down, a scrape target is unreachable (up == 0 for any job), or GPU temperature exceeds safe operating range THEN the system SHALL fire dedicated self-monitoring alerts (PrometheusDown via an external watchdog or absent() pattern, TargetDown for any scrape target, and GpuOverheating for xpumanager temperature metrics)

2.5 WHEN node-exporter instances run in the cluster THEN the system SHALL scrape them via the headless Service (node-exporter.monitoring.svc.cluster.local) using dns_sd_configs or kubernetes_sd_configs, eliminating dependency on specific node IP addresses

### Unchanged Behavior (Regression Prevention)

3.1 WHEN existing alert rules for backend errors, latency, database, disk, memory, SLOs, and business metrics are evaluated THEN the system SHALL CONTINUE TO fire those alerts with the same expressions, thresholds, and severity labels

3.2 WHEN Loki receives log entries within the 7-day reject_old_samples_max_age window THEN the system SHALL CONTINUE TO ingest and store them normally

3.3 WHEN Prometheus scrapes xpumanager, backend, backend-dev, and vllm targets THEN the system SHALL CONTINUE TO scrape them at the same intervals with the same labels and metrics paths

3.4 WHEN Alertmanager receives alerts THEN the system SHALL CONTINUE TO route them through the existing routing tree (kiro-autofix for critical production alerts, default for others) with the same inhibit rules

3.5 WHEN node-exporter exposes metrics on port 9100 THEN the system SHALL CONTINUE TO collect the same node-level metrics with node labels identifying each host (coreos, inference, networkstorage)

3.6 WHEN the networkstorage node (external to the cluster, 192.168.88.22) exposes node-exporter THEN the system SHALL CONTINUE TO scrape it, since it runs outside the cluster and cannot be discovered via Kubernetes service discovery
