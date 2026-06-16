# Org Isolation Fix — Bugfix Design

## Overview

IDOR vulnerability: six endpoints in `indexacion`, `ipi`, and `chatbot` modules accept entity UUIDs without verifying the caller's `organizacion_id` owns the entity. The `configuracion` table is globally shared with no org scoping. Fix adds ownership checks (return 404 on mismatch) and migrates `configuracion` to per-org rows.

## Glossary

- **Bug_Condition (C)**: Request where `caller_org_id ≠ entity_org_id` — cross-org access attempt
- **Property (P)**: Cross-org requests return 404 with no side effects
- **Preservation**: Same-org requests continue working identically
- **IDOR**: Insecure Direct Object Reference — accessing another tenant's data via guessable IDs
- **`organizacion_id`**: UUID column present on every multi-tenant entity, linking it to its owning org

## Bug Details

### Bug Condition

The bug manifests when an authenticated user calls an endpoint with an entity UUID belonging to a different organization. The affected handlers either skip ownership verification entirely or delegate to service functions that query by PK alone (`find_by_id`) without filtering by `organizacion_id`.

**Formal Specification:**
```
FUNCTION isBugCondition(input)
  INPUT: input of type ApiRequest { endpoint, entity_id, caller_org_id }
  OUTPUT: boolean

  LET entity_org := lookupOrganizacionId(input.entity_id)
  RETURN input.caller_org_id ≠ entity_org
END FUNCTION
```

### Examples

- User from Org A calls `GET /api/v1/indexacion/contratos/{org_b_contrato}/propuesta` → currently returns Org B's renewal proposal (should be 404)
- User from Org A calls `POST /api/v1/ipi/copropietarios` with `propiedad_id` from Org B → currently creates record linked to Org B's propiedad (should be 404)
- User from Org A calls `POST /api/v1/chatbot/extractions/{org_b_extraction}/confirm` → currently confirms Org B's receipt and creates a pago in Org B (should be 404)
- Admin from Org A updates `tasa_cambio_dop_usd` → currently overwrites the single global row affecting all orgs (should update Org A's row only)

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**
- Same-org access to indexacion endpoints (propuesta, aprobar) works identically
- Same-org access to IPI copropietarios (list, create) works identically
- Same-org chatbot confirm/reject works identically
- Same-org configuracion read/write works identically
- Background cron jobs continue cross-org operation (correct behavior)
- All other endpoints already filtering by org remain unchanged

**Scope:**
All requests where `caller_org_id = entity_org_id` are unaffected. Only cross-org requests (the bug condition) change behavior.

## Hypothesized Root Cause

1. **Missing ownership check in handlers**: `obtener_propuesta` and `aprobar_renovacion_handler` extract `contrato_id` from path but never pass `org_id` to the service layer. The service calls `find_by_id` which returns any org's contrato.

2. **Missing ownership check in IPI list**: `listar_copropietarios` passes `propiedad_id` directly to `obtener_copropietarios` without verifying the propiedad belongs to the caller's org.

3. **Missing ownership check in chatbot confirm/reject**: `confirm_receipt` and `reject_receipt` accept `extraction_id` from path but never verify `extraction.organizacion_id == caller_org_id`.

4. **Global configuracion table**: Entity has PK `clave` (string) with no `organizacion_id` column. All orgs share the same rows. Needs schema change to composite key `(clave, organizacion_id)`.

## Correctness Properties

Property 1: Bug Condition — Cross-org access denied

_For any_ API request where the bug condition holds (`caller_org_id ≠ entity_org_id`), the fixed endpoint SHALL return HTTP 404 and SHALL NOT perform any mutation (no new rows inserted, no status changes).

**Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8**

Property 2: Preservation — Same-org access unchanged

_For any_ API request where the bug condition does NOT hold (`caller_org_id = entity_org_id`), the fixed endpoint SHALL produce the same response and side effects as the original (unfixed) endpoint.

**Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7**

## Fix Implementation

### Changes Required

**File**: `backend/src/handlers/indexacion.rs`

**Functions**: `obtener_propuesta`, `aprobar_renovacion_handler`

**Changes**:
1. Extract `org_id` from `WriteAccess` claims (already available, just unused)
2. Pass `org_id` to service functions

**File**: `backend/src/services/indexacion.rs`

**Functions**: `calcular_propuesta_renovacion`, `aprobar_renovacion`

**Changes**:
1. Add `org_id: Uuid` parameter
2. Change `find_by_id` to `find_by_id` + filter `organizacion_id.eq(org_id)`, or verify after fetch that `contrato.organizacion_id == org_id` (return 404 if mismatch)

**File**: `backend/src/handlers/ipi.rs`

**Function**: `listar_copropietarios`

**Changes**:
1. Extract `org_id` from `WriteAccess` (currently `_user`)
2. Pass `org_id` to service

**File**: `backend/src/services/ipi.rs`

**Function**: `obtener_copropietarios`

**Changes**:
1. Add `org_id: Uuid` parameter
2. Verify propiedad ownership: fetch propiedad by id, check `organizacion_id == org_id`, return 404 if mismatch

**File**: `backend/src/handlers/chatbot.rs`

**Functions**: `confirm_receipt`, `reject_receipt`

**Changes**:
1. Pass `claims.0.organizacion_id` to service functions

**File**: `backend/src/services/chatbot.rs`

**Functions**: `confirm_receipt`, `reject_receipt`

**Changes**:
1. Add `org_id: Uuid` parameter
2. After fetching extraction by id, verify `extraction.organizacion_id == org_id`, return 404 if mismatch

**File**: `backend/src/entities/configuracion.rs`

**Changes**:
1. Add `organizacion_id: Uuid` field to Model
2. Change PK to composite `(clave, organizacion_id)`

**File**: `backend/src/services/configuracion.rs`

**Changes**:
1. Add `org_id: Uuid` parameter to all functions
2. Query by `(clave, org_id)` instead of `clave` alone

**File**: `backend/src/handlers/configuracion.rs`

**Changes**:
1. Extract `org_id` from claims (AdminOnly extractor has `organizacion_id`)
2. Pass to service functions

**New migration file**: Add `organizacion_id` column to `configuracion` table, drop old PK, create composite PK `(clave, organizacion_id)`, backfill existing rows with a default org or duplicate for all orgs.

## Testing Strategy

### Validation Approach

Two-phase: first surface counterexamples on unfixed code confirming the IDOR, then verify the fix blocks cross-org and preserves same-org.

### Exploratory Bug Condition Checking

**Goal**: Confirm the IDOR exists on unfixed code before implementing the fix.

**Test Plan**: Integration tests that create two orgs, create entities under Org B, then call endpoints authenticated as Org A with Org B's entity IDs.

**Test Cases**:
1. **Indexacion propuesta cross-org**: Org A calls propuesta for Org B's contrato → currently succeeds (will fail after fix)
2. **Indexacion aprobar cross-org**: Org A approves Org B's contrato → currently creates new contrato (will fail after fix)
3. **IPI copropietarios list cross-org**: Org A lists copropietarios for Org B's propiedad → currently returns data (will fail after fix)
4. **Chatbot confirm cross-org**: Org A confirms Org B's extraction → currently creates pago in Org B (will fail after fix)
5. **Configuracion global write**: Org A updates tasa, Org B reads — currently sees Org A's value (will be isolated after fix)

**Expected Counterexamples**:
- All cross-org requests succeed with 200 status (the IDOR)
- Mutations create data under wrong org ownership

### Fix Checking

**Goal**: After fix, all cross-org requests return 404 with no side effects.

**Pseudocode:**
```
FOR ALL input WHERE isBugCondition(input) DO
  result := endpoint_fixed(input)
  ASSERT result.status = 404
  ASSERT no_mutation_occurred(input.entity_id)
END FOR
```

### Preservation Checking

**Goal**: Same-org requests produce identical results before and after fix.

**Pseudocode:**
```
FOR ALL input WHERE NOT isBugCondition(input) DO
  ASSERT endpoint_original(input) = endpoint_fixed(input)
END FOR
```

**Testing Approach**: Property-based testing generates random valid org+entity combinations to verify same-org access remains functional across all affected endpoints.

**Test Cases**:
1. **Indexacion same-org propuesta**: Org A calls propuesta for own contrato → same result
2. **IPI same-org copropietarios**: Org A lists/creates copropietarios on own propiedad → same result
3. **Chatbot same-org confirm/reject**: Org A confirms own extraction → same pago created
4. **Configuracion per-org isolation**: Org A and Org B each set different tasa values, each reads their own

### Unit Tests

- Service functions return `AppError::NotFound` when org_id mismatches entity's org
- Configuracion queries by `(clave, org_id)` composite correctly
- Ownership check short-circuits before any mutation

### Property-Based Tests

- Generate random (org_id, entity_id) pairs; for cross-org pairs assert 404, for same-org assert success
- Generate random configuracion values per org; assert each org reads only its own values
- Generate random contrato states; assert propuesta calculation unchanged for same-org

### Integration Tests

- Full HTTP round-trip: create orgs, create entities, attempt cross-org access, verify 404
- Configuracion migration: verify existing data accessible after schema change
- Background jobs still process all orgs (preservation of cron behavior)
