"""
Preservation Property Test - Existing Monitoring Configuration Unchanged

This test captures snapshots of all existing monitoring configuration that MUST
remain unchanged after bug fixes are applied. It validates the current (unfixed)
manifests to establish the baseline, and should continue to PASS after fixes.

Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6
"""

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


class TestResult:
    def __init__(self):
        self.passed = []
        self.failed = []

    def assert_true(self, condition, category, message):
        if condition:
            self.passed.append(f"[PASS] {category}: {message}")
        else:
            self.failed.append(f"[FAIL] {category}: {message}")

    def assert_equal(self, actual, expected, category, message):
        if actual == expected:
            self.passed.append(f"[PASS] {category}: {message}")
        else:
            self.failed.append(
                f"[FAIL] {category}: {message} (expected={expected!r}, actual={actual!r})"
            )


# ─── Expected Alert Rules Snapshot ───────────────────────────────────────────

EXPECTED_ALERT_GROUPS = {
    "backend": [
        {
            "alert": "HighErrorRate",
            "for": "5m",
            "severity": "critical",
        },
        {
            "alert": "HighLatencyP95",
            "for": "5m",
            "severity": "warning",
        },
        {
            "alert": "PodRestarts",
            "for": "0m",
            "severity": "warning",
        },
        {
            "alert": "PodNotReady",
            "for": "5m",
            "severity": "critical",
        },
    ],
    "database": [
        {
            "alert": "DatabaseConnectionPoolExhausted",
            "for": "2m",
            "severity": "critical",
        },
        {
            "alert": "DatabaseDown",
            "for": "1m",
            "severity": "critical",
        },
    ],
    "infrastructure": [
        {
            "alert": "DiskUsageHigh",
            "for": "10m",
            "severity": "warning",
        },
        {
            "alert": "DiskUsageCritical",
            "for": "5m",
            "severity": "critical",
        },
        {
            "alert": "HighMemoryUsage",
            "for": "10m",
            "severity": "warning",
        },
    ],
    "backups": [
        {
            "alert": "BackupJobFailed",
            "for": "0m",
            "severity": "critical",
        },
        {
            "alert": "BackupJobMissing",
            "for": "0m",
            "severity": "warning",
        },
    ],
    "slo": [
        {
            "alert": "SLOAvailabilityBurnRateCritical",
            "for": "2m",
            "severity": "critical",
        },
        {
            "alert": "SLOAvailabilityBurnRateHigh",
            "for": "5m",
            "severity": "warning",
        },
        {
            "alert": "SLOLatencyBurnRateCritical",
            "for": "2m",
            "severity": "critical",
        },
    ],
    "business": [
        {
            "alert": "HighOverduePayments",
            "for": "10m",
            "severity": "warning",
        },
        {
            "alert": "AIInferenceDown",
            "for": "5m",
            "severity": "warning",
        },
        {
            "alert": "LoginBruteForce",
            "for": "2m",
            "severity": "warning",
        },
    ],
}

ALL_EXPECTED_ALERT_NAMES = [
    "HighErrorRate",
    "HighLatencyP95",
    "PodRestarts",
    "PodNotReady",
    "DatabaseConnectionPoolExhausted",
    "DatabaseDown",
    "DiskUsageHigh",
    "DiskUsageCritical",
    "HighMemoryUsage",
    "BackupJobFailed",
    "BackupJobMissing",
    "SLOAvailabilityBurnRateCritical",
    "SLOAvailabilityBurnRateHigh",
    "SLOLatencyBurnRateCritical",
    "HighOverduePayments",
    "AIInferenceDown",
    "LoginBruteForce",
]

# ─── Expected Scrape Configs Snapshot ────────────────────────────────────────

EXPECTED_SCRAPE_JOBS = {
    "xpumanager": {
        "metrics_path": "/metrics",
        "targets": ["xpumanager.default.svc.cluster.local:9273"],
        "labels": {"node": "inference"},
    },
    "backend": {
        "metrics_path": "/internal/metrics",
        "targets": ["backend.realestate.svc.cluster.local:8080"],
        "labels": {"environment": "production"},
    },
    "backend-dev": {
        "metrics_path": "/internal/metrics",
        "targets": ["backend.realestate-dev.svc.cluster.local:8080"],
        "labels": {"environment": "development"},
    },
    "vllm": {
        "metrics_path": "/metrics",
        "targets": ["vllm-inference.realestate.svc.cluster.local:8000"],
        "labels": {"node": "inference"},
    },
}

# ─── Expected Loki limits_config Snapshot ────────────────────────────────────

EXPECTED_LOKI_LIMITS = {
    "reject_old_samples": True,
    "reject_old_samples_max_age": "168h",
    "max_query_series": 500,
    "max_query_parallelism": 2,
    "allow_structured_metadata": True,
    "volume_enabled": True,
    "discover_log_levels": True,
}


# ─── Test Functions ──────────────────────────────────────────────────────────


def _find_rule_by_name(rules, alert_name):
    """Find a rule by alert name in a list of rules."""
    for rule in rules:
        if rule.get("alert") == alert_name:
            return rule
    return None


def _collect_all_alert_names(groups):
    """Collect all alert names across all groups."""
    names = []
    for group in groups:
        for rule in group.get("rules", []):
            alert_name = rule.get("alert")
            if alert_name:
                names.append(alert_name)
    return names


def _verify_group_rules(results, group, group_name, expected_rules):
    """Verify all expected rules exist in a group with correct for/severity."""
    rules = group.get("rules", [])
    for expected in expected_rules:
        rule_found = _find_rule_by_name(rules, expected["alert"])
        results.assert_true(
            rule_found is not None,
            "Alerts",
            f"Rule '{expected['alert']}' exists in group '{group_name}'",
        )
        if not rule_found:
            continue
        results.assert_equal(
            rule_found.get("for"),
            expected["for"],
            "Alerts",
            f"Rule '{expected['alert']}' has for={expected['for']}",
        )
        actual_severity = rule_found.get("labels", {}).get("severity")
        results.assert_equal(
            actual_severity,
            expected["severity"],
            "Alerts",
            f"Rule '{expected['alert']}' has severity={expected['severity']}",
        )


def test_alert_groups_preserved(results):
    """Requirement 3.1: All existing alert groups contain exactly the expected rules."""
    docs = load_yaml_documents(ALERTS_YML)
    alerts_config = find_configmap_data(docs, "prometheus-alerts", "alerts.yml")

    results.assert_true(
        alerts_config is not None,
        "Alerts",
        "prometheus-alerts ConfigMap with alerts.yml data exists",
    )
    if not alerts_config:
        return

    groups = alerts_config.get("groups", [])
    group_map = {g["name"]: g for g in groups}

    # Check all expected groups exist and verify their rules
    for group_name, expected_rules in EXPECTED_ALERT_GROUPS.items():
        group = group_map.get(group_name)
        results.assert_true(
            group is not None,
            "Alerts",
            f"Alert group '{group_name}' exists",
        )
        if group:
            _verify_group_rules(results, group, group_name, expected_rules)

    # Verify complete set of alert names across all groups
    found_alert_names = _collect_all_alert_names(groups)
    for expected_name in ALL_EXPECTED_ALERT_NAMES:
        results.assert_true(
            expected_name in found_alert_names,
            "Alerts",
            f"Alert '{expected_name}' exists in configuration",
        )


def test_scrape_configs_preserved(results):
    """Requirements 3.3: Scrape configs for xpumanager, backend, backend-dev, vllm unchanged."""
    docs = load_yaml_documents(MONITORING_YML)
    prom_config = find_configmap_data(docs, "prometheus-config", "prometheus.yml")

    results.assert_true(
        prom_config is not None,
        "Scrape",
        "prometheus-config ConfigMap with prometheus.yml data exists",
    )
    if not prom_config:
        return

    scrape_configs = prom_config.get("scrape_configs", [])
    job_map = {j["job_name"]: j for j in scrape_configs}

    for job_name, expected in EXPECTED_SCRAPE_JOBS.items():
        job = job_map.get(job_name)
        results.assert_true(
            job is not None,
            "Scrape",
            f"Scrape job '{job_name}' exists",
        )
        if not job:
            continue

        # Verify metrics_path
        actual_path = job.get("metrics_path", "/metrics")
        results.assert_equal(
            actual_path,
            expected["metrics_path"],
            "Scrape",
            f"Job '{job_name}' has metrics_path={expected['metrics_path']}",
        )

        # Verify targets and labels in static_configs
        static_configs = job.get("static_configs", [])
        results.assert_true(
            len(static_configs) > 0,
            "Scrape",
            f"Job '{job_name}' has static_configs",
        )
        if not static_configs:
            continue

        # Find the matching static config entry
        found_targets = False
        for sc in static_configs:
            if sc.get("targets") == expected["targets"]:
                found_targets = True
                actual_labels = sc.get("labels", {})
                results.assert_equal(
                    actual_labels,
                    expected["labels"],
                    "Scrape",
                    f"Job '{job_name}' has labels={expected['labels']}",
                )
                break

        results.assert_true(
            found_targets,
            "Scrape",
            f"Job '{job_name}' has targets={expected['targets']}",
        )


def test_networkstorage_preserved(results):
    """Requirements 3.5, 3.6: networkstorage (192.168.88.22:9100) remains as static_configs."""
    docs = load_yaml_documents(MONITORING_YML)
    prom_config = find_configmap_data(docs, "prometheus-config", "prometheus.yml")

    results.assert_true(
        prom_config is not None,
        "NetworkStorage",
        "prometheus-config ConfigMap exists",
    )
    if not prom_config:
        return

    scrape_configs = prom_config.get("scrape_configs", [])

    # Find the node-exporter job or any job with networkstorage target
    networkstorage_found = False
    networkstorage_label_correct = False

    for job in scrape_configs:
        static_configs = job.get("static_configs", [])
        for sc in static_configs:
            targets = sc.get("targets", [])
            if "192.168.88.22:9100" in targets:
                networkstorage_found = True
                labels = sc.get("labels", {})
                if labels.get("node") == "networkstorage":
                    networkstorage_label_correct = True
                break
        if networkstorage_found:
            break

    results.assert_true(
        networkstorage_found,
        "NetworkStorage",
        "networkstorage target (192.168.88.22:9100) exists in static_configs",
    )
    results.assert_true(
        networkstorage_label_correct,
        "NetworkStorage",
        'networkstorage has label node="networkstorage"',
    )


def test_loki_limits_preserved(results):
    """Requirement 3.2: Loki limits_config values remain unchanged."""
    docs = load_yaml_documents(LOGGING_YML)
    loki_config = find_configmap_data(docs, "loki-config", "loki.yaml")

    results.assert_true(
        loki_config is not None,
        "Loki",
        "loki-config ConfigMap with loki.yaml data exists",
    )
    if not loki_config:
        return

    limits = loki_config.get("limits_config", {})

    for key, expected_value in EXPECTED_LOKI_LIMITS.items():
        actual_value = limits.get(key)
        results.assert_equal(
            actual_value,
            expected_value,
            "Loki",
            f"limits_config.{key} = {expected_value!r}",
        )


def test_alertmanager_routing_preserved(results):
    """Requirement 3.4: Alertmanager routing reference in Prometheus config is preserved."""
    docs = load_yaml_documents(MONITORING_YML)
    prom_config = find_configmap_data(docs, "prometheus-config", "prometheus.yml")

    results.assert_true(
        prom_config is not None,
        "Alertmanager",
        "prometheus-config exists",
    )
    if not prom_config:
        return

    alerting = prom_config.get("alerting", {})
    alertmanagers = alerting.get("alertmanagers", [])

    results.assert_true(
        len(alertmanagers) > 0,
        "Alertmanager",
        "alertmanagers configuration exists",
    )

    # Verify the alertmanager target
    am_target_found = False
    for am in alertmanagers:
        static_configs = am.get("static_configs", [])
        for sc in static_configs:
            targets = sc.get("targets", [])
            if "alertmanager.monitoring.svc.cluster.local:9093" in targets:
                am_target_found = True
                break
        if am_target_found:
            break

    results.assert_true(
        am_target_found,
        "Alertmanager",
        "alertmanager.monitoring.svc.cluster.local:9093 is configured as target",
    )


def test_global_scrape_interval_preserved(results):
    """Verify global scrape and evaluation intervals are unchanged."""
    docs = load_yaml_documents(MONITORING_YML)
    prom_config = find_configmap_data(docs, "prometheus-config", "prometheus.yml")

    if not prom_config:
        return

    global_config = prom_config.get("global", {})
    results.assert_equal(
        global_config.get("scrape_interval"),
        "15s",
        "Global",
        "scrape_interval = 15s",
    )
    results.assert_equal(
        global_config.get("evaluation_interval"),
        "15s",
        "Global",
        "evaluation_interval = 15s",
    )


def main():
    print("=" * 70)
    print("Preservation Property Test - Existing Configuration Unchanged")
    print("=" * 70)
    print()
    print("This test captures the baseline configuration that MUST be preserved.")
    print("It should PASS on both unfixed and fixed manifests.")
    print()

    results = TestResult()

    test_alert_groups_preserved(results)
    test_scrape_configs_preserved(results)
    test_networkstorage_preserved(results)
    test_loki_limits_preserved(results)
    test_alertmanager_routing_preserved(results)
    test_global_scrape_interval_preserved(results)

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
        print("PRESERVATION VIOLATIONS:")
        for msg in results.failed:
            print(f"  - {msg}")
        print()
        print("TEST OUTCOME: FAIL (existing configuration was modified!)")
        sys.exit(1)
    else:
        print("TEST OUTCOME: PASS (all existing configuration preserved)")
        sys.exit(0)


if __name__ == "__main__":
    main()
