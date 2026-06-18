# Performance — Benchmarking, EXPLAIN, and Tuning

Self-contained guide for PostgreSQL query and system performance optimization.

---

## EXPLAIN Reading Guide

### Always use all four options

```sql
EXPLAIN (ANALYZE, BUFFERS, TIMING, FORMAT TEXT)
SELECT p.*, t.nombre
FROM properties p
JOIN tenants t ON t.property_id = p.id
WHERE p.city = 'Santiago' AND p.status = 'activo';
```

### Output interpretation

| Field | Meaning |
|-------|---------|
| `actual time=X..Y` | X = time to first row, Y = total time (ms) |
| `actual rows` vs `rows` (planned) | Mismatch → stale stats, run `ANALYZE` |
| `Buffers: shared hit=X read=Y` | hit = cache, read = disk. Low hit = cold cache |
| `Sort Method: external merge Disk` | Sort spilled to disk → increase work_mem |
| `Hash Batches: N` (N > 1) | Hash spilled to disk → increase work_mem |
| `Rows removed by Filter: N` | Fetched N rows then discarded → push into index |
| `Planning Time` | If high → use prepared statements |

### Node types to watch

| Node | Behavior | Concern |
|------|----------|---------|
| Seq Scan | Full table scan | Fine < 10k rows. Otherwise: missing index |
| Index Scan | B-tree lookup + heap fetch | Ideal for selective queries |
| Index Only Scan | Answered entirely from index | Best case — use INCLUDE columns |
| Bitmap Index Scan | Builds bitmap, then heap fetch | Multiple index conditions combined |
| Nested Loop | O(n×m) | Dangerous when both sides large |
| Hash Join | Builds hash from smaller set | Watch for disk spill (Batches > 1) |
| Merge Join | Pre-sorted merge | Efficient for large sorted inputs |

---

## pg_stat_statements — Query-Level Performance

```sql
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

SELECT queryid, calls,
  total_exec_time::numeric(12,2) AS total_ms,
  mean_exec_time::numeric(12,2) AS mean_ms,
  shared_blks_hit, shared_blks_read,
  round(shared_blks_hit::numeric /
    nullif(shared_blks_hit + shared_blks_read, 0), 3) AS cache_hit,
  rows, left(query, 100)
FROM pg_stat_statements
ORDER BY total_exec_time DESC LIMIT 10;
```

Reset periodically to track trends:
```sql
SELECT pg_stat_statements_reset();
```

---

## Cache Hit Ratio

Target: > 99% at database level.

```sql
SELECT
  sum(blks_hit) AS hits, sum(blks_read) AS reads,
  round(sum(blks_hit)::numeric /
    nullif(sum(blks_hit) + sum(blks_read), 0), 4) AS ratio
FROM pg_stat_database;
```

Per-table (find cold tables):
```sql
SELECT relname, heap_blks_hit, heap_blks_read,
  round(heap_blks_hit::numeric /
    nullif(heap_blks_hit + heap_blks_read, 0), 4) AS hit_ratio
FROM pg_statio_user_tables
ORDER BY heap_blks_read DESC LIMIT 10;
```

---

## work_mem Tuning

Controls memory per-sort or per-hash operation. A single query can use multiple allocations.

```sql
SHOW work_mem;  -- default: 4MB
```

**Signs you need more:**
- `Sort Method: external merge Disk` in EXPLAIN
- `Batches > 1` in Hash Join nodes
- High `temp_blks_read` in pg_stat_statements

**Formula:**
```
work_mem = available_ram × 0.25 / max_connections / avg_sorts_per_query
```
Example: 16GB RAM, 100 connections, ~3 sorts → `16GB * 0.25 / 100 / 3 ≈ 13MB`

**Per-session override for analytics:**
```sql
SET work_mem = '256MB';
-- run heavy query
RESET work_mem;
```

---

## Load Testing with pgbench

```bash
pgbench -i -s 100 mydb

pgbench -c 10 -j 4 -T 60 mydb

pgbench -c 20 -j 4 -T 120 -f custom_workload.sql mydb

pgbench -c 10 -j 4 -T 60 -S mydb
```

Key output metrics: TPS, latency average, latency stddev. Run before and after config/index changes.

---

## Connection Pool Sizing

**Formula:** `pool_size = (core_count × 2) + effective_spindle_count`

For SSD (spindle = 1): 4-core → pool_size = 9.

**PgBouncer config:**
```ini
[pgbouncer]
pool_mode = transaction
default_pool_size = 9
max_client_conn = 500
reserve_pool_size = 5
reserve_pool_timeout = 3
```

Use `transaction` mode for web apps (releases connection after each transaction).

---

## Autovacuum Tuning

Defaults are conservative. Write-heavy tables need aggressive settings:

```sql
ALTER TABLE payments SET (
  autovacuum_vacuum_scale_factor = 0.01,
  autovacuum_vacuum_threshold = 50,
  autovacuum_analyze_scale_factor = 0.005,
  autovacuum_vacuum_cost_delay = 2
);
```

**Global settings (postgresql.conf):**
```ini
autovacuum_max_workers = 5
autovacuum_naptime = 15s
autovacuum_vacuum_cost_limit = 1000
```

**When autovacuum can't keep up:**
- Table receives > 1000 updates/sec
- Default scale_factor 0.2 means waiting for 20% dead before acting
- Lower to 0.01–0.05 for hot tables

---

## Essential postgresql.conf (Performance)

```ini
# Memory
shared_buffers = '4GB'              # 25% of RAM (max ~8GB Linux)
effective_cache_size = '12GB'       # 75% of RAM (planner hint)
work_mem = '16MB'                   # per-sort/hash
maintenance_work_mem = '512MB'      # VACUUM, CREATE INDEX
huge_pages = 'try'

# Planner (SSD)
random_page_cost = 1.1
effective_io_concurrency = 200
default_statistics_target = 100     # 500 for skewed columns

# WAL
max_wal_size = '4GB'
checkpoint_completion_target = 0.9

# Connections
max_connections = 100               # keep low, use PgBouncer
```

---

## Index Optimization Patterns

### Composite index column order

Put equality columns first, range columns last:
```sql
CREATE INDEX ON payments(contract_id, fecha_pago);
```

### Partial index (filter on subset)

```sql
CREATE INDEX idx_active_contracts ON contracts(property_id)
  WHERE estado = 'activo';
```

### Covering index (avoid heap fetch)

```sql
CREATE INDEX idx_tenants_email ON tenants(email) INCLUDE (nombre, telefono);
```

### Expression index

```sql
CREATE INDEX idx_properties_lower_city ON properties(lower(city));
```

---

## Tables Needing Indexes (Detection)

```sql
SELECT schemaname, relname, seq_scan, seq_tup_read, idx_scan, n_live_tup
FROM pg_stat_user_tables
WHERE seq_scan > 100 AND n_live_tup > 10000
ORDER BY seq_tup_read DESC LIMIT 10;
```

High `seq_scan` with large `seq_tup_read` on big tables = index candidates.

---

## Temp File Usage (Disk Spills)

```sql
SELECT datname, temp_files, temp_bytes,
  pg_size_pretty(temp_bytes) AS temp_size
FROM pg_stat_database
WHERE temp_bytes > 0;
```

High temp file usage indicates work_mem is too low for the workload.
