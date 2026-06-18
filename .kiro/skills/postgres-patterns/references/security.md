# Security — Roles, RLS, Audit, and Network Hardening

Self-contained guide for PostgreSQL security configuration.

---

## Role Hierarchy

Principle: base roles (no login) for permission grouping, login roles for actual connections.

```sql
CREATE ROLE app_readonly;
CREATE ROLE app_readwrite;
CREATE ROLE app_admin;

GRANT app_readonly TO app_readwrite;
GRANT app_readwrite TO app_admin;
```

### Permission grants

```sql
GRANT USAGE ON SCHEMA public TO app_readonly;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO app_readonly;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT SELECT ON TABLES TO app_readonly;

GRANT INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO app_readwrite;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT INSERT, UPDATE, DELETE ON TABLES TO app_readwrite;

GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO app_admin;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO app_admin;
```

### Login roles (actual service accounts)

```sql
CREATE ROLE app_backend LOGIN PASSWORD 'rotated-secret-here';
GRANT app_readwrite TO app_backend;

CREATE ROLE app_reporting LOGIN PASSWORD 'rotated-secret-here';
GRANT app_readonly TO app_reporting;
```

### Lock down public schema

```sql
REVOKE ALL ON SCHEMA public FROM PUBLIC;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
```

---

## Row-Level Security (RLS)

### Basic tenant isolation

```sql
ALTER TABLE properties ENABLE ROW LEVEL SECURITY;
ALTER TABLE properties FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON properties
  FOR ALL
  USING (tenant_id = current_setting('app.current_tenant')::bigint)
  WITH CHECK (tenant_id = current_setting('app.current_tenant')::bigint);
```

### Setting context per request

```sql
SET LOCAL app.current_tenant = '42';
SET LOCAL app.current_user_id = '7';
```

In Rust (sqlx):
```rust
sqlx::query("SET LOCAL app.current_tenant = $1")
    .bind(tenant_id.to_string())
    .execute(&mut *tx)
    .await?;
```

### Role-based policies

```sql
CREATE POLICY admin_full_access ON properties
  FOR ALL TO app_admin
  USING (true);

CREATE POLICY manager_own_properties ON properties
  FOR ALL TO app_readwrite
  USING (manager_id = current_setting('app.current_user_id')::bigint);
```

### RLS with connection pooling (PgBouncer)

With transaction-mode pooling, `SET LOCAL` works within a transaction:
```sql
BEGIN;
SET LOCAL app.current_tenant = '42';
SELECT * FROM properties;
COMMIT;
```

Session-level `SET` leaks across pooled connections — always use `SET LOCAL` inside transactions.

---

## Audit Logging

### Audit table

```sql
CREATE TABLE audit_log (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  table_name text NOT NULL,
  operation text NOT NULL,
  row_id bigint,
  old_data jsonb,
  new_data jsonb,
  changed_by text DEFAULT current_setting('app.current_user_id', true),
  changed_at timestamptz NOT NULL DEFAULT now()
);
```

### Generic audit trigger

```sql
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
```

### Attach to sensitive tables

```sql
CREATE TRIGGER audit_properties
  AFTER INSERT OR UPDATE OR DELETE ON properties
  FOR EACH ROW EXECUTE FUNCTION audit_trigger();

CREATE TRIGGER audit_payments
  AFTER INSERT OR UPDATE OR DELETE ON payments
  FOR EACH ROW EXECUTE FUNCTION audit_trigger();

CREATE TRIGGER audit_contracts
  AFTER INSERT OR UPDATE OR DELETE ON contracts
  FOR EACH ROW EXECUTE FUNCTION audit_trigger();
```

### Query audit trail

```sql
SELECT operation, old_data, new_data, changed_by, changed_at
FROM audit_log
WHERE table_name = 'payments' AND row_id = 42
ORDER BY changed_at DESC;
```

### Partition audit_log for manageability

```sql
CREATE TABLE audit_log (...) PARTITION BY RANGE (changed_at);

CREATE TABLE audit_log_2025 PARTITION OF audit_log
  FOR VALUES FROM ('2025-01-01') TO ('2026-01-01');
```

Purge old partitions:
```sql
SELECT cron.schedule('purge-audit', '0 4 * * 0',
  $$DELETE FROM audit_log WHERE changed_at < now() - interval '90 days'$$);
```

---

## Network Security (pg_hba.conf)

```
# TYPE  DATABASE  USER         ADDRESS         METHOD

# Local socket
local   all       postgres                     peer

# Reject all by default
host    all       all          0.0.0.0/0       reject

# App backend from K8s pod network
hostssl mydb      app_backend  10.42.0.0/16    scram-sha-256

# Replication from known replica
hostssl replication replicator 10.42.1.5/32    scram-sha-256

# Monitoring exporter
hostssl mydb      exporter     10.42.0.0/16    scram-sha-256

# Block all non-SSL remote connections
hostnossl all     all          0.0.0.0/0       reject
```

### Key rules

- Use `scram-sha-256` exclusively (never `md5` or `trust` for remote)
- Restrict to K8s pod CIDR (`10.42.0.0/16`)
- Require SSL (`hostssl`) for all non-local connections
- Separate user for replication with minimal privileges
- Order matters: first matching rule wins

---

## Column-Level Encryption

### Using pgcrypto

```sql
CREATE EXTENSION pgcrypto;

INSERT INTO tenants (nombre, cedula_encrypted)
VALUES ('Juan Perez',
  pgp_sym_encrypt('001-1234567-8', current_setting('app.encryption_key')));

SELECT nombre,
  pgp_sym_decrypt(cedula_encrypted::bytea, current_setting('app.encryption_key')) AS cedula
FROM tenants WHERE id = $1;
```

### Preferred pattern: application-layer encryption

Encrypt in the application, store as `bytea`. Database never sees plaintext, key never in database config.

```sql
ALTER TABLE tenants ADD COLUMN cedula_encrypted bytea;
```

---

## SSL Configuration

```ini
# postgresql.conf
ssl = on
ssl_cert_file = '/etc/ssl/certs/server.crt'
ssl_key_file = '/etc/ssl/private/server.key'
ssl_ca_file = '/etc/ssl/certs/ca.crt'
ssl_min_protocol_version = 'TLSv1.3'
```

---

## Security Checklist

- [ ] `REVOKE ALL ON SCHEMA public FROM PUBLIC` applied
- [ ] Login roles use `scram-sha-256` authentication
- [ ] RLS enabled and forced on multi-tenant tables
- [ ] Audit triggers on sensitive tables (payments, contracts, tenants)
- [ ] pg_hba.conf restricts to known CIDRs
- [ ] SSL required for all remote connections
- [ ] Superuser access restricted to local socket only
- [ ] `log_connections = on` and `log_disconnections = on` enabled
- [ ] Application uses least-privilege role (not superuser)
- [ ] Secrets rotated on schedule (not hardcoded)
