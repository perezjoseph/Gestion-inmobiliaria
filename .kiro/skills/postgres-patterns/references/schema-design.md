# Schema Design — Workflow, Patterns, Normalization

Self-contained guide for designing PostgreSQL schemas from requirements to implementation.

---

## Design Workflow

1. **Requirements** — entities, relationships, access patterns, write/read ratio, data volumes, retention
2. **Entity mapping** — one table per domain concept, singular naming, standard columns
3. **Relationships** — foreign keys with appropriate ON DELETE behavior
4. **Normalization** — start at 3NF, denormalize only when measured performance demands it
5. **Constraints** — NOT NULL default, CHECK domains, UNIQUE natural keys, EXCLUDE ranges
6. **Indexes** — add after query patterns are known (not before)

---

## Standard Table Template

```sql
CREATE TABLE contracts (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  property_id bigint NOT NULL REFERENCES properties(id) ON DELETE RESTRICT,
  tenant_id bigint NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
  start_date date NOT NULL,
  end_date date NOT NULL,
  rent_amt numeric(12,2) NOT NULL,
  estado text NOT NULL DEFAULT 'borrador'
    CHECK (estado IN ('borrador','activo','finalizado','cancelado')),
  period daterange GENERATED ALWAYS AS (daterange(start_date, end_date, '[)')) STORED,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT valid_dates CHECK (start_date < end_date),
  CONSTRAINT no_overlap EXCLUDE USING GIST (property_id WITH =, period WITH &&)
);
```

---

## Naming Convention (ISO-11179)

| Element | Convention | Example |
|---------|-----------|---------|
| Table | Plural noun | `properties`, `payments` |
| Column | `<role_>attribute_property` | `contract_start_date`, `billing_address_id` |
| PK | `id` | `id bigint GENERATED ALWAYS AS IDENTITY` |
| FK | `<referenced_table_singular>_id` | `tenant_id`, `property_id` |
| Index | `idx_<table>_<columns>` | `idx_payments_contract_fecha` |
| Constraint | `<table>_<description>` | `contracts_valid_dates` |

Suffixes: `_id` (identifier), `_date` (temporal), `_amt` (money), `_code` (enum/standard), `_name` (display), `_status`/`_estado` (state machine).

---

## Foreign Key ON DELETE Rules

| Relationship | Rule | Example |
|-------------|------|---------|
| Parent owns children | `CASCADE` | contract → payments |
| Referenced entity | `RESTRICT` | payment → tenant (can't delete tenant with payments) |
| Optional soft reference | `SET NULL` | property → optional manager |

---

## Normalization Checklist

### First Normal Form (1NF)
- No repeating groups (`phone1`, `phone2` → child table)
- Each column holds atomic values
- Each row uniquely identifiable

### Second Normal Form (2NF)
- Every non-key column depends on the entire primary key
- No partial dependencies in composite keys

### Third Normal Form (3NF)
- No transitive dependencies (A → B → C means C belongs in B's table)
- Each fact stored exactly once

### Red flags to split

- Column always NULL for certain row types → separate table
- Same prefix on multiple columns (`emergency_contact_name`, `emergency_contact_phone`) → child table
- Derived/computed values alongside source → view or generated column
- Same fact in multiple rows → normalize into reference table + FK

---

## Denormalization Patterns

### Materialized Views (pre-computed aggregates)

```sql
CREATE MATERIALIZED VIEW mv_property_financials AS
SELECT
  p.id AS property_id, p.nombre,
  count(DISTINCT c.id) AS active_contracts,
  coalesce(sum(pay.monto), 0) AS total_income
FROM properties p
LEFT JOIN contracts c ON c.property_id = p.id AND c.estado = 'activo'
LEFT JOIN payments pay ON pay.contract_id = c.id
GROUP BY p.id, p.nombre;

CREATE UNIQUE INDEX ON mv_property_financials(property_id);
REFRESH MATERIALIZED VIEW CONCURRENTLY mv_property_financials;
```

Schedule refresh with pg_cron:
```sql
SELECT cron.schedule('refresh-financials', '0 * * * *',
  'REFRESH MATERIALIZED VIEW CONCURRENTLY mv_property_financials');
```

### JSONB for semi-structured attributes

```sql
ALTER TABLE properties ADD COLUMN metadata jsonb NOT NULL DEFAULT '{}';
CREATE INDEX idx_properties_metadata ON properties USING GIN (metadata);

SELECT * FROM properties WHERE metadata @> '{"amenities": ["pool"]}';
```

Use JSONB when: attributes vary per row, schema changes frequently, no relational queries needed on the data.

---

## Partitioning

### When to partition

| Signal | Action |
|--------|--------|
| Table > 100M rows | Partition |
| Queries always filter by date range | RANGE on date |
| Queries always filter by tenant/region | LIST on discriminator |
| Need even distribution, no natural key | HASH |
| Table < 10M rows | Skip partitioning |

### Range partitioning (time-series)

```sql
CREATE TABLE payments (
  id bigint GENERATED ALWAYS AS IDENTITY,
  contract_id bigint NOT NULL,
  monto numeric(12,2) NOT NULL,
  fecha_pago date NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
) PARTITION BY RANGE (fecha_pago);

CREATE TABLE payments_2024 PARTITION OF payments
  FOR VALUES FROM ('2024-01-01') TO ('2025-01-01');
CREATE TABLE payments_2025 PARTITION OF payments
  FOR VALUES FROM ('2025-01-01') TO ('2026-01-01');
```

### List partitioning (by category)

```sql
CREATE TABLE properties (
  id bigint GENERATED ALWAYS AS IDENTITY,
  ciudad text NOT NULL, ...
) PARTITION BY LIST (ciudad);

CREATE TABLE properties_santo_domingo PARTITION OF properties
  FOR VALUES IN ('Santo Domingo');
CREATE TABLE properties_santiago PARTITION OF properties
  FOR VALUES IN ('Santiago');
CREATE TABLE properties_other PARTITION OF properties DEFAULT;
```

### Hash partitioning (even distribution)

```sql
CREATE TABLE audit_log (
  id bigint GENERATED ALWAYS AS IDENTITY,
  tenant_id bigint NOT NULL, ...
) PARTITION BY HASH (tenant_id);

CREATE TABLE audit_log_0 PARTITION OF audit_log
  FOR VALUES WITH (MODULUS 4, REMAINDER 0);
CREATE TABLE audit_log_1 PARTITION OF audit_log
  FOR VALUES WITH (MODULUS 4, REMAINDER 1);
CREATE TABLE audit_log_2 PARTITION OF audit_log
  FOR VALUES WITH (MODULUS 4, REMAINDER 2);
CREATE TABLE audit_log_3 PARTITION OF audit_log
  FOR VALUES WITH (MODULUS 4, REMAINDER 3);
```

---

## Multi-Tenant Patterns

### Pattern 1: tenant_id column (recommended)

```sql
ALTER TABLE properties ADD COLUMN tenant_id bigint NOT NULL;
CREATE INDEX idx_properties_tenant ON properties(tenant_id);
```
Pros: simple, single schema, easy migrations.
Cons: must always include WHERE clause, risk of data leak.

### Pattern 2: RLS enforcement on top of tenant_id

```sql
ALTER TABLE properties ENABLE ROW LEVEL SECURITY;
ALTER TABLE properties FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON properties
  USING (tenant_id = current_setting('app.current_tenant')::bigint)
  WITH CHECK (tenant_id = current_setting('app.current_tenant')::bigint);

SET LOCAL app.current_tenant = '42';
```
Pros: database enforces isolation, impossible to forget WHERE.
Cons: slightly slower, complexity with connection pooling.

### Pattern 3: schema-per-tenant

```sql
CREATE SCHEMA tenant_42;
SET search_path = tenant_42, public;
```
Pros: strongest isolation, per-tenant backup/restore.
Cons: migration complexity at scale (> 100 tenants).

---

## Constraint Patterns

### EXCLUDE for non-overlapping ranges

```sql
ALTER TABLE contracts ADD CONSTRAINT no_overlap
  EXCLUDE USING GIST (property_id WITH =, period WITH &&);
```

### CHECK for domain validation

```sql
ALTER TABLE payments ADD CONSTRAINT positive_amount
  CHECK (monto > 0);

ALTER TABLE properties ADD CONSTRAINT valid_ciudad
  CHECK (ciudad IN ('Santo Domingo','Santiago','La Romana','Puerto Plata'));
```

### Generated columns (computed values)

```sql
ALTER TABLE contracts ADD COLUMN duration_days int
  GENERATED ALWAYS AS (end_date - start_date) STORED;
```

---

## Anti-Pattern Detection

```sql
SELECT c.conrelid::regclass AS table_name, a.attname AS fk_column
FROM pg_constraint c
JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = ANY(c.conkey)
WHERE c.contype = 'f'
  AND NOT EXISTS (
    SELECT 1 FROM pg_index i
    WHERE i.indrelid = c.conrelid AND a.attnum = ANY(i.indkey)
  );
```

This finds foreign key columns without indexes — every FK used in JOINs should have a supporting index.
