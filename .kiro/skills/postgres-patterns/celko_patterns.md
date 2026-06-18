# Celko Patterns — PostgreSQL Reference

Distilled from Joe Celko's *SQL for Smarties, 5th Edition*. Each pattern includes the technique, the anti-pattern it replaces, and when to reach for it.

---

## Data Modeling Patterns

### Calendar Table (Ch. 35)

**Pattern:** Pre-computed lookup table for all business-day logic. Julian business day numbering repeats the same integer for weekends/holidays, making date arithmetic a simple subtraction.

```sql
CREATE TABLE calendar (
    cal_date        DATE PRIMARY KEY,
    fiscal_year     SMALLINT NOT NULL,
    fiscal_month    SMALLINT NOT NULL,
    iso_week        TEXT NOT NULL,
    is_business_day BOOLEAN NOT NULL DEFAULT TRUE,
    julian_biz_day  INTEGER NOT NULL,
    holiday_name    TEXT
);

-- Business days between two dates:
SELECT c2.julian_biz_day - c1.julian_biz_day AS biz_days
  FROM calendar c1, calendar c2
 WHERE c1.cal_date = '2025-04-05'
   AND c2.cal_date = '2025-04-10';
```

**Anti-pattern:** Procedural `WHILE` loops counting weekdays, or `EXTRACT(DOW ...)` chains with hardcoded holiday arrays.

**Trigger:** Any requirement involving "business days," fiscal periods, settlement windows, or SLA deadlines.

---

### Series/Tally Table (Ch. 7)

**Pattern:** A pre-built table of integers (1..N) used to replace loops, generate rows, and unpivot columns. PostgreSQL's `generate_series()` is the native equivalent.

```sql
-- Unpivot monthly columns into rows:
SELECT s.salesman,
       (ARRAY['Jan','Feb','Mar','Apr','May','Jun',
              'Jul','Aug','Sep','Oct','Nov','Dec'])[g.n] AS month,
       (ARRAY[s.jan, s.feb, s.mar, s.apr, s.may, s.jun,
              s.jul, s.aug, s.sep, s.oct, s.nov, s.dec])[g.n] AS amount
  FROM annual_sales s
  CROSS JOIN generate_series(1, 12) AS g(n);
```

**Anti-pattern:** Application-side loops issuing one query per month/row, or 12 separate `UNION ALL` selects.

**Trigger:** Pivoting/unpivoting, filling date gaps, generating test data, or any "for each integer in range" operation.

---

### Avoid the One True Lookup Table (Ch. 9)

**Pattern:** One table per encoding, each with proper constraints and foreign keys.

```sql
CREATE TABLE property_status (
    status_code TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    allows_contracts BOOLEAN NOT NULL DEFAULT TRUE
);
-- FK from properties enforces domain:
ALTER TABLE properties
  ADD CONSTRAINT fk_status
  FOREIGN KEY (status_code) REFERENCES property_status(status_code);
```

**Anti-pattern:** A single `generic_lookups(type, code, value)` table that stuffs every enum into one bucket. Loses type-safety, prevents FK enforcement, and makes queries unreadable.

**Trigger:** You're tempted to create a "codes" or "lookups" table with a `type` discriminator column.

---

### ISO-11179 Naming & Attribute Suffixes (Ch. 9)

**Pattern:** Name columns as `<attribute>_<property>`: `contract_start_date`, `payment_amt`, `tenant_id`, `property_status_code`. Tables are collective nouns (`payments`, `tenants`). Never prefix with `tbl_`, `fk_`, or data types.

**Anti-pattern:** `tbl_payment.fk_tenant_id`, `int_amount`, `dt_created`.

**Trigger:** Designing any new table or reviewing naming consistency.

---

### NULL Avoidance via Domain Encoding (Ch. 14)

**Pattern:** Replace NULLs with meaningful domain values where semantics allow. Pair unavoidable NULLs with a reason column.

```sql
-- Instead of NULL hair_color for bald:
CHECK (hair_color IN ('black','brown','blonde','red','bald','unknown'))

-- When NULL is unavoidable, track why:
ALTER TABLE inspections
  ADD COLUMN result_missing_reason TEXT
  CHECK (result_missing_reason IN ('pending','not_applicable','sensor_error'));
```

**Anti-pattern:** Naked NULLs everywhere, forcing `COALESCE` chains and 3-valued logic bugs in WHERE clauses.

**Trigger:** Column allows NULL but you keep writing `IS NULL` / `COALESCE` workarounds in queries.

---

## Query Idioms

### Relational Division — "Has All Of" (Ch. 34)

**Pattern:** Find entities that match *every* item in a requirement set. Two approaches:

```sql
-- COUNT version (returns empty set if divisor is empty):
SELECT ps.pilot_name
  FROM pilot_skills ps
  JOIN hangar h ON ps.plane_name = h.plane_name
 GROUP BY ps.pilot_name
HAVING COUNT(*) = (SELECT COUNT(*) FROM hangar);

-- Double NOT EXISTS version (returns all if divisor is empty):
SELECT DISTINCT ps1.pilot_name
  FROM pilot_skills ps1
 WHERE NOT EXISTS (
    SELECT 1 FROM hangar h
     WHERE NOT EXISTS (
        SELECT 1 FROM pilot_skills ps2
         WHERE ps2.pilot_name = ps1.pilot_name
           AND ps2.plane_name = h.plane_name));
```

**Anti-pattern:** Application-side loops checking each requirement one-by-one, or dynamic SQL building N `AND EXISTS` clauses.

**Trigger:** "Find tenants who have paid ALL months," "properties that satisfy ALL filter criteria," any universal quantifier.

---

### NOT EXISTS over NOT IN (Ch. 18)

**Pattern:** Always prefer `NOT EXISTS` over `NOT IN` with subqueries — it handles NULLs correctly and uses indexes.

```sql
-- Correct and fast:
SELECT p.*
  FROM properties p
 WHERE NOT EXISTS (
    SELECT 1 FROM contracts c
     WHERE c.property_id = p.id AND c.estado = 'activo');

-- Broken if subquery ever returns NULL:
-- SELECT * FROM properties WHERE id NOT IN (SELECT property_id FROM contracts);
```

**Anti-pattern:** `NOT IN (SELECT nullable_column ...)` which returns empty when any NULL appears.

**Trigger:** Any exclusion/anti-join query. Always default to `NOT EXISTS`.

---

### Conditional Aggregation via CASE (Ch. 20)

**Pattern:** Pivot or categorize within a single GROUP BY pass using CASE inside aggregates.

```sql
SELECT property_id,
       SUM(CASE WHEN tipo = 'ingreso' THEN monto ELSE 0 END) AS total_income,
       SUM(CASE WHEN tipo = 'gasto'   THEN monto ELSE 0 END) AS total_expense,
       COUNT(*) FILTER (WHERE estado = 'pendiente') AS pending_count
  FROM transactions
 GROUP BY property_id;
```

**Anti-pattern:** Multiple self-joins or subqueries to get category totals, or doing pivot logic in application code.

**Trigger:** Cross-tab reports, dashboard summary cards, any "show X and Y side-by-side grouped by Z."

---

### Gaps and Islands (Ch. 31)

**Pattern:** Identify contiguous runs of the same state in sequential data using the difference-of-row-numbers technique.

```sql
WITH numbered AS (
    SELECT payment_id, on_time,
           ROW_NUMBER() OVER (ORDER BY payment_id) AS rn,
           ROW_NUMBER() OVER (PARTITION BY on_time ORDER BY payment_id) AS grp_rn
      FROM payments
)
SELECT on_time,
       MIN(payment_id) AS island_start,
       MAX(payment_id) AS island_end,
       COUNT(*) AS streak_length
  FROM numbered
 GROUP BY on_time, (rn - grp_rn)
 ORDER BY island_start;
```

**Anti-pattern:** Cursor-based loops comparing each row to the previous, or application-side iteration.

**Trigger:** "Find consecutive late payments," "detect gaps in occupancy," "group contiguous date ranges."

---

## Hierarchy Solutions

### Adjacency List + Recursive CTE (Ch. 28)

**Pattern:** Simple parent FK, queried with `WITH RECURSIVE`. Best for shallow trees with frequent inserts.

```sql
WITH RECURSIVE subordinates AS (
    SELECT emp_id, boss_id, 0 AS depth
      FROM personnel WHERE boss_id IS NULL
    UNION ALL
    SELECT p.emp_id, p.boss_id, s.depth + 1
      FROM personnel p
      JOIN subordinates s ON p.boss_id = s.emp_id
)
SELECT * FROM subordinates;
```

**Anti-pattern:** N self-joins hardcoded to a fixed depth, or application-side BFS loops.

**Trigger:** Org charts, category trees, comment threads — anywhere depth is unbounded but tree is narrow.

---

### Nested Sets (Ch. 28)

**Pattern:** Each node stores `lft` and `rgt` integers. All descendants of a node have `lft` BETWEEN parent's lft and rgt. Subtree queries become a single range scan.

```sql
-- All descendants of 'Chuck':
SELECT w.emp_id
  FROM personnel m, personnel w
 WHERE m.emp_id = 'Chuck'
   AND w.lft BETWEEN m.lft AND m.rgt;

-- Depth of any node:
SELECT p2.emp_id, COUNT(p1.emp_id) - 1 AS depth
  FROM personnel p1, personnel p2
 WHERE p2.lft BETWEEN p1.lft AND p1.rgt
 GROUP BY p2.emp_id, p2.lft
 ORDER BY p2.lft;
```

**Anti-pattern:** Recursive CTEs on read-heavy, write-rare trees (nested sets trade write cost for read speed).

**Trigger:** Read-dominated hierarchies (category browsing, report drill-down) where subtree aggregation is frequent.

---

### Materialized Path (PostgreSQL ltree)

**Pattern:** Store the full path as a `ltree` column. PostgreSQL's `ltree` extension gives `@>` (ancestor) and `<@` (descendant) operators with GiST index support.

```sql
CREATE EXTENSION IF NOT EXISTS ltree;
ALTER TABLE categories ADD COLUMN path ltree;
CREATE INDEX idx_cat_path ON categories USING GIST (path);

-- All subcategories of 'residential.apartments':
SELECT * FROM categories WHERE path <@ 'residential.apartments';
```

**Anti-pattern:** String `LIKE 'path/%'` queries without index support or recursive CTEs on every read.

**Trigger:** URL-like hierarchies, file trees, or category taxonomies where path display is needed anyway.

---

## Temporal Patterns

### Date Range Overlap Detection (Ch. 35)

**Pattern:** Two ranges `[s1, e1)` and `[s2, e2)` overlap iff `s1 < e2 AND s2 < e1`. PostgreSQL range types make this native.

```sql
-- Using range types:
ALTER TABLE contracts ADD COLUMN period DATERANGE
    GENERATED ALWAYS AS (daterange(start_date, end_date, '[)')) STORED;
CREATE INDEX idx_contract_period ON contracts USING GIST (period);

-- Overlapping contracts for same property:
SELECT a.id, b.id
  FROM contracts a, contracts b
 WHERE a.property_id = b.property_id
   AND a.id < b.id
   AND a.period && b.period;

-- Exclusion constraint prevents overlaps at write time:
ALTER TABLE contracts ADD CONSTRAINT no_overlap
    EXCLUDE USING GIST (property_id WITH =, period WITH &&);
```

**Anti-pattern:** `NOT (end1 <= start2 OR end2 <= start1)` predicates without index support, or application-side validation loops.

**Trigger:** Lease contracts, reservations, scheduling — any "no two active records may overlap for the same entity."

---

### Julian Business Day Arithmetic (Ch. 35)

**Pattern:** Assign a monotonically increasing integer to each business day (weekends/holidays repeat the previous day's number). Duration = simple subtraction.

**Trigger:** SLA calculations, payment due-date computation, settlement windows.

---

### State Machine Transitions (Ch. 9)

**Pattern:** Model allowed state transitions as a lookup table. Validate transitions with a CHECK or trigger.

```sql
CREATE TABLE contract_transitions (
    from_estado TEXT NOT NULL,
    to_estado   TEXT NOT NULL,
    PRIMARY KEY (from_estado, to_estado)
);
INSERT INTO contract_transitions VALUES
    ('borrador','activo'), ('activo','finalizado'), ('activo','cancelado');

-- Validate in trigger or application:
-- SELECT 1 FROM contract_transitions WHERE from_estado = OLD.estado AND to_estado = NEW.estado;
```

**Anti-pattern:** Free-form `UPDATE SET estado = 'anything'` with no constraint, leading to invalid transitions.

**Trigger:** Any entity with lifecycle states (contracts, maintenance tickets, payments).

---

## Queue & Concurrency Patterns

### SKIP LOCKED Queue (Ch. 29, PostgreSQL-adapted)

**Pattern:** Use `FOR UPDATE SKIP LOCKED` to dequeue work items without blocking other workers.

```sql
-- Dequeue one job:
WITH next_job AS (
    SELECT id FROM job_queue
     WHERE status = 'pending'
     ORDER BY priority, created_at
     LIMIT 1
       FOR UPDATE SKIP LOCKED
)
UPDATE job_queue SET status = 'processing', started_at = NOW()
 WHERE id = (SELECT id FROM next_job)
RETURNING *;
```

**Anti-pattern:** `SELECT MIN(id)` + separate `UPDATE` (race condition), or advisory locks with manual bookkeeping.

**Trigger:** Background job processing, notification dispatch, any FIFO/priority queue implemented in PostgreSQL.

---

### Priority Queue with Aging (Ch. 29)

**Pattern:** Jobs enter at a priority level; a scheduled sweep decrements priority for stale jobs so nothing starves.

```sql
UPDATE job_queue
   SET priority = priority - 1,
       updated_at = NOW()
 WHERE status = 'pending'
   AND updated_at + INTERVAL '10 minutes' <= NOW();
```

**Anti-pattern:** Static priorities that let low-priority work starve indefinitely.

**Trigger:** Multi-tenant task queues where fairness matters.

---

### Sequence Gap Recompaction (Ch. 29)

**Pattern:** Use `ROW_NUMBER()` to recompact a sparse ordering column after deletions.

```sql
WITH reordered AS (
    SELECT id, ROW_NUMBER() OVER (ORDER BY sort_order) AS new_order
      FROM queue_items WHERE queue_id = $1
)
UPDATE queue_items q SET sort_order = r.new_order
  FROM reordered r WHERE q.id = r.id;
```

**Anti-pattern:** Procedural loops decrementing each row, or leaving ever-growing gaps that break UI pagination.

**Trigger:** User-reorderable lists (drag-and-drop) that accumulate sparse numbering.

---

## Aggregation Patterns

### GROUPING SETS / ROLLUP / CUBE (Ch. 25)

**Pattern:** Produce subtotals and grand totals in a single pass instead of N union queries.

```sql
SELECT
    COALESCE(property_id::text, 'ALL') AS property,
    COALESCE(tipo, 'ALL') AS type,
    SUM(monto) AS total,
    GROUPING(property_id, tipo) AS grouping_level
  FROM transactions
 WHERE fecha BETWEEN '2025-01-01' AND '2025-12-31'
 GROUP BY ROLLUP (property_id, tipo);
```

**Anti-pattern:** Multiple queries with `UNION ALL` adding totals, or application-side subtotal computation.

**Trigger:** Financial reports needing subtotals by category and grand totals.

---

### Window Functions — Running Totals & Moving Averages (Ch. 25)

**Pattern:** Windowed aggregates compute per-row values without collapsing rows.

```sql
SELECT fecha, monto,
       SUM(monto) OVER (ORDER BY fecha
                        ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW)
           AS running_total,
       AVG(monto) OVER (ORDER BY fecha
                        ROWS BETWEEN 2 PRECEDING AND CURRENT ROW)
           AS moving_avg_3
  FROM payments
 WHERE contract_id = $1;
```

**Anti-pattern:** Correlated subqueries computing `SUM(... WHERE date <= current_row.date)` — O(n²).

**Trigger:** Running balances, moving averages, cumulative counts, lag/lead comparisons.

---

### Percentiles & Statistical Aggregates

**Pattern:** PostgreSQL provides `percentile_cont` and `percentile_disc` as ordered-set aggregates.

```sql
SELECT property_id,
       percentile_cont(0.5) WITHIN GROUP (ORDER BY monto) AS median_payment,
       percentile_cont(0.95) WITHIN GROUP (ORDER BY monto) AS p95_payment
  FROM payments
 GROUP BY property_id;
```

**Anti-pattern:** Sorting in application code to find the middle element, or `OFFSET n/2 LIMIT 1` hacks.

**Trigger:** Reporting dashboards showing median rent, P95 maintenance cost, etc.

---

### FILTER Clause (PostgreSQL extension)

**Pattern:** Cleaner conditional aggregation than CASE.

```sql
SELECT property_id,
       COUNT(*) FILTER (WHERE estado = 'pagado') AS paid,
       COUNT(*) FILTER (WHERE estado = 'pendiente') AS pending,
       SUM(monto) FILTER (WHERE fecha >= DATE_TRUNC('month', NOW())) AS mtd_total
  FROM payments
 GROUP BY property_id;
```

**Anti-pattern:** Verbose `SUM(CASE WHEN ... THEN 1 ELSE 0 END)` patterns.

**Trigger:** Any conditional count/sum — always prefer FILTER in PostgreSQL for readability.
