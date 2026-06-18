                                                                                                 # PostgreSQL Best Practices — Research Notes

## 1. Essential Tools & Extensions

### pg_stat_statements

Tracks execution statistics for all SQL statements. Must be loaded via
`shared_preload_libraries` in `postgresql.conf`.

```sql
-- Enable (requires restart)
-- postgresql.conf: shared_preload_libraries = 'pg_stat_statements'
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

-- Top 10 slowest queries by total time
SELECT
  queryid,
  calls,
  total_exec_time::numeric(12,2) AS total_ms,
  mean_exec_time::numeric(12,2) AS mean_ms,
  rows,
  query
FROM pg_stat_statements
ORDER BY total_exec_time DESC
LIMIT 10;

-- Top queries by I/O (shared blocks read = cache misses)
SELECT
  queryid,
  calls,
  shared_blks_hit,
  shared_blks_read,
  round(shared_blks_hit::numeric /
    nullif(shared_blks_hit + shared_blks_read, 0), 3) AS hit_ratio,
  query
FROM pg_stat_statements
ORDER BY shared_blks_read DESC
LIMIT 10;

-- Reset stats periodically
SELECT pg_stat_statements_reset();
```

### pg_stat_user_tables

Table-level statistics: sequential scans, index scans, dead tuples, last vacuum/analyze.

```sql
-- Tables with most sequential scans (candidates for indexing)
SELECT
  schemaname, relname,
  seq_scan,
  seq_tup_read,
  idx_scan,
  n_live_tup,
  n_dead_tup,
  last_vacuum,
  last_autovacuum,
  last_analyze
FROM pg_stat_user_tables
ORDER BY seq_scan DESC
LIMIT 20;

-- Tables with high dead tuple ratio (need vacuum)
SELECT
  schemaname, relname,
  n_live_tup,
  n_dead_tup,
  round(n_dead_tup::numeric / nullif(n_live_tup + n_dead_tup, 0), 3) AS dead_ratio,
  last_autovacuum
FROM pg_stat_user_tables
WHERE n_dead_tup > 1000
ORDER BY dead_ratio DESC;
```

### pg_stat_activity

Live view of all active connections and their current state.

```sql
-- Active queries (running right now)
SELECT
  pid, usename, datname, state,
  now() - query_start AS duration,
  wait_event_type, wait_event,
  left(query, 100) AS query_preview
FROM pg_stat_activity
WHERE state = 'active'
  AND pid <> pg_backend_pid()
ORDER BY duration DESC;

-- Long-running queries (> 5 minutes)
SELECT pid, usename, query_start, state, query
FROM pg_stat_activity
WHERE state != 'idle'
  AND query_start < now() - interval '5 minutes'
  AND pid <> pg_backend_pid();

-- Kill a specific backend
SELECT pg_terminate_backend(12345);

-- Connection counts by state
SELECT state, count(*)
FROM pg_stat_activity
GROUP BY state;
```

### EXPLAIN ANALYZE Reading Guide

```sql
-- Full diagnostic query plan
EXPLAIN (ANALYZE, BUFFERS, TIMING, FORMAT TEXT)
SELECT * FROM properties WHERE city = 'Santo Domingo';
```

**What to look for:**

1. **Seq Scan** — Full table scan. Acceptable on small tables (<10k rows).
   On larger tables, indicates a missing or unused index.
   
2. **Nested Loop** — O(n×m) join. Fine when inner relation is small or
   indexed. Dangerous when both sides are large.

3. **Hash Join** — Builds hash table from smaller relation, probes with
   larger. Watch for `Batches > 1` which means hash spilled to disk
   (increase `work_mem`).

4. **Sort** — Look for `Sort Method: external merge Disk`. This means
   sort spilled to disk. Increase `work_mem` or add index with matching
   ORDER BY.

5. **Rows removed by Filter** — Large numbers mean the planner fetched
   many rows then discarded them. Push filtering into an index.

6. **actual rows vs planned rows** — Large discrepancy means stale
   statistics. Run `ANALYZE tablename;`.

7. **Buffers: shared hit vs shared read** — `hit` = from cache,
   `read` = from disk. Low hit ratio = insufficient `shared_buffers`
   or working set too large.

### pgbench — Load Testing

```bash
# Initialize with scale factor 100 (~1.5GB data)
pgbench -i -s 100 mydb

# Run 10 clients, 4 threads, 60 seconds
pgbench -c 10 -j 4 -T 60 mydb

# Custom script
pgbench -c 20 -j 4 -T 120 -f custom_workload.sql mydb

# Read-only workload (SELECT only)
pgbench -c 10 -j 4 -T 60 -S mydb
```

Key metrics: TPS (transactions per second), latency average, latency stddev.
Compare before/after index changes or config tuning.

### pg_repack — Online Table/Index Rebuild

Removes bloat without exclusive locks (unlike VACUUM FULL or REINDEX).

```bash
# Repack a specific table (removes dead tuples, reorders by PK)
pg_repack -d mydb -t properties

# Repack all indexes on a table
pg_repack -d mydb -t properties --only-indexes

# Repack entire database
pg_repack -d mydb
```

Use when: table bloat > 20%, after bulk deletes, or when index bloat
causes scan performance degradation.

### pg_cron — Scheduled Maintenance

```sql
CREATE EXTENSION pg_cron;

-- Vacuum analyze every night at 3am
SELECT cron.schedule('nightly-vacuum', '0 3 * * *',
  'VACUUM ANALYZE');

-- Refresh materialized view every hour
SELECT cron.schedule('refresh-mv', '0 * * * *',
  'REFRESH MATERIALIZED VIEW CONCURRENTLY mv_property_stats');

-- Purge old audit logs weekly
SELECT cron.schedule('purge-audit', '0 4 * * 0',
  'DELETE FROM audit_log WHERE created_at < now() - interval ''90 days''');

-- List scheduled jobs
SELECT * FROM cron.job;

-- Unschedule
SELECT cron.unschedule('nightly-vacuum');
```

### pg_trgm — Fuzzy Search

```sql
CREATE EXTENSION pg_trgm;

-- GIN index for trigram similarity
CREATE INDEX idx_properties_address_trgm
  ON properties USING GIN (address gin_trgm_ops);

-- Fuzzy search with similarity threshold
SELECT address, similarity(address, 'Santo Domigo') AS sim
FROM properties
WHERE address % 'Santo Domigo'
ORDER BY sim DESC
LIMIT 10;

-- ILIKE with trigram acceleration
SELECT * FROM tenants
WHERE nombre ILIKE '%gonza%';
-- (GIN trgm index accelerates ILIKE/LIKE patterns)
```

### pgvector — Embeddings

```sql
CREATE EXTENSION vector;

-- Store embeddings (1536 dimensions for OpenAI ada-002)
CREATE TABLE property_embeddings (
  id bigint PRIMARY KEY REFERENCES properties(id),
  embedding vector(1536)
);

-- HNSW index (fast approximate nearest neighbor)
CREATE INDEX idx_property_embedding_hnsw
  ON property_embeddings
  USING hnsw (embedding vector_cosine_ops)
  WITH (m = 16, ef_construction = 64);

-- IVFFlat index (alternative, requires training data)
CREATE INDEX idx_property_embedding_ivf
  ON property_embeddings
  USING ivfflat (embedding vector_cosine_ops)
  WITH (lists = 100);

-- Similarity search (cosine distance)
SELECT p.id, p.address, e.embedding <=> '[0.1, 0.2, ...]'::vector AS distance
FROM property_embeddings e
JOIN properties p ON p.id = e.id
ORDER BY e.embedding <=> '[0.1, 0.2, ...]'::vector
LIMIT 10;

-- Set probes for IVFFlat (trade accuracy for speed)
SET ivfflat.probes = 10;
```

### Useful Catalog Queries

```sql
-- Table bloat estimate (based on fillfactor and dead tuples)
SELECT
  schemaname, tablename,
  pg_size_pretty(pg_total_relation_size(schemaname || '.' || tablename)) AS total_size,
  pg_size_pretty(pg_relation_size(schemaname || '.' || tablename)) AS table_size,
  pg_size_pretty(pg_indexes_size(schemaname || '.' || tablename::regclass)) AS index_size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname || '.' || tablename) DESC;

-- Unused indexes (never scanned since last stats reset)
SELECT
  schemaname, relname AS table, indexrelname AS index,
  pg_size_pretty(pg_relation_size(indexrelid)) AS size,
  idx_scan
FROM pg_stat_user_indexes
WHERE idx_scan = 0
  AND indexrelname NOT LIKE '%_pkey'
ORDER BY pg_relation_size(indexrelid) DESC;

-- Lock waits (who is blocking whom)
SELECT
  blocked.pid AS blocked_pid,
  blocked.query AS blocked_query,
  blocking.pid AS blocking_pid,
  blocking.query AS blocking_query,
  now() - blocked.query_start AS wait_duration
FROM pg_stat_activity blocked
JOIN pg_locks bl ON bl.pid = blocked.pid
JOIN pg_locks kl ON kl.locktype = bl.locktype
  AND kl.database IS NOT DISTINCT FROM bl.database
  AND kl.relation IS NOT DISTINCT FROM bl.relation
  AND kl.page IS NOT DISTINCT FROM bl.page
  AND kl.tuple IS NOT DISTINCT FROM bl.tuple
  AND kl.transactionid IS NOT DISTINCT FROM bl.transactionid
  AND kl.pid != bl.pid
  AND kl.granted
JOIN pg_stat_activity blocking ON blocking.pid = kl.pid
WHERE NOT bl.granted;
```


## 2. Performance Benchmarking Workflow

### Query Benchmarking with EXPLAIN

```sql
-- Full diagnostic (always use all four options)
EXPLAIN (ANALYZE, BUFFERS, TIMING, FORMAT TEXT)
SELECT p.*, t.nombre
FROM properties p
JOIN tenants t ON t.property_id = p.id
WHERE p.city = 'Santiago'
  AND p.status = 'activo';
```

**Reading the output:**

- `actual time=X..Y` — X is startup time (first row), Y is total time
  (all rows). Both in milliseconds.
- `actual rows=N` vs `rows=N (planned)` — Mismatch means stale stats.
  Fix with `ANALYZE tablename;`.
- `Buffers: shared hit=X read=Y` — hit = from shared_buffers cache,
  read = from OS/disk. High read count = cold cache or table too large
  for shared_buffers.
- `I/O Timings` (requires `track_io_timing = on`):
  `I/O read=X write=Y` — actual disk wait time.
- `Planning Time` vs `Execution Time` — if planning time is high,
  consider `plan_cache_mode = force_generic_plan` for parameterized
  queries or use prepared statements.

### Cache Hit Ratio

```sql
-- Database-wide cache hit ratio (should be > 99%)
SELECT
  sum(blks_hit) AS hits,
  sum(blks_read) AS reads,
  round(sum(blks_hit)::numeric /
    nullif(sum(blks_hit) + sum(blks_read), 0), 4) AS ratio
FROM pg_stat_database;

-- Per-table cache hit ratio
SELECT
  relname,
  heap_blks_hit,
  heap_blks_read,
  round(heap_blks_hit::numeric /
    nullif(heap_blks_hit + heap_blks_read, 0), 4) AS hit_ratio
FROM pg_statio_user_tables
ORDER BY heap_blks_read DESC
LIMIT 10;
```

### work_mem Tuning

`work_mem` controls memory per-sort or per-hash operation (not per-query).
A single complex query can use multiple work_mem allocations.

```sql
-- Check current setting
SHOW work_mem;  -- default 4MB

-- Tune per-session for heavy analytics
SET work_mem = '256MB';
-- Then run your query and check EXPLAIN for disk spills

-- Formula: available_ram / max_connections / avg_sorts_per_query
-- Example: 16GB server, 100 connections, ~3 sorts/query
-- work_mem = 16GB * 0.25 / 100 / 3 ≈ 13MB (conservative)
```

Signs you need more work_mem:
- `Sort Method: external merge Disk` in EXPLAIN
- `Batches: N` (N > 1) in Hash Join nodes
- High `temp_blks_read` / `temp_blks_written` in pg_stat_statements

### Connection Pool Sizing

Formula (from PostgreSQL wiki):
```
pool_size = ((core_count * 2) + effective_spindle_count)
```

For SSD: effective_spindle_count = 1.
Example: 4-core server with SSD → pool_size = (4 * 2) + 1 = 9.

In practice with PgBouncer:
- `default_pool_size` = formula above (per database/user pair)
- `max_client_conn` = application connection limit (can be 1000+)
- `reserve_pool_size` = 5 (burst buffer)
- Mode: `transaction` for web apps (releases conn after each txn)

```ini
# pgbouncer.ini
[databases]
mydb = host=127.0.0.1 port=5432 dbname=mydb

[pgbouncer]
pool_mode = transaction
default_pool_size = 9
max_client_conn = 500
reserve_pool_size = 5
reserve_pool_timeout = 3
```

### Autovacuum Tuning

Default settings are conservative. For write-heavy tables:

```sql
-- Check autovacuum activity
SELECT relname, n_dead_tup, last_autovacuum,
  autovacuum_count, autoanalyze_count
FROM pg_stat_user_tables
WHERE n_dead_tup > 0
ORDER BY n_dead_tup DESC;

-- Per-table aggressive autovacuum for hot tables
ALTER TABLE payments SET (
  autovacuum_vacuum_scale_factor = 0.01,    -- default 0.2 (20%)
  autovacuum_vacuum_threshold = 50,          -- default 50
  autovacuum_analyze_scale_factor = 0.005,   -- default 0.1
  autovacuum_vacuum_cost_delay = 2           -- default 2ms (make faster: 0)
);
```

**Key parameters (postgresql.conf):**
- `autovacuum_max_workers = 5` (default 3; increase for many tables)
- `autovacuum_naptime = 15s` (default 1min; check more often)
- `autovacuum_vacuum_cost_limit = 1000` (default 200; let vacuum work harder)

**When autovacuum can't keep up:**
- Table receives >1000 updates/sec
- Scale factor 0.2 means waiting for 20% of table to be dead before acting
- Solution: lower scale_factor to 0.01-0.05 for hot tables

## 3. Schema Design Workflow

### Step-by-Step Process

1. **Requirements** — Identify entities, relationships, access patterns,
   write/read ratio, data volumes, retention needs.

2. **Entities** — Map each domain concept to a table. Use singular names.
   Every table gets: `id` (bigserial or UUID), `created_at`, `updated_at`.

3. **Relationships** — Foreign keys with appropriate ON DELETE behavior:
   - `CASCADE` for owned children (contract → payments)
   - `RESTRICT` for referenced entities (payment → tenant)
   - `SET NULL` for soft references (property → optional manager)

4. **Normalization** — Start at 3NF. Each fact stored once.
   Denormalize only when measured query performance demands it.

5. **Constraints** — NOT NULL by default. CHECK constraints for domains.
   UNIQUE constraints for natural keys. EXCLUDE for ranges.

6. **Indexes** — Add after query patterns are known:
   - B-tree (default): equality and range
   - GIN: arrays, JSONB, full-text, trigrams
   - GiST: geometry, ranges, full-text
   - BRIN: naturally ordered data (timestamps on append-only tables)

### When to Denormalize

**Materialized Views** — Pre-computed aggregates refreshed on schedule:
```sql
CREATE MATERIALIZED VIEW mv_property_financials AS
SELECT
  p.id AS property_id,
  p.nombre,
  count(DISTINCT c.id) AS active_contracts,
  coalesce(sum(pay.monto), 0) AS total_income,
  coalesce(sum(exp.monto), 0) AS total_expenses
FROM properties p
LEFT JOIN contracts c ON c.property_id = p.id AND c.estado = 'activo'
LEFT JOIN payments pay ON pay.contract_id = c.id
LEFT JOIN expenses exp ON exp.property_id = p.id
GROUP BY p.id, p.nombre;

CREATE UNIQUE INDEX ON mv_property_financials(property_id);

-- Refresh without blocking reads
REFRESH MATERIALIZED VIEW CONCURRENTLY mv_property_financials;
```

**JSONB columns** — For semi-structured or variable attributes:
```sql
ALTER TABLE properties ADD COLUMN metadata jsonb DEFAULT '{}';

-- GIN index for containment queries
CREATE INDEX idx_properties_metadata ON properties USING GIN (metadata);

-- Query JSONB
SELECT * FROM properties
WHERE metadata @> '{"amenities": ["pool"]}';

-- Partial index on JSONB key
CREATE INDEX idx_properties_furnished ON properties ((metadata->>'furnished'))
WHERE metadata->>'furnished' = 'true';
```

### Partitioning Strategies

**Range partitioning** — Time-series data, logs, payments by date:
```sql
CREATE TABLE payments (
  id bigserial,
  contract_id bigint NOT NULL,
  monto numeric(12,2) NOT NULL,
  fecha_pago date NOT NULL,
  created_at timestamptz DEFAULT now()
) PARTITION BY RANGE (fecha_pago);

CREATE TABLE payments_2024 PARTITION OF payments
  FOR VALUES FROM ('2024-01-01') TO ('2025-01-01');
CREATE TABLE payments_2025 PARTITION OF payments
  FOR VALUES FROM ('2025-01-01') TO ('2026-01-01');
```

**List partitioning** — By category or region:
```sql
CREATE TABLE properties (
  id bigserial, ciudad text NOT NULL, ...
) PARTITION BY LIST (ciudad);

CREATE TABLE properties_santo_domingo PARTITION OF properties
  FOR VALUES IN ('Santo Domingo');
CREATE TABLE properties_santiago PARTITION OF properties
  FOR VALUES IN ('Santiago');
CREATE TABLE properties_other PARTITION OF properties DEFAULT;
```

**Hash partitioning** — Even distribution when no natural range:
```sql
CREATE TABLE audit_log (
  id bigserial, tenant_id bigint, ...
) PARTITION BY HASH (tenant_id);

CREATE TABLE audit_log_0 PARTITION OF audit_log
  FOR VALUES WITH (MODULUS 4, REMAINDER 0);
CREATE TABLE audit_log_1 PARTITION OF audit_log
  FOR VALUES WITH (MODULUS 4, REMAINDER 1);
-- etc.
```

### Multi-Tenant Patterns

**1. tenant_id column (recommended for most apps):**
```sql
-- Every table gets tenant_id
ALTER TABLE properties ADD COLUMN tenant_id bigint NOT NULL;
CREATE INDEX idx_properties_tenant ON properties(tenant_id);

-- Always filter by tenant_id in application layer
SELECT * FROM properties WHERE tenant_id = $1 AND status = 'activo';
```
Pros: simple, single schema, easy migrations.
Cons: must always remember WHERE clause, risk of data leak.

**2. Row-Level Security (RLS) — enforcement layer on top of tenant_id:**
```sql
ALTER TABLE properties ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON properties
  USING (tenant_id = current_setting('app.current_tenant')::bigint);

-- Set tenant context per request (from app connection pool)
SET app.current_tenant = '42';
```
Pros: database enforces isolation, can't forget WHERE clause.
Cons: slightly slower, complexity in connection pooling.

**3. Schema-per-tenant:**
```sql
CREATE SCHEMA tenant_42;
CREATE TABLE tenant_42.properties (...);

-- Set search_path per connection
SET search_path = tenant_42, public;
```
Pros: strongest isolation, easy per-tenant backup/restore.
Cons: schema proliferation, migration complexity at scale (>100 tenants).

## 4. Troubleshooting Workflow

### Slow Query Diagnosis — Step by Step

1. **Identify the slow query** via pg_stat_statements (highest total_time
   or mean_time) or pg_stat_activity (long-running).

2. **Get the plan:**
   ```sql
   EXPLAIN (ANALYZE, BUFFERS, TIMING) <the_query>;
   ```

3. **Check for seq scans** on large tables → add index.

4. **Check row estimates** — if `actual rows` >> `planned rows`, run
   `ANALYZE tablename;` to update statistics.

5. **Check for disk spills** — Sort/Hash using disk → increase work_mem.

6. **Check buffer reads** — high shared_blks_read → data not in cache.
   Either increase shared_buffers or query touches too much data.

7. **Check for nested loops** on large joins → consider hash/merge join
   hints or restructure query.

8. **Check index usage** — is the right index being chosen? Partial
   index or covering index might help.

### Lock Contention Diagnosis

```sql
-- Find blocked queries and what's blocking them
SELECT
  blocked_locks.pid AS blocked_pid,
  blocked_activity.usename AS blocked_user,
  now() - blocked_activity.query_start AS blocked_duration,
  blocking_locks.pid AS blocking_pid,
  blocking_activity.usename AS blocking_user,
  blocking_activity.query AS blocking_query,
  blocked_activity.query AS blocked_query
FROM pg_catalog.pg_locks blocked_locks
JOIN pg_catalog.pg_stat_activity blocked_activity
  ON blocked_activity.pid = blocked_locks.pid
JOIN pg_catalog.pg_locks blocking_locks
  ON blocking_locks.locktype = blocked_locks.locktype
  AND blocking_locks.database IS NOT DISTINCT FROM blocked_locks.database
  AND blocking_locks.relation IS NOT DISTINCT FROM blocked_locks.relation
  AND blocking_locks.page IS NOT DISTINCT FROM blocked_locks.page
  AND blocking_locks.tuple IS NOT DISTINCT FROM blocked_locks.tuple
  AND blocking_locks.virtualxid IS NOT DISTINCT FROM blocked_locks.virtualxid
  AND blocking_locks.transactionid IS NOT DISTINCT FROM blocked_locks.transactionid
  AND blocking_locks.classid IS NOT DISTINCT FROM blocked_locks.classid
  AND blocking_locks.objid IS NOT DISTINCT FROM blocked_locks.objid
  AND blocking_locks.objsubid IS NOT DISTINCT FROM blocked_locks.objsubid
  AND blocking_locks.pid != blocked_locks.pid
JOIN pg_catalog.pg_stat_activity blocking_activity
  ON blocking_activity.pid = blocking_locks.pid
WHERE NOT blocked_locks.granted;

-- Set statement timeout to prevent indefinite waits
SET statement_timeout = '30s';

-- Set lock timeout to fail fast on lock contention
SET lock_timeout = '5s';
```

### Connection Exhaustion Diagnosis

```sql
-- Current connection usage vs limit
SELECT
  current_setting('max_connections')::int AS max_conn,
  (SELECT count(*) FROM pg_stat_activity) AS current_conn,
  (SELECT count(*) FROM pg_stat_activity WHERE state = 'idle') AS idle_conn,
  (SELECT count(*) FROM pg_stat_activity WHERE state = 'active') AS active_conn,
  (SELECT count(*) FROM pg_stat_activity
   WHERE state = 'idle in transaction') AS idle_in_txn;

-- Connections by application
SELECT application_name, count(*), state
FROM pg_stat_activity
GROUP BY application_name, state
ORDER BY count DESC;

-- Kill idle-in-transaction connections older than 10 minutes
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE state = 'idle in transaction'
  AND query_start < now() - interval '10 minutes';
```

Remediation: use PgBouncer, set `idle_in_transaction_session_timeout`,
reduce application pool sizes.

### Replication Lag Diagnosis

```sql
-- On primary: check replication slots and lag
SELECT
  slot_name,
  active,
  pg_size_pretty(pg_wal_lsn_diff(pg_current_wal_lsn(), restart_lsn)) AS lag_size
FROM pg_replication_slots;

-- On primary: check streaming replication status
SELECT
  client_addr,
  state,
  sent_lsn,
  write_lsn,
  flush_lsn,
  replay_lsn,
  pg_size_pretty(pg_wal_lsn_diff(sent_lsn, replay_lsn)) AS replay_lag,
  write_lag, flush_lag, replay_lag AS replay_delay
FROM pg_stat_replication;

-- On replica: check how far behind
SELECT
  now() - pg_last_xact_replay_timestamp() AS replication_delay;
```

Causes: slow disk on replica, network latency, heavy write load,
long-running queries on replica holding back replay.

### Bloat Diagnosis and Remediation

```sql
-- Estimate table bloat (pgstattuple extension)
CREATE EXTENSION pgstattuple;

SELECT
  table_len,
  tuple_count,
  dead_tuple_count,
  dead_tuple_len,
  round(dead_tuple_len::numeric / table_len * 100, 1) AS dead_pct
FROM pgstattuple('properties');

-- Index bloat estimate
SELECT
  indexrelname,
  pg_size_pretty(pg_relation_size(indexrelid)) AS size,
  idx_scan AS scans
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY pg_relation_size(indexrelid) DESC;
```

**Remediation options:**
1. `VACUUM FULL tablename;` — rewrites table, takes ACCESS EXCLUSIVE lock.
   Only for maintenance windows.
2. `pg_repack -d mydb -t tablename` — online, no lock. Preferred.
3. For indexes: `REINDEX CONCURRENTLY INDEX idx_name;` (PG12+).

### OOM and Memory Pressure Symptoms

Signs:
- PostgreSQL processes killed by Linux OOM killer (check `dmesg`)
- Sudden connection drops
- `could not resize shared memory segment` errors

Diagnosis:
```bash
# Check OOM kills
dmesg | grep -i "out of memory"
dmesg | grep -i "killed process"

# Check PostgreSQL memory usage
ps aux --sort=-rss | grep postgres | head -20
```

Prevention:
- `shared_buffers` = 25% of RAM (never more than 8GB on Linux)
- `effective_cache_size` = 75% of RAM (planner hint, not allocation)
- `work_mem` × max_connections × avg_sorts must fit in remaining RAM
- Set `vm.overcommit_memory = 2` and `vm.overcommit_ratio = 80` in
  `/etc/sysctl.conf` to prevent overcommit
- `huge_pages = try` for shared_buffers > 1GB


## 5. Migration Safety Patterns

### Safe Column Addition

Never add a NOT NULL column without a default in a single step on a
large table — it rewrites the entire table (pre-PG11) or takes a long
lock.

**Pattern: nullable first, backfill, then constraint.**
```sql
-- Step 1: Add nullable column (instant, no rewrite)
ALTER TABLE properties ADD COLUMN manager_id bigint;

-- Step 2: Backfill in batches (avoid long transactions)
UPDATE properties SET manager_id = 1
WHERE id BETWEEN 1 AND 10000 AND manager_id IS NULL;
-- repeat for next batch...

-- Step 3: Add NOT NULL constraint (validates existing rows)
ALTER TABLE properties ALTER COLUMN manager_id SET NOT NULL;

-- Alternative Step 3: Add constraint as NOT VALID then validate separately
ALTER TABLE properties
  ADD CONSTRAINT properties_manager_id_nn
  CHECK (manager_id IS NOT NULL) NOT VALID;

-- Validate in background (doesn't block writes)
ALTER TABLE properties VALIDATE CONSTRAINT properties_manager_id_nn;
```

**PG11+ shortcut:** Adding a column with a non-volatile DEFAULT is instant:
```sql
-- This is instant in PG11+ (stores default in catalog, not on disk)
ALTER TABLE properties ADD COLUMN is_featured boolean NOT NULL DEFAULT false;
```

### Safe Index Creation

```sql
-- NEVER do this on a production table (blocks writes):
-- CREATE INDEX idx_foo ON big_table(col);

-- ALWAYS use CONCURRENTLY (doesn't block writes):
CREATE INDEX CONCURRENTLY idx_properties_city
  ON properties(city);

-- If it fails (marked INVALID), drop and retry:
DROP INDEX CONCURRENTLY idx_properties_city;
CREATE INDEX CONCURRENTLY idx_properties_city
  ON properties(city);

-- Check for invalid indexes:
SELECT indexrelname, indexrelid, indisvalid
FROM pg_stat_user_indexes
JOIN pg_index ON indexrelid = pg_stat_user_indexes.indexrelid
WHERE NOT indisvalid;
```

### Safe Enum Additions

```sql
-- Adding a value to an enum is safe (no rewrite):
ALTER TYPE contract_estado ADD VALUE 'suspendido';

-- BUT: cannot be done inside a transaction block in PG < 12.
-- In PG12+, it's transactional.

-- To add at a specific position:
ALTER TYPE contract_estado ADD VALUE 'suspendido' AFTER 'activo';

-- Removing enum values is NOT supported directly.
-- Pattern: create new type, migrate column, drop old type.
```

### Zero-Downtime Rename Patterns

**Column rename (application code must handle both names temporarily):**
```sql
-- Step 1: Add new column
ALTER TABLE properties ADD COLUMN nombre_propiedad text;

-- Step 2: Backfill from old column
UPDATE properties SET nombre_propiedad = nombre WHERE nombre_propiedad IS NULL;

-- Step 3: Add trigger to keep both in sync during deploy
CREATE OR REPLACE FUNCTION sync_property_name() RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'INSERT' OR NEW.nombre IS DISTINCT FROM OLD.nombre THEN
    NEW.nombre_propiedad := NEW.nombre;
  END IF;
  IF TG_OP = 'INSERT' OR NEW.nombre_propiedad IS DISTINCT FROM OLD.nombre_propiedad THEN
    NEW.nombre := NEW.nombre_propiedad;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_sync_property_name
  BEFORE INSERT OR UPDATE ON properties
  FOR EACH ROW EXECUTE FUNCTION sync_property_name();

-- Step 4: Deploy app reading from new column
-- Step 5: Drop old column and trigger
ALTER TABLE properties DROP COLUMN nombre;
DROP TRIGGER trg_sync_property_name ON properties;
DROP FUNCTION sync_property_name();
```

**Table rename:** Use a view as a compatibility shim:
```sql
ALTER TABLE old_name RENAME TO new_name;
CREATE VIEW old_name AS SELECT * FROM new_name;
-- Deploy app to use new_name, then drop view
```

### Large Table Alterations with pg_repack

When ALTER TABLE would rewrite (e.g., changing column type):
```bash
# pg_repack can reorganize table without long locks
pg_repack -d mydb -t properties

# For column type changes, the pattern is:
# 1. Add new column with new type
# 2. Backfill in batches with CAST
# 3. Swap via rename (or use triggers for sync)
# 4. Drop old column
```

## 6. Monitoring & Alerting

### Key Metrics to Watch

| Metric | Query | Alert Threshold |
|--------|-------|----------------|
| Connection usage | `count(*) FROM pg_stat_activity` | > 80% of max_connections |
| Cache hit ratio | see Section 2 | < 0.99 |
| Replication lag | `pg_wal_lsn_diff(sent, replay)` | > 100MB or > 30s delay |
| Dead tuples | `n_dead_tup` from pg_stat_user_tables | > 10% of live tuples |
| Long-running txns | `now() - xact_start` | > 5 minutes |
| Lock waits | blocked queries count | > 0 for > 30s |
| Temp file usage | `temp_bytes` from pg_stat_database | > 1GB/hour |
| WAL generation rate | `pg_wal_lsn_diff` over time | abnormal spike |
| Idle in transaction | state = 'idle in transaction' | > 5 minutes |

### Dashboard Queries (Prometheus/Grafana Compatible via postgres_exporter)

```sql
-- Connections by state (for time-series dashboard)
SELECT state, count(*) FROM pg_stat_activity GROUP BY state;

-- Transaction rate (commits + rollbacks per second)
SELECT
  xact_commit + xact_rollback AS total_txns,
  xact_commit, xact_rollback
FROM pg_stat_database
WHERE datname = current_database();

-- Tuple operations (inserts, updates, deletes per table)
SELECT
  relname,
  n_tup_ins AS inserts,
  n_tup_upd AS updates,
  n_tup_del AS deletes,
  n_tup_hot_upd AS hot_updates
FROM pg_stat_user_tables
ORDER BY n_tup_upd + n_tup_del DESC
LIMIT 10;

-- WAL generation rate
SELECT
  pg_wal_lsn_diff(pg_current_wal_lsn(), '0/0') AS wal_bytes_total;

-- Checkpoint activity
SELECT
  checkpoints_timed, checkpoints_req,
  checkpoint_write_time, checkpoint_sync_time,
  buffers_checkpoint, buffers_clean, buffers_backend
FROM pg_stat_bgwriter;
```

### Alert Thresholds (Recommended Starting Points)

- **Connection saturation:** WARN at 70%, CRIT at 85% of max_connections
- **Cache hit ratio:** WARN < 0.995, CRIT < 0.99
- **Replication lag:** WARN > 10s, CRIT > 60s
- **Dead tuple ratio:** WARN > 5%, CRIT > 20% (per table)
- **Long-running transactions:** WARN > 5min, CRIT > 30min
- **Temp file usage:** WARN > 500MB in single query
- **Idle in transaction:** WARN > 5min (auto-kill at 10min via
  `idle_in_transaction_session_timeout`)
- **Lock wait time:** WARN any query blocked > 30s

## 7. Security Hardening

### Role Hierarchy Design

```sql
-- Base roles (no login, used for permission grouping)
CREATE ROLE app_readonly;
CREATE ROLE app_readwrite;
CREATE ROLE app_admin;

-- Grant hierarchy
GRANT app_readonly TO app_readwrite;
GRANT app_readwrite TO app_admin;

-- Permissions
GRANT USAGE ON SCHEMA public TO app_readonly;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO app_readonly;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT SELECT ON TABLES TO app_readonly;

GRANT INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO app_readwrite;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT INSERT, UPDATE, DELETE ON TABLES TO app_readwrite;

GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO app_admin;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO app_admin;

-- Login roles (actual users/services)
CREATE ROLE app_backend LOGIN PASSWORD 'xxx';
GRANT app_readwrite TO app_backend;

CREATE ROLE app_reporting LOGIN PASSWORD 'xxx';
GRANT app_readonly TO app_reporting;

-- Revoke public schema access from PUBLIC
REVOKE ALL ON SCHEMA public FROM PUBLIC;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
```

### Row-Level Security (RLS) Patterns

```sql
-- Enable RLS on a table
ALTER TABLE properties ENABLE ROW LEVEL SECURITY;

-- Force RLS even for table owners (important!)
ALTER TABLE properties FORCE ROW LEVEL SECURITY;

-- Policy: users see only their tenant's data
CREATE POLICY tenant_isolation ON properties
  FOR ALL
  USING (tenant_id = current_setting('app.current_tenant')::bigint)
  WITH CHECK (tenant_id = current_setting('app.current_tenant')::bigint);

-- Policy: role-based access (admin sees all, gerente sees own)
CREATE POLICY admin_full_access ON properties
  FOR ALL
  TO app_admin
  USING (true);

CREATE POLICY manager_own_properties ON properties
  FOR ALL
  TO app_readwrite
  USING (manager_id = current_setting('app.current_user_id')::bigint);

-- Set context at connection time (from application)
-- In Rust (sqlx): sqlx::query("SET app.current_tenant = $1")
SET LOCAL app.current_tenant = '42';
SET LOCAL app.current_user_id = '7';
```

### Column-Level Encryption Patterns

```sql
-- Using pgcrypto for sensitive fields
CREATE EXTENSION pgcrypto;

-- Encrypt on insert
INSERT INTO tenants (nombre, cedula_encrypted)
VALUES (
  'Juan Perez',
  pgp_sym_encrypt('001-1234567-8', current_setting('app.encryption_key'))
);

-- Decrypt on read
SELECT
  nombre,
  pgp_sym_decrypt(cedula_encrypted::bytea,
    current_setting('app.encryption_key')) AS cedula
FROM tenants
WHERE id = $1;

-- Better pattern: encrypt/decrypt in application layer, store as bytea.
-- Database never sees plaintext, key never in DB config.
ALTER TABLE tenants ADD COLUMN cedula_encrypted bytea;
```

### Network Security (pg_hba.conf Patterns)

```
# TYPE  DATABASE  USER         ADDRESS         METHOD

# Local socket connections
local   all       postgres                     peer

# Reject all by default
host    all       all          0.0.0.0/0       reject

# Allow app backend from k8s pod network only
host    mydb      app_backend  10.42.0.0/16    scram-sha-256

# Allow replication from known replica
host    replication replicator 10.42.1.5/32    scram-sha-256

# Allow monitoring from Prometheus exporter
host    mydb      exporter     10.42.0.0/16    scram-sha-256

# SSL required for all remote connections
hostssl mydb      app_backend  10.42.0.0/16    scram-sha-256
hostnossl all     all          0.0.0.0/0       reject
```

Key rules:
- Always use `scram-sha-256` (never `md5` or `trust` for remote).
- Restrict to specific subnet (K8s pod CIDR).
- Require SSL (`hostssl`) for all non-local connections.
- Separate user for replication with minimal privileges.

### Audit Logging

```sql
-- Audit table
CREATE TABLE audit_log (
  id bigserial PRIMARY KEY,
  table_name text NOT NULL,
  operation text NOT NULL,
  row_id bigint,
  old_data jsonb,
  new_data jsonb,
  changed_by text DEFAULT current_setting('app.current_user_id', true),
  changed_at timestamptz DEFAULT now()
);

-- Generic audit trigger function
CREATE OR REPLACE FUNCTION audit_trigger() RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'INSERT' THEN
    INSERT INTO audit_log(table_name, operation, row_id, new_data)
    VALUES (TG_TABLE_NAME, 'INSERT', NEW.id, to_jsonb(NEW));
    RETURN NEW;
  ELSIF TG_OP = 'UPDATE' THEN
    INSERT INTO audit_log(table_name, operation, row_id, old_data, new_data)
    VALUES (TG_TABLE_NAME, 'UPDATE', NEW.id, to_jsonb(OLD), to_jsonb(NEW));
    RETURN NEW;
  ELSIF TG_OP = 'DELETE' THEN
    INSERT INTO audit_log(table_name, operation, row_id, old_data)
    VALUES (TG_TABLE_NAME, 'DELETE', OLD.id, to_jsonb(OLD));
    RETURN OLD;
  END IF;
  RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Attach to sensitive tables
CREATE TRIGGER audit_properties
  AFTER INSERT OR UPDATE OR DELETE ON properties
  FOR EACH ROW EXECUTE FUNCTION audit_trigger();

CREATE TRIGGER audit_payments
  AFTER INSERT OR UPDATE OR DELETE ON payments
  FOR EACH ROW EXECUTE FUNCTION audit_trigger();

-- Query audit trail
SELECT * FROM audit_log
WHERE table_name = 'payments'
  AND row_id = 42
ORDER BY changed_at DESC;

-- Partition audit_log by month for manageability
-- (use range partitioning on changed_at as shown in Section 3)
```

---

## Appendix: Quick Reference — Index Type Selection

| Use Case | Index Type | Example |
|----------|-----------|---------|
| Equality/range on scalar | B-tree (default) | `CREATE INDEX ON t(col)` |
| Multi-column prefix search | B-tree composite | `CREATE INDEX ON t(a, b, c)` |
| Array containment / JSONB | GIN | `USING GIN (col)` |
| ILIKE / fuzzy text search | GIN + pg_trgm | `USING GIN (col gin_trgm_ops)` |
| Full-text search | GIN / GiST | `USING GIN (to_tsvector(...))` |
| Geospatial / ranges | GiST | `USING GIST (col)` |
| Large naturally-ordered tables | BRIN | `USING BRIN (created_at)` |
| Vector similarity | HNSW / IVFFlat | `USING hnsw (col vector_cosine_ops)` |
| Unique partial filter | B-tree partial | `WHERE active = true` |
| Covering index (index-only scan) | B-tree INCLUDE | `(a) INCLUDE (b, c)` |

## Appendix: Essential postgresql.conf Settings

```ini
# Memory
shared_buffers = '4GB'            # 25% of RAM (max ~8GB on Linux)
effective_cache_size = '12GB'     # 75% of RAM (planner hint)
work_mem = '16MB'                 # per-sort/hash operation
maintenance_work_mem = '512MB'    # for VACUUM, CREATE INDEX
huge_pages = 'try'                # reduces TLB misses for large shared_buffers

# WAL
wal_level = 'replica'             # or 'logical' for logical replication
max_wal_size = '4GB'
min_wal_size = '1GB'
checkpoint_completion_target = 0.9

# Query Planner
random_page_cost = 1.1            # SSD (default 4.0 is for spinning disk)
effective_io_concurrency = 200    # SSD
default_statistics_target = 100   # increase to 500 for skewed distributions

# Connections
max_connections = 100             # keep low, use PgBouncer
idle_in_transaction_session_timeout = '10min'

# Logging
log_min_duration_statement = 500  # log queries > 500ms
log_checkpoints = on
log_lock_waits = on
log_temp_files = 0                # log all temp file usage

# Autovacuum
autovacuum_max_workers = 5
autovacuum_naptime = '15s'
autovacuum_vacuum_cost_limit = 1000

# Monitoring
shared_preload_libraries = 'pg_stat_statements,pg_cron'
track_io_timing = on
track_activity_query_size = 4096
```
