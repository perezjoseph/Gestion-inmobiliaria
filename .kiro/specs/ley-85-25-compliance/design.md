# Ley 85-25 Compliance Bugfix Design

## Overview

The system enforces four outdated Ley 4314 rules instead of the current Ley 85-25. The fix targets specific validation logic in `contratos.rs` (deposit cap, rent increase fallback), `desahucios.rs` (eviction time gaps), and adds custody tracking to the deposit state machine. Changes are minimal and scoped: update constants, add a fallback branch, add date comparisons, and expose a warning field.

## Glossary

- **Bug_Condition (C)**: Any of four scenarios where outdated Ley 4314 logic is applied instead of Ley 85-25
- **Property (P)**: Correct Ley 85-25 behavior for each scenario
- **Preservation**: Existing behavior for inputs that do NOT trigger the four bug conditions
- **`contratos::create`**: Function in `backend/src/services/contratos.rs` that validates deposit amounts on contract creation
- **`contratos::renovar`**: Function that validates rent increases during contract renewal
- **`contratos::cambiar_estado_deposito`**: Function that transitions deposit state (`pendiente` → `cobrado` → `devuelto`/`retenido`)
- **`desahucios::update`**: Function in `backend/src/services/desahucios.rs` that transitions eviction states
- **IPC**: Índice de Precios al Consumidor from Banco Central (inflation index used to cap rent increases)

## Bug Details

### Bug Condition

The bug manifests in four distinct scenarios where the system applies outdated Ley 4314 rules:

1. Deposit cap rejects amounts > 1 month (should allow up to 2 months)
2. No custody tracking when deposit transitions to `cobrado` (15-day Banco Agrícola obligation)
3. Rent increase validation skipped entirely when IPC is `None` (should enforce hard 10% cap)
4. Eviction state transitions allowed instantly (should enforce 30-day and 90-day minimums)

**Formal Specification:**
```
FUNCTION isBugCondition(input)
  INPUT: input of type ServiceRequest
  OUTPUT: boolean

  RETURN (input.type == "create_contrato" AND input.deposito > input.monto_mensual AND input.deposito <= 2 * input.monto_mensual)
         OR (input.type == "cambiar_estado_deposito" AND input.nuevo_estado == "cobrado" AND no custody_tracking_exists)
         OR (input.type == "renovar_contrato" AND ipc_data IS None AND input.monto_mensual > original.monto_mensual * 1.10)
         OR (input.type == "update_desahucio" AND days_since_last_transition < required_minimum_days)
END FUNCTION
```

### Examples

- Deposit of 1.5× rent rejected with "Ley 4314" error → should be accepted under Ley 85-25 (max 2×)
- Deposit transitions to `cobrado` with no custody deadline tracking → should flag after 15 days without Banco Agrícola transfer confirmation
- IPC not configured, tenant renewal requests 20% increase → system allows it (no validation) → should reject (hard 10% cap)
- Desahucio created today, immediately transitioned to `en_progreso` → system allows → should reject (minimum 30 days required)

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**
- Deposits within the 2-month cap continue to be accepted without error
- Deposit state transitions (`pendiente` → `cobrado` → `devuelto`/`retenido`) continue to work when valid
- When IPC IS configured, rent increase validation against IPC-derived cap remains unchanged
- When IPC IS configured and increase exceeds cap, `ValidationWithFields` error with `maxAllowed` remains unchanged
- Eviction state transitions that respect minimum time gaps continue to work
- Eviction creation still initializes with `iniciado` estado and current date as `fecha_inicio`
- Mouse/keyboard/API interactions unrelated to these four flows remain completely unaffected

**Scope:**
All inputs that do NOT trigger the four bug conditions should produce identical behavior to the current system.

## Hypothesized Root Cause

Based on the code investigation:

1. **Deposit cap**: `contratos::create` line 293 compares `deposito > input.monto_mensual` (1× cap). Error message explicitly references "Ley 4314". Fix: change to `deposito > input.monto_mensual * 2` and update error message.

2. **Deposit custody tracking**: `cambiar_estado_deposito` sets `fecha_cobro_deposito` but nothing tracks the 15-day Banco Agrícola transfer obligation. No field or warning exists. Fix: add computed warning exposure in the response when 15 days have elapsed since `fecha_cobro_deposito` without confirmation.

3. **Rent increase fallback**: `contratos::renovar` has a `None =>` branch that only logs a warning and skips validation entirely. Fix: add a 10% hard cap in the `None` branch.

4. **Eviction time gaps**: `desahucios::update` calls `validate_estado_transition` which only checks valid directional transitions (graph edges) but not elapsed time. The `desahucio` entity has `fecha_inicio` and `updated_at` but no per-state timestamp tracking. Fix: use `updated_at` or add state-transition dates to enforce minimums.

## Correctness Properties

Property 1: Bug Condition - Deposit Cap Ley 85-25

_For any_ contract creation where the deposit amount is between 1× and 2× of monthly rent (inclusive), the fixed `create` function SHALL accept the deposit without error, and for any deposit exceeding 2× monthly rent, it SHALL reject with an error referencing Ley 85-25.

**Validates: Requirements 2.1**

Property 2: Bug Condition - Deposit Custody Warning

_For any_ contract where `estado_deposito == "cobrado"` and `fecha_cobro_deposito` is more than 15 days ago without custody confirmation, the fixed system SHALL expose a warning flag indicating the Banco Agrícola transfer obligation is overdue.

**Validates: Requirements 2.2**

Property 3: Bug Condition - IPC Fallback Cap

_For any_ contract renewal where IPC data is not configured (`None`), the fixed `renovar` function SHALL enforce a hard 10% cap on rent increases, rejecting amounts exceeding `original.monto_mensual * 1.10`.

**Validates: Requirements 2.3**

Property 4: Bug Condition - Eviction Time Gaps

_For any_ desahucio state transition, the fixed `update` function SHALL enforce minimum elapsed time: at least 30 days from `iniciado` to `en_progreso`, and at least 90 days from `en_progreso` to `completado` (for non-payment evictions).

**Validates: Requirements 2.4**

Property 5: Preservation - Existing Valid Operations

_For any_ input where none of the four bug conditions hold (deposit within cap, IPC configured, sufficient eviction time elapsed), the fixed functions SHALL produce the same result as the original functions, preserving all existing validations, state transitions, and error messages.

**Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**

## Fix Implementation

### Changes Required

**File**: `backend/src/services/contratos.rs`

**Function**: `create`

**Change 1 — Deposit cap update**:
- Change `if deposito > input.monto_mensual` to `if deposito > input.monto_mensual * Decimal::from(2)`
- Update error message from "Ley 4314" to reference Ley 85-25 with 2-month cap

**Function**: `renovar`

**Change 2 — IPC fallback enforcement**:
- Replace the `None =>` branch (currently just logs and skips) with a hard 10% cap check
- Compute `max_allowed = original.monto_mensual * Decimal::from_str("1.10")` 
- If `input.monto_mensual > max_allowed`, return `ValidationWithFields` error with the same structure as the IPC branch

**Function**: `cambiar_estado_deposito`

**Change 3 — Custody tracking warning**:
- After transitioning to `cobrado`, the `fecha_cobro_deposito` is already recorded
- Add a `custodia_vencida` boolean field to `ContratoResponse` that is `true` when `estado_deposito == "cobrado"` AND `fecha_cobro_deposito + 15 days < now`
- This is a computed response field, no schema migration needed for storage

---

**File**: `backend/src/services/desahucios.rs`

**Function**: `update`

**Change 4 — Eviction time gap enforcement**:
- After `validate_estado_transition` succeeds, add time-gap validation
- For `iniciado → en_progreso`: require `(now - existing.updated_at) >= 30 days`
- For `en_progreso → completado` (or `iniciado → completado`): require `(now - existing.updated_at) >= 90 days`
- Return validation error with clear message about minimum waiting period

---

**File**: `backend/src/models/contrato.rs` (response DTO)

**Change 5 — Add custody warning field**:
- Add `custodia_vencida: Option<bool>` to `ContratoResponse`
- Compute in the `From<contrato::Model>` impl or in the service layer before returning

## Testing Strategy

### Validation Approach

The testing strategy follows a two-phase approach: first, surface counterexamples that demonstrate the bug on unfixed code, then verify the fix works correctly and preserves existing behavior.

### Exploratory Bug Condition Checking

**Goal**: Surface counterexamples that demonstrate the bug BEFORE implementing the fix. Confirm or refute the root cause analysis.

**Test Plan**: Write unit tests that exercise each of the four bug conditions against the current code and observe failures.

**Test Cases**:
1. **Deposit 1.5× test**: Create contract with deposit = 1.5 × monto_mensual (will be rejected on unfixed code)
2. **IPC None renewal test**: Attempt renewal with 15% increase when IPC is None (will succeed on unfixed code — demonstrates the skip)
3. **Instant eviction transition test**: Create desahucio, immediately transition to `en_progreso` (will succeed on unfixed code)
4. **Custody elapsed test**: Transition deposit to `cobrado`, check response after 15 days (no warning on unfixed code)

**Expected Counterexamples**:
- Deposit 1.5× rejected with "Ley 4314" message
- 15% rent increase silently allowed with only a warning log
- Instant state transition allowed without time check
- No custody warning field in response

### Fix Checking

**Goal**: Verify that for all inputs where the bug condition holds, the fixed function produces the expected behavior.

**Pseudocode:**
```
FOR ALL input WHERE isBugCondition(input) DO
  result := fixedFunction(input)
  ASSERT expectedBehavior(result)
END FOR
```

### Preservation Checking

**Goal**: Verify that for all inputs where the bug condition does NOT hold, the fixed function produces the same result as the original function.

**Pseudocode:**
```
FOR ALL input WHERE NOT isBugCondition(input) DO
  ASSERT originalFunction(input) = fixedFunction(input)
END FOR
```

**Testing Approach**: Property-based testing is recommended for preservation checking because:
- It generates many random contract amounts, dates, and state combinations automatically
- It catches edge cases (boundary deposits at exactly 2×, time gaps at exactly 30 days)
- It provides strong guarantees that non-buggy paths are unchanged

**Test Plan**: Observe behavior on UNFIXED code first for valid operations, then write property-based tests capturing that behavior.

**Test Cases**:
1. **Deposit within cap preservation**: Verify deposits ≤ 2× rent continue to work (including deposits ≤ 1× which worked before)
2. **IPC-configured renewal preservation**: Verify rent validation with IPC configured produces identical results
3. **Valid eviction transitions preservation**: Verify transitions with sufficient elapsed time continue to work
4. **Deposit state machine preservation**: Verify `pendiente→cobrado→devuelto/retenido` transitions still work correctly

### Unit Tests

- Test deposit cap at boundary values: exactly 2×, 2× + 0.01, 1.99×
- Test IPC fallback: exactly 10% increase, 10.01%, 9.99%
- Test eviction gaps: exactly 30 days, 29 days, 31 days for `iniciado→en_progreso`
- Test eviction gaps: exactly 90 days, 89 days, 91 days for `en_progreso→completado`
- Test custody warning: 14 days (no warning), 15 days (warning), 16 days (warning)

### Property-Based Tests

- Generate random `(monto_mensual, deposito)` pairs and verify correct accept/reject behavior against 2× cap
- Generate random `(monto_actual, monto_nuevo)` pairs with IPC=None and verify 10% cap enforcement
- Generate random `(fecha_inicio, transition_date)` pairs and verify time gap enforcement
- Generate random valid operations (deposits within cap, IPC-configured renewals) and verify unchanged behavior

### Integration Tests

- Full contract lifecycle: create with 1.8× deposit, renew with IPC=None at 8% increase, verify success
- Full eviction lifecycle: create desahucio, wait 30+ days, transition to `en_progreso`, wait 90+ days, complete
- Custody warning: create contract, collect deposit, verify warning appears in GET response after 15 days
