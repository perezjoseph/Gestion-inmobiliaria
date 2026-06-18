# Troubleshooting — Diagnosis Workflows

Self-contained guide for diagnosing PostgreSQL performance and stability issues.

---

## Slow Query Diagnosis

### Step 1: Identify the culprit

```sql
SELECT queryid, calls,
  total_exec_time::numeric(12,2) AS total_ms,
  mean_exec_time::numeric(12,2) AS mean_ms,
  rows, left(query, 120)
FROM pg_stat_statements
ORDER BY total_exec_time DESC LIMIT 10;
```

### Step 2: Get the execution plan

```sql
EXPLAIN (ANALYZE, BUFFERS, TIMING, FORMAT TEXT)
<your_query_here>;
```

### Step 3: Interpret the plan

| Symptom | Root Cause | Fix |
|---------|-----------|-----|
| Seq Scan on large table | Missing or unused index | Add targeted index |
| actual rows >> planned rows | Stale statistics | `ANALYZE tablename;` |
| Sort Method: external merge Disk | work_mem too small | `SET work_mem = '64MB';` |
| Hash Batches > 1 | Hash spilled to disk | Increase work_mem |
| Nested Loop with large inner | Wrong join strategy | Add index on join column |
| Rows removed by Filter: large N | Over-fetching then discarding | Push filter into index |
| Buffers: shared read >> hit | Cold cache / undersized shared_buffers | Increase shared_buffers or reduce working set |

### Step 4: Fix and verify

```sql
ANALYZE properties;

CREATE INDEX CONCURRENTLY idx_properties_city_status
  ON properties(city, status);

EXPLAIN (ANALYZE, BUFFERS, TIMING)
SELECT * FROM properties WHERE city = 'Santo Domingo' AND status = 'activo';
```

---

## Lock Contention Diagnosis

### Find blocked queries

```sql
SELECT
  blocked.pid AS blocked_pid,
  blocked.usename,
  now() - blocked.query_start AS wait_duration,
  left(blocked.query, 80) AS blocked_query,
  blocking.pid AS blocking_pid,
  left(blocking.query, 80) AS blocking_query
FROM pg_stat_activity blocked
JOIN pg_locks bl ON bl.pid = blocked.pid AND NOT bl.granted
JOIN pg_locks kl ON kl.locktype = bl.locktype
  AND kl.relation IS NOT DISTINCT FROM bl.relation
  AND kl.page IS NOT DISTINCT FROM bl.page
  AND kl.tuple IS NOT DISTINCT FROM bl.tuple
  AND kl.transactionid IS NOT DISTINCT FROM bl.transactionid
  AND kl.pid != bl.pid AND kl.granted
JOIN pg_stat_activity blocking ON blocking.pid = kl.pid;
```

### Prevent lock pile-ups

```sql
SET lock_timeout = '5s';
SET statement_timeout = '30s';
SET idle_in_transaction_session_timeout = '10min';
```

### Emergency: terminate blocking session

```sql
SELECT pg_terminate_backend(12345);
```

---

## Connection Exhaustion

### Diagnosis

```sql
SELECT
  current_setting('max_connections')::int AS max_conn,
  count(*) AS total,
  count(*) FILTER (WHERE state = 'idle') AS idle,
  count(*) FILTER (WHERE state = 'active') AS active,
  count(*) FILTER (WHERE state = 'idle in transaction') AS idle_in_txn
FROM pg_stat_activity;

SELECT application_name, state, count(*)
FROM pg_stat_activity
GROUP BY application_name, state
ORDER BY count DESC;
```

### Remediation

```sql
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE state = 'idle in transaction'
  AND query_start < now() - interval '10 minutes';
```

Use PgBouncer in transaction mode. Set `idle_in_transaction_session_timeout`. Reduce application pool sizes.

**Connection pool formula:**
```
pool_size = (core_count * 2) + effective_spindle_count
```
For SSD: `(4 cores * 2) + 1 = 9` connections per pool.

---

## Replication Lag

### On primary

```sql
SELECT slot_name, active,
  pg_size_pretty(pg_wal_lsn_diff(pg_current_wal_lsn(), restart_lsn)) AS lag
FROM pg_replication_slots;

SELECT client_addr, state,
  pg_size_pretty(pg_wal_lsn_diff(sent_lsn, replay_lsn)) AS replay_lag
FROM pg_stat_replication;
```

### On replica

```sql
SELECT now() - pg_last_xact_replay_timestamp() AS replication_delay;
```

### Common causes

- Slow disk I/O on replica
- Long-running queries on replica holding back WAL replay
- Network latency between primary and replica
- Heavy write load generating WAL faster than replay can apply

---

## Bloat Diagnosis

### Estimate table bloat

```sql
CREATE EXTENSION IF NOT EXISTS pgstattuple;

SELECT table_len, tuple_count, dead_tuple_count,
  round(dead_tuple_len::numeric / table_len * 100, 1) AS dead_pct
FROM pgstattuple('properties');
```

### Estimate via catalog (no extension needed)

```sql
SELECT schemaname, relname,
  pg_size_pretty(pg_total_relation_size(schemaname || '.' || relname)) AS total,
  pg_size_pretty(pg_relation_size(schemaname || '.' || relname)) AS table_only,
  n_dead_tup,
  round(n_dead_tup::numeric / nullif(n_live_tup + n_dead_tup, 0), 3) AS dead_ratio
FROM pg_stat_user_tables
WHERE n_dead_tup > 1000
ORDER BY n_dead_tup DESC;
```

### Remediation

| Method | Lock Level | When to Use |
|--------|-----------|-------------|
| `VACUUM ANALYZE` | ShareUpdateExclusiveLock (safe) | Routine maintenance |
| `VACUUM FULL` | AccessExclusiveLock (blocks all) | Maintenance window only |
| `pg_repack -d mydb -t tablename` | No lock | Production, bloat > 20% |
| `REINDEX CONCURRENTLY idx_name` | No exclusive lock | Index bloat (PG12+) |

---

## OOM and Memory Pressure

### Symptoms

- Processes killed by Linux OOM killer (`dmesg | grep -i "out of memory"`)
- Sudden connection drops
- `could not resize shared memory segment` errors

### Diagnosis

```bash
dmesg | grep -i "killed process"
ps aux --sort=-rss | grep postgres | head -20
```

### Prevention

```ini
shared_buffers = '4GB'          # 25% of RAM, max ~8GB on Linux
effective_cache_size = '12GB'   # 75% of RAM (planner hint only)
# work_mem × max_connections × avg_sorts_per_query must fit in remaining RAM
```

```bash
# /etc/sysctl.conf — prevent overcommit
vm.overcommit_memory = 2
vm.overcommit_ratio = 80
```

---

## Unused Index Detection

```sql
SELECT schemaname, relname AS table, indexrelname AS index,
  pg_size_pretty(pg_relation_size(indexrelid)) AS size, idx_scan
FROM pg_stat_user_indexes
WHERE idx_scan = 0 AND indexrelname NOT LIKE '%_pkey'
ORDER BY pg_relation_size(indexrelid) DESC;
```

Safe to drop indexes with `idx_scan = 0` after stats have been running for representative traffic period (reset with `SELECT pg_stat_reset()`).

---

## Invalid Indexes

After a failed `CREATE INDEX CONCURRENTLY`:

```sql
SELECT indexrelname, indisvalid
FROM pg_stat_user_indexes
JOIN pg_index ON indexrelid = pg_stat_user_indexes.indexrelid
WHERE NOT indisvalid;

DROP INDEX CONCURRENTLY idx_that_failed;
CREATE INDEX CONCURRENTLY idx_that_failed ON t(col);
```
