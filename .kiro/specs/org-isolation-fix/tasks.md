# Implementation Plan

## Overview

Fix IDOR vulnerability: six endpoints in `indexacion`, `ipi`, `chatbot` modules lack `organizacion_id` ownership checks, allowing cross-org data access. Additionally, `configuracion` table is globally shared with no org scoping. Fix adds post-fetch ownership verification (404 on mismatch) and migrates `configuracion` to composite PK `(clave, organizacion_id)`.

## Task Dependency Graph

```json
{
  "waves": [
    {
      "wave": 1,
      "description": "Write bug condition exploration test and preservation tests on unfixed code",
      "tasks": ["1", "2"]
    },
    {
      "wave": 2,
      "description": "Create migration and implement ownership checks across all affected modules",
      "tasks": ["3.1", "3.2", "3.3", "3.4", "3.5"]
    },
    {
      "wave": 3,
      "description": "Verify bug condition test passes and preservation tests still pass",
      "tasks": ["3.6", "3.7"]
    },
    {
      "wave": 4,
      "description": "Final validation checkpoint",
      "tasks": ["4"]
    }
  ]
}
```

## Tasks

- [x] 1. Write bug condition exploration test
  - **Property 1: Bug Condition** - Cross-Org IDOR Access
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **DO NOT attempt to fix the test or the code when it fails**
  - **NOTE**: This test encodes the expected behavior - it will validate the fix when it passes after implementation
  - **GOAL**: Surface counterexamples that demonstrate cross-org access is not blocked
  - **Scoped PBT Approach**: For each affected endpoint, generate random (caller_org_id, entity_org_id) pairs where caller_org_id ≠ entity_org_id, then assert the endpoint returns 404
  - Bug Condition from design: `isBugCondition(X) = X.caller_org_id ≠ lookupOrganizacionId(X.entity_id)`
  - Test cases to cover (all cross-org):
    - `GET /api/v1/indexacion/contratos/{org_b_contrato}/propuesta` → assert 404
    - `POST /api/v1/indexacion/contratos/{org_b_contrato}/aprobar` → assert 404 and no new contrato
    - `GET /api/v1/ipi/propiedades/{org_b_propiedad}/copropietarios` → assert 404
    - `POST /api/v1/ipi/copropietarios` with org_b propiedad_id → assert 404 and no record created
    - `POST /api/v1/chatbot/extractions/{org_b_extraction}/confirm` → assert 404 and no pago
    - `POST /api/v1/chatbot/extractions/{org_b_extraction}/reject` → assert 404 and no status change
    - Configuracion: Org A writes tasa, Org B reads → assert Org B does NOT see Org A's value
  - Property: for all (org_a, org_b, entity_id) where entity belongs to org_b and caller is org_a (a ≠ b), response.status == 404 AND no_mutation_occurred
  - Run test on UNFIXED code
  - **EXPECTED OUTCOME**: Test FAILS (cross-org requests currently succeed with 200 — this proves the IDOR exists)
  - Document counterexamples (e.g., "Org A gets Org B's propuesta with 200 OK")
  - Mark task complete when test is written, run, and failure is documented
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8_

- [x] 2. Write preservation property tests (BEFORE implementing fix)
  - **Property 2: Preservation** - Same-Org Access Unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe on UNFIXED code: same-org requests to all affected endpoints succeed normally
  - Observe: Org A calls propuesta for own contrato → 200 with valid proposal
  - Observe: Org A approves own contrato → 200 and new contrato created
  - Observe: Org A lists copropietarios for own propiedad → 200 with list
  - Observe: Org A creates copropietario on own propiedad → 201
  - Observe: Org A confirms own extraction → 200 and pago created
  - Observe: Org A rejects own extraction → 200 and status updated
  - Observe: Org A reads/writes own configuracion → 200 with correct value
  - Write property-based test: for all (org_id, entity_id) where entity belongs to org_id (same org), endpoint returns success and expected side effects occur
  - Property: for all X where NOT isBugCondition(X), endpoint(X).status is success AND correct_response_body AND expected_mutations_applied
  - Verify tests PASS on UNFIXED code
  - **EXPECTED OUTCOME**: Tests PASS (confirms baseline same-org behavior to preserve)
  - Mark task complete when tests are written, run, and passing on unfixed code
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7_

- [ ] 3. Fix org isolation IDOR vulnerability

  - [x] 3.1 Create migration: add organizacion_id to configuracion table
    - Add `organizacion_id UUID NOT NULL` column to `configuracion`
    - Drop existing PK on `clave`
    - Create composite PK `(clave, organizacion_id)`
    - Add FK referencing `organizaciones(id)`
    - Backfill existing rows: duplicate each row for every existing org
    - _Bug_Condition: configuracion queries without org scope return global values_
    - _Expected_Behavior: each org reads/writes only its own configuracion rows_
    - _Preservation: existing configuracion values remain accessible per-org after migration_
    - _Requirements: 2.7, 2.8, 3.7_

  - [x] 3.2 Update configuracion entity and service
    - Add `organizacion_id: Uuid` field to `configuracion` entity Model
    - Update PK definition to composite `(Clave, OrganizacionId)`
    - Add `org_id: Uuid` parameter to all configuracion service functions
    - Query by `(clave, org_id)` instead of `clave` alone
    - Update configuracion handlers: extract `org_id` from `AdminOnly` extractor claims
    - _Bug_Condition: isBugCondition(X) where X targets configuracion with caller_org ≠ entity_org_
    - _Expected_Behavior: per-org isolation of configuracion values_
    - _Preservation: same-org configuracion reads/writes unchanged_
    - _Requirements: 2.7, 2.8, 3.7_

  - [x] 3.3 Add org ownership check to indexacion service
    - Add `org_id: Uuid` parameter to `calcular_propuesta_renovacion` and `aprobar_renovacion`
    - After fetching contrato by id, verify `contrato.organizacion_id == org_id`
    - Return `AppError::NotFound` if mismatch (404, no info leak)
    - Update handlers `obtener_propuesta` and `aprobar_renovacion_handler`: extract `org_id` from `WriteAccess` claims, pass to service
    - _Bug_Condition: isBugCondition(X) where X.caller_org_id ≠ contrato.organizacion_id_
    - _Expected_Behavior: return 404, no new contrato created_
    - _Preservation: same-org propuesta/aprobar unchanged_
    - _Requirements: 2.1, 2.2, 3.1_

  - [x] 3.4 Add org ownership check to IPI service
    - Add `org_id: Uuid` parameter to `obtener_copropietarios` and copropietario creation
    - Fetch propiedad by id, verify `propiedad.organizacion_id == org_id`
    - Return `AppError::NotFound` if mismatch
    - Update `listar_copropietarios` handler: extract `org_id` from `WriteAccess`, pass to service
    - _Bug_Condition: isBugCondition(X) where X.caller_org_id ≠ propiedad.organizacion_id_
    - _Expected_Behavior: return 404, no copropietario record created_
    - _Preservation: same-org copropietarios list/create unchanged_
    - _Requirements: 2.3, 2.4, 3.2, 3.3_

  - [x] 3.5 Add org ownership check to chatbot service
    - Add `org_id: Uuid` parameter to `confirm_receipt` and `reject_receipt`
    - After fetching extraction by id, verify `extraction.organizacion_id == org_id`
    - Return `AppError::NotFound` if mismatch
    - Update handlers: pass `claims.0.organizacion_id` to service functions
    - _Bug_Condition: isBugCondition(X) where X.caller_org_id ≠ extraction.organizacion_id_
    - _Expected_Behavior: return 404, no pago created, no status change_
    - _Preservation: same-org confirm/reject unchanged_
    - _Requirements: 2.5, 2.6, 3.4_

  - [x] 3.6 Verify bug condition exploration test now passes
    - **Property 1: Expected Behavior** - Cross-Org Access Denied
    - **IMPORTANT**: Re-run the SAME test from task 1 - do NOT write a new test
    - The test from task 1 encodes the expected behavior (404 for cross-org)
    - When this test passes, it confirms all cross-org requests are blocked
    - Run bug condition exploration test from step 1
    - **EXPECTED OUTCOME**: Test PASSES (confirms IDOR is fixed)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8_

  - [~] 3.7 Verify preservation tests still pass
    - **Property 2: Preservation** - Same-Org Access Unchanged
    - **IMPORTANT**: Re-run the SAME tests from task 2 - do NOT write new tests
    - Run preservation property tests from step 2
    - **EXPECTED OUTCOME**: Tests PASS (confirms no regressions for same-org access)
    - Confirm all tests still pass after fix (no regressions)

- [~] 4. Checkpoint - Ensure all tests pass
  - Run full test suite: `cargo test` in backend
  - Verify no clippy warnings on changed files
  - Ensure migration applies cleanly on fresh DB
  - Ensure all bug condition tests pass (IDOR fixed)
  - Ensure all preservation tests pass (no regressions)
  - Ask the user if questions arise

## Notes

- Ownership check pattern: fetch-then-verify (not filter in query) to return consistent 404 whether entity doesn't exist or belongs to another org — prevents enumeration attacks
- Reuse existing `WriteAccess` and `AdminOnly` extractors which already carry `organizacion_id` in claims — no new extractors needed
- Background cron jobs are intentionally exempt from org filtering (correct cross-org behavior)
- `proptest` crate (already in dev-dependencies) used for generating random UUID pairs and entity combinations
- Migration must handle backfill: duplicate each existing configuracion row for every org in the system
