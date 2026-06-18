# Monitoring — Metrics, Alerts, and Dashboards

Self-contained guide for PostgreSQL observability and alerting.

---

## Key Metrics & Alert Thresholds

| Metric | WARN | CRITICAL | Query Source |
|--------|------|----------|--------------|
| Connection usage | > 70% of max | > 85% of max | `pg_stat_activity` |
| Cache hit ratio | < 0.995 | < 0.99 | `pg_stat_database` |
| Replication lag | > 10s | > 60s | `pg_stat_replication` |
| Dead tuple ratio | > 5% per table | > 20% per table | `pg_stat_user_tables` |
| Long-running txns | > 5 min | > 30 min | `pg_stat_activity` |
| Idle in transaction | > 5 min | > 10 min (auto-kill) | `pg_stat_activity` |
| Lock wait duration | > 30s | > 2 min | `pg_locks` |
| Temp file usage | > 500MB/query | > 1GB/hour | `pg_stat_database` |
| WAL generation rate | 2× baseline | 5× baseline | `pg_current_wal_lsn()` |

---

## Dashboard Queries

### Connection state breakdown

```sql
SELECT state, count(*)
FROM pg_stat_activity
GROUP BY state;
```

### Connection saturation percentage

```sql
SELECT round(count(*)::numeric /
  current_setting('max_connections')::int * 100, 1) AS usage_pct
FROM pg_stat_activity;
```

### Transaction rate (commits + rollbacks per second)

```sql
SELECT datname,
  xact_commit + xact_rollback AS total_txns,
  xact_commit, xact_rollback
FROM pg_stat_database
WHERE datname = current_database();
```

### Cache hit ratio (database-wide)

```sql
SELECT round(sum(blks_hit)::numeric /
  nullif(sum(blks_hit) + sum(blks_read), 0), 4) AS cache_hit_ratio
FROM pg_stat_database;
```

### Tuple operations per table

```sql
SELECT relname,
  n_tup_ins AS inserts,
  n_tup_upd AS updates,
  n_tup_del AS deletes,
  n_tup_hot_upd AS hot_updates
FROM pg_stat_user_tables
ORDER BY n_tup_upd + n_tup_del DESC LIMIT 10;
```

### Dead tuples (vacuum candidates)

```sql
SELECT relname, n_live_tup, n_dead_tup,
  round(n_dead_tup::numeric / nullif(n_live_tup + n_dead_tup, 0), 3) AS dead_ratio,
  last_autovacuum
FROM pg_stat_user_tables
WHERE n_dead_tup > 100
ORDER BY dead_ratio DESC;
```

### WAL generation (for rate-of-change monitoring)

```sql
SELECT pg_wal_lsn_diff(pg_current_wal_lsn(), '0/0') AS wal_bytes_total;
```

### Checkpoint activity

```sql
SELECT checkpoints_timed, checkpoints_req,
  checkpoint_write_time, checkpoint_sync_time,
  buffers_checkpoint, buffers_clean, buffers_backend
FROM pg_stat_bgwriter;
```

### Replication lag

```sql
SELECT client_addr, state,
  pg_size_pretty(pg_wal_lsn_diff(sent_lsn, replay_lsn)) AS replay_lag
FROM pg_stat_replication;
```

### Table sizes (growth tracking)

```sql
SELECT relname,
  pg_size_pretty(pg_total_relation_size(relid)) AS total,
  pg_size_pretty(pg_relation_size(relid)) AS table_only,
  pg_size_pretty(pg_indexes_size(relid)) AS indexes
FROM pg_stat_user_tables
ORDER BY pg_total_relation_size(relid) DESC LIMIT 10;
```

---

## Prometheus / postgres_exporter

Standard exporter metrics map to the queries above:

| Metric Name | Source |
|-------------|--------|
| `pg_stat_activity_count` | Connection count by state |
| `pg_stat_database_blks_hit` | Cache hits |
| `pg_stat_database_blks_read` | Cache misses |
| `pg_stat_database_xact_commit` | Transaction commits |
| `pg_stat_user_tables_n_dead_tup` | Dead tuples per table |
| `pg_replication_lag` | Replica delay in seconds |

---

## postgresql.conf for Observability

```ini
shared_preload_libraries = 'pg_stat_statements,pg_cron'
track_io_timing = on
track_activity_query_size = 4096
log_min_duration_statement = 500
log_lock_waits = on
log_checkpoints = on
log_temp_files = 0
log_autovacuum_min_duration = 250
```

---

## Scheduled Maintenance with pg_cron

```sql
CREATE EXTENSION pg_cron;

SELECT cron.schedule('nightly-vacuum', '0 3 * * *', 'VACUUM ANALYZE');

SELECT cron.schedule('refresh-mv', '0 * * * *',
  'REFRESH MATERIALIZED VIEW CONCURRENTLY mv_property_financials');

SELECT cron.schedule('purge-audit', '0 4 * * 0',
  $$DELETE FROM audit_log WHERE created_at < now() - interval '90 days'$$);

SELECT * FROM cron.job;
SELECT cron.unschedule('nightly-vacuum');
```

---

## Alerting Recommendations

### Connection exhaustion alert

Trigger when usage > 85%:
```sql
SELECT count(*)::numeric / current_setting('max_connections')::int > 0.85 AS alert
FROM pg_stat_activity;
```

### Idle in transaction auto-kill

```ini
idle_in_transaction_session_timeout = '10min'
```

### Statement timeout (prevent runaway queries)

```sql
SET statement_timeout = '30s';
```

### Lock timeout (prevent DDL pile-ups)

```sql
SET lock_timeout = '5s';
```

---

## Health Check Query

Single query for application health endpoint:

```sql
SELECT
  (SELECT count(*) FROM pg_stat_activity) AS connections,
  (SELECT round(sum(blks_hit)::numeric /
    nullif(sum(blks_hit) + sum(blks_read), 0), 4)
   FROM pg_stat_database) AS cache_ratio,
  (SELECT max(now() - query_start) FROM pg_stat_activity
   WHERE state = 'active' AND pid <> pg_backend_pid()) AS longest_query,
  (SELECT count(*) FROM pg_stat_activity
   WHERE state = 'idle in transaction'
   AND query_start < now() - interval '5 minutes') AS stale_txns;
```

---

## Bloat Monitoring

Track weekly to catch creep before it impacts performance:

```sql
SELECT relname,
  pg_size_pretty(pg_total_relation_size(relid)) AS size,
  n_dead_tup,
  last_autovacuum,
  autovacuum_count
FROM pg_stat_user_tables
WHERE pg_total_relation_size(relid) > 100 * 1024 * 1024
ORDER BY pg_total_relation_size(relid) DESC;
```

Action triggers:
- Dead ratio > 20% → run `pg_repack`
- Autovacuum not keeping up → tune per-table autovacuum settings
- Table size growing without row count increase → bloat accumulating
