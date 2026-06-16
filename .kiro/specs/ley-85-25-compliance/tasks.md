# Implementation Plan

## Overview

Fix four Ley 4314 → Ley 85-25 compliance violations in the backend: deposit cap (1× → 2×), IPC fallback enforcement (hard 10% cap), custody tracking warning (15-day computed field), and eviction time gap enforcement (30-day and 90-day minimums). Changes span `contratos.rs`, `desahucios.rs`, and the `ContratoResponse` DTO.

## Task Dependency Graph

```json
{
  "waves": [
    {
      "wave": 1,
      "description": "Write bug condition exploration tests and preservation tests on unfixed code",
      "tasks": ["1", "2"]
    },
    {
      "wave": 2,
      "description": "Implement all four fixes and add custodia_vencida field",
      "tasks": ["3.1", "3.2", "3.3", "3.4"]
    },
    {
      "wave": 3,
      "description": "Verify bug condition tests pass and preservation tests still pass",
      "tasks": ["3.5", "3.6"]
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

- [x] 1. Write bug condition exploration tests
  - **Property 1: Bug Condition** - Ley 85-25 Compliance Violations
  - **CRITICAL**: These tests MUST FAIL on unfixed code — failure confirms the bugs exist
  - **DO NOT attempt to fix the tests or the code when they fail**
  - **NOTE**: These tests encode the expected behavior — they will validate the fix when they pass after implementation
  - **GOAL**: Surface counterexamples that demonstrate all four bug conditions exist
  - **Scoped PBT Approach**: Scope properties to concrete failing cases for each bug condition
  - Test 1a: `contratos::create` with deposit = 1.5× monto_mensual → assert accepted (Bug Condition: `deposito > monto_mensual AND deposito <= 2 * monto_mensual`)
  - Test 1b: `contratos::renovar` with IPC = None and monto_mensual increase of 15% → assert rejected with ValidationWithFields (Bug Condition: `ipc_data IS None AND monto_nuevo > original * 1.10`)
  - Test 1c: `desahucios::update` transitioning `iniciado → en_progreso` with 0 days elapsed → assert rejected (Bug Condition: `days_since_last_transition < 30`)
  - Test 1d: `ContratoResponse` for deposit in `cobrado` state 16 days after collection → assert `custodia_vencida == true` (Bug Condition: `estado == cobrado AND elapsed > 15 days AND no confirmation`)
  - Run tests on UNFIXED code
  - **EXPECTED OUTCOME**: All four tests FAIL (this proves the bugs exist)
  - Document counterexamples: deposit rejected with "Ley 4314" message, 15% increase allowed silently, instant transition allowed, no custodia_vencida field
  - Mark task complete when tests are written, run, and failures documented
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [~] 2. Write preservation property tests (BEFORE implementing fix)
  - **Property 2: Preservation** - Existing Valid Operations Unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe on UNFIXED code: deposits ≤ 1× monto_mensual accepted, IPC-configured renewals within cap accepted, IPC-configured renewals exceeding cap rejected with `maxAllowed`, valid deposit state transitions work, desahucio creation initializes correctly
  - Write property-based tests:
  - Test 2a: For all `(monto_mensual, deposito)` where `deposito <= monto_mensual`, contract creation succeeds (from Preservation Requirement 3.1)
  - Test 2b: For all `(monto_actual, monto_nuevo, ipc_cap)` where IPC IS configured and `monto_nuevo <= monto_actual * ipc_cap`, renewal succeeds (from Preservation Requirement 3.3)
  - Test 2c: For all `(monto_actual, monto_nuevo, ipc_cap)` where IPC IS configured and `monto_nuevo > monto_actual * ipc_cap`, renewal rejected with `ValidationWithFields` including `maxAllowed` (from Preservation Requirement 3.4)
  - Test 2d: For all valid deposit state transitions (`pendiente → cobrado → devuelto/retenido`), transition succeeds (from Preservation Requirement 3.2)
  - Test 2e: For all desahucio state transitions where elapsed time ≥ required minimum, transition succeeds (from Preservation Requirement 3.5)
  - Test 2f: Desahucio creation initializes with estado `iniciado` and current date as `fecha_inicio` (from Preservation Requirement 3.6)
  - Run tests on UNFIXED code
  - **EXPECTED OUTCOME**: All tests PASS (this confirms baseline behavior to preserve)
  - Mark task complete when tests are written, run, and passing on unfixed code
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [ ] 3. Fix for Ley 85-25 compliance violations

  - [~] 3.1 Update deposit cap in `contratos::create`
    - Change comparison from `deposito > input.monto_mensual` to `deposito > input.monto_mensual * Decimal::from(2)`
    - Update error message from "Ley 4314" reference to Ley 85-25 with 2-month cap
    - _Bug_Condition: input.deposito > input.monto_mensual AND input.deposito <= 2 * input.monto_mensual_
    - _Expected_Behavior: accept deposit without error when within 2× cap; reject with Ley 85-25 message when exceeding_
    - _Preservation: deposits ≤ 2× continue accepted, error structure unchanged for over-cap_
    - _Requirements: 2.1, 3.1_

  - [~] 3.2 Enforce IPC fallback 10% cap in `contratos::renovar`
    - Replace `None =>` branch (currently logs warning + skips) with hard 10% cap check
    - Compute `max_allowed = original.monto_mensual * Decimal::from_str("1.10")`
    - If `input.monto_mensual > max_allowed`, return `ValidationWithFields` error matching existing IPC branch structure
    - _Bug_Condition: ipc_data IS None AND input.monto_mensual > original.monto_mensual * 1.10_
    - _Expected_Behavior: reject with ValidationWithFields including maxAllowed when exceeding 10% cap_
    - _Preservation: IPC-configured path unchanged, renewals within 10% when IPC=None accepted_
    - _Requirements: 2.3, 3.3, 3.4_

  - [~] 3.3 Add custody tracking warning in `contratos::cambiar_estado_deposito` and response DTO
    - Add `custodia_vencida: Option<bool>` field to `ContratoResponse` in `backend/src/models/contrato.rs`
    - Compute `custodia_vencida = true` when `estado_deposito == "cobrado"` AND `fecha_cobro_deposito + 15 days < now` without custody confirmation
    - Populate field in service layer before returning response
    - _Bug_Condition: estado_deposito == "cobrado" AND days_since_cobro > 15 AND no confirmation_
    - _Expected_Behavior: custodia_vencida == true exposed in ContratoResponse_
    - _Preservation: deposit state transitions continue working, no schema migration needed_
    - _Requirements: 2.2, 3.2_

  - [~] 3.4 Enforce eviction time gaps in `desahucios::update`
    - After `validate_estado_transition` succeeds, add elapsed-time validation
    - For `iniciado → en_progreso`: require `(now - existing.updated_at) >= 30 days`
    - For `en_progreso → completado`: require `(now - existing.updated_at) >= 90 days`
    - Return validation error with message about minimum waiting period and days remaining
    - _Bug_Condition: days_since_last_transition < required_minimum (30 or 90)_
    - _Expected_Behavior: reject transition with clear error about minimum waiting period_
    - _Preservation: transitions with sufficient elapsed time continue working, creation unchanged_
    - _Requirements: 2.4, 3.5, 3.6_

  - [~] 3.5 Verify bug condition exploration tests now pass
    - **Property 1: Expected Behavior** - Ley 85-25 Compliance Satisfied
    - **IMPORTANT**: Re-run the SAME tests from task 1 — do NOT write new tests
    - The tests from task 1 encode the expected behavior per design
    - When these tests pass, it confirms all four bug conditions are resolved
    - Run bug condition exploration tests from step 1
    - **EXPECTED OUTCOME**: All four tests PASS (confirms bugs are fixed)
    - _Requirements: 2.1, 2.2, 2.3, 2.4_

  - [~] 3.6 Verify preservation tests still pass
    - **Property 2: Preservation** - Existing Valid Operations Still Unchanged
    - **IMPORTANT**: Re-run the SAME tests from task 2 — do NOT write new tests
    - Run preservation property tests from step 2
    - **EXPECTED OUTCOME**: All tests PASS (confirms no regressions)
    - Confirm all preservation tests still pass after fix
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [~] 4. Checkpoint - Ensure all tests pass
  - Run full test suite (`cargo test` in backend)
  - Ensure all bug condition tests pass (fix verified)
  - Ensure all preservation tests pass (no regressions)
  - Ensure existing test suite passes (no collateral damage)
  - Ask the user if questions arise

## Notes

- All changes are in backend Rust code — no migrations needed (custodia_vencida is a computed response field)
- Property-based tests use `proptest` crate (already in dev-dependencies) for generating random Decimal amounts and dates
- The 10% IPC fallback cap is the Ley 85-25 absolute ceiling when Banco Central IPC data is unavailable
- Time gap enforcement uses `updated_at` from existing entity — no new columns needed
- Eviction 90-day minimum applies to non-payment evictions (`en_progreso → completado`)
