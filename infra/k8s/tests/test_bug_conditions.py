"""
Bug Condition Exploration Test - Monitoring Stack Infrastructure Defects

This test encodes the EXPECTED behavior for the monitoring stack.
On unfixed manifests, it MUST FAIL — failure proves the five defects exist.
After fixes are applied, this same test should PASS.

Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5
"""

import glob
import os
import sys

import yaml

# Resolve paths relative to infra/k8s/ directory
K8S_DIR = os.path.normpath(os.path.join(os.path.dirname(__file__), ".."))
MONITORING_YML = os.path.join(K8S_DIR, "monitoring.yml")
LOGGING_YML = os.path.join(K8S_DIR, "logging.yml")
ALERTS_YML = os.path.join(K8S_DIR, "alerts.yml")


def load_yaml_documents(filepath):
    """Load all YAML documents from a multi-document file."""
    with open(filepath, "r", encoding="utf-8") as f:
        return list(yaml.safe_load_all(f))


def find_configmap_data(documents, configmap_name, data_key):
    """Find a ConfigMap by name and parse the nested YAML from a data key."""
    for doc in documents:
        if (
            doc
            and doc.get("kind") == "ConfigMap"
            and doc.get("metadata", {}).get("name") == configmap_name
        ):
            raw = doc.get("data", {}).get(data_key, "")
            return yaml.safe_load(raw) if raw else None
    return None


def find_deployment(documents, deployment_name):
    """Find a Deployment by name."""
    for doc in documents:
        if (
            doc
            and doc.get("kind") == "Deployment"
            and doc.get("metadata", {}).get("name") == deployment_name
        ):
            return doc
    return None


def parse_memory_bytes(memory_str):
    """Convert Kubernetes memory string (e.g., '512Mi', '1Gi') to bytes."""
    memory_str = str(memory_str)
    if memory_str.endswith("Gi"):
        return int(float(memory_str[:-2]) * 1024 * 1024 * 1024)
    elif memory_str.endswith("Mi"):
        return int(float(memory_str[:-2]) * 1024 * 1024)
    elif memory_str.endswith("Ki"):
        return int(float(memory_str[:-2]) * 1024)
    else:
        return int(memory_str)


class TestResult:
    def __init__(self):
        self.passed = []
        self.failed = []

    def assert_true(self, condition, defect_id, message):
        if condition:
            self.passed.append(f"[PASS] Defect {defect_id}: {message}")
        else:
            self.failed.append(f"[FAIL] Defect {defect_id}: {message}")


def test_defect_1_ksm_deployed(results):
    """Defect 1: Assert a kube-state-metrics Deployment exists in infra/k8s/."""
    # Search all YAML files in infra/k8s/ for a kube-state-metrics Deployment
    ksm_found = False
    yaml_files = glob.glob(os.path.join(K8S_DIR, "*.yml")) + glob.glob(
        os.path.join(K8S_DIR, "*.yaml")
    )

    for filepath in yaml_files:
        try:
            docs = load_yaml_documents(filepath)
            for doc in docs:
                if (
                    doc
                    and doc.get("kind") == "Deployment"
                    and "kube-state-metrics"
                    in doc.get("metadata", {}).get("name", "")
                ):
                    ksm_found = True
                    break
        except Exception:
            continue
        if ksm_found:
            break

    results.assert_true(
        ksm_found,
        "1",
        "kube-state-metrics Deployment exists in infra/k8s/",
    )


def test_defect_2_loki_retention(results):
    """Defect 2: Assert Loki config contains retention_enabled: true in compactor section."""
    docs = load_yaml_documents(LOGGING_YML)
    loki_config = find_configmap_data(docs, "loki-config", "loki.yaml")

    has_retention = False
    if loki_config:
        compactor = loki_config.get("compactor", {})
        has_retention = compactor.get("retention_enabled") is True

    results.assert_true(
        has_retention,
        "2",
        "Loki compactor has retention_enabled: true",
    )


def test_defect_3_prometheus_memory(results):
    """Defect 3: Assert Prometheus memory limit >= 1Gi (1073741824 bytes)."""
    docs = load_yaml_documents(MONITORING_YML)
    prom_deploy = find_deployment(docs, "prometheus")

    memory_limit_bytes = 0
    if prom_deploy:
        containers = (
            prom_deploy.get("spec", {})
            .get("template", {})
            .get("spec", {})
            .get("containers", [])
        )
        for container in containers:
            if container.get("name") == "prometheus":
                limit = (
                    container.get("resources", {})
                    .get("limits", {})
                    .get("memory", "0")
                )
                memory_limit_bytes = parse_memory_bytes(limit)
                break

    one_gi = 1073741824  # 1Gi in bytes
    results.assert_true(
        memory_limit_bytes >= one_gi,
        "3",
        f"Prometheus memory limit >= 1Gi (actual: {memory_limit_bytes} bytes, "
        f"need: {one_gi} bytes)",
    )


def test_defect_4_self_monitoring_alerts(results):
    """Defect 4: Assert alerts.yml contains PrometheusDown, TargetDown, and GpuOverheating rules."""
    docs = load_yaml_documents(ALERTS_YML)
    alerts_config = find_configmap_data(docs, "prometheus-alerts", "alerts.yml")

    required_alerts = {"PrometheusDown", "TargetDown", "GpuOverheating"}
    found_alerts = set()

    if alerts_config:
        for group in alerts_config.get("groups", []):
            for rule in group.get("rules", []):
                alert_name = rule.get("alert", "")
                if alert_name in required_alerts:
                    found_alerts.add(alert_name)

    missing = required_alerts - found_alerts
    results.assert_true(
        len(missing) == 0,
        "4",
        f"Self-monitoring alerts exist (missing: {missing if missing else 'none'})",
    )


def test_defect_5_dns_sd_configs(results):
    """Defect 5: Assert node-exporter job uses dns_sd_configs for in-cluster nodes."""
    docs = load_yaml_documents(MONITORING_YML)
    prom_config = find_configmap_data(docs, "prometheus-config", "prometheus.yml")

    uses_dns_sd = False
    if prom_config:
        scrape_configs = prom_config.get("scrape_configs", [])
        for job in scrape_configs:
            if job.get("job_name") == "node-exporter":
                # Check that dns_sd_configs is used (not hardcoded IPs for in-cluster)
                if "dns_sd_configs" in job:
                    uses_dns_sd = True
                break

    results.assert_true(
        uses_dns_sd,
        "5",
        "node-exporter job uses dns_sd_configs for in-cluster nodes "
        "(not hardcoded IPs 192.168.88.112, 192.168.88.115)",
    )


def main():
    print("=" * 70)
    print("Bug Condition Exploration Test - Monitoring Stack Defects")
    print("=" * 70)
    print()
    print("This test asserts the EXPECTED (fixed) behavior.")
    print("On UNFIXED manifests, failures prove the bugs exist.")
    print()

    results = TestResult()

    test_defect_1_ksm_deployed(results)
    test_defect_2_loki_retention(results)
    test_defect_3_prometheus_memory(results)
    test_defect_4_self_monitoring_alerts(results)
    test_defect_5_dns_sd_configs(results)

    print("Results:")
    print("-" * 70)
    for msg in results.passed:
        print(f"  {msg}")
    for msg in results.failed:
        print(f"  {msg}")
    print("-" * 70)
    print(f"\nTotal: {len(results.passed)} passed, {len(results.failed)} failed")
    print()

    if results.failed:
        print("COUNTEREXAMPLES (bugs confirmed):")
        for msg in results.failed:
            print(f"  - {msg}")
        print()
        print("TEST OUTCOME: FAIL (expected on unfixed code — bugs exist)")
        sys.exit(1)
    else:
        print("TEST OUTCOME: PASS (all defects are fixed)")
        sys.exit(0)


if __name__ == "__main__":
    main()
