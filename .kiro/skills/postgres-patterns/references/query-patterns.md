# Query Patterns — Celko Idioms and Advanced SQL

Self-contained reference for advanced PostgreSQL query techniques. For the full Celko pattern catalog, see `celko_patterns.md` in the parent directory.

---

## Set-Based Thinking

Replace procedural loops with declarative expressions:

| Procedural Habit | Set-Based Replacement |
|-----------------|----------------------|
| Cursor loop updating rows | Single UPDATE with JOIN or subquery |
| Temp table + loop for sequences | `generate_series()` |
| IF/ELSE per row | CASE expression |
| Iterative accumulation | Window function (`SUM OVER`) |
| App-side counting | `COUNT + GROUP BY` |
| N queries in a loop | Single query with JOIN or CTE |

---

## EXISTS vs IN vs JOIN

| Pattern | When to Use |
|---------|-------------|
| `EXISTS` | Correlated check, stops at first match, NULL-safe |
| `IN` | Small known list, or subquery guaranteed non-NULL |
| `NOT EXISTS` | Anti-join (always prefer over `NOT IN`) |
| `JOIN` | Need columns from both tables in output |

### NOT EXISTS over NOT IN (critical)

`NOT IN` returns empty set when subquery contains any NULL:

```sql
SELECT p.* FROM properties p
WHERE NOT EXISTS (
  SELECT 1 FROM contracts c
  WHERE c.property_id = p.id AND c.estado = 'activo'
);
```

---

## Relational Division — "Has ALL Of"

Find entities matching every item in a requirement set:

### COUNT version (empty divisor → empty result)

```sql
SELECT t.id, t.nombre
FROM tenants t
JOIN tenant_documents td ON td.tenant_id = t.id
JOIN required_documents rd ON rd.doc_type = td.doc_type
GROUP BY t.id, t.nombre
HAVING COUNT(*) = (SELECT COUNT(*) FROM required_documents);
```

### Double NOT EXISTS (empty divisor → all entities)

```sql
SELECT DISTINCT t.id, t.nombre
FROM tenants t
WHERE NOT EXISTS (
  SELECT 1 FROM required_documents rd
  WHERE NOT EXISTS (
    SELECT 1 FROM tenant_documents td
    WHERE td.tenant_id = t.id AND td.doc_type = rd.doc_type
  )
);
```

---

## Conditional Aggregation

### FILTER clause (PostgreSQL-specific, cleaner than CASE)

```sql
SELECT property_id,
  COUNT(*) FILTER (WHERE estado = 'pagado') AS paid,
  COUNT(*) FILTER (WHERE estado = 'pendiente') AS pending,
  SUM(monto) FILTER (WHERE fecha >= date_trunc('month', now())) AS mtd_total
FROM payments
GROUP BY property_id;
```

### CASE-based pivot

```sql
SELECT property_id,
  SUM(CASE WHEN tipo = 'ingreso' THEN monto ELSE 0 END) AS income,
  SUM(CASE WHEN tipo = 'gasto' THEN monto ELSE 0 END) AS expense
FROM transactions
GROUP BY property_id;
```

---

## Window Functions

### Running total

```sql
SELECT fecha, monto,
  SUM(monto) OVER (ORDER BY fecha ROWS UNBOUNDED PRECEDING) AS balance
FROM payments WHERE contract_id = $1;
```

### Moving average (3-period)

```sql
SELECT fecha, monto,
  AVG(monto) OVER (ORDER BY fecha ROWS BETWEEN 2 PRECEDING AND CURRENT ROW) AS avg_3
FROM payments WHERE contract_id = $1;
```

### Lag/Lead comparison (month-over-month)

```sql
SELECT month, revenue,
  revenue - LAG(revenue) OVER (ORDER BY month) AS mom_change,
  round((revenue - LAG(revenue) OVER (ORDER BY month))::numeric /
    nullif(LAG(revenue) OVER (ORDER BY month), 0) * 100, 1) AS mom_pct
FROM monthly_revenue;
```

### Rank within groups

```sql
SELECT property_id, tenant_id, monto,
  RANK() OVER (PARTITION BY property_id ORDER BY monto DESC) AS rank
FROM payments
WHERE fecha >= date_trunc('year', now());
```

### Percentiles

```sql
SELECT property_id,
  percentile_cont(0.5) WITHIN GROUP (ORDER BY monto) AS median,
  percentile_cont(0.95) WITHIN GROUP (ORDER BY monto) AS p95
FROM payments
GROUP BY property_id;
```

---

## Gaps and Islands

Identify contiguous runs of the same state:

```sql
WITH numbered AS (
  SELECT payment_id, on_time,
    ROW_NUMBER() OVER (ORDER BY payment_id) AS rn,
    ROW_NUMBER() OVER (PARTITION BY on_time ORDER BY payment_id) AS grp_rn
  FROM payments WHERE contract_id = $1
)
SELECT on_time,
  MIN(payment_id) AS island_start,
  MAX(payment_id) AS island_end,
  COUNT(*) AS streak_length
FROM numbered
GROUP BY on_time, (rn - grp_rn)
ORDER BY island_start;
```

### Date gaps detection

```sql
SELECT prev_end + 1 AS gap_start, next_start - 1 AS gap_end
FROM (
  SELECT end_date AS prev_end,
    LEAD(start_date) OVER (ORDER BY start_date) AS next_start
  FROM contracts WHERE property_id = $1
) sq
WHERE next_start > prev_end + 1;
```

---

## GROUPING SETS / ROLLUP / CUBE

Subtotals and grand totals in a single pass:

```sql
SELECT
  COALESCE(property_id::text, 'ALL') AS property,
  COALESCE(tipo, 'ALL') AS type,
  SUM(monto) AS total,
  GROUPING(property_id, tipo) AS level
FROM transactions
WHERE fecha BETWEEN '2025-01-01' AND '2025-12-31'
GROUP BY ROLLUP (property_id, tipo);
```

---

## Temporal Patterns

### Overlap detection

```sql
SELECT a.id, b.id
FROM contracts a, contracts b
WHERE a.property_id = b.property_id
  AND a.id < b.id
  AND a.start_date < b.end_date
  AND b.start_date < a.end_date;
```

### With range types and exclusion constraint

```sql
ALTER TABLE contracts ADD COLUMN period daterange
  GENERATED ALWAYS AS (daterange(start_date, end_date, '[)')) STORED;

CREATE INDEX idx_contract_period ON contracts USING GIST (period);

ALTER TABLE contracts ADD CONSTRAINT no_overlap
  EXCLUDE USING GIST (property_id WITH =, period WITH &&);
```

### Calendar/business day table

```sql
CREATE TABLE calendar (
  cal_date date PRIMARY KEY,
  is_business_day boolean NOT NULL DEFAULT true,
  julian_biz_day integer NOT NULL
);

SELECT c2.julian_biz_day - c1.julian_biz_day AS biz_days
FROM calendar c1, calendar c2
WHERE c1.cal_date = '2025-04-05' AND c2.cal_date = '2025-04-10';
```

---

## Hierarchy Patterns

### Adjacency list + recursive CTE

```sql
WITH RECURSIVE tree AS (
  SELECT id, parent_id, nombre, 0 AS depth
  FROM categories WHERE parent_id IS NULL
  UNION ALL
  SELECT c.id, c.parent_id, c.nombre, t.depth + 1
  FROM categories c JOIN tree t ON c.parent_id = t.id
)
SELECT * FROM tree ORDER BY depth, nombre;
```

### Materialized path (ltree)

```sql
CREATE EXTENSION ltree;
ALTER TABLE categories ADD COLUMN path ltree;
CREATE INDEX idx_cat_path ON categories USING GIST (path);

SELECT * FROM categories WHERE path <@ 'residential.apartments';
```

### Nested sets (read-optimized)

```sql
SELECT child.*
FROM categories parent, categories child
WHERE parent.id = $1
  AND child.lft BETWEEN parent.lft AND parent.rgt;
```

---

## Queue Patterns

### SKIP LOCKED (concurrent job processing)

```sql
WITH next AS (
  SELECT id FROM job_queue
  WHERE status = 'pending'
  ORDER BY priority, created_at LIMIT 1
  FOR UPDATE SKIP LOCKED
)
UPDATE job_queue SET status = 'processing', started_at = now()
WHERE id = (SELECT id FROM next) RETURNING *;
```

### Priority aging (prevent starvation)

```sql
UPDATE job_queue SET priority = priority - 1, updated_at = now()
WHERE status = 'pending'
  AND updated_at < now() - interval '10 minutes';
```

---

## State Machine Transitions

```sql
CREATE TABLE contract_transitions (
  from_estado text NOT NULL,
  to_estado text NOT NULL,
  PRIMARY KEY (from_estado, to_estado)
);

INSERT INTO contract_transitions VALUES
  ('borrador','activo'), ('activo','finalizado'), ('activo','cancelado');
```

Validate in application or trigger:
```sql
SELECT EXISTS (
  SELECT 1 FROM contract_transitions
  WHERE from_estado = $old_estado AND to_estado = $new_estado
) AS valid_transition;
```

---

## NULL Handling

### Three-valued logic traps

```sql
SELECT * FROM properties WHERE status <> 'vendido';
```
This silently excludes rows where `status IS NULL`.

Fix:
```sql
SELECT * FROM properties WHERE status IS DISTINCT FROM 'vendido';
```

### COALESCE for fallback

```sql
SELECT COALESCE(preferred_name, legal_name, 'Desconocido') AS display_name
FROM tenants;
```

### NULLIF to prevent division by zero

```sql
SELECT total_amt / NULLIF(unit_count, 0) AS unit_price FROM line_items;
```

---

## Generate Series (Replace Loops)

### Fill date gaps

```sql
SELECT d::date, COALESCE(p.monto, 0) AS monto
FROM generate_series('2025-01-01'::date, '2025-12-31'::date, '1 month') AS d
LEFT JOIN payments p ON date_trunc('month', p.fecha) = d
  AND p.contract_id = $1;
```

### Generate test data

```sql
INSERT INTO properties (nombre, ciudad, rent_amt)
SELECT
  'Propiedad ' || n,
  (ARRAY['Santo Domingo','Santiago','La Romana'])[1 + (n % 3)],
  (random() * 50000 + 10000)::numeric(12,2)
FROM generate_series(1, 1000) AS n;
```

---

## UPSERT (INSERT ON CONFLICT)

```sql
INSERT INTO settings (user_id, key, value)
VALUES ($1, $2, $3)
ON CONFLICT (user_id, key)
DO UPDATE SET value = EXCLUDED.value, updated_at = now();
```

---

## Cursor Pagination (Keyset)

```sql
SELECT * FROM properties
WHERE id > $last_seen_id
ORDER BY id
LIMIT 20;
```

Faster than OFFSET for deep pages. Requires stable sort column.
