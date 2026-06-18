---
name: postgres-patterns
description: >
  PostgreSQL and SQL patterns: schema design, query optimization, indexing, EXPLAIN analysis,
  migrations, monitoring, security, NULLs, CTEs, window functions, aggregation, partitioning,
  normalization, Celko idioms, set-based thinking, relational division, troubleshooting slow
  queries, and database anti-pattern detection. Activate proactively for any SQL, PostgreSQL,
  or database question — even tangential ones about data modeling, query performance, or
  relational design.
metadata:
  origin: ECC
  credit:
    - Supabase team (MIT License) — performance patterns and Supabase-specific RLS
    - Joe Celko, SQL for Smarties 5th Edition — relational idioms, set-based thinking, hierarchy patterns
---

# PostgreSQL & SQL Patterns

## Decision Workflow

What are you doing right now? Follow the first match:

| Task | Reference | Tool to Reach For |
|------|-----------|-------------------|
| Designing a new table or schema | § Schema Design → `references/schema-design.md` | Context7 (PG docs), database-specialist agent |
| Query returns wrong results | § Query Writing → `celko_patterns.md` | grep.app (find examples) |
| Query is slow | § Performance → `references/performance.md` | Grafana MCP (pg_stat metrics) |
| Writing a migration | § Migrations → `references/migrations.md` | database-specialist agent |
| Need advanced SQL pattern | § Query Writing → `references/query-patterns.md` | grep.app, Context7 |
| Setting up monitoring/alerts | § Monitoring → `references/monitoring.md` | Grafana MCP (dashboards, alert rules) |
| Hardening security | § Security → `references/security.md` | database-specialist agent |
| Choosing an index type | § Quick Reference Tables below | Context7 (verify syntax for PG version) |
| Troubleshooting locks/connections/replication | `references/troubleshooting.md` | Grafana MCP (live metrics) |

---

## Quick Reference Tables

### Index Type Selection

| Use Case | Index Type | Syntax |
|----------|-----------|--------|
| Equality / range on scalar | B-tree | `CREATE INDEX ON t(col)` |
| Multi-column prefix | B-tree composite | `CREATE INDEX ON t(a, b, c)` |
| Array / JSONB containment | GIN | `USING GIN (col)` |
| ILIKE / fuzzy text | GIN + pg_trgm | `USING GIN (col gin_trgm_ops)` |
| Full-text search | GIN | `USING GIN (to_tsvector(...))` |
| Geospatial / ranges | GiST | `USING GIST (col)` |
| Append-only timestamps | BRIN | `USING BRIN (created_at)` |
| Vector similarity | HNSW | `USING hnsw (col vector_cosine_ops)` |
| Partial filter | B-tree | `WHERE estado = 'activo'` |
| Covering (index-only scan) | B-tree INCLUDE | `(a) INCLUDE (b, c)` |

### Data Type Selection

| Use Case | Type | Avoid |
|----------|------|-------|
| Primary keys | `bigint GENERATED ALWAYS AS IDENTITY` | `serial`, random UUID v4 |
| Strings | `text` | `varchar(255)` |
| Timestamps | `timestamptz` | `timestamp` (no timezone) |
| Money | `numeric(12,2)` | `float`, `money` |
| Booleans | `boolean` | `int`, `varchar` |
| IP addresses | `inet` | `text` |
| Date ranges | `daterange` | two separate columns |

### Essential Extensions

| Extension | Purpose | When to reach for it |
|-----------|---------|---------------------|
| `pg_stat_statements` | Query performance stats | Always (preload) |
| `pg_trgm` | Fuzzy/ILIKE search | User-facing text search |
| `pgcrypto` | Encryption primitives | Sensitive field encryption |
| `pg_cron` | Scheduled jobs | Maintenance, MV refresh |
| `ltree` | Hierarchical paths | Category trees |
| `pgvector` | Embedding similarity | AI/semantic search |
| `pg_repack` | Online table rebuild | Bloat removal without locks |

---

## Schema Design

**Naming (ISO-11179):** `<role_>attribute_property` — `contract_start_date`, `rent_amt`, `tenant_id`. Tables use plural nouns: `properties`, `payments`.

**Every table gets:**
```sql
CREATE TABLE contracts (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  -- domain columns here
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
```

**Foreign key ON DELETE rules:**
- `CASCADE` — owned children (contract → payments)
- `RESTRICT` — referenced entities (payment → tenant)
- `SET NULL` — optional soft references (property → manager)

**Normalization red flags:**
- Column always NULL for some row types → separate table
- Repeating groups (`phone1`, `phone2`) → child table
- Same fact in multiple places → single source + FK
- Derived values alongside source → view or generated column

**Constraint priority:** NOT NULL by default. CHECK for domain rules. UNIQUE for natural keys. EXCLUDE for non-overlapping ranges.

**Partitioning decision matrix:**

| Signal | Strategy |
|--------|----------|
| Time-series, queries always filter by date range | `RANGE` on date column |
| Multi-tenant, queries always filter by tenant | `LIST` on tenant_id |
| Even distribution needed, no natural range | `HASH` on id |
| Table < 10M rows | Skip partitioning |

→ Deep dive: `research_notes.md` §3 (Schema Design Workflow)

---

## Query Writing

**EXISTS over NOT IN** — always. `NOT IN` breaks silently when subquery returns NULL:
```sql
SELECT p.* FROM properties p
WHERE NOT EXISTS (
  SELECT 1 FROM contracts c
  WHERE c.property_id = p.id AND c.estado = 'activo'
);
```

**FILTER clause** — cleaner than CASE for conditional aggregation:
```sql
SELECT property_id,
  COUNT(*) FILTER (WHERE estado = 'pagado') AS paid,
  COUNT(*) FILTER (WHERE estado = 'pendiente') AS pending,
  SUM(monto) FILTER (WHERE fecha >= date_trunc('month', now())) AS mtd
FROM payments GROUP BY property_id;
```

**Window functions** — running totals without self-joins:
```sql
SELECT fecha, monto,
  SUM(monto) OVER (ORDER BY fecha ROWS UNBOUNDED PRECEDING) AS balance
FROM payments WHERE contract_id = $1;
```

**Relational division** — "properties with ALL required amenities":
```sql
SELECT p.id FROM properties p
JOIN property_amenities pa ON pa.property_id = p.id
JOIN required r ON r.amenity_id = pa.amenity_id
GROUP BY p.id
HAVING COUNT(*) = (SELECT COUNT(*) FROM required);
```

**SKIP LOCKED queue** — concurrent job processing:
```sql
WITH next AS (
  SELECT id FROM job_queue WHERE status = 'pending'
  ORDER BY priority, created_at LIMIT 1
  FOR UPDATE SKIP LOCKED
)
UPDATE job_queue SET status = 'processing', started_at = now()
WHERE id = (SELECT id FROM next) RETURNING *;
```

→ Deep dive: `celko_patterns.md`, `research_notes.md` §1

---

## Performance

**First diagnostic — always run EXPLAIN with all options:**
```sql
EXPLAIN (ANALYZE, BUFFERS, TIMING, FORMAT TEXT)
SELECT ...your slow query...;
```

**What to look for (in priority order):**

1. **Seq Scan on large table** → add index
2. **actual rows >> planned rows** → run `ANALYZE tablename;`
3. **Sort Method: external merge Disk** → increase `work_mem`
4. **Hash Batches > 1** → hash spilled to disk, increase `work_mem`
5. **Nested Loop on two large tables** → restructure or add index on inner
6. **Buffers: shared read >> shared hit** → cold cache or undersized `shared_buffers`

**Quick wins:**
```sql
SET random_page_cost = 1.1;     -- SSD (default 4.0 assumes spinning disk)
SET effective_io_concurrency = 200;  -- SSD
ANALYZE;                        -- refresh all table statistics
```

**Cache hit ratio (target: > 99%):**
```sql
SELECT round(sum(blks_hit)::numeric /
  nullif(sum(blks_hit) + sum(blks_read), 0), 4) AS ratio
FROM pg_stat_database;
```

→ Deep dive: `research_notes.md` §2 (Performance Benchmarking)

---

## Migrations

**Three rules that prevent outages:**

1. **Create indexes CONCURRENTLY** — never block writes
   ```sql
   CREATE INDEX CONCURRENTLY idx_properties_city ON properties(city);
   ```

2. **Add columns as nullable first** — then backfill in batches, then add constraint
   ```sql
   ALTER TABLE properties ADD COLUMN manager_id bigint;
   -- backfill in batches of 10k
   ALTER TABLE properties ADD CONSTRAINT chk_mgr
     CHECK (manager_id IS NOT NULL) NOT VALID;
   ALTER TABLE properties VALIDATE CONSTRAINT chk_mgr;
   ```

3. **Set lock_timeout** — fail fast instead of blocking the world
   ```sql
   SET lock_timeout = '5s';
   ```

**PG11+ shortcut:** non-volatile DEFAULT is instant (no rewrite):
```sql
ALTER TABLE properties ADD COLUMN is_featured boolean NOT NULL DEFAULT false;
```

→ Deep dive: `research_notes.md` §5 (Migration Safety Patterns)

---

## Troubleshooting

**The 5 diagnostic queries to run first:**

```sql
-- 1. Long-running queries
SELECT pid, now() - query_start AS duration, left(query, 80)
FROM pg_stat_activity WHERE state = 'active' AND pid <> pg_backend_pid()
ORDER BY duration DESC LIMIT 5;

-- 2. Lock waits
SELECT blocked.pid, blocking.pid AS blocker,
  now() - blocked.query_start AS wait_time
FROM pg_stat_activity blocked
JOIN pg_locks bl ON bl.pid = blocked.pid AND NOT bl.granted
JOIN pg_locks kl ON kl.locktype = bl.locktype
  AND kl.relation IS NOT DISTINCT FROM bl.relation
  AND kl.pid != bl.pid AND kl.granted
JOIN pg_stat_activity blocking ON blocking.pid = kl.pid;

-- 3. Connection saturation
SELECT count(*) AS total,
  count(*) FILTER (WHERE state = 'idle') AS idle,
  count(*) FILTER (WHERE state = 'idle in transaction') AS idle_in_txn,
  current_setting('max_connections')::int AS max
FROM pg_stat_activity;

-- 4. Top slow queries (requires pg_stat_statements)
SELECT calls, mean_exec_time::numeric(10,2) AS avg_ms,
  total_exec_time::numeric(12,2) AS total_ms, left(query, 80)
FROM pg_stat_statements ORDER BY total_exec_time DESC LIMIT 5;

-- 5. Tables needing vacuum
SELECT relname, n_dead_tup, last_autovacuum
FROM pg_stat_user_tables WHERE n_dead_tup > 1000
ORDER BY n_dead_tup DESC LIMIT 5;
```

→ Deep dive: `research_notes.md` §4 (Troubleshooting Workflow)

---

## Monitoring

**Key metrics and alert thresholds:**

| Metric | WARN | CRITICAL |
|--------|------|----------|
| Connection usage (% of max) | 70% | 85% |
| Cache hit ratio | < 0.995 | < 0.99 |
| Replication lag | > 10s | > 60s |
| Dead tuple ratio (per table) | > 5% | > 20% |
| Long-running transactions | > 5 min | > 30 min |
| Idle in transaction | > 5 min | > 10 min (auto-kill) |
| Lock wait duration | > 30s | > 2 min |

**Essential postgresql.conf for observability:**
```ini
shared_preload_libraries = 'pg_stat_statements,pg_cron'
track_io_timing = on
log_min_duration_statement = 500
log_lock_waits = on
log_checkpoints = on
```

→ Deep dive: `research_notes.md` §6 (Monitoring & Alerting)

---

## Security Checklist

**Role hierarchy (principle of least privilege):**
```sql
CREATE ROLE app_readonly;
CREATE ROLE app_readwrite;
GRANT app_readonly TO app_readwrite;

GRANT USAGE ON SCHEMA public TO app_readonly;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO app_readonly;
GRANT INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO app_readwrite;

REVOKE ALL ON SCHEMA public FROM PUBLIC;
```

**RLS for tenant isolation:**
```sql
ALTER TABLE properties ENABLE ROW LEVEL SECURITY;
ALTER TABLE properties FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON properties
  USING (tenant_id = current_setting('app.current_tenant')::bigint)
  WITH CHECK (tenant_id = current_setting('app.current_tenant')::bigint);
```

**Audit trigger (attach to sensitive tables):**
```sql
CREATE TRIGGER audit_payments
  AFTER INSERT OR UPDATE OR DELETE ON payments
  FOR EACH ROW EXECUTE FUNCTION audit_trigger();
```

**pg_hba.conf rules:**
- Use `scram-sha-256` exclusively (never `md5` or `trust` for remote)
- Restrict to K8s pod CIDR (`10.42.0.0/16`)
- Require SSL (`hostssl`) for all non-local connections

→ Deep dive: `research_notes.md` §7 (Security Hardening)

---

## Tools & Extensions

| Tool | One-liner | When |
|------|-----------|------|
| `pg_stat_statements` | Per-query execution stats | Always-on performance baseline |
| `pgbench` | Built-in load testing | Before/after config changes |
| `pg_repack` | Online bloat removal | Table bloat > 20% |
| `pg_cron` | In-database scheduler | MV refresh, partition maintenance |
| `pg_trgm` | Trigram similarity + ILIKE acceleration | Fuzzy user search |
| `pgvector` | HNSW/IVFFlat vector indexes | Semantic/AI search |
| `ltree` | Hierarchical path queries | Category trees, org charts |
| `pgcrypto` | Symmetric/asymmetric encryption | Field-level PII encryption |
| `PgBouncer` | Connection pooling proxy | Always (transaction mode) |
| `pgstattuple` | Physical storage inspection | Diagnosing bloat |

---

## Agent Tool Integration

The Decision Workflow table above maps each task to its tool. Below are multi-step procedures for common compound tasks.

### Workflow: Troubleshooting a Slow Query

1. **Grafana MCP** → `query_prometheus` for `pg_stat_statements_mean_exec_time` to identify the query
2. **Context7** → look up any unfamiliar PostgreSQL feature in the query plan
3. Read `references/performance.md` for EXPLAIN interpretation
4. **grep.app** → search for similar patterns and how others index them
5. Apply fix, verify with `EXPLAIN (ANALYZE, BUFFERS, TIMING)`

### Workflow: Designing a New Table

1. Read `references/schema-design.md` for naming and normalization rules
2. **Context7** → verify correct data types and constraint syntax for target PG version
3. **database-specialist** agent → delegate the full migration authoring
4. Read `references/migrations.md` for safety checklist before applying

### Workflow: Setting Up Monitoring

1. **Grafana MCP** → `list_datasources` to find the Prometheus datasource UID
2. **Grafana MCP** → `list_prometheus_metric_names` with regex `pg_` to discover available metrics
3. Read `references/monitoring.md` for alert thresholds and dashboard queries
4. **Grafana MCP** → `update_dashboard` to create panels, or `alerting_manage_rules` to create alerts

---

## Related Agents & Skills

- `database-specialist` agent — full database review workflow, migration authoring
- `celko_patterns.md` — complete Celko idiom reference (same directory)
- `research_notes.md` — raw research notes with extended examples
