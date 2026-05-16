# WhatsApp Self-Message Fix — Bugfix Design

## Overview

The WhatsApp chatbot silently discards messages when the user messages themselves (same phone number as the bot) because the sender policy enforcement in `incoming_webhook` checks whether the sender's phone exists in the `inquilino` table. The bot's own phone number is never registered as a tenant, so the check fails and the message is discarded with `{"status": "discarded"}`.

The fix adds a self-message bypass: before evaluating the sender policy, the handler compares the sender's phone number against the connected WhatsApp session's phone number. If they match, the sender policy check is skipped entirely and the message proceeds through the full AI pipeline.

## Glossary

- **Bug_Condition (C)**: The sender phone matches the connected WhatsApp session's phone number AND the sender policy would reject the message
- **Property (P)**: Self-messages bypass the sender policy and are processed through the full AI pipeline
- **Preservation**: All existing sender policy enforcement for non-self messages remains unchanged
- **`incoming_webhook`**: The handler in `backend/src/handlers/chatbot_internal.rs` that receives forwarded WhatsApp messages from the baileys-service sidecar
- **`is_sender_allowed`**: The function in `backend/src/services/chatbot.rs` that evaluates sender policy rules
- **`connectedPhone`**: The E.164 phone number of the connected WhatsApp session, tracked by the baileys-service per realm
- **`sender_policy`**: Configuration field on `chatbot_config` that controls which phone numbers may interact with the bot (`tenants_only`, `tenants_and_prospects`, `allowlist`)

## Bug Details

### Bug Condition

The bug manifests when a user sends a WhatsApp message to themselves (the same phone number as the bot's connected session). The `incoming_webhook` handler loads the chatbot config, confirms `activo = true`, then calls `is_sender_allowed` which checks the `sender_policy`. For `tenants_only` (the default), it queries the `inquilino` table — the bot's phone is never a tenant, so it returns `false`. For `allowlist`, the bot's phone is typically not in the allowlist either. The message is silently discarded.

**Formal Specification:**
```
FUNCTION isBugCondition(input)
  INPUT: input of type IncomingWebhookPayload + SessionContext
  OUTPUT: boolean

  LET session_phone = getConnectedPhone(input.realm_id)
  LET sender_phone = input.sender_phone
  LET config = getChatbotConfig(input.realm_id)

  RETURN config.activo == true
         AND sender_phone == session_phone
         AND NOT is_sender_allowed(config.sender_policy, sender_phone, config.allowlist, input.realm_id)
END FUNCTION
```

### Examples

- **Default policy, self-message**: User sends "Hola" from `+18091234567` to `+18091234567` (bot's number). Policy is `tenants_only`. Phone not in `inquilino` table → message discarded. **Expected**: message processed through AI pipeline.
- **Allowlist policy, self-message**: User sends "Test" from `+18091234567`. Policy is `allowlist`, allowlist is `["+18095551234"]`. Bot's phone not in allowlist → message discarded. **Expected**: message processed.
- **tenants_and_prospects policy, self-message**: User sends "Hola" from `+18091234567`. Policy is `tenants_and_prospects` → message already allowed (no bug for this policy). **Expected**: message processed (already works).
- **Non-self message, tenants_only**: Message from `+18095559999` (not a tenant). Policy is `tenants_only` → message correctly discarded. **Expected**: remains discarded (no change).

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**
- Messages from non-self phone numbers with `tenants_only` policy that are not tenants must continue to be discarded
- Messages from non-self phone numbers with `allowlist` policy that are not in the allowlist must continue to be discarded
- Messages when `activo = false` must continue to be discarded regardless of sender
- Messages from registered tenants must continue to be allowed through
- Messages with `tenants_and_prospects` policy must continue to allow all senders
- The `sentMessageIds` tracking in baileys-service must continue to filter out bot-sent echoes
- Handoff logic, conversation history, AI processing, and reply sending must remain unchanged

**Scope:**
All inputs where `sender_phone != session_phone` should be completely unaffected by this fix. This includes:
- Messages from tenants
- Messages from unknown numbers
- Messages from allowlisted numbers
- Messages when chatbot is disabled

## Hypothesized Root Cause

Based on the code analysis, the root cause is clear and singular:

1. **Missing self-message bypass in `incoming_webhook`**: The handler at Step 3 unconditionally applies the sender policy check via `is_sender_allowed`. There is no special case for when the sender is the bot itself. The baileys-service correctly forwards self-messages (it has explicit logic in `shouldForwardMessage` to allow self-chat through), but the backend discards them at the policy enforcement step.

2. **No session phone available in the backend handler**: The `IncomingWebhookPayload` does not include the session's connected phone number. The backend handler has no way to determine whether the sender is the bot's own number without either:
   - Receiving the session phone in the webhook payload from baileys-service, OR
   - Querying the baileys-service status API (adds latency to every message), OR
   - Storing the connected phone in the `chatbot_config` table

   The simplest and most reliable approach is to include `sessionPhone` in the webhook payload sent by the baileys-service, since it already has this information in `sessionInfo.connectedPhone`.

## Correctness Properties

Property 1: Bug Condition - Self-Messages Bypass Sender Policy

_For any_ incoming webhook payload where the sender phone equals the session's connected phone number AND the chatbot is active (`activo = true`), the fixed `incoming_webhook` handler SHALL bypass the sender policy check and process the message through the full AI pipeline (history loading, AI invocation, reply sending), regardless of the configured `sender_policy` value.

**Validates: Requirements 2.1, 2.2, 2.3**

Property 2: Preservation - Non-Self Message Policy Enforcement

_For any_ incoming webhook payload where the sender phone does NOT equal the session's connected phone number, the fixed `incoming_webhook` handler SHALL produce exactly the same sender policy enforcement result as the original handler, preserving the discard behavior for unauthorized senders and the allow behavior for authorized senders.

**Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**

## Fix Implementation

### Changes Required

**File**: `baileys-service/src/session-manager.ts`

**Function**: `forwardToBackend`

**Specific Changes**:
1. **Include session phone in webhook payload**: Add `sessionPhone: sessionInfo.connectedPhone` to the payload object sent to the backend. This is the E.164 phone number of the connected WhatsApp session (e.g., `+18091234567`). It may be `null` if the session is in a degraded state (unlikely since messages only arrive on connected sessions).

---

**File**: `backend/src/models/chatbot.rs`

**Struct**: `IncomingWebhookPayload`

**Specific Changes**:
2. **Add `session_phone` field**: Add `pub session_phone: Option<String>` to `IncomingWebhookPayload`. Use `Option<String>` for backward compatibility — if the baileys-service hasn't been updated yet, the field will deserialize as `None`.

---

**File**: `backend/src/handlers/chatbot_internal.rs`

**Function**: `incoming_webhook`

**Specific Changes**:
3. **Add self-message bypass before sender policy check**: Between Step 2 (activo check) and Step 3 (sender policy), add a check: if `payload.session_phone` is `Some(ref phone)` and `phone == payload.sender_phone`, skip the sender policy evaluation entirely and proceed to Step 4 (tenant resolution). Log the bypass at `debug` level.

4. **Handle missing session_phone gracefully**: If `payload.session_phone` is `None`, fall through to the existing sender policy check (backward-compatible behavior).

---

**File**: `backend/src/services/chatbot.rs`

**No changes required** — the `is_sender_allowed` and `check_sender_policy_no_db` functions remain unchanged. The bypass happens at the handler level before these functions are called.

## Testing Strategy

### Validation Approach

The testing strategy follows a two-phase approach: first, surface counterexamples that demonstrate the bug on unfixed code, then verify the fix works correctly and preserves existing behavior.

### Exploratory Bug Condition Checking

**Goal**: Surface counterexamples that demonstrate the bug BEFORE implementing the fix. Confirm the root cause by showing that self-messages are discarded by the sender policy.

**Test Plan**: Write integration tests that simulate the `incoming_webhook` handler receiving a payload where `sender_phone` matches the session phone. Run on UNFIXED code to observe the `{"status": "discarded"}` response.

**Test Cases**:
1. **Self-message with tenants_only**: Send payload with `sender_phone = "+18091234567"` and `session_phone = Some("+18091234567")`, policy `tenants_only`, phone not in inquilino table (will return "discarded" on unfixed code)
2. **Self-message with allowlist**: Send payload with matching phones, policy `allowlist`, phone not in allowlist (will return "discarded" on unfixed code)
3. **Self-message with tenants_and_prospects**: Send payload with matching phones, policy `tenants_and_prospects` (will pass even on unfixed code — confirms this policy is not affected)

**Expected Counterexamples**:
- Test cases 1 and 2 return `{"status": "discarded"}` on unfixed code
- Root cause confirmed: `is_sender_allowed` returns `false` for the bot's own phone under restrictive policies

### Fix Checking

**Goal**: Verify that for all inputs where the bug condition holds, the fixed handler processes the message instead of discarding it.

**Pseudocode:**
```
FOR ALL input WHERE isBugCondition(input) DO
  result := incoming_webhook_fixed(input)
  ASSERT result.status == "processed" OR result.status == "handoff_active"
  ASSERT message_persisted(input.sender_phone, input.content)
END FOR
```

### Preservation Checking

**Goal**: Verify that for all inputs where the bug condition does NOT hold, the fixed handler produces the same result as the original handler.

**Pseudocode:**
```
FOR ALL input WHERE NOT isBugCondition(input) DO
  ASSERT incoming_webhook_original(input) == incoming_webhook_fixed(input)
END FOR
```

**Testing Approach**: Property-based testing is recommended for preservation checking because:
- It generates many combinations of sender_policy, sender_phone, session_phone, and activo values
- It catches edge cases like `session_phone = None`, empty strings, or phone format variations
- It provides strong guarantees that non-self-message behavior is unchanged

**Test Plan**: Observe behavior on UNFIXED code for non-self messages across all policy types, then write property-based tests capturing that behavior.

**Test Cases**:
1. **Non-self tenants_only discard**: Verify that messages from non-tenant, non-self phones continue to be discarded
2. **Non-self allowlist discard**: Verify that messages from non-allowlisted, non-self phones continue to be discarded
3. **Tenant messages allowed**: Verify that messages from registered tenants continue to be processed
4. **Chatbot disabled discard**: Verify that messages when `activo = false` continue to be discarded regardless of self-message status

### Unit Tests

- Test the self-message detection logic: `session_phone == sender_phone` comparison
- Test backward compatibility: `session_phone = None` falls through to normal policy check
- Test phone normalization edge cases: ensure comparison handles E.164 format consistently
- Test that `is_sender_allowed` and `check_sender_policy_no_db` remain unchanged (existing PBT tests pass)

### Property-Based Tests

- Generate random (sender_phone, session_phone, sender_policy, activo) tuples and verify:
  - When `sender_phone == session_phone` AND `activo == true`: message is never discarded by policy
  - When `sender_phone != session_phone`: result matches original `is_sender_allowed` behavior exactly
  - When `activo == false`: message is always discarded regardless of phone match
- Generate random E.164 phone numbers to test the comparison logic across the phone number space

### Integration Tests

- End-to-end test: baileys-service forwards a self-message with `sessionPhone` field → backend processes it → AI reply is sent back
- Test the full pipeline: self-message → bypass policy → load history → invoke AI → persist messages → send reply
- Test that the baileys-service `sentMessageIds` filter still prevents echo loops when the bot replies to itself
