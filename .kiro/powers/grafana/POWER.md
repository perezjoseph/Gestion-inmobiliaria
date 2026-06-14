---
name: "grafana"
displayName: "Grafana"
description: "Query Grafana metrics, logs, and traces, and create production-ready dashboards through the Grafana MCP server. Covers dashboard design patterns (RED/USE), Prometheus and Loki queries, alerting, and incident workflows."
keywords: ["grafana", "dashboard", "prometheus", "loki", "observability", "metrics", "alerting", "monitoring"]
author: "realestate"
---

# Grafana

Connect to a Grafana instance through the Grafana MCP server to query metrics
and logs, search and manage dashboards, inspect alert rules, work with
incidents and Sift, and build production-ready dashboards.

This power combines two things:

1. **Live access** to a Grafana instance via the `grafana` MCP server (tools
   for dashboards, Prometheus, Loki, alerting, incidents, OnCall, Pyroscope,
   and deeplinks).
2. **Design knowledge** for building effective dashboards (RED/USE methods,
   panel templates, variables, alerts, provisioning).

## Configuration

The `grafana` MCP server needs two environment variables, set in `mcp.json`:

| Variable | Purpose | Example |
|---|---|---|
| `GRAFANA_URL` | Base URL of the Grafana instance | `http://localhost:3000` or `https://myinstance.grafana.net` |
| `GRAFANA_SERVICE_ACCOUNT_TOKEN` | Service account token with the needed scopes | `glsa_xxx...` |

Create a service account token in Grafana under **Administration → Service
accounts**. Grant only the scopes the work needs (read-only for querying,
editor for dashboard writes). Never commit the real token — keep the
placeholder in version control and set the value locally.

The default config runs the `mcp-grafana` binary over STDIO. Install it from
the [grafana/mcp-grafana](https://github.com/grafana/mcp-grafana) releases, or
run the official Docker image instead (see Troubleshooting).

## Available MCP Servers

### grafana

Key tools, grouped by task. Tool names are exact — use them as written.

**Dashboards**
- `search_dashboards` — find dashboards by query string.
- `get_dashboard_summary` — compact overview (title, panel count, types). Prefer this over the full dashboard to save context.
- `get_dashboard_property` — extract specific parts via JSONPath (e.g. `$.panels[*].title`).
- `get_dashboard_panel_queries` — read panel queries (all datasource types).
- `get_dashboard_by_uid` — full dashboard JSON (large; use only when needed).
- `update_dashboard` — create or patch a dashboard (full JSON or targeted patch operations).

**Prometheus (discovery → query)**
- `list_prometheus_metric_names` — discover metrics by regex. Call this first.
- `list_prometheus_label_names` / `list_prometheus_label_values` — find labels to filter on.
- `query_prometheus` — run instant or range PromQL.
- `query_prometheus_histogram` — histogram percentiles (find `*_bucket` metrics first).

**Loki (logs)**
- `list_loki_label_names` / `list_loki_label_values` — verify labels exist.
- `query_loki_stats` — cheap check whether a stream has data before an expensive query.
- `query_loki_logs` — run LogQL; use `count_over_time()` for exact line counts.
- `query_loki_patterns` — detected log patterns for a stream selector.

**Datasources**
- `list_datasources` — discover datasources and their UIDs.
- `get_datasource` — full datasource details by UID or name.

**Alerting**
- `alerting_manage_rules` — list/get/create/update/delete alert rules.
- `alerting_manage_routing` — notification policies, contact points, time intervals.

**Incidents / Sift / OnCall**
- `list_incidents`, `get_incident`, `create_incident`, `add_activity_to_incident`.
- `list_sift_investigations`, `get_sift_investigation`, `find_slow_requests`, `find_error_pattern_logs`.
- `list_oncall_schedules`, `get_current_oncall_users`, `list_alert_groups`.

**Other**
- `generate_deeplink` — shareable links to dashboards, panels, or Explore.
- `get_panel_image` — render a panel/dashboard as PNG (needs image renderer).

## Tool Usage Examples

**Find a dashboard and read its panel queries**
```
search_dashboards(query="backend health")
get_dashboard_summary(uid="abc123")
get_dashboard_panel_queries(uid="abc123")
```

**Query a metric (discovery first)**
```
list_prometheus_metric_names(datasourceUid="prometheus", regex="http_requests.*")
query_prometheus(
  datasourceUid="prometheus",
  expr="sum(rate(http_requests_total[5m])) by (service)",
  queryType="range", startTime="now-1h", endTime="now", stepSeconds=60
)
```

**Check logs cheaply, then query**
```
query_loki_stats(datasourceUid="loki", logql="{app=\"backend\"}")
query_loki_logs(datasourceUid="loki", logql="{app=\"backend\"} |= \"error\"", limit=50)
```

## Workflow: Build a Dashboard

1. **Discover** datasources (`list_datasources`) and metrics (`list_prometheus_metric_names`).
2. **Validate** the PromQL/LogQL with `query_prometheus` / `query_loki_logs` before saving.
3. **Design** panels using the RED/USE methods and templates in `dashboard-design.md`.
4. **Create or patch** with `update_dashboard` (use patch operations for small edits to avoid resending large JSON).
5. **Verify** every panel returns data by re-running its query after saving.

## Best Practices

- Run discovery tools before querying — never guess metric or label names.
- Prefer `get_dashboard_summary` / `get_dashboard_property` over full dashboard JSON to preserve context.
- For small dashboard edits, use `update_dashboard` patch operations instead of full-JSON replacement.
- Validate queries return data before considering a dashboard done.
- Scope service account tokens to the minimum needed; read-only for query work.
- For multi-value template variables used inside regex matchers, save `${var:regex}` rather than `$var`.

## Available Steering Files

- **dashboard-design.md** — Dashboard design principles (RED/USE), panel JSON templates (stat, time series, table, heatmap), template variables, dashboard alerts, and provisioning (file, Terraform, Ansible). Read this when designing or hand-authoring dashboard JSON.

## Troubleshooting

**`mcp-grafana: command not found`**
Install the binary from the grafana/mcp-grafana releases and ensure it is on
PATH, or switch `mcp.json` to the Docker image:
```json
{
  "mcpServers": {
    "grafana": {
      "command": "docker",
      "args": ["run", "--rm", "-i",
        "-e", "GRAFANA_URL", "-e", "GRAFANA_SERVICE_ACCOUNT_TOKEN",
        "mcp/grafana", "-t", "stdio"],
      "env": {
        "GRAFANA_URL": "http://localhost:3000",
        "GRAFANA_SERVICE_ACCOUNT_TOKEN": "REPLACE_WITH_SERVICE_ACCOUNT_TOKEN"
      }
    }
  }
}
```

**401 / 403 from Grafana**
The token is missing or under-scoped. Recreate the service account token with
the needed role and update `GRAFANA_SERVICE_ACCOUNT_TOKEN`.

**Tools return no data**
Confirm the datasource UID with `list_datasources`, and verify metric/label
names exist with the discovery tools before querying.

**`get_panel_image` fails**
The Grafana Image Renderer plugin must be installed on the instance.
