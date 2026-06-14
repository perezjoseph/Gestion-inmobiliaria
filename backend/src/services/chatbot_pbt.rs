#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::doc_markdown,
    clippy::empty_line_after_doc_comments
)]

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

use crate::models::chatbot::{Confidence, map_confidence};
use crate::services::chatbot::{check_sender_policy_no_db, is_phone_in_allowlist};

fn arb_e164_phone() -> impl Strategy<Value = String> {
    (1..=9u8, proptest::collection::vec(0..=9u8, 6..14)).prop_map(|(first, rest)| {
        let mut phone = format!("+{first}");
        for d in rest {
            phone.push(char::from(b'0' + d));
        }
        phone
    })
}

fn arb_allowlist() -> impl Strategy<Value = Vec<String>> {
    proptest::collection::vec(arb_e164_phone(), 1..20)
}

fn arb_unrecognized_policy() -> impl Strategy<Value = String> {
    "[a-z_]{1,30}".prop_filter("must not be a recognized policy", |s| {
        s != "tenants_only" && s != "tenants_and_prospects" && s != "allowlist"
    })
}

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

fn invalid_e164() -> impl Strategy<Value = String> {
    prop_oneof![
        (1u8..=9, prop::collection::vec(0u8..=9, 1..=14)).prop_map(|(first, rest)| {
            let mut s = String::new();
            s.push(char::from(b'0' + first));
            for d in rest {
                s.push(char::from(b'0' + d));
            }
            s
        }),
        prop::collection::vec(0u8..=9, 1..=14).prop_map(|rest| {
            let mut s = String::from("+0");
            for d in rest {
                s.push(char::from(b'0' + d));
            }
            s
        }),
        (1u8..=9, prop::collection::vec(0u8..=9, 15..=20)).prop_map(|(first, rest)| {
            let mut s = String::from("+");
            s.push(char::from(b'0' + first));
            for d in rest {
                s.push(char::from(b'0' + d));
            }
            s
        }),
        Just(String::new()),
        Just("+".to_string()),
        (1u8..=9, prop::collection::vec(0u8..=9, 3..=8)).prop_map(|(first, rest)| {
            let mut s = String::from("+");
            s.push(char::from(b'0' + first));
            s.push('-');
            for d in rest {
                s.push(char::from(b'0' + d));
            }
            s
        }),
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

fn valid_country_code() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("+1".to_string()),
        Just("+44".to_string()),
        Just("+34".to_string()),
        Just("+1809".to_string()),
    ]
}

use crate::services::chatbot::{enforce_config_role, normalize_phone, validate_e164};

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
                Ok(())
            },
        )
        .expect("normalize_phone_output_always_valid_e164 failed");
}

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

use chrono::{Duration, Utc};
use sea_orm::prelude::DateTimeWithTimeZone;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct TestMessage {
    id: Uuid,
    created_at: chrono::DateTime<Utc>,
}

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

fn arb_timestamp_offset_days() -> impl Strategy<Value = i64> {
    -730i64..=730
}

fn arb_retention_days() -> impl Strategy<Value = i64> {
    1i64..=365
}

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

                prop_assert_eq!(
                    kept.len() + deleted.len(),
                    messages.len(),
                    "Partition sizes don't sum to total: kept={}, deleted={}, total={}",
                    kept.len(),
                    deleted.len(),
                    messages.len()
                );

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

            let msg = TestMessage {
                id: Uuid::from_u128(99),
                created_at: cutoff,
            };

            let (kept, deleted) =
                retention_partition(std::slice::from_ref(&msg), now, retention_days);

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

use chrono::DateTime;

use crate::services::chatbot::{TimestampedMessage, window_history};

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

fn arb_history_limit() -> impl Strategy<Value = usize> {
    1..=50usize
}

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

                for pair in result.windows(2) {
                    prop_assert!(
                        pair[0].created_at >= pair[1].created_at,
                        "Messages not in DESC order: {:?} before {:?}",
                        pair[0].created_at,
                        pair[1].created_at
                    );
                }

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

use crate::entities::chatbot_config;
use crate::models::chatbot::{Capabilities, ChatbotConfigUpdateRequest, FaqEntry};
use crate::services::chatbot::{config_model_to_response, validate_config};

fn valid_display_name() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 ]{1,100}"
}

fn valid_tone() -> impl Strategy<Value = String> {
    "[a-z]{1,50}"
}

fn valid_faq_entry() -> impl Strategy<Value = FaqEntry> {
    ("[A-Za-z0-9 ?]{1,200}", "[A-Za-z0-9 .]{1,200}")
        .prop_map(|(question, answer)| FaqEntry { question, answer })
}

fn valid_faqs() -> impl Strategy<Value = Vec<FaqEntry>> {
    prop::collection::vec(valid_faq_entry(), 0..=10)
}

fn valid_policies() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 .\n]{0,500}"
}

fn valid_sender_policy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("tenants_only".to_string()),
        Just("tenants_and_prospects".to_string()),
        Just("allowlist".to_string()),
    ]
}

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

fn valid_history_limit() -> impl Strategy<Value = i32> {
    1..=50i32
}

fn valid_retention_days() -> impl Strategy<Value = i32> {
    1..=365i32
}

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

#[test]
fn config_round_trip_preserves_all_fields() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&valid_config_update(), |input| {
            prop_assert!(
                validate_config(&input).is_ok(),
                "validate_config rejected a valid input: {:?}",
                input
            );

            let model = build_model_from_input(&input);
            let response =
                config_model_to_response(model).expect("config_model_to_response failed");

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

            prop_assert_eq!(
                response.policies.as_deref(),
                input.policies.as_deref(),
                "policies mismatch"
            );

            prop_assert_eq!(
                &response.sender_policy,
                input.sender_policy.as_ref().unwrap(),
                "sender_policy mismatch"
            );

            prop_assert_eq!(
                response.allowlist.as_ref(),
                input.allowlist.as_ref(),
                "allowlist mismatch"
            );

            prop_assert_eq!(
                response.handoff_keywords.as_ref(),
                input.handoff_keywords.as_ref(),
                "handoff_keywords mismatch"
            );

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

            prop_assert_eq!(response.activo, input.activo.unwrap(), "activo mismatch");

            Ok(())
        })
        .expect("config_round_trip_preserves_all_fields failed");
}

fn arb_confidence_score() -> impl Strategy<Value = f64> {
    0.0..=1.0f64
}

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

fn arb_confidence() -> impl Strategy<Value = Confidence> {
    prop_oneof![
        Just(Confidence::High),
        Just(Confidence::Medium),
        Just(Confidence::Low),
    ]
}

fn arb_inquilino_id() -> impl Strategy<Value = Option<Uuid>> {
    prop_oneof![
        Just(None),
        any::<u128>().prop_map(|v| Some(Uuid::from_u128(v))),
    ]
}

fn determine_extraction_status(
    _confidence: &Confidence,
    _inquilino_id: Option<Uuid>,
) -> &'static str {
    "pending_confirmation"
}

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

use rust_decimal::Decimal;

use crate::models::chatbot::format_currency;

const OUTSTANDING_STATUSES: &[&str] = &["pendiente", "atrasado"];

#[derive(Debug, Clone)]
struct TestPayment {
    amount: Decimal,
    currency: String,
    status: String,
}

fn arb_payment_amount() -> impl Strategy<Value = Decimal> {
    (1i64..=999_999_999, 0u32..=2).prop_map(|(cents, scale)| Decimal::new(cents, scale))
}

fn arb_currency() -> impl Strategy<Value = String> {
    prop_oneof![Just("DOP".to_string()), Just("USD".to_string()),]
}

fn arb_payment_status() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("pendiente".to_string()),
        Just("pagado".to_string()),
        Just("atrasado".to_string()),
    ]
}

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

#[test]
fn balance_calculation_sums_outstanding_only() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_payments(30), |payments| {
            let totals = calculate_balance(&payments);

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

#[test]
fn balance_never_mixes_currencies() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_payments(30), |payments| {
            let totals = calculate_balance(&payments);

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

#[test]
fn balance_empty_payments_produces_empty_totals() {
    let totals = calculate_balance(&[]);
    assert!(
        totals.is_empty(),
        "Empty payments should produce empty balance"
    );
}

fn arb_format_amount() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        (0i64..=999_999_999, 0u32..=2).prop_map(|(v, s)| Decimal::new(v, s)),
        (-999_999_999i64..=-1, 0u32..=2).prop_map(|(v, s)| Decimal::new(v, s)),
        Just(Decimal::ZERO),
        (1i64..=99, 2u32..=2).prop_map(|(v, s)| Decimal::new(v, s)),
    ]
}

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

            prop_assert!(
                !dop_formatted.contains("US$"),
                "DOP format '{}' contains US$ symbol (amount: {})",
                dop_formatted,
                amount
            );

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

use crate::services::chatbot::resolve_maintenance_defaults;

const TEST_VALID_PRIORITIES: &[&str] = &["baja", "media", "alta", "urgente"];

fn arb_valid_description() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 .,!?]{2,200}"
}

fn arb_short_description() -> impl Strategy<Value = String> {
    prop_oneof![Just(String::new()), "[a-z]{1,1}",]
}

fn arb_long_description() -> impl Strategy<Value = String> {
    "[a-z]{1001,1100}"
}

fn arb_valid_priority() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("baja".to_string()),
        Just("media".to_string()),
        Just("alta".to_string()),
        Just("urgente".to_string()),
    ]
}

fn arb_invalid_priority() -> impl Strategy<Value = String> {
    "[a-z]{1,20}".prop_filter("must not be a valid priority", |s| {
        !TEST_VALID_PRIORITIES.contains(&s.as_str())
    })
}

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

const RECEIPT_STATUS_PENDING: &str = "pending_confirmation";
const RECEIPT_STATUS_CONFIRMED: &str = "confirmed";
const RECEIPT_STATUS_REJECTED: &str = "rejected";

#[derive(Debug, Clone)]
struct TestReceiptExtraction {
    id: Uuid,
    organizacion_id: Uuid,
    status: String,
}

fn filter_pending_receipts(extractions: &[TestReceiptExtraction], org_id: Uuid) -> Vec<Uuid> {
    extractions
        .iter()
        .filter(|e| e.organizacion_id == org_id && e.status == RECEIPT_STATUS_PENDING)
        .map(|e| e.id)
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
enum TransitionResult {
    Success {
        new_status: String,
        confirmed_by: Option<Uuid>,
    },
    Rejected,
}

fn try_confirm(extraction: &TestReceiptExtraction, user_id: Uuid) -> TransitionResult {
    if extraction.status != RECEIPT_STATUS_PENDING {
        return TransitionResult::Rejected;
    }
    TransitionResult::Success {
        new_status: RECEIPT_STATUS_CONFIRMED.to_string(),
        confirmed_by: Some(user_id),
    }
}

fn try_reject(extraction: &TestReceiptExtraction) -> TransitionResult {
    if extraction.status != RECEIPT_STATUS_PENDING {
        return TransitionResult::Rejected;
    }
    TransitionResult::Success {
        new_status: RECEIPT_STATUS_REJECTED.to_string(),
        confirmed_by: None,
    }
}

fn arb_receipt_status() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(RECEIPT_STATUS_PENDING.to_string()),
        Just(RECEIPT_STATUS_CONFIRMED.to_string()),
        Just(RECEIPT_STATUS_REJECTED.to_string()),
    ]
}

fn arb_extraction_with_status(
    status: &'static str,
) -> impl Strategy<Value = TestReceiptExtraction> {
    (any::<u128>(), any::<u128>()).prop_map(move |(id_seed, org_seed)| TestReceiptExtraction {
        id: Uuid::from_u128(id_seed),
        organizacion_id: Uuid::from_u128(org_seed),
        status: status.to_string(),
    })
}

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

                if let TransitionResult::Success { ref new_status, .. } = confirm_result {
                    prop_assert_eq!(
                        new_status,
                        RECEIPT_STATUS_CONFIRMED,
                        "Confirm must produce 'confirmed', got '{}'",
                        new_status
                    );
                }

                if let TransitionResult::Success { ref new_status, .. } = reject_result {
                    prop_assert_eq!(
                        new_status,
                        RECEIPT_STATUS_REJECTED,
                        "Reject must produce 'rejected', got '{}'",
                        new_status
                    );
                }

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

use crate::services::ai_module::{ChatbotPersona, TenantContext, compose_system_prompt};

fn arb_nonempty_string() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 \u{e1}\u{e9}\u{ed}\u{f3}\u{fa}\u{f1}]{1,80}"
}

fn arb_faq_list() -> impl Strategy<Value = Vec<FaqEntry>> {
    prop::collection::vec(
        (arb_nonempty_string(), arb_nonempty_string()).prop_map(|(q, a)| FaqEntry {
            question: q,
            answer: a,
        }),
        1..=10,
    )
}

fn arb_handoff_keywords() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arb_nonempty_string(), 1..=5)
}

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

#[test]
fn system_prompt_ignores_deprecated_system_prompt_field() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&"SYSPROMPT_[A-Za-z0-9]{5,40}", |sys_prompt| {
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

#[test]
fn system_prompt_composition_completeness_combined() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    let strategy = (
        arb_nonempty_string(),
        arb_nonempty_string(),
        arb_nonempty_string(),
        arb_faq_list(),
        arb_nonempty_string(),
        arb_handoff_keywords(),
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

                prop_assert!(
                    prompt.contains(&tone),
                    "Combined: prompt must contain tone '{}'",
                    tone
                );
                prop_assert!(
                    prompt.contains(&greeting),
                    "Combined: prompt must contain greeting '{}'",
                    greeting
                );
                prop_assert!(
                    prompt.contains(&tenant_name),
                    "Combined: prompt must contain tenant name '{}'",
                    tenant_name
                );
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
                prop_assert!(
                    prompt.contains(&policies),
                    "Combined: prompt must contain policies '{}'",
                    policies
                );
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

use crate::services::ai_module::get_enabled_tools;
use std::collections::HashSet;

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

    expected
}

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

use crate::services::crypto::constant_time_eq;

#[derive(Debug, Clone, PartialEq)]
enum AuthResult {
    Accepted,
    Rejected,
}

fn authenticate_webhook(configured_secret: &str, provided_token: Option<&str>) -> AuthResult {
    provided_token.map_or(AuthResult::Rejected, |token| {
        if constant_time_eq(token.as_bytes(), configured_secret.as_bytes()) {
            AuthResult::Accepted
        } else {
            AuthResult::Rejected
        }
    })
}

fn arb_secret() -> impl Strategy<Value = String> {
    "[A-Za-z0-9!@#$%^&*]{32,64}"
}

fn arb_token() -> impl Strategy<Value = String> {
    "[A-Za-z0-9!@#$%^&*]{1,64}"
}

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

#[derive(Debug, Clone, Copy, PartialEq)]
enum AiInvocationDecision {
    Invoke,
    Skip,
}

fn pipeline_decision(activo: bool, sender_authorized: bool) -> AiInvocationDecision {
    if !activo {
        return AiInvocationDecision::Skip;
    }
    if !sender_authorized {
        return AiInvocationDecision::Skip;
    }
    AiInvocationDecision::Invoke
}

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

#[test]
fn pipeline_decision_with_sender_policy() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    let strategy = (
        any::<bool>(),
        arb_e164_phone(),
        arb_allowlist(),
        prop_oneof![
            Just("tenants_and_prospects".to_string()),
            Just("allowlist".to_string()),
            arb_unrecognized_policy(),
        ],
    );

    runner
        .run(&strategy, |(activo, phone, allowlist, policy)| {
            let policy_result = check_sender_policy_no_db(&policy, &phone, Some(&allowlist));

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

            Ok(())
        })
        .expect("pipeline_decision_with_sender_policy failed");
}

fn arb_non_admin_role() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("visualizador".to_string()),
        "[a-z_]{1,30}".prop_filter("must not be admin or gerente", |s| {
            s != "admin" && s != "gerente"
        }),
    ]
}

#[test]
fn rbac_admin_always_allowed() {
    let result = enforce_config_role("admin");
    assert!(
        result.is_ok(),
        "admin role must always be allowed, got: {:?}",
        result.err()
    );
}

#[test]
fn rbac_gerente_always_allowed() {
    let result = enforce_config_role("gerente");
    assert!(
        result.is_ok(),
        "gerente role must always be allowed, got: {:?}",
        result.err()
    );
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandoffState {
    None,
    AwaitingHuman,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandoffAction {
    IncomingMessage,
    SetHandoff,
    ClearHandoff,
}

fn should_invoke_ai(state: HandoffState) -> bool {
    match state {
        HandoffState::None => true,
        HandoffState::AwaitingHuman => false,
    }
}

fn handoff_transition(state: HandoffState, action: HandoffAction) -> HandoffState {
    match (state, action) {
        (HandoffState::None | HandoffState::AwaitingHuman, HandoffAction::SetHandoff) => {
            HandoffState::AwaitingHuman
        }
        (HandoffState::AwaitingHuman | HandoffState::None, HandoffAction::ClearHandoff) => {
            HandoffState::None
        }
        (s, HandoffAction::IncomingMessage) => s,
    }
}

fn arb_handoff_action() -> impl Strategy<Value = HandoffAction> {
    prop_oneof![
        Just(HandoffAction::IncomingMessage),
        Just(HandoffAction::SetHandoff),
        Just(HandoffAction::ClearHandoff),
    ]
}

fn arb_handoff_action_sequence(max_len: usize) -> impl Strategy<Value = Vec<HandoffAction>> {
    prop::collection::vec(arb_handoff_action(), 1..=max_len)
}

fn arb_handoff_state() -> impl Strategy<Value = HandoffState> {
    prop_oneof![Just(HandoffState::None), Just(HandoffState::AwaitingHuman),]
}

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
                let mut state = HandoffState::AwaitingHuman;

                for (i, action) in messages.iter().enumerate() {
                    prop_assert!(
                        !should_invoke_ai(state),
                        "Message #{}: AI should NOT be invoked when handoff is active (state={:?})",
                        i,
                        state
                    );
                    state = handoff_transition(state, *action);
                }

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
                let state = HandoffState::AwaitingHuman;
                let state = handoff_transition(state, HandoffAction::ClearHandoff);

                prop_assert_eq!(
                    state,
                    HandoffState::None,
                    "State must be None after clearing handoff"
                );

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
                let state = HandoffState::None;
                let state = handoff_transition(state, HandoffAction::SetHandoff);

                prop_assert!(
                    !should_invoke_ai(state),
                    "AI must NOT be invoked after SetHandoff"
                );

                let state = handoff_transition(state, HandoffAction::ClearHandoff);

                prop_assert_eq!(
                    state,
                    HandoffState::None,
                    "State must be None after setâ†’clear cycle"
                );

                let mut current = state;
                for (i, action) in messages.iter().enumerate() {
                    prop_assert!(
                        should_invoke_ai(current),
                        "Message #{}: AI should be invoked after setâ†’clear (state={:?})",
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

fn arb_message_role() -> impl Strategy<Value = String> {
    prop_oneof![Just("user".to_string()), Just("assistant".to_string()),]
}

fn arb_message_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("text".to_string()),
        Just("image".to_string()),
        Just("receipt_result".to_string()),
    ]
}

fn arb_message_content() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 .,!?\u{e1}\u{e9}\u{ed}\u{f3}\u{fa}\u{f1}]{1,500}"
}

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
