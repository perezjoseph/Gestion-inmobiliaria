# Dashboard Design

Design principles and copy-paste JSON templates for building Grafana
dashboards. Read this when designing dashboards or hand-authoring panel JSON.

## Design Principles

### Hierarchy of Information
```
┌─────────────────────────────────────┐
│  Critical Metrics (Big Numbers)     │
├─────────────────────────────────────┤
│  Key Trends (Time Series)           │
├─────────────────────────────────────┤
│  Detailed Metrics (Tables/Heatmaps) │
└─────────────────────────────────────┘
```

### RED Method (Services)
- **Rate** — requests per second.
- **Errors** — error rate.
- **Duration** — latency / response time.

### USE Method (Resources)
- **Utilization** — % time the resource is busy.
- **Saturation** — queue length / wait time.
- **Errors** — error count.

## Dashboard Skeleton

```json
{
  "dashboard": {
    "title": "API Monitoring",
    "tags": ["api", "production"],
    "timezone": "browser",
    "refresh": "30s",
    "panels": [
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(http_requests_total[5m])) by (service)",
            "legendFormat": "{{service}}"
          }
        ],
        "gridPos": {"x": 0, "y": 0, "w": 12, "h": 8}
      },
      {
        "title": "Error Rate %",
        "type": "graph",
        "targets": [
          {
            "expr": "(sum(rate(http_requests_total{status=~\"5..\"}[5m])) / sum(rate(http_requests_total[5m]))) * 100",
            "legendFormat": "Error Rate"
          }
        ],
        "gridPos": {"x": 12, "y": 0, "w": 12, "h": 8}
      },
      {
        "title": "P95 Latency",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket[5m])) by (le, service))",
            "legendFormat": "{{service}}"
          }
        ],
        "gridPos": {"x": 0, "y": 8, "w": 24, "h": 8}
      }
    ]
  }
}
```

## Panel Templates

### Stat Panel (single value)
```json
{
  "type": "stat",
  "title": "Total Requests",
  "targets": [{"expr": "sum(http_requests_total)"}],
  "options": {
    "reduceOptions": {"values": false, "calcs": ["lastNotNull"]},
    "orientation": "auto",
    "textMode": "auto",
    "colorMode": "value"
  },
  "fieldConfig": {
    "defaults": {
      "thresholds": {
        "mode": "absolute",
        "steps": [
          {"value": 0, "color": "green"},
          {"value": 80, "color": "yellow"},
          {"value": 90, "color": "red"}
        ]
      }
    }
  }
}
```

### Time Series Graph
```json
{
  "type": "graph",
  "title": "CPU Usage",
  "targets": [{
    "expr": "100 - (avg by (instance) (rate(node_cpu_seconds_total{mode=\"idle\"}[5m])) * 100)"
  }],
  "yaxes": [
    {"format": "percent", "max": 100, "min": 0},
    {"format": "short"}
  ]
}
```

### Table Panel
```json
{
  "type": "table",
  "title": "Service Status",
  "targets": [{"expr": "up", "format": "table", "instant": true}],
  "transformations": [
    {
      "id": "organize",
      "options": {
        "excludeByName": {"Time": true},
        "indexByName": {},
        "renameByName": {"instance": "Instance", "job": "Service", "Value": "Status"}
      }
    }
  ]
}
```

### Heatmap
```json
{
  "type": "heatmap",
  "title": "Latency Heatmap",
  "targets": [{
    "expr": "sum(rate(http_request_duration_seconds_bucket[5m])) by (le)",
    "format": "heatmap"
  }],
  "dataFormat": "tsbuckets",
  "yAxis": {"format": "s"}
}
```

## Template Variables

```json
{
  "templating": {
    "list": [
      {
        "name": "namespace",
        "type": "query",
        "datasource": "Prometheus",
        "query": "label_values(kube_pod_info, namespace)",
        "refresh": 1,
        "multi": false
      },
      {
        "name": "service",
        "type": "query",
        "datasource": "Prometheus",
        "query": "label_values(kube_service_info{namespace=\"$namespace\"}, service)",
        "refresh": 1,
        "multi": true
      }
    ]
  }
}
```

Use in queries:
```
sum(rate(http_requests_total{namespace="$namespace", service=~"$service"}[5m]))
```

For multi-value variables inside a regex matcher, reference `${service:regex}`
so all selected values are escaped and OR-joined correctly.

## Dashboard Alerts

```json
{
  "alert": {
    "name": "High Error Rate",
    "conditions": [
      {
        "evaluator": {"params": [5], "type": "gt"},
        "operator": {"type": "and"},
        "query": {"params": ["A", "5m", "now"]},
        "reducer": {"type": "avg"},
        "type": "query"
      }
    ],
    "executionErrorState": "alerting",
    "for": "5m",
    "frequency": "1m",
    "message": "Error rate is above 5%",
    "noDataState": "no_data",
    "notifications": [{"uid": "slack-channel"}]
  }
}
```

Prefer Grafana-managed alert rules via the `alerting_manage_rules` MCP tool for
new alerts; embedded panel alerts are legacy.

## Provisioning

### File provider (`dashboards.yml`)
```yaml
apiVersion: 1
providers:
  - name: 'default'
    orgId: 1
    folder: 'General'
    type: file
    disableDeletion: false
    updateIntervalSeconds: 10
    allowUiUpdates: true
    options:
      path: /etc/grafana/dashboards
```

### Terraform
```hcl
resource "grafana_folder" "monitoring" {
  title = "Production Monitoring"
}

resource "grafana_dashboard" "api_monitoring" {
  config_json = file("${path.module}/dashboards/api-monitoring.json")
  folder      = grafana_folder.monitoring.id
}
```

### Ansible
```yaml
- name: Deploy Grafana dashboards
  copy:
    src: "{{ item }}"
    dest: /etc/grafana/dashboards/
  with_fileglob:
    - "dashboards/*.json"
  notify: restart grafana
```

## Common Dashboard Patterns

**Infrastructure** — CPU/memory per node, disk I/O, network traffic, pod count
by namespace, node status.

**Database** — queries/sec, connection pool usage, query latency (P50/P95/P99),
active connections, database size, replication lag, slow queries.

**Application** — request rate, error rate, response-time percentiles, active
sessions, cache hit rate, queue length.

## Checklist

1. Start from a community template when one fits.
2. Use consistent panel and variable naming.
3. Group related metrics into rows.
4. Set a sensible default time range (e.g. last 6 hours).
5. Configure units and meaningful color thresholds.
6. Add panel descriptions for context.
7. Validate every panel returns data across different time ranges.
