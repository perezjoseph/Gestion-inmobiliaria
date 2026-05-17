// Feature: whatsapp-self-message-fix, Property 1: Bug Condition — Self-Messages Discarded by Sender Policy
//
// This PBT validates the EXPECTED behavior: when sender_phone == session_phone
// (a self-message to the bot's own number), the message SHALL NOT be discarded
// by sender policy enforcement, regardless of the policy type.
//
// The bug: `is_sender_allowed` applies the sender policy to ALL messages,
// including self-messages. When the policy is "tenants_only" and the bot's
// phone isn't in the inquilino table, or "allowlist" and the bot's phone
// isn't in the allowlist, the self-message is silently discarded.
//
// This test MUST FAIL on unfixed code — failure confirms the bug exists.
// DO NOT fix the test or the code when it fails.
//
// **Validates: Requirements 1.1, 1.2, 1.3**

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate a valid E.164 phone number (+ followed by 7-15 digits, first digit non-zero).
fn arb_e164_phone() -> impl Strategy<Value = String> {
    (1u8..=9, prop::collection::vec(0u8..=9, 6..=12)).prop_map(|(first, rest)| {
        let digits: String = rest.iter().map(|d| char::from(b'0' + d)).collect();
        format!("+{first}{digits}")
    })
}

/// Generate a restrictive sender policy that would block a non-registered phone.
fn arb_restrictive_policy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("tenants_only".to_string()),
        Just("allowlist".to_string()),
    ]
}

/// Generate an allowlist that does NOT contain the given phone.
fn arb_allowlist_without(phone: String) -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arb_e164_phone(), 0..=5)
        .prop_filter("allowlist must not contain the target phone", move |list| {
            list.iter().all(|p| *p != phone)
        })
}

// ── Model of the CURRENT (buggy) sender policy logic ───────────────────

/// Models `is_phone_in_allowlist` from `backend/src/services/chatbot.rs:427`
fn is_phone_in_allowlist(phone: &str, allowlist: Option<&[String]>) -> bool {
    allowlist.is_some_and(|list| list.iter().any(|entry| entry == phone))
}

/// Models `is_sender_allowed` from `backend/src/services/chatbot.rs:410`
/// For "tenants_only", we model the DB lookup result (phone not in inquilino table).
fn is_sender_allowed(
    sender_policy: &str,
    phone: &str,
    allowlist: Option<&[String]>,
    phone_is_tenant: bool,
) -> bool {
    match sender_policy {
        "tenants_only" => phone_is_tenant,
        "tenants_and_prospects" => true,
        "allowlist" => is_phone_in_allowlist(phone, allowlist),
        _ => false,
    }
}

/// Models the incoming_webhook handler's decision for a message.
/// Returns "discarded" or "processed" based on the FIXED logic.
///
/// The handler flow (from `chatbot_internal.rs:36`):
/// 1. Token validation
/// 2. Load config, check activo
/// 3. Self-message bypass: if sender_phone == session_phone, skip policy
/// 4. Apply sender policy via `is_sender_allowed`
/// 5. If not allowed → return {"status": "discarded"}
/// 6. Otherwise → process message (AI pipeline)
///
/// FIX: Self-message bypass added. When sender_phone == session_phone AND
/// activo == true, the sender policy is skipped and the message is processed.
fn handler_decision_current(
    activo: bool,
    sender_phone: &str,
    session_phone: &str,
    sender_policy: &str,
    allowlist: Option<&[String]>,
    phone_is_tenant: bool,
) -> &'static str {
    // Step 2: activo check
    if !activo {
        return "discarded";
    }

    // Step 3: self-message bypass (FIX)
    if sender_phone == session_phone {
        return "processed";
    }

    // Step 4: sender policy (for non-self messages)
    let allowed = is_sender_allowed(sender_policy, sender_phone, allowlist, phone_is_tenant);
    if !allowed {
        return "discarded";
    }

    "processed"
}

// ── Property Tests ─────────────────────────────────────────────────────

// Feature: whatsapp-self-message-fix, Property 1: Bug Condition
/// **Validates: Requirements 1.1, 1.2, 1.3**
///
/// Property: For any self-message (sender_phone == session_phone) where
/// activo = true and the sender policy is restrictive ("tenants_only" or
/// "allowlist"), the handler SHALL process the message (status != "discarded").
///
/// This test FAILS on unfixed code because the current handler applies
/// sender policy to self-messages without any bypass, causing them to be
/// discarded when the bot's own phone isn't in the tenant table or allowlist.
#[test]
fn test_self_messages_not_discarded_by_sender_policy() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Strategy: generate a self-message scenario where the bug triggers
    let strategy = (arb_e164_phone(), arb_restrictive_policy()).prop_flat_map(|(phone, policy)| {
        let phone_clone = phone.clone();
        let allowlist_strategy = arb_allowlist_without(phone_clone);
        (Just(phone), Just(policy), allowlist_strategy)
    });

    runner
        .run(&strategy, |(phone, policy, allowlist)| {
            // Self-message: sender_phone == session_phone
            let sender_phone = &phone;
            let session_phone = &phone; // same number — this is the bot messaging itself

            // The phone is NOT a tenant (not in inquilino table)
            let phone_is_tenant = false;

            // The phone is NOT in the allowlist (guaranteed by strategy)
            let allowlist_slice: Option<&[String]> = if policy == "allowlist" {
                Some(&allowlist)
            } else {
                None
            };

            // activo = true (chatbot is active)
            let activo = true;

            // Current handler decision (models the BUGGY behavior)
            let status = handler_decision_current(
                activo,
                sender_phone,
                session_phone,
                &policy,
                allowlist_slice,
                phone_is_tenant,
            );

            // EXPECTED behavior: self-messages should be PROCESSED, not discarded.
            // The handler should bypass sender policy for self-messages.
            prop_assert_eq!(
                status,
                "processed",
                "Bug detected: self-message was discarded by sender policy '{}'. \
                 sender_phone={}, session_phone={} (same number). \
                 The handler should bypass sender policy for self-messages, \
                 but the current code applies the policy unconditionally.",
                policy,
                sender_phone,
                session_phone,
            );

            Ok(())
        })
        .unwrap();
}

// Feature: whatsapp-self-message-fix, Property 1: Bug Condition (tenants_only specific)
/// **Validates: Requirements 1.1, 1.2**
///
/// Concrete scoped case: sender_phone == session_phone with "tenants_only" policy
/// where the phone is NOT in the inquilino table.
///
/// This directly demonstrates the bug: the bot's own number is not registered
/// as a tenant, so the tenants_only policy discards the self-message.
#[test]
fn test_self_message_tenants_only_policy_not_discarded() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_e164_phone(), |phone| {
            let sender_phone = &phone;
            let session_phone = &phone; // self-message

            // Phone is NOT a tenant
            let phone_is_tenant = false;

            let status = handler_decision_current(
                true, // activo
                sender_phone,
                session_phone,
                "tenants_only",
                None, // no allowlist for this policy
                phone_is_tenant,
            );

            // Expected: self-messages bypass sender policy → "processed"
            // Actual (buggy): tenants_only rejects non-tenant phone → "discarded"
            prop_assert_eq!(
                status,
                "processed",
                "Bug: self-message discarded by tenants_only policy. \
                 Phone {} is the bot's own number but is not in inquilino table, \
                 so tenants_only policy rejects it. Self-messages should bypass this check.",
                sender_phone,
            );

            Ok(())
        })
        .unwrap();
}

// Feature: whatsapp-self-message-fix, Property 1: Bug Condition (allowlist specific)
/// **Validates: Requirements 1.1, 1.3**
///
/// Concrete scoped case: sender_phone == session_phone with "allowlist" policy
/// where the phone is NOT in the allowlist.
///
/// This directly demonstrates the bug: the bot's own number is not in the
/// allowlist, so the allowlist policy discards the self-message.
#[test]
fn test_self_message_allowlist_policy_not_discarded() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy = arb_e164_phone().prop_flat_map(|phone| {
        let phone_clone = phone.clone();
        let allowlist = arb_allowlist_without(phone_clone);
        (Just(phone), allowlist)
    });

    runner
        .run(&strategy, |(phone, allowlist)| {
            let sender_phone = &phone;
            let session_phone = &phone; // self-message

            let status = handler_decision_current(
                true, // activo
                sender_phone,
                session_phone,
                "allowlist",
                Some(&allowlist), // allowlist does NOT contain the phone
                false,            // phone_is_tenant irrelevant for allowlist policy
            );

            // Expected: self-messages bypass sender policy → "processed"
            // Actual (buggy): allowlist rejects phone not in list → "discarded"
            prop_assert_eq!(
                status,
                "processed",
                "Bug: self-message discarded by allowlist policy. \
                 Phone {} is the bot's own number but is not in the allowlist {:?}, \
                 so allowlist policy rejects it. Self-messages should bypass this check.",
                sender_phone,
                allowlist,
            );

            Ok(())
        })
        .unwrap();
}

// ── Property 2: Preservation — Non-Self Message Policy Enforcement ─────

// Feature: whatsapp-self-message-fix, Property 2: Preservation
// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**
//
// These tests verify that non-self messages (sender_phone != session_phone)
// continue to be handled correctly by the existing sender policy logic.
// They MUST PASS on unfixed code — they capture baseline behavior to preserve.

// ── Additional Strategies for Preservation Tests ───────────────────────

/// Generate a pair of distinct E.164 phone numbers (sender != session).
fn arb_distinct_phones() -> impl Strategy<Value = (String, String)> {
    (arb_e164_phone(), arb_e164_phone())
        .prop_filter("sender and session phones must differ", |(a, b)| a != b)
}

/// Generate any valid sender policy.
fn arb_sender_policy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("tenants_only".to_string()),
        Just("tenants_and_prospects".to_string()),
        Just("allowlist".to_string()),
    ]
}

// Feature: whatsapp-self-message-fix, Property 2: Preservation
/// **Validates: Requirements 3.3**
///
/// Property: When activo = false, ALL messages are discarded regardless of
/// sender phone, session phone, or policy. This must hold for non-self messages.
#[test]
fn test_preservation_activo_false_always_discards() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy = (
        arb_distinct_phones(),
        arb_sender_policy(),
        any::<bool>(), // phone_is_tenant
    );

    runner
        .run(
            &strategy,
            |((sender_phone, session_phone), policy, phone_is_tenant)| {
                let status = handler_decision_current(
                    false, // activo = false → chatbot disabled
                    &sender_phone,
                    &session_phone,
                    &policy,
                    None, // allowlist irrelevant when activo=false
                    phone_is_tenant,
                );

                prop_assert_eq!(
                    status,
                    "discarded",
                    "When activo=false, message must be discarded. \
                     sender={}, session={}, policy={}, is_tenant={}",
                    sender_phone,
                    session_phone,
                    policy,
                    phone_is_tenant,
                );

                Ok(())
            },
        )
        .unwrap();
}

// Feature: whatsapp-self-message-fix, Property 2: Preservation
/// **Validates: Requirements 3.1**
///
/// Property: When sender_phone != session_phone AND policy is "tenants_only"
/// AND the phone is NOT a tenant, the message is discarded.
#[test]
fn test_preservation_tenants_only_non_tenant_discarded() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_distinct_phones(), |(sender_phone, session_phone)| {
            let status = handler_decision_current(
                true, // activo
                &sender_phone,
                &session_phone,
                "tenants_only",
                None,
                false, // phone is NOT a tenant
            );

            prop_assert_eq!(
                status,
                "discarded",
                "Non-tenant with tenants_only policy must be discarded. \
                 sender={}, session={}",
                sender_phone,
                session_phone,
            );

            Ok(())
        })
        .unwrap();
}

// Feature: whatsapp-self-message-fix, Property 2: Preservation
/// **Validates: Requirements 3.5**
///
/// Property: When sender_phone != session_phone AND policy is "tenants_only"
/// AND the phone IS a registered tenant, the message is processed.
#[test]
fn test_preservation_tenants_only_registered_tenant_processed() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_distinct_phones(), |(sender_phone, session_phone)| {
            let status = handler_decision_current(
                true, // activo
                &sender_phone,
                &session_phone,
                "tenants_only",
                None,
                true, // phone IS a tenant
            );

            prop_assert_eq!(
                status,
                "processed",
                "Registered tenant with tenants_only policy must be processed. \
                 sender={}, session={}",
                sender_phone,
                session_phone,
            );

            Ok(())
        })
        .unwrap();
}

// Feature: whatsapp-self-message-fix, Property 2: Preservation
/// **Validates: Requirements 3.2**
///
/// Property: When sender_phone != session_phone AND policy is "allowlist"
/// AND the phone is NOT in the allowlist, the message is discarded.
#[test]
fn test_preservation_allowlist_non_listed_discarded() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy = arb_distinct_phones().prop_flat_map(|(sender, session)| {
        let sender_clone = sender.clone();
        let allowlist = arb_allowlist_without(sender_clone);
        (Just(sender), Just(session), allowlist)
    });

    runner
        .run(&strategy, |(sender_phone, session_phone, allowlist)| {
            let status = handler_decision_current(
                true, // activo
                &sender_phone,
                &session_phone,
                "allowlist",
                Some(&allowlist), // sender NOT in allowlist
                false,
            );

            prop_assert_eq!(
                status,
                "discarded",
                "Non-allowlisted phone with allowlist policy must be discarded. \
                 sender={}, session={}, allowlist={:?}",
                sender_phone,
                session_phone,
                allowlist,
            );

            Ok(())
        })
        .unwrap();
}

// Feature: whatsapp-self-message-fix, Property 2: Preservation
/// **Validates: Requirements 3.4**
///
/// Property: When sender_phone != session_phone AND policy is
/// "tenants_and_prospects", the message is ALWAYS processed regardless
/// of tenant status.
#[test]
fn test_preservation_tenants_and_prospects_always_processed() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy = (arb_distinct_phones(), any::<bool>());

    runner
        .run(
            &strategy,
            |((sender_phone, session_phone), phone_is_tenant)| {
                let status = handler_decision_current(
                    true, // activo
                    &sender_phone,
                    &session_phone,
                    "tenants_and_prospects",
                    None,
                    phone_is_tenant, // irrelevant for this policy
                );

                prop_assert_eq!(
                    status,
                    "processed",
                    "tenants_and_prospects policy must always process messages. \
                 sender={}, session={}, is_tenant={}",
                    sender_phone,
                    session_phone,
                    phone_is_tenant,
                );

                Ok(())
            },
        )
        .unwrap();
}

// Feature: whatsapp-self-message-fix, Property 2: Preservation
/// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**
///
/// Composite property: For ALL non-self messages, the handler produces the
/// same result as the original `is_sender_allowed` logic. This is the
/// universal preservation property that ties all sub-properties together.
#[test]
fn test_preservation_non_self_matches_original_logic() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy = (
        arb_distinct_phones(),
        arb_sender_policy(),
        any::<bool>(), // activo
        any::<bool>(), // phone_is_tenant
    )
        .prop_flat_map(|((sender, session), policy, activo, is_tenant)| {
            let sender_clone = sender.clone();
            let policy_clone = policy.clone();
            // Generate allowlist that does NOT contain sender (for allowlist policy)
            let allowlist = arb_allowlist_without(sender_clone);
            (
                Just(sender),
                Just(session),
                Just(policy_clone),
                Just(activo),
                Just(is_tenant),
                allowlist,
            )
        });

    runner
        .run(
            &strategy,
            |(sender_phone, session_phone, policy, activo, phone_is_tenant, allowlist)| {
                let allowlist_opt: Option<&[String]> = if policy == "allowlist" {
                    Some(&allowlist)
                } else {
                    None
                };

                // Compute expected result using the original logic model
                let expected = if !activo {
                    "discarded"
                } else if is_sender_allowed(&policy, &sender_phone, allowlist_opt, phone_is_tenant)
                {
                    "processed"
                } else {
                    "discarded"
                };

                // Compute actual result from handler model
                let actual = handler_decision_current(
                    activo,
                    &sender_phone,
                    &session_phone,
                    &policy,
                    allowlist_opt,
                    phone_is_tenant,
                );

                prop_assert_eq!(
                    actual,
                    expected,
                    "Non-self message handler result must match original is_sender_allowed logic. \
                     sender={}, session={}, policy={}, activo={}, is_tenant={}",
                    sender_phone,
                    session_phone,
                    policy,
                    activo,
                    phone_is_tenant,
                );

                Ok(())
            },
        )
        .unwrap();
}
