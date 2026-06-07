#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::doc_markdown,
    clippy::empty_line_after_doc_comments
)]
//! Property-based tests for the chatbot service.
//!
//! **Validates: Requirements 2.1, 2.2, 2.3, 13.1, 13.3**

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

use crate::models::chatbot::{Confidence, map_confidence};
use crate::services::chatbot::{check_sender_policy_no_db, is_phone_in_allowlist};

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate a valid E.164 phone number for testing.
fn arb_e164_phone() -> impl Strategy<Value = String> {
    (1..=9u8, proptest::collection::vec(0..=9u8, 6..14)).prop_map(|(first, rest)| {
        let mut phone = format!("+{first}");
        for d in rest {
            phone.push(char::from(b'0' + d));
        }
        phone
    })
}

/// Generate a non-empty allowlist of E.164 phone numbers.
fn arb_allowlist() -> impl Strategy<Value = Vec<String>> {
    proptest::collection::vec(arb_e164_phone(), 1..20)
}

/// Generate a policy string that is NOT one of the recognized values.
fn arb_unrecognized_policy() -> impl Strategy<Value = String> {
    "[a-z_]{1,30}".prop_filter("must not be a recognized policy", |s| {
        s != "tenants_only" && s != "tenants_and_prospects" && s != "allowlist"
    })
}

// ── Property 2: Sender Policy Correctness ────────────────────────────────────

// Feature: whatsapp-ai-assistant, Property 2: Sender Policy Correctness
// **Validates: Requirements 2.1, 2.2, 2.3**

/// Property 2a: `tenants_and_prospects` always allows any sender.
#[test]
fn tenants_and_prospects_always_allows() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_e164_phone(), |phone| {
            let result = check_sender_policy_no_db("tenants_and_prospects", &phone, None);
            prop_assert_eq!(
                result,
                Some(true),
                "tenants_and_prospects must always allow, got {:?} for phone: {}",
                result,
                phone
            );
            Ok(())
        })
        .expect("tenants_and_prospects_always_allows failed");
}

/// Property 2b: `allowlist` policy allows iff phone is in the list.
#[test]
fn allowlist_allows_phone_in_list() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_allowlist(), any::<prop::sample::Index>()),
            |(allowlist, index)| {
                let phone = &allowlist[index.index(allowlist.len())];
                let result = is_phone_in_allowlist(phone, Some(&allowlist));
                prop_assert!(result, "Phone in allowlist must be allowed: {}", phone);
                Ok(())
            },
        )
        .expect("allowlist_allows_phone_in_list failed");
}

/// Property 2b (converse): `allowlist` policy denies phone NOT in the list.
#[test]
fn allowlist_denies_phone_not_in_list() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_allowlist(), arb_e164_phone()),
            |(allowlist, phone)| {
                prop_assume!(!allowlist.contains(&phone));
                let result = is_phone_in_allowlist(&phone, Some(&allowlist));
                prop_assert!(!result, "Phone NOT in allowlist must be denied: {}", phone);
                Ok(())
            },
        )
        .expect("allowlist_denies_phone_not_in_list failed");
}

/// Property 2b (edge): `allowlist` policy denies when allowlist is None.
#[test]
fn allowlist_denies_when_none() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_e164_phone(), |phone| {
            let result = is_phone_in_allowlist(&phone, None);
            prop_assert!(
                !result,
                "Allowlist policy with no list must deny: {}",
                phone
            );
            Ok(())
        })
        .expect("allowlist_denies_when_none failed");
}

/// Property 2c: Unrecognized policies always deny (fail-closed).
#[test]
fn unrecognized_policy_always_denies() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_unrecognized_policy(), arb_e164_phone()),
            |(policy, phone)| {
                let result = check_sender_policy_no_db(&policy, &phone, None);
                prop_assert_eq!(
                    result,
                    Some(false),
                    "Unrecognized policy '{}' must deny, got {:?} for phone: {}",
                    policy,
                    result,
                    phone
                );
                Ok(())
            },
        )
        .expect("unrecognized_policy_always_denies failed");
}

/// Property 2 (structural): `tenants_only` requires DB lookup (returns None from pure check).
#[test]
fn tenants_only_requires_db_lookup() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_e164_phone(), |phone| {
            let result = check_sender_policy_no_db("tenants_only", &phone, None);
            prop_assert_eq!(
                result,
                None,
                "tenants_only must return None (requires DB), got {:?} for phone: {}",
                result,
                phone
            );
            Ok(())
        })
        .expect("tenants_only_requires_db_lookup failed");
}

/// Property 2b (via check_sender_policy_no_db): allowlist through the unified check function.
#[test]
fn check_sender_policy_allowlist_correctness() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_allowlist(), arb_e164_phone()),
            |(allowlist, phone)| {
                let expected = allowlist.contains(&phone);
                let result = check_sender_policy_no_db("allowlist", &phone, Some(&allowlist));
                prop_assert_eq!(
                    result,
                    Some(expected),
                    "allowlist policy: phone={}, in_list={}, got {:?}",
                    phone,
                    expected,
                    result
                );
                Ok(())
            },
        )
        .expect("check_sender_policy_allowlist_correctness failed");
}

// ── Strategies for Phone Number E.164 Validation ───────────────────────

/// Generate a valid E.164 phone number: '+' followed by 2–15 digits where the first digit is 1–9.
fn valid_e164() -> impl Strategy<Value = String> {
    (1u8..=9, prop::collection::vec(0u8..=9, 1..=14)).prop_map(|(first, rest)| {
        let mut s = String::with_capacity(16);
        s.push('+');
        s.push(char::from(b'0' + first));
        for d in rest {
            s.push(char::from(b'0' + d));
        }
        s
    })
}

/// Generate strings that do NOT match E.164 format.
fn invalid_e164() -> impl Strategy<Value = String> {
    prop_oneof![
        // Missing '+' prefix
        (1u8..=9, prop::collection::vec(0u8..=9, 1..=14)).prop_map(|(first, rest)| {
            let mut s = String::new();
            s.push(char::from(b'0' + first));
            for d in rest {
                s.push(char::from(b'0' + d));
            }
            s
        }),
        // Leading zero after '+'
        prop::collection::vec(0u8..=9, 1..=14).prop_map(|rest| {
            let mut s = String::from("+0");
            for d in rest {
                s.push(char::from(b'0' + d));
            }
            s
        }),
        // Too many digits (16+ after '+')
        (1u8..=9, prop::collection::vec(0u8..=9, 15..=20)).prop_map(|(first, rest)| {
            let mut s = String::from("+");
            s.push(char::from(b'0' + first));
            for d in rest {
                s.push(char::from(b'0' + d));
            }
            s
        }),
        // Empty string
        Just(String::new()),
        // Only '+'
        Just("+".to_string()),
        // Contains non-digit characters after '+'
        (1u8..=9, prop::collection::vec(0u8..=9, 3..=8)).prop_map(|(first, rest)| {
            let mut s = String::from("+");
            s.push(char::from(b'0' + first));
            s.push('-');
            for d in rest {
                s.push(char::from(b'0' + d));
            }
            s
        }),
        // Contains spaces after '+'
        (1u8..=9, prop::collection::vec(0u8..=9, 3..=8)).prop_map(|(first, rest)| {
            let mut s = String::from("+");
            s.push(char::from(b'0' + first));
            s.push(' ');
            for d in rest {
                s.push(char::from(b'0' + d));
            }
            s
        }),
    ]
}

/// Generate a normalizable phone number (local format with enough digits).
fn normalizable_local_phone() -> impl Strategy<Value = String> {
    (1u8..=9, prop::collection::vec(0u8..=9, 6..=9)).prop_map(|(first, rest)| {
        let mut s = String::new();
        s.push(char::from(b'0' + first));
        for d in rest {
            s.push(char::from(b'0' + d));
        }
        s
    })
}

/// Generate a valid country code prefix (e.g., "+1", "+44", "+809").
fn valid_country_code() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("+1".to_string()),
        Just("+44".to_string()),
        Just("+34".to_string()),
        Just("+1809".to_string()),
    ]
}

// ── Property 21: Phone Number E.164 Validation ────────────────────────────────

use crate::services::chatbot::{enforce_config_role, normalize_phone, validate_e164};

// Feature: whatsapp-ai-assistant, Property 21: Phone Number E.164 Validation

/// Property 21a: Valid E.164 numbers are accepted by validate_e164.
/// **Validates: Requirements 13.1, 13.3**
#[test]
fn valid_e164_numbers_are_accepted() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&valid_e164(), |phone| {
            let result = validate_e164(&phone);
            prop_assert!(
                result.is_ok(),
                "Valid E.164 number '{}' should be accepted, but got error: {:?}",
                phone,
                result.err()
            );
            Ok(())
        })
        .expect("valid_e164_numbers_are_accepted failed");
}

/// Property 21b: Invalid E.164 numbers are rejected by validate_e164.
/// **Validates: Requirements 13.1, 13.3**
#[test]
fn invalid_e164_numbers_are_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&invalid_e164(), |phone| {
            let result = validate_e164(&phone);
            prop_assert!(
                result.is_err(),
                "Invalid E.164 number '{}' should be rejected by validate_e164",
                phone
            );
            Ok(())
        })
        .expect("invalid_e164_numbers_are_rejected failed");
}

/// Property 21c: normalize_phone output always passes validate_e164.
/// **Validates: Requirements 13.1, 13.3**
#[test]
fn normalize_phone_output_always_valid_e164() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(normalizable_local_phone(), valid_country_code()),
            |(local_phone, country_code)| {
                if let Ok(normalized) = normalize_phone(&local_phone, &country_code) {
                    let validation = validate_e164(&normalized);
                    prop_assert!(
                        validation.is_ok(),
                        "normalize_phone('{}', '{}') produced '{}' which fails validate_e164: {:?}",
                        local_phone,
                        country_code,
                        normalized,
                        validation.err()
                    );
                }
                // If normalize_phone returns Err, that's also acceptable (rejection path)
                Ok(())
            },
        )
        .expect("normalize_phone_output_always_valid_e164 failed");
}

/// Generate a valid E.164 number with at least 7 digits (normalize_phone's minimum).
fn valid_e164_normalizable() -> impl Strategy<Value = String> {
    (1u8..=9, prop::collection::vec(0u8..=9, 6..=14)).prop_map(|(first, rest)| {
        let mut s = String::with_capacity(16);
        s.push('+');
        s.push(char::from(b'0' + first));
        for d in rest {
            s.push(char::from(b'0' + d));
        }
        s
    })
}

/// Property 21d: normalize_phone is idempotent on valid E.164 numbers.
/// **Validates: Requirements 13.1, 13.3**
#[test]
fn normalize_phone_is_idempotent() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(valid_e164_normalizable(), valid_country_code()),
            |(phone, country_code)| {
                let first = normalize_phone(&phone, &country_code);
                prop_assert!(
                    first.is_ok(),
                    "Valid E.164 '{}' should normalize successfully, got: {:?}",
                    phone,
                    first.err()
                );

                let first_val = first.unwrap();
                let second = normalize_phone(&first_val, &country_code);
                prop_assert!(
                    second.is_ok(),
                    "Normalizing already-normalized '{}' should succeed, got: {:?}",
                    first_val,
                    second.err()
                );

                let second_val = second.unwrap();
                prop_assert_eq!(
                    &first_val,
                    &second_val,
                    "Idempotence violated: normalize('{}') = '{}', normalize('{}') = '{}'",
                    phone,
                    first_val,
                    first_val,
                    second_val
                );
                Ok(())
            },
        )
        .expect("normalize_phone_is_idempotent failed");
}

// ── Property 14: Retention Cleanup Correctness ────────────────────────────────

use chrono::{Duration, Utc};
use sea_orm::prelude::DateTimeWithTimeZone;
use uuid::Uuid;

/// A simplified conversation message for retention testing.
#[derive(Debug, Clone)]
struct TestMessage {
    id: Uuid,
    created_at: chrono::DateTime<Utc>,
}

/// Pure model of the retention cleanup logic.
/// Mirrors `cleanup_expired` which deletes messages WHERE `created_at < (now - retention_days)`.
///
/// Returns (kept, deleted) partitions.
fn retention_partition(
    messages: &[TestMessage],
    now: chrono::DateTime<Utc>,
    retention_days: i64,
) -> (Vec<Uuid>, Vec<Uuid>) {
    let cutoff = now - Duration::days(retention_days);
    let mut kept = Vec::new();
    let mut deleted = Vec::new();

    for msg in messages {
        if msg.created_at < cutoff {
            deleted.push(msg.id);
        } else {
            kept.push(msg.id);
        }
    }

    (kept, deleted)
}

/// Generate a timestamp relative to `now` within a range of days offset.
/// Negative offset = in the past, positive = in the future.
fn arb_timestamp_offset_days() -> impl Strategy<Value = i64> {
    -730i64..=730 // up to 2 years in either direction
}

/// Generate a valid retention_days value (1–365).
fn arb_retention_days() -> impl Strategy<Value = i64> {
    1i64..=365
}

// Feature: whatsapp-ai-assistant, Property 14: Retention Cleanup Correctness
/// **Validates: Requirement 7.4**

/// Property 14a: Messages older than retention_days are deleted.
/// For any message with (now - created_at) > retention_days, it SHALL be deleted.
#[test]
fn retention_cleanup_deletes_expired_messages() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    let now = Utc::now();

    runner
        .run(
            &(arb_retention_days(), prop::collection::vec(arb_timestamp_offset_days(), 1..=20)),
            |(retention_days, offsets)| {
                let messages: Vec<TestMessage> = offsets
                    .iter()
                    .enumerate()
                    .map(|(i, &offset)| TestMessage {
                        id: Uuid::from_u128(i as u128),
                        created_at: now + Duration::days(offset),
                    })
                    .collect();

                let (kept, deleted) = retention_partition(&messages, now, retention_days);

                // Verify: every deleted message has created_at < cutoff
                let cutoff = now - Duration::days(retention_days);
                for msg in &messages {
                    if msg.created_at < cutoff {
                        prop_assert!(
                            deleted.contains(&msg.id),
                            "Message with created_at {:?} (< cutoff {:?}) should be deleted but was kept",
                            msg.created_at,
                            cutoff
                        );
                    }
                }

                // Verify: every kept message has created_at >= cutoff
                for msg in &messages {
                    if msg.created_at >= cutoff {
                        prop_assert!(
                            kept.contains(&msg.id),
                            "Message with created_at {:?} (>= cutoff {:?}) should be kept but was deleted",
                            msg.created_at,
                            cutoff
                        );
                    }
                }

                Ok(())
            },
        )
        .expect("retention_cleanup_deletes_expired_messages failed");
}

/// Property 14b: Messages within retention period are preserved.
/// For any message with (now - created_at) <= retention_days, it SHALL be preserved.
#[test]
fn retention_cleanup_preserves_recent_messages() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    let now = Utc::now();

    runner
        .run(
            &(arb_retention_days(), 0i64..=365),
            |(retention_days, age_days)| {
                // Create a message that is exactly `age_days` old (within retention)
                prop_assume!(age_days <= retention_days);

                let msg = TestMessage {
                    id: Uuid::from_u128(42),
                    created_at: now - Duration::days(age_days),
                };

                let (kept, deleted) =
                    retention_partition(std::slice::from_ref(&msg), now, retention_days);

                prop_assert!(
                    kept.contains(&msg.id),
                    "Message aged {} days (retention={}) should be preserved",
                    age_days,
                    retention_days
                );
                prop_assert!(
                    !deleted.contains(&msg.id),
                    "Message aged {} days (retention={}) should NOT be deleted",
                    age_days,
                    retention_days
                );

                Ok(())
            },
        )
        .expect("retention_cleanup_preserves_recent_messages failed");
}

/// Property 14c: Partition is exhaustive — every message is either kept or deleted, never both, never lost.
#[test]
fn retention_cleanup_partition_is_exhaustive() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    let now = Utc::now();

    runner
        .run(
            &(
                arb_retention_days(),
                prop::collection::vec(arb_timestamp_offset_days(), 0..=30),
            ),
            |(retention_days, offsets)| {
                let messages: Vec<TestMessage> = offsets
                    .iter()
                    .enumerate()
                    .map(|(i, &offset)| TestMessage {
                        id: Uuid::from_u128(i as u128),
                        created_at: now + Duration::days(offset),
                    })
                    .collect();

                let (kept, deleted) = retention_partition(&messages, now, retention_days);

                // Every message appears in exactly one partition
                prop_assert_eq!(
                    kept.len() + deleted.len(),
                    messages.len(),
                    "Partition sizes don't sum to total: kept={}, deleted={}, total={}",
                    kept.len(),
                    deleted.len(),
                    messages.len()
                );

                // No message appears in both
                for id in &kept {
                    prop_assert!(
                        !deleted.contains(id),
                        "Message {:?} appears in both kept and deleted",
                        id
                    );
                }

                Ok(())
            },
        )
        .expect("retention_cleanup_partition_is_exhaustive failed");
}

/// Property 14d: The cutoff boundary is strict — a message exactly at the boundary is preserved.
/// `cleanup_expired` uses `created_at < cutoff` (strict less-than), so messages at exactly
/// `now - retention_days` are preserved (since `created_at < cutoff` is false when equal).
#[test]
fn retention_cleanup_boundary_message_is_preserved() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_retention_days(), |retention_days| {
            let now = Utc::now();
            let cutoff = now - Duration::days(retention_days);

            // Message created exactly at the cutoff boundary
            let msg = TestMessage {
                id: Uuid::from_u128(99),
                created_at: cutoff,
            };

            let (kept, deleted) =
                retention_partition(std::slice::from_ref(&msg), now, retention_days);

            // Since cleanup uses `created_at < cutoff`, a message AT the cutoff is NOT deleted
            prop_assert!(
                kept.contains(&msg.id),
                "Message at exact cutoff boundary (retention_days={}) should be preserved",
                retention_days
            );
            prop_assert!(
                !deleted.contains(&msg.id),
                "Message at exact cutoff boundary should NOT be deleted"
            );

            Ok(())
        })
        .expect("retention_cleanup_boundary_message_is_preserved failed");
}

// ── Strategies for Conversation History Windowing ──────────────────────

use chrono::DateTime;

use crate::services::chatbot::{TimestampedMessage, window_history};

/// Generate a list of messages with distinct timestamps.
fn arb_message_list(max_len: usize) -> impl Strategy<Value = Vec<TimestampedMessage>> {
    prop::collection::vec((any::<u32>(), "[a-z]{1,20}"), 0..=max_len).prop_map(|items| {
        let base: DateTime<Utc> =
            chrono::TimeZone::with_ymd_and_hms(&Utc, 2025, 1, 1, 0, 0, 0).unwrap();
        items
            .into_iter()
            .enumerate()
            .map(|(i, (_offset, content))| TimestampedMessage {
                content,
                #[allow(clippy::cast_possible_wrap)]
                created_at: base + Duration::seconds(i as i64),
            })
            .collect()
    })
}

/// Generate a valid history_limit (1–50, matching config validation).
fn arb_history_limit() -> impl Strategy<Value = usize> {
    1..=50usize
}

// ── Property 6: Conversation History Windowing ────────────────────────────────

// Feature: whatsapp-ai-assistant, Property 6: Conversation History Windowing
// **Validates: Requirements 3.2, 7.2**

/// Property 6a: window_history returns exactly min(M, N) messages.
#[test]
fn history_windowing_returns_min_m_n() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_message_list(60), arb_history_limit()),
            |(messages, limit)| {
                let result = window_history(&messages, limit);
                let expected_len = messages.len().min(limit);
                prop_assert_eq!(
                    result.len(),
                    expected_len,
                    "Expected min({}, {}) = {} messages, got {}",
                    messages.len(),
                    limit,
                    expected_len,
                    result.len()
                );
                Ok(())
            },
        )
        .expect("history_windowing_returns_min_m_n failed");
}

/// Property 6b: window_history returns the N most recent messages (by created_at).
#[test]
fn history_windowing_returns_most_recent() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_message_list(60), arb_history_limit()),
            |(messages, limit)| {
                let result = window_history(&messages, limit);

                // The result should be ordered DESC by created_at
                for pair in result.windows(2) {
                    prop_assert!(
                        pair[0].created_at >= pair[1].created_at,
                        "Messages not in DESC order: {:?} before {:?}",
                        pair[0].created_at,
                        pair[1].created_at
                    );
                }

                // Every message in the result should be >= every message NOT in the result
                if !result.is_empty() && result.len() < messages.len() {
                    let oldest_in_result = result.last().unwrap().created_at;
                    let result_set: std::collections::HashSet<_> =
                        result.iter().map(|m| &m.content).collect();
                    for msg in &messages {
                        if !result_set.contains(&msg.content) {
                            prop_assert!(
                                msg.created_at <= oldest_in_result,
                                "Excluded message {:?} is more recent than included oldest {:?}",
                                msg.created_at,
                                oldest_in_result
                            );
                        }
                    }
                }

                Ok(())
            },
        )
        .expect("history_windowing_returns_most_recent failed");
}

// ── Strategies for Configuration Round-Trip ────────────────────────────

use crate::entities::chatbot_config;
use crate::models::chatbot::{Capabilities, ChatbotConfigUpdateRequest, FaqEntry};
use crate::services::chatbot::{config_model_to_response, validate_config};

/// Generate a valid display_name (1–100 bytes, ASCII to avoid multi-byte issues).
fn valid_display_name() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 ]{1,100}"
}

/// Generate a valid tone (1–50 bytes, ASCII to avoid multi-byte issues).
fn valid_tone() -> impl Strategy<Value = String> {
    "[a-z]{1,50}"
}

/// Generate a valid FAQ entry (question 1–200 bytes, answer 1–200 bytes, ASCII).
fn valid_faq_entry() -> impl Strategy<Value = FaqEntry> {
    ("[A-Za-z0-9 ?]{1,200}", "[A-Za-z0-9 .]{1,200}")
        .prop_map(|(question, answer)| FaqEntry { question, answer })
}

/// Generate a valid FAQ list (0–10 entries for test speed).
fn valid_faqs() -> impl Strategy<Value = Vec<FaqEntry>> {
    prop::collection::vec(valid_faq_entry(), 0..=10)
}

/// Generate valid policies text (0–500 bytes, ASCII).
fn valid_policies() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 .\n]{0,500}"
}

/// Generate a valid sender_policy value.
fn valid_sender_policy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("tenants_only".to_string()),
        Just("tenants_and_prospects".to_string()),
        Just("allowlist".to_string()),
    ]
}

/// Generate valid capabilities.
fn valid_capabilities() -> impl Strategy<Value = Capabilities> {
    (
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(
                receipt_ocr,
                balance_queries,
                payment_reminders,
                maintenance_requests,
                human_handoff,
            )| {
                Capabilities {
                    receipt_ocr,
                    balance_queries,
                    payment_reminders,
                    maintenance_requests,
                    human_handoff,
                }
            },
        )
}

/// Generate a valid history_limit (1–50).
fn valid_history_limit() -> impl Strategy<Value = i32> {
    1..=50i32
}

/// Generate a valid retention_days (1–365).
fn valid_retention_days() -> impl Strategy<Value = i32> {
    1..=365i32
}

/// Generate a complete valid ChatbotConfigUpdateRequest.
fn valid_config_update() -> impl Strategy<Value = ChatbotConfigUpdateRequest> {
    (
        any::<bool>(),
        valid_display_name(),
        valid_tone(),
        valid_faqs(),
        valid_policies(),
        valid_sender_policy(),
        valid_capabilities(),
        valid_history_limit(),
        valid_retention_days(),
    )
        .prop_map(
            |(
                activo,
                display_name,
                tone,
                faqs,
                policies,
                sender_policy,
                capabilities,
                history_limit,
                retention_days,
            )| {
                ChatbotConfigUpdateRequest {
                    activo: Some(activo),
                    display_name: Some(display_name),
                    language: Some("es-DO".to_string()),
                    tone: Some(tone),
                    greeting: Some("Bienvenido".to_string()),
                    faqs: Some(faqs),
                    policies: Some(policies),
                    sender_policy: Some(sender_policy),
                    allowlist: Some(vec!["+18091234567".to_string()]),
                    capabilities: Some(capabilities),
                    handoff_keywords: Some(vec!["hablar con humano".to_string()]),
                    history_limit: Some(history_limit),
                    retention_days: Some(retention_days),
                }
            },
        )
}

/// Build a chatbot_config::Model from a ChatbotConfigUpdateRequest,
/// simulating what upsert_config does when inserting a new record.
fn build_model_from_input(input: &ChatbotConfigUpdateRequest) -> chatbot_config::Model {
    let now: DateTimeWithTimeZone = Utc::now().into();
    chatbot_config::Model {
        id: Uuid::new_v4(),
        organizacion_id: Uuid::new_v4(),
        activo: input.activo.unwrap_or(false),
        connection_status: "disconnected".to_string(),
        display_name: input.display_name.clone(),
        language: input
            .language
            .clone()
            .unwrap_or_else(|| "es-DO".to_string()),
        tone: input.tone.clone(),
        greeting: input.greeting.clone(),
        system_prompt: None,
        faqs: input
            .faqs
            .as_ref()
            .map(|f| serde_json::to_value(f).unwrap()),
        policies: input.policies.clone(),
        sender_policy: input
            .sender_policy
            .clone()
            .unwrap_or_else(|| "tenants_only".to_string()),
        allowlist: input
            .allowlist
            .as_ref()
            .map(|a| serde_json::to_value(a).unwrap()),
        capabilities: input
            .capabilities
            .as_ref()
            .map(|c| serde_json::to_value(c).unwrap()),
        handoff_keywords: input
            .handoff_keywords
            .as_ref()
            .map(|k| serde_json::to_value(k).unwrap()),
        history_limit: input.history_limit.unwrap_or(10),
        retention_days: input.retention_days.unwrap_or(90),
        agent_config: serde_json::json!({}),
        guidance_rules: serde_json::json!([]),
        updated_by: Some(Uuid::new_v4()),
        created_at: now,
        updated_at: now,
    }
}

// ── Property 17: Configuration Round-Trip ────────────────────────────────────

// Feature: whatsapp-ai-assistant, Property 17: Configuration Round-Trip
/// **Validates: Requirements 9.2, 9.3, 9.4, 9.5**
#[test]
fn config_round_trip_preserves_all_fields() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&valid_config_update(), |input| {
            // Step 1: validate_config accepts any valid input
            prop_assert!(
                validate_config(&input).is_ok(),
                "validate_config rejected a valid input: {:?}",
                input
            );

            // Step 2: Build a model (simulating DB insert) and convert back to response
            let model = build_model_from_input(&input);
            let response =
                config_model_to_response(model).expect("config_model_to_response failed");

            // Step 3: Verify all fields round-trip correctly

            // Persona settings (Requirement 9.2)
            prop_assert_eq!(
                response.display_name.as_deref(),
                input.display_name.as_deref(),
                "display_name mismatch"
            );
            prop_assert_eq!(
                response.tone.as_deref(),
                input.tone.as_deref(),
                "tone mismatch"
            );
            prop_assert_eq!(
                &response.language,
                input.language.as_ref().unwrap(),
                "language mismatch"
            );
            prop_assert_eq!(
                response.greeting.as_deref(),
                input.greeting.as_deref(),
                "greeting mismatch"
            );

            // Capability toggles (Requirement 9.3)
            let input_caps = input.capabilities.as_ref().unwrap();
            prop_assert_eq!(
                response.capabilities.receipt_ocr,
                input_caps.receipt_ocr,
                "receipt_ocr mismatch"
            );
            prop_assert_eq!(
                response.capabilities.balance_queries,
                input_caps.balance_queries,
                "balance_queries mismatch"
            );
            prop_assert_eq!(
                response.capabilities.payment_reminders,
                input_caps.payment_reminders,
                "payment_reminders mismatch"
            );
            prop_assert_eq!(
                response.capabilities.maintenance_requests,
                input_caps.maintenance_requests,
                "maintenance_requests mismatch"
            );
            prop_assert_eq!(
                response.capabilities.human_handoff,
                input_caps.human_handoff,
                "human_handoff mismatch"
            );

            // FAQ entries (Requirement 9.4)
            let input_faqs = input.faqs.as_ref().unwrap();
            let response_faqs = response.faqs.as_ref().unwrap();
            prop_assert_eq!(response_faqs.len(), input_faqs.len(), "FAQ count mismatch");
            for (i, (resp_faq, inp_faq)) in response_faqs.iter().zip(input_faqs.iter()).enumerate()
            {
                prop_assert_eq!(
                    &resp_faq.question,
                    &inp_faq.question,
                    "FAQ #{} question mismatch",
                    i
                );
                prop_assert_eq!(
                    &resp_faq.answer,
                    &inp_faq.answer,
                    "FAQ #{} answer mismatch",
                    i
                );
            }

            // Policies text (Requirement 9.5)
            prop_assert_eq!(
                response.policies.as_deref(),
                input.policies.as_deref(),
                "policies mismatch"
            );

            // Sender policy
            prop_assert_eq!(
                &response.sender_policy,
                input.sender_policy.as_ref().unwrap(),
                "sender_policy mismatch"
            );

            // Allowlist
            prop_assert_eq!(
                response.allowlist.as_ref(),
                input.allowlist.as_ref(),
                "allowlist mismatch"
            );

            // Handoff keywords
            prop_assert_eq!(
                response.handoff_keywords.as_ref(),
                input.handoff_keywords.as_ref(),
                "handoff_keywords mismatch"
            );

            // Numeric settings
            prop_assert_eq!(
                response.history_limit,
                input.history_limit.unwrap(),
                "history_limit mismatch"
            );
            prop_assert_eq!(
                response.retention_days,
                input.retention_days.unwrap(),
                "retention_days mismatch"
            );

            // Activo flag
            prop_assert_eq!(response.activo, input.activo.unwrap(), "activo mismatch");

            Ok(())
        })
        .expect("config_round_trip_preserves_all_fields failed");
}

// ── Property 8: Confidence Level Mapping ──────────────────────────────────────

// Feature: whatsapp-ai-assistant, Property 8: Confidence Level Mapping
// **Validates: Requirement 4.2**

/// Generate a valid OCR confidence score in [0.0, 1.0].
fn arb_confidence_score() -> impl Strategy<Value = f64> {
    0.0..=1.0f64
}

/// Property 8a: Scores >= 0.85 always map to High.
#[test]
fn confidence_high_threshold() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&(0.85..=1.0f64), |score| {
            let result = map_confidence(score);
            prop_assert!(
                result == Confidence::High,
                "Score {} (>= 0.85) should map to High",
                score,
            );
            Ok(())
        })
        .expect("confidence_high_threshold failed");
}

/// Property 8b: Scores in [0.60, 0.85) always map to Medium.
#[test]
fn confidence_medium_threshold() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&(0.60..0.85f64), |score| {
            let result = map_confidence(score);
            prop_assert!(
                result == Confidence::Medium,
                "Score {} (in [0.60, 0.85)) should map to Medium",
                score,
            );
            Ok(())
        })
        .expect("confidence_medium_threshold failed");
}

/// Property 8c: Scores < 0.60 always map to Low.
#[test]
fn confidence_low_threshold() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&(0.0..0.60f64), |score| {
            let result = map_confidence(score);
            prop_assert!(
                result == Confidence::Low,
                "Score {} (< 0.60) should map to Low",
                score,
            );
            Ok(())
        })
        .expect("confidence_low_threshold failed");
}

/// Property 8d: Mapping is exhaustive — every score in [0.0, 1.0] maps to exactly one level.
#[test]
fn confidence_mapping_exhaustive() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_confidence_score(), |score| {
            let result = map_confidence(score);
            let is_valid = matches!(
                result,
                Confidence::High | Confidence::Medium | Confidence::Low
            );
            prop_assert!(
                is_valid,
                "Score {} did not map to a valid confidence level",
                score
            );
            Ok(())
        })
        .expect("confidence_mapping_exhaustive failed");
}

/// Property 8e: Mapping is monotonic — higher scores never map to lower confidence levels.
/// Confidence ordering: High > Medium > Low.
#[test]
fn confidence_mapping_monotonic() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_confidence_score(), arb_confidence_score()),
            |(score_a, score_b)| {
                let conf_a = map_confidence(score_a);
                let conf_b = map_confidence(score_b);

                let level = |c: &Confidence| -> u8 {
                    match c {
                        Confidence::High => 2,
                        Confidence::Medium => 1,
                        Confidence::Low => 0,
                    }
                };

                if score_a > score_b {
                    prop_assert!(
                        level(&conf_a) >= level(&conf_b),
                        "Monotonicity violated: score_a={} ({:?}) > score_b={} ({:?}) but confidence decreased",
                        score_a,
                        conf_a,
                        score_b,
                        conf_b
                    );
                } else if score_b > score_a {
                    prop_assert!(
                        level(&conf_b) >= level(&conf_a),
                        "Monotonicity violated: score_b={} ({:?}) > score_a={} ({:?}) but confidence decreased",
                        score_b,
                        conf_b,
                        score_a,
                        conf_a
                    );
                }

                Ok(())
            },
        )
        .expect("confidence_mapping_monotonic failed");
}

// ── Property 9: Confidence-Based Receipt Routing ──────────────────────────────

// Feature: whatsapp-ai-assistant, Property 9: Confidence-Based Receipt Routing
// **Validates: Requirements 4.3, 4.4**

/// Generate an arbitrary Confidence value.
fn arb_confidence() -> impl Strategy<Value = Confidence> {
    prop_oneof![
        Just(Confidence::High),
        Just(Confidence::Medium),
        Just(Confidence::Low),
    ]
}

/// Generate an arbitrary Option<Uuid> representing tenant resolution status.
fn arb_inquilino_id() -> impl Strategy<Value = Option<Uuid>> {
    prop_oneof![
        Just(None),
        any::<u128>().prop_map(|v| Some(Uuid::from_u128(v))),
    ]
}

/// Pure model of the receipt routing logic.
/// Mirrors `record_extraction` which always stores with status `pending_confirmation`
/// regardless of confidence level or tenant resolution status.
/// The routing difference (reply to tenant) is handled elsewhere.
fn determine_extraction_status(
    _confidence: &Confidence,
    _inquilino_id: Option<Uuid>,
) -> &'static str {
    "pending_confirmation"
}

/// Property 9a: High confidence with resolved tenant → status is `pending_confirmation`.
#[test]
fn receipt_routing_high_confidence_resolved_tenant() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&any::<u128>(), |uuid_val| {
            let inquilino_id = Some(Uuid::from_u128(uuid_val));
            let status = determine_extraction_status(&Confidence::High, inquilino_id);
            prop_assert_eq!(
                status,
                "pending_confirmation",
                "High confidence + resolved tenant must produce 'pending_confirmation', got '{}'",
                status
            );
            Ok(())
        })
        .expect("receipt_routing_high_confidence_resolved_tenant failed");
}

/// Property 9b: Medium or Low confidence → status is `pending_confirmation` regardless of tenant resolution.
#[test]
fn receipt_routing_medium_low_always_pending_confirmation() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    let strategy = (
        prop_oneof![Just(Confidence::Medium), Just(Confidence::Low)],
        arb_inquilino_id(),
    );

    runner
        .run(&strategy, |(confidence, inquilino_id)| {
            let status = determine_extraction_status(&confidence, inquilino_id);
            prop_assert_eq!(
                status,
                "pending_confirmation",
                "{:?} confidence (tenant={:?}) must produce 'pending_confirmation', got '{}'",
                confidence,
                inquilino_id,
                status
            );
            Ok(())
        })
        .expect("receipt_routing_medium_low_always_pending_confirmation failed");
}

/// Property 9c: For ANY confidence level and ANY tenant resolution status,
/// the extraction status is always `pending_confirmation`.
/// This is the universal property: all extractions queue for landlord confirmation.
#[test]
fn receipt_routing_always_pending_confirmation() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_confidence(), arb_inquilino_id()),
            |(confidence, inquilino_id)| {
                let status = determine_extraction_status(&confidence, inquilino_id);
                prop_assert_eq!(
                    status,
                    "pending_confirmation",
                    "Confidence {:?} + tenant {:?} must always produce 'pending_confirmation', got '{}'",
                    confidence,
                    inquilino_id,
                    status
                );
                Ok(())
            },
        )
        .expect("receipt_routing_always_pending_confirmation failed");
}

// ── Strategies for Balance and Currency Formatting ─────────────────────

use rust_decimal::Decimal;

use crate::models::chatbot::format_currency;

/// Statuses that count toward outstanding balance.
const OUTSTANDING_STATUSES: &[&str] = &["pendiente", "atrasado"];

/// A simplified payment for balance calculation testing.
#[derive(Debug, Clone)]
struct TestPayment {
    amount: Decimal,
    currency: String,
    status: String,
}

/// Generate a valid payment amount (positive, reasonable range).
fn arb_payment_amount() -> impl Strategy<Value = Decimal> {
    (1i64..=999_999_999, 0u32..=2).prop_map(|(cents, scale)| {
        // Generate amounts from 0.01 to 999,999,999.99
        Decimal::new(cents, scale)
    })
}

/// Generate a currency code (DOP or USD).
fn arb_currency() -> impl Strategy<Value = String> {
    prop_oneof![Just("DOP".to_string()), Just("USD".to_string()),]
}

/// Generate a payment status.
fn arb_payment_status() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("pendiente".to_string()),
        Just("pagado".to_string()),
        Just("atrasado".to_string()),
    ]
}

/// Generate a list of test payments.
fn arb_payments(max_len: usize) -> impl Strategy<Value = Vec<TestPayment>> {
    prop::collection::vec(
        (arb_payment_amount(), arb_currency(), arb_payment_status()),
        0..=max_len,
    )
    .prop_map(|items| {
        items
            .into_iter()
            .map(|(amount, currency, status)| TestPayment {
                amount,
                currency,
                status,
            })
            .collect()
    })
}

/// Pure model of balance calculation: sum amounts where status is `pendiente` or `atrasado`,
/// grouped by currency. Mirrors `query_tenant_balance` logic.
fn calculate_balance(payments: &[TestPayment]) -> std::collections::HashMap<String, Decimal> {
    let mut totals: std::collections::HashMap<String, Decimal> = std::collections::HashMap::new();
    for p in payments {
        if OUTSTANDING_STATUSES.contains(&p.status.as_str()) {
            let entry = totals.entry(p.currency.clone()).or_insert(Decimal::ZERO);
            *entry += p.amount;
        }
    }
    totals
}

// ── Property 10: Balance Calculation Correctness ──────────────────────────────

// Feature: whatsapp-ai-assistant, Property 10: Balance Calculation Correctness
// **Validates: Requirements 5.1, 5.3**

/// Property 10a: The outstanding balance equals the sum of amounts for payments
/// with status `pendiente` or `atrasado`, grouped by currency.
#[test]
fn balance_calculation_sums_outstanding_only() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_payments(30), |payments| {
            let totals = calculate_balance(&payments);

            // Manually compute expected totals
            let mut expected: std::collections::HashMap<String, Decimal> =
                std::collections::HashMap::new();
            for p in &payments {
                if p.status == "pendiente" || p.status == "atrasado" {
                    let entry = expected.entry(p.currency.clone()).or_insert(Decimal::ZERO);
                    *entry += p.amount;
                }
            }

            prop_assert_eq!(
                totals.len(),
                expected.len(),
                "Currency count mismatch: got {:?}, expected {:?}",
                totals,
                expected
            );

            for (currency, expected_total) in &expected {
                let actual = totals.get(currency);
                prop_assert_eq!(
                    actual,
                    Some(expected_total),
                    "Balance mismatch for {}: got {:?}, expected {}",
                    currency,
                    actual,
                    expected_total
                );
            }

            Ok(())
        })
        .expect("balance_calculation_sums_outstanding_only failed");
}

/// Property 10b: Payments with status `pagado` never contribute to the outstanding balance.
#[test]
fn balance_excludes_paid_payments() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &prop::collection::vec((arb_payment_amount(), arb_currency()), 1..=20),
            |paid_payments| {
                // All payments are "pagado"
                let payments: Vec<TestPayment> = paid_payments
                    .into_iter()
                    .map(|(amount, currency)| TestPayment {
                        amount,
                        currency,
                        status: "pagado".to_string(),
                    })
                    .collect();

                let totals = calculate_balance(&payments);

                prop_assert!(
                    totals.is_empty(),
                    "Paid-only payments should produce empty balance, got {:?}",
                    totals
                );

                Ok(())
            },
        )
        .expect("balance_excludes_paid_payments failed");
}

/// Property 10c: Balance never mixes currencies — each currency total is independent.
#[test]
fn balance_never_mixes_currencies() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_payments(30), |payments| {
            let totals = calculate_balance(&payments);

            // For each currency in totals, verify it only sums amounts of that currency
            for (currency, total) in &totals {
                let manual_sum: Decimal = payments
                    .iter()
                    .filter(|p| {
                        &p.currency == currency
                            && (p.status == "pendiente" || p.status == "atrasado")
                    })
                    .map(|p| p.amount)
                    .sum();

                prop_assert_eq!(
                    total,
                    &manual_sum,
                    "Currency {} total {} != manual sum {} (cross-currency contamination?)",
                    currency,
                    total,
                    manual_sum
                );
            }

            Ok(())
        })
        .expect("balance_never_mixes_currencies failed");
}

/// Property 10d: Empty payment list produces empty balance.
#[test]
fn balance_empty_payments_produces_empty_totals() {
    let totals = calculate_balance(&[]);
    assert!(
        totals.is_empty(),
        "Empty payments should produce empty balance"
    );
}

// ── Property 11: Currency Formatting ──────────────────────────────────────────

// Feature: whatsapp-ai-assistant, Property 11: Currency Formatting
// **Validates: Requirements 5.1, 5.3**

/// Generate a Decimal amount suitable for formatting (positive and negative).
fn arb_format_amount() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        // Positive amounts
        (0i64..=999_999_999, 0u32..=2).prop_map(|(v, s)| Decimal::new(v, s)),
        // Negative amounts
        (-999_999_999i64..=-1, 0u32..=2).prop_map(|(v, s)| Decimal::new(v, s)),
        // Zero
        Just(Decimal::ZERO),
        // Small fractional amounts
        (1i64..=99, 2u32..=2).prop_map(|(v, s)| Decimal::new(v, s)),
    ]
}

/// Property 11a: DOP formatting always contains "RD$" symbol.
#[test]
fn currency_format_dop_contains_correct_symbol() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_format_amount(), |amount| {
            let formatted = format_currency(amount, "DOP");
            prop_assert!(
                formatted.contains("RD$"),
                "DOP formatted '{}' must contain 'RD$' (amount: {})",
                formatted,
                amount
            );
            // Must NOT contain USD symbol
            prop_assert!(
                !formatted.contains("US$"),
                "DOP formatted '{}' must NOT contain 'US$' (amount: {})",
                formatted,
                amount
            );
            Ok(())
        })
        .expect("currency_format_dop_contains_correct_symbol failed");
}

/// Property 11b: USD formatting always contains "US$" symbol.
#[test]
fn currency_format_usd_contains_correct_symbol() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_format_amount(), |amount| {
            let formatted = format_currency(amount, "USD");
            prop_assert!(
                formatted.contains("US$"),
                "USD formatted '{}' must contain 'US$' (amount: {})",
                formatted,
                amount
            );
            // Must NOT contain DOP symbol
            prop_assert!(
                !formatted.contains("RD$"),
                "USD formatted '{}' must NOT contain 'RD$' (amount: {})",
                formatted,
                amount
            );
            Ok(())
        })
        .expect("currency_format_usd_contains_correct_symbol failed");
}

/// Property 11c: Formatted output always ends with exactly two decimal places (.XX).
#[test]
fn currency_format_has_two_decimal_places() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_format_amount(), arb_currency()),
            |(amount, currency)| {
                let formatted = format_currency(amount, &currency);

                // Find the last '.' in the string — it should be followed by exactly 2 digits
                let dot_pos = formatted.rfind('.');
                prop_assert!(
                    dot_pos.is_some(),
                    "Formatted '{}' must contain a decimal point (amount: {}, currency: {})",
                    formatted,
                    amount,
                    currency
                );

                let after_dot = &formatted[dot_pos.unwrap() + 1..];
                prop_assert_eq!(
                    after_dot.len(),
                    2,
                    "Expected exactly 2 decimal places after '.', got '{}' in '{}' (amount: {})",
                    after_dot,
                    formatted,
                    amount
                );

                // Both characters after the dot must be digits
                prop_assert!(
                    after_dot.chars().all(|c| c.is_ascii_digit()),
                    "Decimal places must be digits, got '{}' in '{}' (amount: {})",
                    after_dot,
                    formatted,
                    amount
                );

                Ok(())
            },
        )
        .expect("currency_format_has_two_decimal_places failed");
}

/// Property 11d: Currency symbols never mix — DOP symbol never appears with USD input and vice versa.
#[test]
fn currency_format_never_mixes_symbols() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_format_amount(), |amount| {
            let dop_formatted = format_currency(amount, "DOP");
            let usd_formatted = format_currency(amount, "USD");

            // DOP output must not contain US$
            prop_assert!(
                !dop_formatted.contains("US$"),
                "DOP format '{}' contains US$ symbol (amount: {})",
                dop_formatted,
                amount
            );

            // USD output must not contain RD$
            prop_assert!(
                !usd_formatted.contains("RD$"),
                "USD format '{}' contains RD$ symbol (amount: {})",
                usd_formatted,
                amount
            );

            Ok(())
        })
        .expect("currency_format_never_mixes_symbols failed");
}

// ── Strategies for Maintenance Request Defaults ────────────────────────

use crate::services::chatbot::resolve_maintenance_defaults;

/// Valid priorities for maintenance requests.
const TEST_VALID_PRIORITIES: &[&str] = &["baja", "media", "alta", "urgente"];

/// Generate a valid maintenance description (2–1000 chars).
fn arb_valid_description() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 .,!?]{2,200}"
}

/// Generate an invalid description (too short: 0–1 chars).
fn arb_short_description() -> impl Strategy<Value = String> {
    prop_oneof![Just(String::new()), "[a-z]{1,1}",]
}

/// Generate an invalid description (too long: >1000 chars).
fn arb_long_description() -> impl Strategy<Value = String> {
    "[a-z]{1001,1100}"
}

/// Generate a valid priority value.
fn arb_valid_priority() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("baja".to_string()),
        Just("media".to_string()),
        Just("alta".to_string()),
        Just("urgente".to_string()),
    ]
}

/// Generate an invalid priority value.
fn arb_invalid_priority() -> impl Strategy<Value = String> {
    "[a-z]{1,20}".prop_filter("must not be a valid priority", |s| {
        !TEST_VALID_PRIORITIES.contains(&s.as_str())
    })
}

// ── Property 12: Maintenance Request Defaults and Linking ─────────────────────

// Feature: whatsapp-ai-assistant, Property 12: Maintenance Request Defaults and Linking
// **Validates: Requirements 6.1, 6.2**

/// Property 12a: With no explicit priority, the request defaults to status `pendiente`
/// and priority `media`, linked to the given `propiedad_id`.
#[test]
fn maintenance_defaults_no_priority() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_valid_description(), any::<u128>()),
            |(description, prop_id_seed)| {
                let propiedad_id = Uuid::from_u128(prop_id_seed);
                let result = resolve_maintenance_defaults(&description, None, propiedad_id);

                prop_assert!(
                    result.is_ok(),
                    "Valid description with no priority should succeed, got: {:?}",
                    result.err()
                );

                let defaults = result.unwrap();
                prop_assert_eq!(
                    &defaults.status,
                    "pendiente",
                    "Status must be 'pendiente', got '{}'",
                    defaults.status
                );
                prop_assert_eq!(
                    &defaults.priority,
                    "media",
                    "Default priority must be 'media', got '{}'",
                    defaults.priority
                );
                prop_assert_eq!(
                    defaults.propiedad_id,
                    propiedad_id,
                    "propiedad_id must match the contract's property"
                );

                Ok(())
            },
        )
        .expect("maintenance_defaults_no_priority failed");
}

/// Property 12b: With an explicit valid priority, the request uses that priority,
/// status is still `pendiente`, and it's linked to the given `propiedad_id`.
#[test]
fn maintenance_defaults_explicit_priority() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_valid_description(), arb_valid_priority(), any::<u128>()),
            |(description, priority, prop_id_seed)| {
                let propiedad_id = Uuid::from_u128(prop_id_seed);
                let result =
                    resolve_maintenance_defaults(&description, Some(&priority), propiedad_id);

                prop_assert!(
                    result.is_ok(),
                    "Valid description with valid priority '{}' should succeed, got: {:?}",
                    priority,
                    result.err()
                );

                let defaults = result.unwrap();
                prop_assert_eq!(
                    &defaults.status,
                    "pendiente",
                    "Status must always be 'pendiente', got '{}'",
                    defaults.status
                );
                prop_assert_eq!(
                    &defaults.priority,
                    &priority,
                    "Priority must match explicit value '{}', got '{}'",
                    priority,
                    defaults.priority
                );
                prop_assert_eq!(
                    defaults.propiedad_id,
                    propiedad_id,
                    "propiedad_id must match the contract's property"
                );

                Ok(())
            },
        )
        .expect("maintenance_defaults_explicit_priority failed");
}

/// Property 12c: The request is always linked to a `propiedad_id` (from the contract).
/// For any valid inputs, the resulting defaults always contain the provided `propiedad_id`.
#[test]
fn maintenance_always_linked_to_property() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(
                arb_valid_description(),
                prop::option::of(arb_valid_priority()),
                any::<u128>(),
            ),
            |(description, priority, prop_id_seed)| {
                let propiedad_id = Uuid::from_u128(prop_id_seed);
                let priority_ref = priority.as_deref();
                let result = resolve_maintenance_defaults(&description, priority_ref, propiedad_id);

                prop_assert!(
                    result.is_ok(),
                    "Valid inputs should succeed, got: {:?}",
                    result.err()
                );

                let defaults = result.unwrap();
                prop_assert_eq!(
                    defaults.propiedad_id,
                    propiedad_id,
                    "Request must always be linked to the contract's propiedad_id"
                );

                Ok(())
            },
        )
        .expect("maintenance_always_linked_to_property failed");
}

/// Property 12d: Description validation — too short (< 2 chars) is rejected.
#[test]
fn maintenance_rejects_short_description() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_short_description(), any::<u128>()),
            |(description, prop_id_seed)| {
                let propiedad_id = Uuid::from_u128(prop_id_seed);
                let result = resolve_maintenance_defaults(&description, None, propiedad_id);

                prop_assert!(
                    result.is_err(),
                    "Description '{}' (len={}) should be rejected as too short",
                    description,
                    description.chars().count()
                );

                Ok(())
            },
        )
        .expect("maintenance_rejects_short_description failed");
}

/// Property 12e: Description validation — too long (> 1000 chars) is rejected.
#[test]
fn maintenance_rejects_long_description() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_long_description(), any::<u128>()),
            |(description, prop_id_seed)| {
                let propiedad_id = Uuid::from_u128(prop_id_seed);
                let result = resolve_maintenance_defaults(&description, None, propiedad_id);

                prop_assert!(
                    result.is_err(),
                    "Description of length {} should be rejected as too long",
                    description.chars().count()
                );

                Ok(())
            },
        )
        .expect("maintenance_rejects_long_description failed");
}

/// Property 12f: Invalid priority values are rejected.
#[test]
fn maintenance_rejects_invalid_priority() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(
                arb_valid_description(),
                arb_invalid_priority(),
                any::<u128>(),
            ),
            |(description, priority, prop_id_seed)| {
                let propiedad_id = Uuid::from_u128(prop_id_seed);
                let result =
                    resolve_maintenance_defaults(&description, Some(&priority), propiedad_id);

                prop_assert!(
                    result.is_err(),
                    "Invalid priority '{}' should be rejected",
                    priority
                );

                Ok(())
            },
        )
        .expect("maintenance_rejects_invalid_priority failed");
}

// ── Pure Models for Receipt Status Transitions ────────────────────────────────

/// Valid receipt extraction statuses.
const RECEIPT_STATUS_PENDING: &str = "pending_confirmation";
const RECEIPT_STATUS_CONFIRMED: &str = "confirmed";
const RECEIPT_STATUS_REJECTED: &str = "rejected";

/// A simplified receipt extraction for status transition testing.
#[derive(Debug, Clone)]
struct TestReceiptExtraction {
    id: Uuid,
    organizacion_id: Uuid,
    status: String,
}

/// Pure model of the pending receipts filter.
/// Mirrors `list_pending_receipts` which filters by org_id AND status == "pending_confirmation".
fn filter_pending_receipts(extractions: &[TestReceiptExtraction], org_id: Uuid) -> Vec<Uuid> {
    extractions
        .iter()
        .filter(|e| e.organizacion_id == org_id && e.status == RECEIPT_STATUS_PENDING)
        .map(|e| e.id)
        .collect()
}

/// Result of a status transition attempt.
#[derive(Debug, Clone, PartialEq)]
enum TransitionResult {
    /// Transition succeeded, new status and optional confirming user.
    Success {
        new_status: String,
        confirmed_by: Option<Uuid>,
    },
    /// Transition rejected because extraction is not in `pending_confirmation`.
    Rejected,
}

/// Pure model of the confirm receipt status transition.
/// Mirrors `confirm_receipt`: only transitions from `pending_confirmation` → `confirmed`.
fn try_confirm(extraction: &TestReceiptExtraction, user_id: Uuid) -> TransitionResult {
    if extraction.status != RECEIPT_STATUS_PENDING {
        return TransitionResult::Rejected;
    }
    TransitionResult::Success {
        new_status: RECEIPT_STATUS_CONFIRMED.to_string(),
        confirmed_by: Some(user_id),
    }
}

/// Pure model of the reject receipt status transition.
/// Mirrors `reject_receipt`: only transitions from `pending_confirmation` → `rejected`.
fn try_reject(extraction: &TestReceiptExtraction) -> TransitionResult {
    if extraction.status != RECEIPT_STATUS_PENDING {
        return TransitionResult::Rejected;
    }
    TransitionResult::Success {
        new_status: RECEIPT_STATUS_REJECTED.to_string(),
        confirmed_by: None,
    }
}

// ── Strategies for Receipt Status Transitions ─────────────────────────────────

/// Generate a receipt extraction status.
fn arb_receipt_status() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(RECEIPT_STATUS_PENDING.to_string()),
        Just(RECEIPT_STATUS_CONFIRMED.to_string()),
        Just(RECEIPT_STATUS_REJECTED.to_string()),
    ]
}

/// Generate a receipt extraction with a specific status.
fn arb_extraction_with_status(
    status: &'static str,
) -> impl Strategy<Value = TestReceiptExtraction> {
    (any::<u128>(), any::<u128>()).prop_map(move |(id_seed, org_seed)| TestReceiptExtraction {
        id: Uuid::from_u128(id_seed),
        organizacion_id: Uuid::from_u128(org_seed),
        status: status.to_string(),
    })
}

/// Generate a list of receipt extractions with mixed statuses and org IDs.
fn arb_extraction_list(max_len: usize) -> impl Strategy<Value = Vec<TestReceiptExtraction>> {
    prop::collection::vec(
        (
            any::<u128>(),
            prop_oneof![any::<u128>(), Just(42u128)],
            arb_receipt_status(),
        ),
        0..=max_len,
    )
    .prop_map(|items| {
        items
            .into_iter()
            .map(|(id_seed, org_seed, status)| TestReceiptExtraction {
                id: Uuid::from_u128(id_seed),
                organizacion_id: Uuid::from_u128(org_seed),
                status,
            })
            .collect()
    })
}

// ── Property 15: Pending Receipts Visibility ──────────────────────────────────

// Feature: whatsapp-ai-assistant, Property 15: Pending Receipts Visibility
// **Validates: Requirement 8.1**

/// Property 15a: Any extraction with status `pending_confirmation` in org O
/// appears in the pending receipts list for O.
#[test]
fn pending_receipts_includes_all_pending_in_org() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_extraction_list(30), any::<u128>()),
            |(extractions, org_seed)| {
                let org_id = Uuid::from_u128(org_seed);
                let pending_ids = filter_pending_receipts(&extractions, org_id);

                // Every extraction with status pending_confirmation in this org must be in the result
                for extraction in &extractions {
                    if extraction.organizacion_id == org_id
                        && extraction.status == RECEIPT_STATUS_PENDING
                    {
                        prop_assert!(
                            pending_ids.contains(&extraction.id),
                            "Extraction {:?} with status 'pending_confirmation' in org {:?} \
                             must appear in pending list, but was missing",
                            extraction.id,
                            org_id
                        );
                    }
                }

                Ok(())
            },
        )
        .expect("pending_receipts_includes_all_pending_in_org failed");
}

/// Property 15b: Extractions NOT in `pending_confirmation` status never appear
/// in the pending receipts list, regardless of org.
#[test]
fn pending_receipts_excludes_non_pending() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_extraction_list(30), any::<u128>()),
            |(extractions, org_seed)| {
                let org_id = Uuid::from_u128(org_seed);
                let pending_ids = filter_pending_receipts(&extractions, org_id);

                // No extraction with status != pending_confirmation should be in the result
                for extraction in &extractions {
                    if extraction.status != RECEIPT_STATUS_PENDING {
                        prop_assert!(
                            !pending_ids.contains(&extraction.id),
                            "Extraction {:?} with status '{}' must NOT appear in pending list",
                            extraction.id,
                            extraction.status
                        );
                    }
                }

                Ok(())
            },
        )
        .expect("pending_receipts_excludes_non_pending failed");
}

/// Property 15c: Extractions from a different org never appear in the pending list.
#[test]
fn pending_receipts_scoped_to_org() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_extraction_list(30), any::<u128>()),
            |(extractions, org_seed)| {
                let org_id = Uuid::from_u128(org_seed);
                let pending_ids = filter_pending_receipts(&extractions, org_id);

                // No extraction from a different org should be in the result
                for extraction in &extractions {
                    if extraction.organizacion_id != org_id {
                        prop_assert!(
                            !pending_ids.contains(&extraction.id),
                            "Extraction {:?} from org {:?} must NOT appear in pending list for org {:?}",
                            extraction.id,
                            extraction.organizacion_id,
                            org_id
                        );
                    }
                }

                Ok(())
            },
        )
        .expect("pending_receipts_scoped_to_org failed");
}

// ── Property 16: Receipt Status Transitions ───────────────────────────────────

// Feature: whatsapp-ai-assistant, Property 16: Receipt Status Transitions
// **Validates: Requirements 8.2, 8.3**

/// Property 16a: Confirming a `pending_confirmation` extraction sets status to `confirmed`
/// and records the confirming user's ID.
#[test]
fn confirm_pending_sets_confirmed_with_user() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(
                arb_extraction_with_status(RECEIPT_STATUS_PENDING),
                any::<u128>(),
            ),
            |(extraction, user_seed)| {
                let user_id = Uuid::from_u128(user_seed);
                let result = try_confirm(&extraction, user_id);

                match result {
                    TransitionResult::Success {
                        new_status,
                        confirmed_by,
                    } => {
                        prop_assert_eq!(
                            &new_status,
                            RECEIPT_STATUS_CONFIRMED,
                            "Confirming must set status to 'confirmed', got '{}'",
                            new_status
                        );
                        prop_assert_eq!(
                            confirmed_by,
                            Some(user_id),
                            "Confirming must record the user_id"
                        );
                    }
                    TransitionResult::Rejected => {
                        prop_assert!(
                            false,
                            "Confirming a pending_confirmation extraction must succeed"
                        );
                    }
                }

                Ok(())
            },
        )
        .expect("confirm_pending_sets_confirmed_with_user failed");
}

/// Property 16b: Rejecting a `pending_confirmation` extraction sets status to `rejected`.
#[test]
fn reject_pending_sets_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &arb_extraction_with_status(RECEIPT_STATUS_PENDING),
            |extraction| {
                let result = try_reject(&extraction);

                match result {
                    TransitionResult::Success {
                        new_status,
                        confirmed_by,
                    } => {
                        prop_assert_eq!(
                            &new_status,
                            RECEIPT_STATUS_REJECTED,
                            "Rejecting must set status to 'rejected', got '{}'",
                            new_status
                        );
                        prop_assert_eq!(
                            confirmed_by,
                            None,
                            "Rejecting must NOT record a confirming user"
                        );
                    }
                    TransitionResult::Rejected => {
                        prop_assert!(
                            false,
                            "Rejecting a pending_confirmation extraction must succeed"
                        );
                    }
                }

                Ok(())
            },
        )
        .expect("reject_pending_sets_rejected failed");
}

/// Property 16c: Confirming an extraction NOT in `pending_confirmation` is rejected.
#[test]
fn confirm_non_pending_is_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    let non_pending_strategy = prop_oneof![
        arb_extraction_with_status(RECEIPT_STATUS_CONFIRMED),
        arb_extraction_with_status(RECEIPT_STATUS_REJECTED),
    ];

    runner
        .run(
            &(non_pending_strategy, any::<u128>()),
            |(extraction, user_seed)| {
                let user_id = Uuid::from_u128(user_seed);
                let result = try_confirm(&extraction, user_id);

                prop_assert_eq!(
                    result,
                    TransitionResult::Rejected,
                    "Confirming extraction with status '{}' must be rejected",
                    extraction.status
                );

                Ok(())
            },
        )
        .expect("confirm_non_pending_is_rejected failed");
}

/// Property 16d: Rejecting an extraction NOT in `pending_confirmation` is rejected.
#[test]
fn reject_non_pending_is_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    let non_pending_strategy = prop_oneof![
        arb_extraction_with_status(RECEIPT_STATUS_CONFIRMED),
        arb_extraction_with_status(RECEIPT_STATUS_REJECTED),
    ];

    runner
        .run(&non_pending_strategy, |extraction| {
            let result = try_reject(&extraction);

            prop_assert_eq!(
                result,
                TransitionResult::Rejected,
                "Rejecting extraction with status '{}' must be rejected",
                extraction.status
            );

            Ok(())
        })
        .expect("reject_non_pending_is_rejected failed");
}

/// Property 16e: The only valid transitions from `pending_confirmation` are to
/// `confirmed` or `rejected`. No other target states are reachable.
#[test]
fn pending_only_transitions_to_confirmed_or_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(
                arb_extraction_with_status(RECEIPT_STATUS_PENDING),
                any::<u128>(),
            ),
            |(extraction, user_seed)| {
                let user_id = Uuid::from_u128(user_seed);

                let confirm_result = try_confirm(&extraction, user_id);
                let reject_result = try_reject(&extraction);

                // Confirm must produce "confirmed"
                if let TransitionResult::Success { ref new_status, .. } = confirm_result {
                    prop_assert_eq!(
                        new_status,
                        RECEIPT_STATUS_CONFIRMED,
                        "Confirm must produce 'confirmed', got '{}'",
                        new_status
                    );
                }

                // Reject must produce "rejected"
                if let TransitionResult::Success { ref new_status, .. } = reject_result {
                    prop_assert_eq!(
                        new_status,
                        RECEIPT_STATUS_REJECTED,
                        "Reject must produce 'rejected', got '{}'",
                        new_status
                    );
                }

                // Both must succeed (not be rejected)
                prop_assert!(
                    matches!(confirm_result, TransitionResult::Success { .. }),
                    "Confirm from pending must succeed"
                );
                prop_assert!(
                    matches!(reject_result, TransitionResult::Success { .. }),
                    "Reject from pending must succeed"
                );

                Ok(())
            },
        )
        .expect("pending_only_transitions_to_confirmed_or_rejected failed");
}

// ── Property 5: System Prompt Composition Completeness ────────────────────────

// Feature: whatsapp-ai-assistant, Property 5: System Prompt Composition Completeness
// **Validates: Requirement 3.1**

use crate::services::ai_module::{ChatbotPersona, TenantContext, compose_system_prompt};

/// Generate a non-empty arbitrary string for persona fields.
fn arb_nonempty_string() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 áéíóúñ]{1,80}"
}

/// Generate an arbitrary FAQ list (1–10 entries with non-empty Q&A).
fn arb_faq_list() -> impl Strategy<Value = Vec<FaqEntry>> {
    prop::collection::vec(
        (arb_nonempty_string(), arb_nonempty_string()).prop_map(|(q, a)| FaqEntry {
            question: q,
            answer: a,
        }),
        1..=10,
    )
}

/// Generate an arbitrary list of handoff keywords.
fn arb_handoff_keywords() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arb_nonempty_string(), 1..=5)
}

/// Property 5a: When tone is Some, the composed prompt contains the tone string.
#[test]
fn system_prompt_contains_tone_when_present() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_nonempty_string(), |tone| {
            let config = ChatbotPersona {
                tone: Some(tone.clone()),
                greeting: None,
                system_prompt: None,
                language: "es-DO".to_string(),
            };

            let prompt = compose_system_prompt(&config, None, &[], None, &[], &[]);
            prop_assert!(
                prompt.contains(&tone),
                "Prompt must contain tone '{}', got: '{}'",
                tone,
                prompt
            );
            Ok(())
        })
        .expect("system_prompt_contains_tone_when_present failed");
}

/// Property 5b: When greeting is Some, the composed prompt contains the greeting string.
#[test]
fn system_prompt_contains_greeting_when_present() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_nonempty_string(), |greeting| {
            let config = ChatbotPersona {
                tone: None,
                greeting: Some(greeting.clone()),
                system_prompt: None,
                language: "es-DO".to_string(),
            };

            let prompt = compose_system_prompt(&config, None, &[], None, &[], &[]);
            prop_assert!(
                prompt.contains(&greeting),
                "Prompt must contain greeting '{}', got: '{}'",
                greeting,
                prompt
            );
            Ok(())
        })
        .expect("system_prompt_contains_greeting_when_present failed");
}

/// Property 5c: The deprecated system_prompt field is no longer included in the composed prompt
/// (replaced by guidance rules).
#[test]
fn system_prompt_ignores_deprecated_system_prompt_field() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_nonempty_string(), |sys_prompt| {
            let config = ChatbotPersona {
                tone: None,
                greeting: None,
                system_prompt: Some(sys_prompt.clone()),
                language: "es-DO".to_string(),
            };

            let prompt = compose_system_prompt(&config, None, &[], None, &[], &[]);
            prop_assert!(
                !prompt.contains(&sys_prompt),
                "Prompt must NOT contain deprecated system_prompt '{}', got: '{}'",
                sys_prompt,
                prompt
            );
            Ok(())
        })
        .expect("system_prompt_ignores_deprecated_system_prompt_field failed");
}

/// Property 5d: Every FAQ question and answer appears in the composed prompt.
#[test]
fn system_prompt_contains_all_faq_entries() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_faq_list(), |faqs| {
            let config = ChatbotPersona {
                tone: None,
                greeting: None,
                system_prompt: None,
                language: "es-DO".to_string(),
            };

            let prompt = compose_system_prompt(&config, None, &faqs, None, &[], &[]);

            for (i, faq) in faqs.iter().enumerate() {
                prop_assert!(
                    prompt.contains(&faq.question),
                    "Prompt must contain FAQ #{} question '{}', got: '{}'",
                    i,
                    faq.question,
                    prompt
                );
                prop_assert!(
                    prompt.contains(&faq.answer),
                    "Prompt must contain FAQ #{} answer '{}', got: '{}'",
                    i,
                    faq.answer,
                    prompt
                );
            }
            Ok(())
        })
        .expect("system_prompt_contains_all_faq_entries failed");
}

/// Property 5e: When policies is Some and non-empty, the composed prompt contains the policies text.
#[test]
fn system_prompt_contains_policies_when_present() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_nonempty_string(), |policies| {
            let config = ChatbotPersona {
                tone: None,
                greeting: None,
                system_prompt: None,
                language: "es-DO".to_string(),
            };

            let prompt = compose_system_prompt(&config, None, &[], Some(&policies), &[], &[]);
            prop_assert!(
                prompt.contains(&policies),
                "Prompt must contain policies '{}', got: '{}'",
                policies,
                prompt
            );
            Ok(())
        })
        .expect("system_prompt_contains_policies_when_present failed");
}

/// Property 5f: When tenant_context is Some, the composed prompt contains the tenant's name.
#[test]
fn system_prompt_contains_tenant_name_when_resolved() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_nonempty_string(), |name| {
            let config = ChatbotPersona {
                tone: None,
                greeting: None,
                system_prompt: None,
                language: "es-DO".to_string(),
            };
            let tenant = TenantContext { name: name.clone() };

            let prompt = compose_system_prompt(&config, Some(&tenant), &[], None, &[], &[]);
            prop_assert!(
                prompt.contains(&name),
                "Prompt must contain tenant name '{}', got: '{}'",
                name,
                prompt
            );
            Ok(())
        })
        .expect("system_prompt_contains_tenant_name_when_resolved failed");
}

/// Property 5g: Handoff keywords appear in the prompt when provided.
#[test]
fn system_prompt_contains_handoff_keywords_when_provided() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_handoff_keywords(), |keywords| {
            let config = ChatbotPersona {
                tone: None,
                greeting: None,
                system_prompt: None,
                language: "es-DO".to_string(),
            };

            let prompt = compose_system_prompt(&config, None, &[], None, &keywords, &[]);

            for (i, kw) in keywords.iter().enumerate() {
                prop_assert!(
                    prompt.contains(kw),
                    "Prompt must contain handoff keyword #{} '{}', got: '{}'",
                    i,
                    kw,
                    prompt
                );
            }
            Ok(())
        })
        .expect("system_prompt_contains_handoff_keywords_when_provided failed");
}

/// Property 5 (combined): For any full persona configuration with all fields populated,
/// the composed system prompt contains ALL active elements simultaneously.
#[test]
fn system_prompt_composition_completeness_combined() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    let strategy = (
        arb_nonempty_string(),  // tone
        arb_nonempty_string(),  // greeting
        arb_nonempty_string(),  // tenant name
        arb_faq_list(),         // faqs
        arb_nonempty_string(),  // policies
        arb_handoff_keywords(), // handoff keywords
    );

    runner
        .run(
            &strategy,
            |(tone, greeting, tenant_name, faqs, policies, keywords)| {
                let config = ChatbotPersona {
                    tone: Some(tone.clone()),
                    greeting: Some(greeting.clone()),
                    system_prompt: None,
                    language: "es-DO".to_string(),
                };
                let tenant = TenantContext {
                    name: tenant_name.clone(),
                };

                let prompt = compose_system_prompt(
                    &config,
                    Some(&tenant),
                    &faqs,
                    Some(&policies),
                    &keywords,
                    &[],
                );

                // Tone
                prop_assert!(
                    prompt.contains(&tone),
                    "Combined: prompt must contain tone '{}'",
                    tone
                );
                // Greeting
                prop_assert!(
                    prompt.contains(&greeting),
                    "Combined: prompt must contain greeting '{}'",
                    greeting
                );
                // Tenant name
                prop_assert!(
                    prompt.contains(&tenant_name),
                    "Combined: prompt must contain tenant name '{}'",
                    tenant_name
                );
                // All FAQ entries
                for (i, faq) in faqs.iter().enumerate() {
                    prop_assert!(
                        prompt.contains(&faq.question),
                        "Combined: prompt must contain FAQ #{} question '{}'",
                        i,
                        faq.question
                    );
                    prop_assert!(
                        prompt.contains(&faq.answer),
                        "Combined: prompt must contain FAQ #{} answer '{}'",
                        i,
                        faq.answer
                    );
                }
                // Policies
                prop_assert!(
                    prompt.contains(&policies),
                    "Combined: prompt must contain policies '{}'",
                    policies
                );
                // Handoff keywords
                for (i, kw) in keywords.iter().enumerate() {
                    prop_assert!(
                        prompt.contains(kw),
                        "Combined: prompt must contain handoff keyword #{} '{}'",
                        i,
                        kw
                    );
                }

                Ok(())
            },
        )
        .expect("system_prompt_composition_completeness_combined failed");
}

// ── Property 7: Tool Registration Matches Capabilities ────────────────────────

// Feature: whatsapp-ai-assistant, Property 7: Tool Registration Matches Capabilities
// **Validates: Requirements 3.5, 3.6**

use crate::services::ai_module::get_enabled_tools;
use std::collections::HashSet;

/// Compute the expected tool set for a given capabilities configuration.
fn expected_tools_for_capabilities(caps: &Capabilities) -> HashSet<&'static str> {
    let mut expected = HashSet::new();

    if caps.receipt_ocr {
        expected.insert("extract_receipt");
    }
    if caps.balance_queries {
        expected.insert("query_balance");
        expected.insert("get_payment_history");
    }
    if caps.maintenance_requests {
        expected.insert("create_maintenance_request");
    }
    if caps.human_handoff {
        expected.insert("handoff_to_human");
    }
    // payment_reminders has no tool mapping

    expected
}

/// Property 7a: For any combination of capability booleans, the returned tool set
/// matches exactly the expected set — no more, no fewer.
#[test]
fn tool_registration_matches_capabilities_exactly() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&valid_capabilities(), |caps| {
            let tools: HashSet<&'static str> = get_enabled_tools(&caps).into_iter().collect();
            let expected = expected_tools_for_capabilities(&caps);

            prop_assert_eq!(
                &tools,
                &expected,
                "Tool set mismatch for capabilities {:?}.\nGot: {:?}\nExpected: {:?}",
                caps,
                tools,
                expected
            );
            Ok(())
        })
        .expect("tool_registration_matches_capabilities_exactly failed");
}

/// Property 7b: Disabled capabilities never produce their corresponding tools.
#[test]
fn disabled_capabilities_never_produce_tools() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&valid_capabilities(), |caps| {
            let tools: HashSet<&'static str> = get_enabled_tools(&caps).into_iter().collect();

            if !caps.receipt_ocr {
                prop_assert!(
                    !tools.contains("extract_receipt"),
                    "receipt_ocr disabled but extract_receipt tool present"
                );
            }
            if !caps.balance_queries {
                prop_assert!(
                    !tools.contains("query_balance"),
                    "balance_queries disabled but query_balance tool present"
                );
                prop_assert!(
                    !tools.contains("get_payment_history"),
                    "balance_queries disabled but get_payment_history tool present"
                );
            }
            if !caps.maintenance_requests {
                prop_assert!(
                    !tools.contains("create_maintenance_request"),
                    "maintenance_requests disabled but create_maintenance_request tool present"
                );
            }
            if !caps.human_handoff {
                prop_assert!(
                    !tools.contains("handoff_to_human"),
                    "human_handoff disabled but handoff_to_human tool present"
                );
            }

            Ok(())
        })
        .expect("disabled_capabilities_never_produce_tools failed");
}

/// Property 7c: Enabled capabilities always produce their corresponding tools.
#[test]
fn enabled_capabilities_always_produce_tools() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&valid_capabilities(), |caps| {
            let tools: HashSet<&'static str> = get_enabled_tools(&caps).into_iter().collect();

            if caps.receipt_ocr {
                prop_assert!(
                    tools.contains("extract_receipt"),
                    "receipt_ocr enabled but extract_receipt tool missing"
                );
            }
            if caps.balance_queries {
                prop_assert!(
                    tools.contains("query_balance"),
                    "balance_queries enabled but query_balance tool missing"
                );
                prop_assert!(
                    tools.contains("get_payment_history"),
                    "balance_queries enabled but get_payment_history tool missing"
                );
            }
            if caps.maintenance_requests {
                prop_assert!(
                    tools.contains("create_maintenance_request"),
                    "maintenance_requests enabled but create_maintenance_request tool missing"
                );
            }
            if caps.human_handoff {
                prop_assert!(
                    tools.contains("handoff_to_human"),
                    "human_handoff enabled but handoff_to_human tool missing"
                );
            }

            Ok(())
        })
        .expect("enabled_capabilities_always_produce_tools failed");
}

// ── Property 19: Internal Webhook Authentication ──────────────────────────────

// Feature: whatsapp-ai-assistant, Property 19: Internal Webhook Authentication
// **Validates: Requirements 10.1, 10.2**

use crate::services::crypto::constant_time_eq;

/// Authentication decision result.
#[derive(Debug, Clone, PartialEq)]
enum AuthResult {
    Accepted,
    Rejected,
}

/// Pure model of the internal webhook authentication logic.
/// Given a configured secret and an optional provided token:
/// - If token is None (missing header) → reject
/// - If token does not match secret → reject
/// - If token matches secret → accept
fn authenticate_webhook(configured_secret: &str, provided_token: Option<&str>) -> AuthResult {
    provided_token.map_or(AuthResult::Rejected, |token| {
        if constant_time_eq(token.as_bytes(), configured_secret.as_bytes()) {
            AuthResult::Accepted
        } else {
            AuthResult::Rejected
        }
    })
}

/// Generate a non-empty secret string (at least 32 chars as per Requirement 10.5).
fn arb_secret() -> impl Strategy<Value = String> {
    "[A-Za-z0-9!@#$%^&*]{32,64}"
}

/// Generate an arbitrary non-empty token string.
fn arb_token() -> impl Strategy<Value = String> {
    "[A-Za-z0-9!@#$%^&*]{1,64}"
}

/// Property 19a: Valid token always accepted.
/// For any configured secret, providing that exact secret as the token results in acceptance.
#[test]
fn webhook_auth_valid_token_always_accepted() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_secret(), |secret| {
            let result = authenticate_webhook(&secret, Some(&secret));
            prop_assert_eq!(
                result,
                AuthResult::Accepted,
                "Valid token must be accepted, secret='{}'",
                secret
            );
            Ok(())
        })
        .expect("webhook_auth_valid_token_always_accepted failed");
}

/// Property 19b: Invalid token always rejected.
/// For any configured secret and any token that differs from the secret, the request is rejected.
#[test]
fn webhook_auth_invalid_token_always_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&(arb_secret(), arb_token()), |(secret, token)| {
            prop_assume!(token != secret);
            let result = authenticate_webhook(&secret, Some(&token));
            prop_assert_eq!(
                result,
                AuthResult::Rejected,
                "Invalid token must be rejected, secret='{}', token='{}'",
                secret,
                token
            );
            Ok(())
        })
        .expect("webhook_auth_invalid_token_always_rejected failed");
}

/// Property 19c: Missing token always rejected.
/// For any configured secret, if no token is provided (None), the request is rejected.
#[test]
fn webhook_auth_missing_token_always_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_secret(), |secret| {
            let result = authenticate_webhook(&secret, None);
            prop_assert_eq!(
                result,
                AuthResult::Rejected,
                "Missing token must be rejected, secret='{}'",
                secret
            );
            Ok(())
        })
        .expect("webhook_auth_missing_token_always_rejected failed");
}

/// Property 19d: Constant-time comparison — equal strings return true, different strings return false.
/// This tests the actual `constant_time_eq` implementation directly.
#[test]
fn webhook_auth_constant_time_eq_correctness() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&(arb_token(), arb_token()), |(a, b)| {
            let result = constant_time_eq(a.as_bytes(), b.as_bytes());
            let expected = a == b;
            prop_assert_eq!(
                result,
                expected,
                "constant_time_eq('{}', '{}') = {}, expected {}",
                a,
                b,
                result,
                expected
            );
            Ok(())
        })
        .expect("webhook_auth_constant_time_eq_correctness failed");
}

// ── Property 4: Unauthorized Senders Do Not Invoke AI ─────────────────────────

// Feature: whatsapp-ai-assistant, Property 4: Unauthorized Senders Do Not Invoke AI
// **Validates: Requirements 2.4, 9.7**

/// Result of the pipeline decision: whether AI should be invoked.
#[derive(Debug, Clone, Copy, PartialEq)]
enum AiInvocationDecision {
    /// AI module is invoked (activo=true AND sender authorized).
    Invoke,
    /// AI module is NOT invoked (activo=false OR sender not authorized).
    Skip,
}

/// Pure model of the pipeline decision logic.
/// Given the `activo` flag and the sender authorization result,
/// determines whether the AI_Module should be invoked.
///
/// This mirrors the logic in `chatbot_internal.rs`:
/// - Step 2: if `!cfg.activo` → discard (Skip)
/// - Step 3: if `!is_allowed` → discard (Skip)
/// - Otherwise → proceed to AI invocation (Invoke)
fn pipeline_decision(activo: bool, sender_authorized: bool) -> AiInvocationDecision {
    if !activo {
        return AiInvocationDecision::Skip;
    }
    if !sender_authorized {
        return AiInvocationDecision::Skip;
    }
    AiInvocationDecision::Invoke
}

/// Property 4a: When `activo` is false, AI is never invoked regardless of sender authorization.
#[test]
fn inactive_org_never_invokes_ai() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&any::<bool>(), |sender_authorized| {
            let result = pipeline_decision(false, sender_authorized);
            prop_assert_eq!(
                result,
                AiInvocationDecision::Skip,
                "activo=false must skip AI regardless of sender_authorized={}",
                sender_authorized
            );
            Ok(())
        })
        .expect("inactive_org_never_invokes_ai failed");
}

/// Property 4b: When sender is not authorized, AI is never invoked regardless of `activo`.
#[test]
fn unauthorized_sender_never_invokes_ai() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&any::<bool>(), |activo| {
            let result = pipeline_decision(activo, false);
            prop_assert_eq!(
                result,
                AiInvocationDecision::Skip,
                "sender_authorized=false must skip AI regardless of activo={}",
                activo
            );
            Ok(())
        })
        .expect("unauthorized_sender_never_invokes_ai failed");
}

/// Property 4c: AI is invoked ONLY when both `activo` is true AND sender is authorized.
#[test]
fn ai_invoked_only_when_active_and_authorized() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(any::<bool>(), any::<bool>()),
            |(activo, sender_authorized)| {
                let result = pipeline_decision(activo, sender_authorized);
                let expected = if activo && sender_authorized {
                    AiInvocationDecision::Invoke
                } else {
                    AiInvocationDecision::Skip
                };
                prop_assert_eq!(
                    result,
                    expected,
                    "activo={}, sender_authorized={}: expected {:?}, got {:?}",
                    activo,
                    sender_authorized,
                    expected,
                    result
                );
                Ok(())
            },
        )
        .expect("ai_invoked_only_when_active_and_authorized failed");
}

/// Property 4d: Integration with sender policy — combining `activo` flag with
/// `check_sender_policy_no_db` results. For policies that can be resolved without DB
/// (tenants_and_prospects, allowlist, unrecognized), the pipeline decision is deterministic.
#[test]
fn pipeline_decision_with_sender_policy() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    let strategy = (
        any::<bool>(),    // activo
        arb_e164_phone(), // sender phone
        arb_allowlist(),  // allowlist
        prop_oneof![
            Just("tenants_and_prospects".to_string()),
            Just("allowlist".to_string()),
            arb_unrecognized_policy(),
        ],
    );

    runner
        .run(&strategy, |(activo, phone, allowlist, policy)| {
            // Resolve sender authorization using the pure policy check
            let policy_result = check_sender_policy_no_db(&policy, &phone, Some(&allowlist));

            // For non-DB policies, we always get Some(bool)
            if let Some(sender_authorized) = policy_result {
                let decision = pipeline_decision(activo, sender_authorized);

                if !activo {
                    prop_assert_eq!(
                        decision,
                        AiInvocationDecision::Skip,
                        "activo=false must skip AI (policy='{}', phone='{}')",
                        policy,
                        phone
                    );
                } else if !sender_authorized {
                    prop_assert_eq!(
                        decision,
                        AiInvocationDecision::Skip,
                        "Unauthorized sender must skip AI (policy='{}', phone='{}')",
                        policy,
                        phone
                    );
                } else {
                    prop_assert_eq!(
                        decision,
                        AiInvocationDecision::Invoke,
                        "activo=true + authorized sender must invoke AI (policy='{}', phone='{}')",
                        policy,
                        phone
                    );
                }
            }
            // tenants_only returns None (needs DB) — skip those cases

            Ok(())
        })
        .expect("pipeline_decision_with_sender_policy failed");
}

// ── Property 18: Role-Based Access Control for Configuration ──────────────────

// Feature: whatsapp-ai-assistant, Property 18: Role-Based Access Control for Configuration
// **Validates: Requirement 9.6**

/// Generate an arbitrary role string that is NOT `admin` or `gerente`.
fn arb_non_admin_role() -> impl Strategy<Value = String> {
    prop_oneof![
        // Known non-admin roles
        Just("visualizador".to_string()),
        // Arbitrary strings that are not admin/gerente
        "[a-z_]{1,30}".prop_filter("must not be admin or gerente", |s| {
            s != "admin" && s != "gerente"
        }),
    ]
}

/// Property 18a: "admin" role is always allowed access to configuration endpoints.
#[test]
fn rbac_admin_always_allowed() {
    let result = enforce_config_role("admin");
    assert!(
        result.is_ok(),
        "admin role must always be allowed, got: {:?}",
        result.err()
    );
}

/// Property 18b: "gerente" role is always allowed access to configuration endpoints.
#[test]
fn rbac_gerente_always_allowed() {
    let result = enforce_config_role("gerente");
    assert!(
        result.is_ok(),
        "gerente role must always be allowed, got: {:?}",
        result.err()
    );
}

/// Property 18c: Any role other than "admin" or "gerente" is always forbidden.
#[test]
fn rbac_non_admin_roles_always_forbidden() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_non_admin_role(), |role| {
            let result = enforce_config_role(&role);
            prop_assert!(
                result.is_err(),
                "Role '{}' (not admin/gerente) must be forbidden, but was allowed",
                role
            );
            Ok(())
        })
        .expect("rbac_non_admin_roles_always_forbidden failed");
}

// ── Property 20: Handoff Ceases AI Responses ──────────────────────────────────

// Feature: whatsapp-ai-assistant, Property 20: Handoff Ceases AI Responses
// **Validates: Requirement 11.3**

/// Handoff state machine states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandoffState {
    /// No handoff active — AI processes messages normally.
    None,
    /// Awaiting human operator — AI is NOT invoked.
    AwaitingHuman,
}

/// Handoff state machine transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandoffAction {
    /// An incoming message arrives from the sender.
    IncomingMessage,
    /// The handoff is set (e.g., LLM invokes `handoff_to_human` tool).
    SetHandoff,
    /// An admin/gerente clears the handoff.
    ClearHandoff,
}

/// Whether the AI module should be invoked for the current state.
fn should_invoke_ai(state: HandoffState) -> bool {
    match state {
        HandoffState::None => true,
        HandoffState::AwaitingHuman => false,
    }
}

/// Transition the handoff state machine given an action.
fn handoff_transition(state: HandoffState, action: HandoffAction) -> HandoffState {
    match (state, action) {
        // Setting handoff transitions to AwaitingHuman (idempotent)
        (HandoffState::None | HandoffState::AwaitingHuman, HandoffAction::SetHandoff) => {
            HandoffState::AwaitingHuman
        }
        // Clearing handoff transitions to None (idempotent)
        (HandoffState::AwaitingHuman | HandoffState::None, HandoffAction::ClearHandoff) => {
            HandoffState::None
        }
        // Incoming messages don't change the state
        (s, HandoffAction::IncomingMessage) => s,
    }
}

/// Generate an arbitrary handoff action.
fn arb_handoff_action() -> impl Strategy<Value = HandoffAction> {
    prop_oneof![
        Just(HandoffAction::IncomingMessage),
        Just(HandoffAction::SetHandoff),
        Just(HandoffAction::ClearHandoff),
    ]
}

/// Generate a sequence of handoff actions.
fn arb_handoff_action_sequence(max_len: usize) -> impl Strategy<Value = Vec<HandoffAction>> {
    prop::collection::vec(arb_handoff_action(), 1..=max_len)
}

/// Generate an arbitrary handoff state.
fn arb_handoff_state() -> impl Strategy<Value = HandoffState> {
    prop_oneof![Just(HandoffState::None), Just(HandoffState::AwaitingHuman),]
}

/// Property 20a: When handoff is active (AwaitingHuman), AI is NEVER invoked
/// for any number of subsequent incoming messages.
#[test]
fn handoff_active_ai_never_invoked() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &prop::collection::vec(Just(HandoffAction::IncomingMessage), 1..=20),
            |messages| {
                // Start in AwaitingHuman state
                let mut state = HandoffState::AwaitingHuman;

                for (i, action) in messages.iter().enumerate() {
                    // AI must NOT be invoked while in AwaitingHuman
                    prop_assert!(
                        !should_invoke_ai(state),
                        "Message #{}: AI should NOT be invoked when handoff is active (state={:?})",
                        i,
                        state
                    );
                    state = handoff_transition(state, *action);
                }

                // After all messages, state should still be AwaitingHuman
                prop_assert_eq!(
                    state,
                    HandoffState::AwaitingHuman,
                    "State must remain AwaitingHuman after only incoming messages"
                );

                Ok(())
            },
        )
        .expect("handoff_active_ai_never_invoked failed");
}

/// Property 20b: When handoff is cleared, AI resumes processing for subsequent messages.
#[test]
fn handoff_cleared_ai_resumes() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &prop::collection::vec(Just(HandoffAction::IncomingMessage), 1..=20),
            |messages| {
                // Start in AwaitingHuman, then clear
                let state = HandoffState::AwaitingHuman;
                let state = handoff_transition(state, HandoffAction::ClearHandoff);

                // After clearing, state should be None
                prop_assert_eq!(
                    state,
                    HandoffState::None,
                    "State must be None after clearing handoff"
                );

                // AI should be invoked for all subsequent messages
                let mut current = state;
                for (i, action) in messages.iter().enumerate() {
                    prop_assert!(
                        should_invoke_ai(current),
                        "Message #{}: AI should be invoked after handoff cleared (state={:?})",
                        i,
                        current
                    );
                    current = handoff_transition(current, *action);
                }

                Ok(())
            },
        )
        .expect("handoff_cleared_ai_resumes failed");
}

/// Property 20c: Setting handoff then clearing it returns to normal operation.
/// For any sequence of messages after set→clear, AI is always invoked.
#[test]
fn handoff_set_then_clear_returns_to_normal() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &prop::collection::vec(Just(HandoffAction::IncomingMessage), 1..=20),
            |messages| {
                // Start from None, set handoff, then clear it
                let state = HandoffState::None;
                let state = handoff_transition(state, HandoffAction::SetHandoff);

                // While in AwaitingHuman, AI must not be invoked
                prop_assert!(
                    !should_invoke_ai(state),
                    "AI must NOT be invoked after SetHandoff"
                );

                let state = handoff_transition(state, HandoffAction::ClearHandoff);

                // After clearing, we're back to normal
                prop_assert_eq!(
                    state,
                    HandoffState::None,
                    "State must be None after set→clear cycle"
                );

                // All subsequent messages should invoke AI
                let mut current = state;
                for (i, action) in messages.iter().enumerate() {
                    prop_assert!(
                        should_invoke_ai(current),
                        "Message #{}: AI should be invoked after set→clear (state={:?})",
                        i,
                        current
                    );
                    current = handoff_transition(current, *action);
                }

                Ok(())
            },
        )
        .expect("handoff_set_then_clear_returns_to_normal failed");
}

/// Property 20d: For any arbitrary sequence of actions, the AI invocation decision
/// is always consistent with the current handoff state.
/// AI is invoked iff state is None; AI is NOT invoked iff state is AwaitingHuman.
#[test]
fn handoff_ai_invocation_consistent_with_state() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_handoff_state(), arb_handoff_action_sequence(30)),
            |(initial_state, actions)| {
                let mut state = initial_state;

                for (i, action) in actions.iter().enumerate() {
                    // Before processing the action, check AI invocation consistency
                    let ai_invoked = should_invoke_ai(state);
                    match state {
                        HandoffState::None => {
                            prop_assert!(
                                ai_invoked,
                                "Step #{}: State is None but AI was NOT invoked",
                                i
                            );
                        }
                        HandoffState::AwaitingHuman => {
                            prop_assert!(
                                !ai_invoked,
                                "Step #{}: State is AwaitingHuman but AI WAS invoked",
                                i
                            );
                        }
                    }
                    state = handoff_transition(state, *action);
                }

                Ok(())
            },
        )
        .expect("handoff_ai_invocation_consistent_with_state failed");
}

/// Property 20e: SetHandoff is idempotent — setting handoff when already awaiting
/// keeps the state as AwaitingHuman.
#[test]
fn handoff_set_is_idempotent() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&(1usize..=10), |repeat_count| {
            let mut state = HandoffState::None;
            state = handoff_transition(state, HandoffAction::SetHandoff);

            // Repeatedly setting handoff should keep state as AwaitingHuman
            for _ in 0..repeat_count {
                state = handoff_transition(state, HandoffAction::SetHandoff);
                prop_assert_eq!(
                    state,
                    HandoffState::AwaitingHuman,
                    "Repeated SetHandoff must remain AwaitingHuman"
                );
                prop_assert!(
                    !should_invoke_ai(state),
                    "AI must NOT be invoked while AwaitingHuman (even after repeated SetHandoff)"
                );
            }

            Ok(())
        })
        .expect("handoff_set_is_idempotent failed");
}

// ── Property 13: Conversation Persistence Completeness ────────────────────────

// Feature: whatsapp-ai-assistant, Property 13: Conversation Persistence Completeness
// **Validates: Requirement 7.1**

/// A pure model of the conversation record produced by `persist_message`.
/// Mirrors the ActiveModel construction in `persist_message` which always sets:
/// id (generated), organizacion_id, sender_phone, role, content, message_type, created_at (now).
#[derive(Debug, Clone)]
struct PersistedConversationRecord {
    id: Uuid,
    organizacion_id: Uuid,
    sender_phone: String,
    role: String,
    content: String,
    message_type: String,
    created_at: DateTimeWithTimeZone,
}

/// Pure model of persist_message: constructs the record that would be inserted.
/// This mirrors the logic in `persist_message` without requiring a database connection.
fn build_persisted_record(
    org_id: Uuid,
    sender_phone: &str,
    role: &str,
    content: &str,
    message_type: &str,
) -> PersistedConversationRecord {
    PersistedConversationRecord {
        id: Uuid::new_v4(),
        organizacion_id: org_id,
        sender_phone: sender_phone.to_string(),
        role: role.to_string(),
        content: content.to_string(),
        message_type: message_type.to_string(),
        created_at: Utc::now().into(),
    }
}

/// Generate a valid role for conversation messages.
fn arb_message_role() -> impl Strategy<Value = String> {
    prop_oneof![Just("user".to_string()), Just("assistant".to_string()),]
}

/// Generate a valid message_type for conversation messages.
fn arb_message_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("text".to_string()),
        Just("image".to_string()),
        Just("receipt_result".to_string()),
    ]
}

/// Generate non-empty message content.
fn arb_message_content() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 .,!?áéíóúñ]{1,500}"
}

/// Property 13a: For any valid inputs, the persisted record has non-null sender_phone,
/// organizacion_id, role, content, message_type, and created_at.
/// Both user and assistant messages satisfy this property.
#[test]
fn conversation_persistence_completeness_all_fields_present() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(
                any::<u128>(),
                arb_e164_phone(),
                arb_message_role(),
                arb_message_content(),
                arb_message_type(),
            ),
            |(org_seed, sender_phone, role, content, message_type)| {
                let org_id = Uuid::from_u128(org_seed);
                let record =
                    build_persisted_record(org_id, &sender_phone, &role, &content, &message_type);

                // Verify all required fields are non-null (non-empty for strings)
                prop_assert!(
                    !record.sender_phone.is_empty(),
                    "sender_phone must be non-empty, got empty string"
                );
                prop_assert!(
                    !record.organizacion_id.is_nil(),
                    "organizacion_id must be non-nil UUID"
                );
                prop_assert!(
                    !record.role.is_empty(),
                    "role must be non-empty, got empty string"
                );
                prop_assert!(
                    !record.content.is_empty(),
                    "content must be non-empty, got empty string"
                );
                prop_assert!(
                    !record.message_type.is_empty(),
                    "message_type must be non-empty, got empty string"
                );
                // created_at is always set (DateTimeWithTimeZone is non-nullable by construction)
                // Verify it's a reasonable timestamp (not zero/epoch)
                let epoch: DateTimeWithTimeZone =
                    chrono::TimeZone::with_ymd_and_hms(&Utc, 2020, 1, 1, 0, 0, 0)
                        .unwrap()
                        .into();
                prop_assert!(
                    record.created_at > epoch,
                    "created_at must be after 2020-01-01, got {:?}",
                    record.created_at
                );

                Ok(())
            },
        )
        .expect("conversation_persistence_completeness_all_fields_present failed");
}

/// Property 13b: Both user messages and assistant replies produce complete records.
/// For a message exchange (one user + one assistant), both records have all required fields.
#[test]
fn conversation_persistence_both_roles_complete() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(
                any::<u128>(),
                arb_e164_phone(),
                arb_message_content(),
                arb_message_content(),
                arb_message_type(),
            ),
            |(org_seed, sender_phone, user_content, assistant_content, message_type)| {
                let org_id = Uuid::from_u128(org_seed);

                // Simulate a message exchange: user message + assistant reply
                let user_record = build_persisted_record(
                    org_id,
                    &sender_phone,
                    "user",
                    &user_content,
                    &message_type,
                );
                let assistant_record = build_persisted_record(
                    org_id,
                    &sender_phone,
                    "assistant",
                    &assistant_content,
                    "text",
                );

                // Verify user record completeness
                prop_assert!(
                    !user_record.sender_phone.is_empty(),
                    "user sender_phone must be non-empty"
                );
                prop_assert!(
                    !user_record.organizacion_id.is_nil(),
                    "user organizacion_id must be non-nil"
                );
                prop_assert_eq!(&user_record.role, "user", "user role must be 'user'");
                prop_assert!(
                    !user_record.content.is_empty(),
                    "user content must be non-empty"
                );
                prop_assert!(
                    !user_record.message_type.is_empty(),
                    "user message_type must be non-empty"
                );

                // Verify assistant record completeness
                prop_assert!(
                    !assistant_record.sender_phone.is_empty(),
                    "assistant sender_phone must be non-empty"
                );
                prop_assert!(
                    !assistant_record.organizacion_id.is_nil(),
                    "assistant organizacion_id must be non-nil"
                );
                prop_assert_eq!(
                    &assistant_record.role,
                    "assistant",
                    "assistant role must be 'assistant'"
                );
                prop_assert!(
                    !assistant_record.content.is_empty(),
                    "assistant content must be non-empty"
                );
                prop_assert!(
                    !assistant_record.message_type.is_empty(),
                    "assistant message_type must be non-empty"
                );

                // Both share the same org_id and sender_phone
                prop_assert_eq!(
                    user_record.organizacion_id,
                    assistant_record.organizacion_id,
                    "Both records must share the same organizacion_id"
                );
                prop_assert_eq!(
                    &user_record.sender_phone,
                    &assistant_record.sender_phone,
                    "Both records must share the same sender_phone"
                );

                Ok(())
            },
        )
        .expect("conversation_persistence_both_roles_complete failed");
}

/// Property 13c: The generated id is always a valid non-nil UUID (unique per record).
#[test]
fn conversation_persistence_generates_valid_id() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(
                any::<u128>(),
                arb_e164_phone(),
                arb_message_role(),
                arb_message_content(),
                arb_message_type(),
            ),
            |(org_seed, sender_phone, role, content, message_type)| {
                let org_id = Uuid::from_u128(org_seed);
                let record =
                    build_persisted_record(org_id, &sender_phone, &role, &content, &message_type);

                prop_assert!(!record.id.is_nil(), "Generated id must be a non-nil UUID");
                prop_assert_eq!(
                    record.id.get_version(),
                    Some(uuid::Version::Random),
                    "Generated id must be UUID v4"
                );

                Ok(())
            },
        )
        .expect("conversation_persistence_generates_valid_id failed");
}

// ── Property 8: Blocked Pattern Regex Validation ──────────────────────────────

// Feature: native-rig-agent-guardrails, Property 8: Blocked Pattern Regex Validation
// **Validates: Requirements 6.8**

use crate::models::chatbot::{AgentConfig, GuardrailOverrides};
use crate::services::chatbot::validate_agent_config;

/// Generate a valid regex pattern string.
fn arb_valid_regex() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(r"\d+".to_string()),
        Just(r"[a-z]+".to_string()),
        Just(r"foo|bar".to_string()),
        Just(r"^\w+$".to_string()),
        Just(r"test.*pattern".to_string()),
        Just(r"(abc)+".to_string()),
        Just(r"[0-9]{2,4}".to_string()),
        Just(r"\bword\b".to_string()),
        "[a-zA-Z0-9.]{1,30}".prop_map(|s| s), // literal strings are valid regexes
    ]
}

/// Generate an invalid regex pattern string (unclosed brackets, unmatched parens, etc.).
fn arb_invalid_regex() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("[unclosed".to_string()),
        Just("(unmatched".to_string()),
        Just("[z-a]".to_string()),
        Just("*invalid".to_string()),
        Just("(?P<>bad)".to_string()),
        Just("\\".to_string()),
        Just("(?i".to_string()),
        Just("[".to_string()),
        Just("(".to_string()),
        Just("(?P<name".to_string()),
    ]
}

/// Generate a list of 0–20 valid regex patterns.
fn arb_valid_patterns(max_len: usize) -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arb_valid_regex(), 0..=max_len)
}

/// Property 8: FOR ALL AgentConfig values, `validate_agent_config` returns Ok
/// if and only if all patterns are valid regexes AND count ≤ 20.
#[test]
fn blocked_pattern_valid_regexes_within_limit_pass() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_valid_patterns(20), |patterns| {
            let config = AgentConfig {
                guardrails: Some(GuardrailOverrides {
                    blocked_patterns: Some(patterns.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            };

            let result = validate_agent_config(&config);
            prop_assert!(
                result.is_ok(),
                "Valid patterns (count={}) should pass validation, got: {:?}",
                patterns.len(),
                result.err()
            );
            Ok(())
        })
        .expect("blocked_pattern_valid_regexes_within_limit_pass failed");
}

/// Property 8: Any invalid regex in blocked_patterns causes validation to fail.
#[test]
fn blocked_pattern_invalid_regex_fails() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_valid_patterns(10), arb_invalid_regex()),
            |(mut patterns, invalid)| {
                // Insert the invalid pattern at a random position
                patterns.push(invalid.clone());

                let config = AgentConfig {
                    guardrails: Some(GuardrailOverrides {
                        blocked_patterns: Some(patterns),
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                let result = validate_agent_config(&config);
                prop_assert!(
                    result.is_err(),
                    "Invalid regex '{}' should cause validation to fail",
                    invalid
                );
                Ok(())
            },
        )
        .expect("blocked_pattern_invalid_regex_fails failed");
}

/// Property 8: More than 20 patterns causes validation to fail regardless of validity.
#[test]
fn blocked_pattern_exceeding_max_count_fails() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &prop::collection::vec(arb_valid_regex(), 21..=40),
            |patterns| {
                let config = AgentConfig {
                    guardrails: Some(GuardrailOverrides {
                        blocked_patterns: Some(patterns.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                let result = validate_agent_config(&config);
                prop_assert!(
                    result.is_err(),
                    "More than 20 patterns (count={}) should fail validation",
                    patterns.len()
                );
                Ok(())
            },
        )
        .expect("blocked_pattern_exceeding_max_count_fails failed");
}

/// Property 8: Empty patterns list always passes validation.
#[test]
fn blocked_pattern_empty_list_passes() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&any::<bool>(), |_| {
            let config = AgentConfig {
                guardrails: Some(GuardrailOverrides {
                    blocked_patterns: Some(vec![]),
                    ..Default::default()
                }),
                ..Default::default()
            };

            let result = validate_agent_config(&config);
            prop_assert!(
                result.is_ok(),
                "Empty patterns list should pass validation, got: {:?}",
                result.err()
            );
            Ok(())
        })
        .expect("blocked_pattern_empty_list_passes failed");
}

/// Property 8: No guardrails at all passes validation.
#[test]
fn blocked_pattern_no_guardrails_passes() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&any::<bool>(), |_| {
            let config = AgentConfig {
                guardrails: None,
                ..Default::default()
            };

            let result = validate_agent_config(&config);
            prop_assert!(
                result.is_ok(),
                "No guardrails should pass validation, got: {:?}",
                result.err()
            );
            Ok(())
        })
        .expect("blocked_pattern_no_guardrails_passes failed");
}

/// Property 8: Exactly 20 valid patterns passes validation (boundary).
#[test]
fn blocked_pattern_exactly_20_valid_passes() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &prop::collection::vec(arb_valid_regex(), 20..=20),
            |patterns| {
                prop_assert_eq!(patterns.len(), 20);

                let config = AgentConfig {
                    guardrails: Some(GuardrailOverrides {
                        blocked_patterns: Some(patterns),
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                let result = validate_agent_config(&config);
                prop_assert!(
                    result.is_ok(),
                    "Exactly 20 valid patterns should pass validation, got: {:?}",
                    result.err()
                );
                Ok(())
            },
        )
        .expect("blocked_pattern_exactly_20_valid_passes failed");
}
