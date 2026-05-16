# Implementation Plan

## Overview

Fix the WhatsApp chatbot self-message bug where messages sent to the bot's own number are silently discarded by sender policy enforcement. The fix adds a self-message bypass in the incoming webhook handler by comparing sender_phone against the session's connected phone number.

## Tasks

- [x] 1. Write bug condition exploration test
  - **Property 1: Bug Condition** - Self-Messages Discarded by Sender Policy
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **DO NOT attempt to fix the test or the code when it fails**
  - **NOTE**: This test encodes the expected behavior - it will validate the fix when it passes after implementation
  - **GOAL**: Surface counterexamples that demonstrate self-messages are discarded by sender policy enforcement
  - **Scoped PBT Approach**: Scope the property to concrete failing cases: `sender_phone == session_phone` with `activo = true` and restrictive policies (`tenants_only`, `allowlist`)
  - Bug Condition from design: `isBugCondition(input)` returns true when `config.activo == true AND sender_phone == session_phone AND NOT is_sender_allowed(config.sender_policy, sender_phone, ...)`
  - Generate random E.164 phone numbers for `sender_phone` and set `session_phone = sender_phone`
  - Test with `sender_policy = "tenants_only"` where phone is not in `inquilino` table
  - Test with `sender_policy = "allowlist"` where phone is not in the allowlist
  - Assert that the handler processes the message (returns status != "discarded") for all inputs satisfying the bug condition
  - Run test on UNFIXED code
  - **EXPECTED OUTCOME**: Test FAILS (this is correct - it proves the bug exists because self-messages are discarded)
  - Document counterexamples found
  - Mark task complete when test is written, run, and failure is documented
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 2. Write preservation property tests (BEFORE implementing fix)
  - **Property 2: Preservation** - Non-Self Message Policy Enforcement Unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe behavior on UNFIXED code for non-self messages (`sender_phone != session_phone`)
  - Observe: non-tenant phone with `tenants_only` policy is discarded
  - Observe: non-allowlisted phone with `allowlist` policy is discarded
  - Observe: any phone with `activo = false` is discarded
  - Observe: registered tenant phone with `tenants_only` is processed
  - Observe: any phone with `tenants_and_prospects` is processed
  - Write property-based tests generating random (sender_phone, session_phone, sender_policy, activo) tuples where `sender_phone != session_phone`
  - Assert: for all non-self inputs, the handler produces the same result as the original `is_sender_allowed` logic
  - Assert: when `activo = false`, message is always discarded regardless of phone values
  - Assert: when `sender_policy = "tenants_only"` and phone not in inquilino table, message is discarded
  - Assert: when `sender_policy = "allowlist"` and phone not in allowlist, message is discarded
  - Assert: when `sender_policy = "tenants_and_prospects"`, message is processed
  - Verify tests pass on UNFIXED code
  - **EXPECTED OUTCOME**: Tests PASS (this confirms baseline behavior to preserve)
  - Mark task complete when tests are written, run, and passing on unfixed code
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [x] 3. Add `sessionPhone` to webhook payload in baileys-service
  - File: `baileys-service/src/session-manager.ts`, function `forwardToBackend`
  - Add `sessionPhone: sessionInfo.connectedPhone` to the payload object sent to the backend
  - This provides the connected WhatsApp session's E.164 phone number
  - Handle case where `connectedPhone` may be undefined (send `null`)
  - _Requirements: 2.1_

- [x] 4. Add `session_phone` field to `IncomingWebhookPayload` struct
  - File: `backend/src/models/chatbot.rs`, struct `IncomingWebhookPayload`
  - Add `pub session_phone: Option<String>` field with serde rename `sessionPhone`
  - Use `Option<String>` for backward compatibility (deserializes as `None` if field missing)
  - _Requirements: 2.1_

- [x] 5. Add self-message bypass before sender policy check
  - File: `backend/src/handlers/chatbot_internal.rs`, function `incoming_webhook`
  - Between Step 2 (activo check) and Step 3 (sender policy), add bypass logic
  - If `payload.session_phone == Some(sender_phone)` then skip `is_sender_allowed` check
  - Log bypass at `debug` level: `"Self-message detected, bypassing sender policy"`
  - If `payload.session_phone` is `None` then fall through to existing sender policy check (backward compatible)
  - _Requirements: 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 6. Verify bug condition exploration test now passes
  - **Property 1: Expected Behavior** - Self-Messages Processed Through AI Pipeline
  - **IMPORTANT**: Re-run the SAME test from task 1 - do NOT write a new test
  - The test from task 1 encodes the expected behavior (self-messages should not be discarded)
  - When this test passes, it confirms the expected behavior is satisfied
  - Run bug condition exploration test from step 1
  - **EXPECTED OUTCOME**: Test PASSES (confirms bug is fixed - self-messages now bypass policy)
  - _Requirements: 2.1, 2.2, 2.3_

- [x] 7. Verify preservation tests still pass
  - **Property 2: Preservation** - Non-Self Message Policy Enforcement Unchanged
  - **IMPORTANT**: Re-run the SAME tests from task 2 - do NOT write new tests
  - Run preservation property tests from step 2
  - **EXPECTED OUTCOME**: Tests PASS (confirms no regressions - non-self messages still enforced by policy)
  - Confirm all tests still pass after fix (no regressions)

- [x] 8. Checkpoint - Ensure all tests pass
  - Run full test suite: `cargo test` in backend
  - Run baileys-service tests if available
  - Verify no regressions in existing chatbot PBT tests (`chatbot_pbt.rs`)
  - Ensure all tests pass, ask the user if questions arise

## Task Dependency Graph

```json
{
  "waves": [
    ["1", "2"],
    ["3", "4"],
    ["5"],
    ["6", "7"],
    ["8"]
  ]
}
```

## Notes

- Property-based tests use random E.164 phone numbers to cover the input space
- The baileys-service change (3) and backend model change (4) can be done in parallel
- Task 5 depends on both 3 and 4 being complete
- Existing PBT tests in `chatbot_pbt.rs` for `is_sender_allowed` and `check_sender_policy_no_db` should continue passing unchanged
