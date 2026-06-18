# Migrations — Safe Schema Change Patterns

Self-contained guide for zero-downtime PostgreSQL schema migrations.

---

## Three Cardinal Rules

1. **Create indexes CONCURRENTLY** — `CREATE INDEX` without CONCURRENTLY takes a SHARE lock that blocks all writes for the duration.
2. **Add columns nullable first** — backfill in batches, then add constraint separately.
3. **Set lock_timeout** — fail fast instead of queuing behind a stuck DDL.

```sql
SET lock_timeout = '5s';
SET statement_timeout = '30s';
```

---

## Safe Column Addition

### Pattern: nullable → backfill → constraint

```sql
-- Step 1: instant (no rewrite, no lock beyond brief AccessExclusive)
ALTER TABLE properties ADD COLUMN manager_id bigint;

-- Step 2: backfill in batches (avoid long transactions)
UPDATE properties SET manager_id = 1
WHERE id BETWEEN 1 AND 10000 AND manager_id IS NULL;
UPDATE properties SET manager_id = 1
WHERE id BETWEEN 10001 AND 20000 AND manager_id IS NULL;

-- Step 3a: add NOT VALID constraint (instant, no full table scan)
ALTER TABLE properties
  ADD CONSTRAINT properties_manager_id_nn
  CHECK (manager_id IS NOT NULL) NOT VALID;

-- Step 3b: validate in background (doesn't block writes)
ALTER TABLE properties VALIDATE CONSTRAINT properties_manager_id_nn;
```

### PG11+ shortcut: non-volatile DEFAULT is instant

```sql
ALTER TABLE properties ADD COLUMN is_featured boolean NOT NULL DEFAULT false;
```

This stores the default in the catalog, not on disk. No table rewrite.

---

## Safe Index Creation

```sql
CREATE INDEX CONCURRENTLY idx_properties_city ON properties(city);
```

If it fails (marked INVALID):
```sql
DROP INDEX CONCURRENTLY idx_properties_city;
CREATE INDEX CONCURRENTLY idx_properties_city ON properties(city);
```

Check for invalid indexes:
```sql
SELECT indexrelname, indisvalid
FROM pg_stat_user_indexes
JOIN pg_index ON indexrelid = pg_stat_user_indexes.indexrelid
WHERE NOT indisvalid;
```

---

## Safe Enum Modifications

### Adding a value (safe)

```sql
ALTER TYPE contract_estado ADD VALUE 'suspendido' AFTER 'activo';
```

PG12+: transactional. PG < 12: cannot run inside a transaction block.

### Removing a value (requires type replacement)

```sql
-- 1. Create new type
CREATE TYPE contract_estado_v2 AS ENUM ('borrador','activo','finalizado','cancelado');

-- 2. Migrate column
ALTER TABLE contracts ALTER COLUMN estado TYPE contract_estado_v2
  USING estado::text::contract_estado_v2;

-- 3. Drop old type
DROP TYPE contract_estado;
ALTER TYPE contract_estado_v2 RENAME TO contract_estado;
```

---

## Zero-Downtime Column Rename

Application must handle both names during the transition window.

```sql
-- 1. Add new column
ALTER TABLE properties ADD COLUMN property_name text;

-- 2. Backfill
UPDATE properties SET property_name = nombre WHERE property_name IS NULL;

-- 3. Sync trigger (keeps both in sync during rolling deploy)
CREATE OR REPLACE FUNCTION sync_property_name() RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'INSERT' OR NEW.nombre IS DISTINCT FROM OLD.nombre THEN
    NEW.property_name := NEW.nombre;
  END IF;
  IF TG_OP = 'INSERT' OR NEW.property_name IS DISTINCT FROM OLD.property_name THEN
    NEW.nombre := NEW.property_name;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_sync_name BEFORE INSERT OR UPDATE ON properties
  FOR EACH ROW EXECUTE FUNCTION sync_property_name();

-- 4. Deploy app reading new column
-- 5. Drop old column and trigger
ALTER TABLE properties DROP COLUMN nombre;
DROP TRIGGER trg_sync_name ON properties;
DROP FUNCTION sync_property_name();
```

---

## Zero-Downtime Table Rename

```sql
ALTER TABLE old_name RENAME TO new_name;
CREATE VIEW old_name AS SELECT * FROM new_name;
-- Deploy app to use new_name
-- Drop view when no traffic uses old name
DROP VIEW old_name;
```

---

## Column Type Change (Large Tables)

Never use `ALTER COLUMN TYPE` on large tables — it rewrites the entire table under AccessExclusiveLock.

### Pattern: add → backfill → swap

```sql
-- 1. Add new column with target type
ALTER TABLE properties ADD COLUMN price_new numeric(12,2);

-- 2. Backfill in batches
UPDATE properties SET price_new = price::numeric(12,2)
WHERE id BETWEEN 1 AND 10000;

-- 3. Add sync trigger during transition
-- 4. Swap columns via rename
ALTER TABLE properties RENAME COLUMN price TO price_old;
ALTER TABLE properties RENAME COLUMN price_new TO price;

-- 5. Drop old column after deploy
ALTER TABLE properties DROP COLUMN price_old;
```

---

## Foreign Key Addition

Adding a FK validates all existing rows (full table scan under lock).

### Pattern: NOT VALID then VALIDATE

```sql
-- Instant (no validation scan)
ALTER TABLE payments
  ADD CONSTRAINT fk_payments_contract
  FOREIGN KEY (contract_id) REFERENCES contracts(id) NOT VALID;

-- Validate without blocking writes (ShareUpdateExclusiveLock)
ALTER TABLE payments VALIDATE CONSTRAINT fk_payments_contract;
```

---

## Batch Backfill Pattern

For large tables, avoid one massive UPDATE (holds locks, generates WAL, blocks autovacuum):

```sql
DO $$
DECLARE
  batch_size int := 10000;
  affected int;
BEGIN
  LOOP
    UPDATE properties SET manager_id = 1
    WHERE id IN (
      SELECT id FROM properties
      WHERE manager_id IS NULL
      LIMIT batch_size
      FOR UPDATE SKIP LOCKED
    );
    GET DIAGNOSTICS affected = ROW_COUNT;
    EXIT WHEN affected = 0;
    COMMIT;
    PERFORM pg_sleep(0.1);
  END LOOP;
END $$;
```

---

## Partition Maintenance

### Adding new partitions (before they're needed)

```sql
CREATE TABLE payments_2026 PARTITION OF payments
  FOR VALUES FROM ('2026-01-01') TO ('2027-01-01');
```

Schedule with pg_cron to auto-create next quarter's partition:
```sql
SELECT cron.schedule('create-next-partition', '0 0 1 */3 *',
  $$ SELECT create_next_payment_partition() $$);
```

### Detaching old partitions (for archival)

```sql
ALTER TABLE payments DETACH PARTITION payments_2022 CONCURRENTLY;
-- Now payments_2022 is a standalone table — archive or drop
```

---

## Migration Checklist

Before applying any migration to production:

- [ ] Set `lock_timeout = '5s'` at top of migration
- [ ] All `CREATE INDEX` uses `CONCURRENTLY`
- [ ] No `ALTER COLUMN TYPE` on tables > 100k rows (use add/backfill/swap)
- [ ] No `ALTER COLUMN SET NOT NULL` without prior NOT VALID CHECK
- [ ] FK additions use `NOT VALID` + separate `VALIDATE`
- [ ] Backfills use batched updates with `pg_sleep` between batches
- [ ] Tested on staging with production-scale data
- [ ] Rollback plan documented (what to DROP if it fails)
