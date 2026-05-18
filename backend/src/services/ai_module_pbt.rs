#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::doc_markdown,
    clippy::empty_line_after_doc_comments
)]
//! Property-based tests for tool Args schema round-trip.
//!
//! **Validates: Requirements 7.4**

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

use crate::services::ai_module::tools::ExtractReceiptInput;
use crate::services::ai_module::{
    CreateMaintenanceRequestInput, GetPaymentHistoryInput, HandoffToHumanInput, QueryBalanceInput,
};

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate a valid UUID string.
fn arb_uuid_string() -> impl Strategy<Value = String> {
    "[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}"
}

/// Generate a non-empty base64-like string (alphanumeric + /+=).
fn arb_base64_string() -> impl Strategy<Value = String> {
    "[A-Za-z0-9+/]{4,100}={0,2}"
}

/// Generate an arbitrary ExtractReceiptInput.
fn arb_extract_receipt_input() -> impl Strategy<Value = ExtractReceiptInput> {
    (
        arb_base64_string(),
        proptest::option::of("[a-zA-Z0-9 ]{0,50}"),
    )
        .prop_map(|(image_base64, caption)| ExtractReceiptInput {
            image_base64,
            caption,
        })
}

/// Generate an arbitrary QueryBalanceInput.
fn arb_query_balance_input() -> impl Strategy<Value = QueryBalanceInput> {
    (arb_uuid_string(), arb_uuid_string()).prop_map(|(inquilino_id, organizacion_id)| {
        QueryBalanceInput {
            inquilino_id,
            organizacion_id,
        }
    })
}

/// Generate an arbitrary GetPaymentHistoryInput.
fn arb_get_payment_history_input() -> impl Strategy<Value = GetPaymentHistoryInput> {
    (
        arb_uuid_string(),
        arb_uuid_string(),
        proptest::option::of(1..100u32),
    )
        .prop_map(
            |(inquilino_id, organizacion_id, limit)| GetPaymentHistoryInput {
                inquilino_id,
                organizacion_id,
                limit,
            },
        )
}

/// Generate an arbitrary CreateMaintenanceRequestInput.
fn arb_create_maintenance_request_input() -> impl Strategy<Value = CreateMaintenanceRequestInput> {
    (
        arb_uuid_string(),
        arb_uuid_string(),
        "[a-zA-Z0-9 ]{2,200}",
        proptest::option::of(prop_oneof![
            Just("baja".to_string()),
            Just("media".to_string()),
            Just("alta".to_string()),
            Just("urgente".to_string()),
        ]),
    )
        .prop_map(|(inquilino_id, organizacion_id, description, priority)| {
            CreateMaintenanceRequestInput {
                inquilino_id,
                organizacion_id,
                description,
                priority,
            }
        })
}

/// Generate an arbitrary HandoffToHumanInput.
fn arb_handoff_to_human_input() -> impl Strategy<Value = HandoffToHumanInput> {
    proptest::option::of("[a-zA-Z0-9 ]{1,100}").prop_map(|reason| HandoffToHumanInput { reason })
}

// ── Property 9: Tool Args Schema Round-Trip ────────────────────────────

// Feature: native-rig-agent-guardrails, Property 9: Tool Args Schema Round-Trip
// **Validates: Requirements 7.4**

#[test]
fn test_tool_args_round_trip_extract_receipt() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_extract_receipt_input(), |input| {
            let json = serde_json::to_value(&input).expect("serialize");
            let deserialized: ExtractReceiptInput =
                serde_json::from_value(json).expect("deserialize");
            prop_assert_eq!(input, deserialized);
            Ok(())
        })
        .unwrap();
}

#[test]
fn test_tool_args_round_trip_query_balance() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_query_balance_input(), |input| {
            let json = serde_json::to_value(&input).expect("serialize");
            let deserialized: QueryBalanceInput =
                serde_json::from_value(json).expect("deserialize");
            prop_assert_eq!(input, deserialized);
            Ok(())
        })
        .unwrap();
}

#[test]
fn test_tool_args_round_trip_get_payment_history() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_get_payment_history_input(), |input| {
            let json = serde_json::to_value(&input).expect("serialize");
            let deserialized: GetPaymentHistoryInput =
                serde_json::from_value(json).expect("deserialize");
            prop_assert_eq!(input, deserialized);
            Ok(())
        })
        .unwrap();
}

#[test]
fn test_tool_args_round_trip_create_maintenance_request() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_create_maintenance_request_input(), |input| {
            let json = serde_json::to_value(&input).expect("serialize");
            let deserialized: CreateMaintenanceRequestInput =
                serde_json::from_value(json).expect("deserialize");
            prop_assert_eq!(input, deserialized);
            Ok(())
        })
        .unwrap();
}

#[test]
fn test_tool_args_round_trip_handoff_to_human() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_handoff_to_human_input(), |input| {
            let json = serde_json::to_value(&input).expect("serialize");
            let deserialized: HandoffToHumanInput =
                serde_json::from_value(json).expect("deserialize");
            prop_assert_eq!(input, deserialized);
            Ok(())
        })
        .unwrap();
}
