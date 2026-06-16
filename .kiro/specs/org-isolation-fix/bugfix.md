# Bugfix Requirements Document

## Introduction

Several backend endpoints lack `organizacion_id` filtering, allowing authenticated users from one organization to access or mutate data belonging to another organization by knowing/guessing UUIDs. This is an IDOR (Insecure Direct Object Reference) vulnerability affecting the `indexacion`, `ipi`, and `chatbot` modules. Two additional design-level concerns (`configuracion` global table, background jobs manual trigger) are documented for confirmation.

## Bug Analysis

### Current Behavior (Defect)

1.1 WHEN a user from Org A calls `GET /api/v1/indexacion/contratos/{contrato_id}/propuesta` with a `contrato_id` belonging to Org B THEN the system returns the renewal proposal for Org B's contrato without rejecting the request

1.2 WHEN a user from Org A calls `POST /api/v1/indexacion/contratos/{contrato_id}/aprobar` with a `contrato_id` belonging to Org B THEN the system creates a new contrato linked to Org B's original contrato without verifying ownership

1.3 WHEN a user from Org A calls `GET /api/v1/ipi/propiedades/{propiedad_id}/copropietarios` with a `propiedad_id` belonging to Org B THEN the system returns copropietarios of Org B's propiedad

1.4 WHEN a user from Org A calls `POST /api/v1/ipi/copropietarios` with a `propiedad_id` belonging to Org B THEN the system creates a copropietario record linked to Org B's propiedad despite `org_id` being stored

1.5 WHEN a user from Org A calls `POST /api/v1/chatbot/extractions/{extraction_id}/confirm` with an `extraction_id` belonging to Org B THEN the system confirms Org B's receipt extraction

1.6 WHEN a user from Org A calls `POST /api/v1/chatbot/extractions/{extraction_id}/reject` with an `extraction_id` belonging to Org B THEN the system rejects Org B's receipt extraction

1.7 WHEN an admin from Org A updates `tasa_cambio_dop_usd` or `recargo_porcentaje_defecto` THEN the system changes the value globally for ALL organizations because the `configuracion` table has no `organizacion_id` column

1.8 WHEN an admin from Org A reads `tasa_cambio_dop_usd` THEN the system returns a single global value that may have been set by an admin from a different organization

### Expected Behavior (Correct)

2.1 WHEN a user from Org A calls `GET /api/v1/indexacion/contratos/{contrato_id}/propuesta` with a `contrato_id` NOT belonging to Org A THEN the system SHALL return a 404 Not Found error

2.2 WHEN a user from Org A calls `POST /api/v1/indexacion/contratos/{contrato_id}/aprobar` with a `contrato_id` NOT belonging to Org A THEN the system SHALL return a 404 Not Found error and SHALL NOT create any new contrato

2.3 WHEN a user from Org A calls `GET /api/v1/ipi/propiedades/{propiedad_id}/copropietarios` with a `propiedad_id` NOT belonging to Org A THEN the system SHALL return a 404 Not Found error

2.4 WHEN a user from Org A calls `POST /api/v1/ipi/copropietarios` with a `propiedad_id` NOT belonging to Org A THEN the system SHALL return a 404 Not Found error and SHALL NOT create a copropietario record

2.5 WHEN a user from Org A calls `POST /api/v1/chatbot/extractions/{extraction_id}/confirm` with an `extraction_id` NOT belonging to Org A THEN the system SHALL return a 404 Not Found error and SHALL NOT modify the extraction status

2.6 WHEN a user from Org A calls `POST /api/v1/chatbot/extractions/{extraction_id}/reject` with an `extraction_id` NOT belonging to Org A THEN the system SHALL return a 404 Not Found error and SHALL NOT modify the extraction status

2.7 WHEN an admin from Org A updates `tasa_cambio_dop_usd` or `recargo_porcentaje_defecto` THEN the system SHALL update the value only for Org A, leaving other organizations' values unchanged

2.8 WHEN an admin from Org A reads `tasa_cambio_dop_usd` THEN the system SHALL return the value configured specifically for Org A

### Unchanged Behavior (Regression Prevention)

3.1 WHEN a user from Org A calls any indexacion endpoint with a `contrato_id` belonging to Org A THEN the system SHALL CONTINUE TO process the request normally (calculate proposal, approve renewal)

3.2 WHEN a user from Org A calls `GET /api/v1/ipi/propiedades/{propiedad_id}/copropietarios` with a `propiedad_id` belonging to Org A THEN the system SHALL CONTINUE TO return the copropietarios list

3.3 WHEN a user from Org A calls `POST /api/v1/ipi/copropietarios` with a `propiedad_id` belonging to Org A THEN the system SHALL CONTINUE TO create the copropietario record

3.4 WHEN a user from Org A calls chatbot confirm/reject with an `extraction_id` belonging to Org A THEN the system SHALL CONTINUE TO process the confirmation or rejection normally

3.5 WHEN background jobs run on their scheduled cron THEN the system SHALL CONTINUE TO operate cross-org (marking overdue pagos across all orgs is correct cron behavior)

3.6 WHEN any endpoint that already correctly filters by `organizacion_id` is called THEN the system SHALL CONTINUE TO behave identically (no regression from this fix)

3.7 WHEN an admin from Org A reads or updates their own org's configuracion values THEN the system SHALL CONTINUE TO return and persist those values correctly

---

## Design Decision (Confirmed)

**Bug 4 — `configuracion` table scoped per-org:** The `configuracion` table will be updated to include `organizacion_id`. Each org will have its own `tasa_cambio_dop_usd` and `recargo_porcentaje_defecto` values. This is a multi-tenant deployment.

**Bug 5 — Background jobs manual trigger:** Left as-is. Any org admin can trigger system-wide jobs. The cross-org operation is correct cron behavior.

---

## Bug Condition

```pascal
FUNCTION isBugCondition(X)
  INPUT: X of type ApiRequest (endpoint, entity_id, caller_org_id)
  OUTPUT: boolean

  LET entity_org := lookupOrganizacionId(X.entity_id)
  RETURN X.caller_org_id ≠ entity_org
END FUNCTION
```

## Property Specification

```pascal
// Property: Fix Checking — Cross-org access is denied
FOR ALL X WHERE isBugCondition(X) DO
  result ← endpoint'(X)
  ASSERT result.status = 404
  ASSERT no_mutation_occurred(X.entity_id)
END FOR
```

## Preservation Goal

```pascal
// Property: Preservation Checking — Same-org access unchanged
FOR ALL X WHERE NOT isBugCondition(X) DO
  ASSERT endpoint(X) = endpoint'(X)
END FOR
```
