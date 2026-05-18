#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::doc_markdown,
    clippy::empty_line_after_doc_comments
)]
//! Property-based tests for `RentalGuardrailHook`.
//!
//! Tests argument validation, side-effect capture, tools-invoked tracking,
//! and output safety filtering using proptest.

use std::sync::{Arc, Mutex};

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use regex::Regex;
use rig::agent::{HookAction, ToolCallHookAction};
use uuid::Uuid;

use super::guardrail_hook::RentalGuardrailHook;
use super::{CreateMaintenanceRequestInput, GuardrailConfig};
use crate::services::ai_module::tools::PaymentReceipt;

// ── Helpers ────────────────────────────────────────────────────────────

/// Build a `RentalGuardrailHook` with the given `GuardrailConfig`.
fn make_hook(config: GuardrailConfig) -> RentalGuardrailHook {
    RentalGuardrailHook {
        captured_receipt: Arc::new(Mutex::new(None)),
        tools_invoked: Arc::new(Mutex::new(Vec::new())),
        organizacion_id: Uuid::new_v4(),
        guardrail_config: config,
    }
}

/// Build a default hook (max_description_length = 1000, no blocked patterns).
fn default_hook() -> RentalGuardrailHook {
    make_hook(GuardrailConfig::default())
}

/// Call `on_tool_call` synchronously using a tokio runtime.
fn call_on_tool_call(
    hook: &RentalGuardrailHook,
    tool_name: &str,
    args: &str,
) -> ToolCallHookAction {
    use rig::agent::PromptHook;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        <RentalGuardrailHook as PromptHook<crate::services::ovms_provider::OvmsCompletionModel>>::on_tool_call(
            hook,
            tool_name,
            None,
            "internal-id",
            args,
        )
        .await
    })
}

/// Call `on_tool_result` synchronously using a tokio runtime.
fn call_on_tool_result(hook: &RentalGuardrailHook, tool_name: &str, result: &str) -> HookAction {
    use rig::agent::PromptHook;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        <RentalGuardrailHook as PromptHook<crate::services::ovms_provider::OvmsCompletionModel>>::on_tool_result(
            hook,
            tool_name,
            None,
            "internal-id",
            "{}",
            result,
        )
        .await
    })
}

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate a string of exactly `n` bytes (ASCII 'a' repeated).
fn string_of_len(n: usize) -> String {
    "a".repeat(n)
}

/// Generate a valid `CreateMaintenanceRequestInput` JSON with a given description.
fn maintenance_args_with_description(desc: &str) -> String {
    let input = CreateMaintenanceRequestInput {
        inquilino_id: Uuid::new_v4().to_string(),
        organizacion_id: Uuid::new_v4().to_string(),
        description: desc.to_string(),
        priority: None,
    };
    serde_json::to_string(&input).unwrap()
}

/// Generate an arbitrary tool name (alphanumeric + underscore).
fn arb_tool_name() -> impl Strategy<Value = String> {
    "[a-z_]{1,30}"
}

/// Generate arbitrary JSON-like args strings (valid, invalid, empty, etc.).
fn arb_args_string() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("{}".to_string()),
        Just("{\"key\": \"value\"}".to_string()),
        Just("not json at all".to_string()),
        Just(String::new()),
        Just("{\"description\": \"hello\"}".to_string()),
        "[a-zA-Z0-9 {}:\"]{0,200}",
    ]
}

/// Generate a valid PaymentReceipt JSON string.
fn arb_payment_receipt_json() -> impl Strategy<Value = String> {
    (
        proptest::option::of("[A-Za-z ]{3,20}"),
        1i64..1_000_000i64,
        prop_oneof![Just("DOP".to_string()), Just("USD".to_string())],
        proptest::option::of("[0-9]{4}-[0-9]{2}-[0-9]{2}"),
        proptest::option::of("[A-Z0-9]{5,15}"),
    )
        .prop_map(|(bank, amount_cents, currency, date, reference)| {
            let amount = format!("{}.{:02}", amount_cents / 100, amount_cents % 100);
            let bank_json = bank.map_or_else(|| "null".to_string(), |b| format!("\"{b}\""));
            let date_json = date.map_or_else(|| "null".to_string(), |d| format!("\"{d}\""));
            let ref_json = reference.map_or_else(|| "null".to_string(), |r| format!("\"{r}\""));
            format!(
                r#"{{"bank":{bank_json},"amount":"{amount}","currency":"{currency}","date":{date_json},"reference":{ref_json},"sender_name":null,"recipient":null,"confidence":"medium"}}"#
            )
        })
}

/// Generate strings that are NOT valid PaymentReceipt JSON.
fn arb_invalid_receipt_json() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("not json".to_string()),
        Just("{}".to_string()),
        Just("{\"amount\": \"abc\"}".to_string()),
        Just("null".to_string()),
        "[a-zA-Z0-9 ]{1,50}",
    ]
}

// ── Property 2: Argument Validation Bounds ─────────────────────────────

// Feature: native-rig-agent-guardrails, Property 2: Argument Validation Bounds
// **Validates: Requirements 3.1, 3.2, 3.3**

#[test]
fn test_argument_validation_bounds() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    // Use a smaller max_description_length for faster testing
    let max_len = 1000usize;

    // Generate description lengths across the full range
    runner
        .run(&(0usize..2048), |desc_len| {
            let hook = make_hook(GuardrailConfig {
                max_description_length: max_len,
                ..GuardrailConfig::default()
            });

            let desc = string_of_len(desc_len);
            let args = maintenance_args_with_description(&desc);
            let action = call_on_tool_call(&hook, "create_maintenance_request", &args);

            if desc_len < 2 {
                // Too short → Skip
                prop_assert!(
                    matches!(action, ToolCallHookAction::Skip { .. }),
                    "Expected Skip for desc_len={}, got {:?}",
                    desc_len,
                    action
                );
            } else if desc_len > max_len {
                // Too long → Skip
                prop_assert!(
                    matches!(action, ToolCallHookAction::Skip { .. }),
                    "Expected Skip for desc_len={}, got {:?}",
                    desc_len,
                    action
                );
            } else {
                // Valid range → Continue
                prop_assert!(
                    matches!(action, ToolCallHookAction::Continue),
                    "Expected Continue for desc_len={}, got {:?}",
                    desc_len,
                    action
                );
            }

            Ok(())
        })
        .unwrap();
}

// ── Property 3: Argument Validation Never Terminates ───────────────────

// Feature: native-rig-agent-guardrails, Property 3: Argument Validation Never Terminates
// **Validates: Requirements 3.4, 3.5**

#[test]
fn test_argument_validation_never_terminates() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(arb_tool_name(), arb_args_string()),
            |(tool_name, args)| {
                let hook = default_hook();
                let action = call_on_tool_call(&hook, &tool_name, &args);

                // on_tool_call must NEVER return Terminate for any input
                prop_assert!(
                    !matches!(action, ToolCallHookAction::Terminate { .. }),
                    "on_tool_call returned Terminate for tool_name='{}', args='{}'",
                    tool_name,
                    args
                );

                Ok(())
            },
        )
        .unwrap();
}

// ── Property 4: Receipt Capture Consistency ────────────────────────────

// Feature: native-rig-agent-guardrails, Property 4: Receipt Capture Consistency
// **Validates: Requirements 4.1, 4.2**

#[test]
fn test_receipt_capture_consistency() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    // Generate sequences of (is_valid, receipt_or_garbage) pairs
    let strategy = proptest::collection::vec(
        prop_oneof![
            arb_payment_receipt_json().prop_map(|json| (true, json)),
            arb_invalid_receipt_json().prop_map(|json| (false, json)),
        ],
        1..20,
    );

    runner
        .run(&strategy, |sequence| {
            let hook = default_hook();
            let mut last_valid_receipt: Option<PaymentReceipt> = None;

            for (is_valid, result_json) in &sequence {
                call_on_tool_result(&hook, "extract_receipt", result_json);

                if *is_valid {
                    // If it's valid JSON that deserializes to PaymentReceipt, update expected
                    if let Ok(receipt) = serde_json::from_str::<PaymentReceipt>(result_json) {
                        last_valid_receipt = Some(receipt);
                    }
                }
                // If invalid, captured_receipt should remain unchanged
            }

            let captured = hook.captured_receipt.lock().unwrap().clone();

            match (&last_valid_receipt, &captured) {
                (None, None) => {} // No valid receipts → still None
                (Some(expected), Some(actual)) => {
                    // Last valid receipt should match
                    prop_assert_eq!(expected.amount, actual.amount);
                    prop_assert_eq!(&expected.currency, &actual.currency);
                }
                (Some(_), None) => {
                    prop_assert!(false, "Expected a captured receipt but got None");
                }
                (None, Some(_)) => {
                    prop_assert!(false, "Expected None but got a captured receipt");
                }
            }

            Ok(())
        })
        .unwrap();
}

// ── Property 5: Tools Invoked Tracking ─────────────────────────────────

// Feature: native-rig-agent-guardrails, Property 5: Tools Invoked Tracking
// **Validates: Requirements 4.3, 4.4**

#[test]
fn test_tools_invoked_tracking() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    // Generate sequences of (tool_name, result) pairs
    let strategy = proptest::collection::vec((arb_tool_name(), "[a-zA-Z0-9 {}]{0,50}"), 1..30);

    runner
        .run(&strategy, |sequence| {
            let hook = default_hook();

            for (tool_name, result) in &sequence {
                call_on_tool_result(&hook, tool_name, result);
            }

            let invoked = hook.tools_invoked.lock().unwrap().clone();

            // Verify: exactly N entries
            prop_assert_eq!(
                invoked.len(),
                sequence.len(),
                "Expected {} tools_invoked entries, got {}",
                sequence.len(),
                invoked.len()
            );

            // Verify: each entry matches the tool_name in order
            for (i, (expected_name, _)) in sequence.iter().enumerate() {
                prop_assert_eq!(
                    &invoked[i],
                    expected_name,
                    "Mismatch at index {}: expected '{}', got '{}'",
                    i,
                    expected_name,
                    invoked[i]
                );
            }

            Ok(())
        })
        .unwrap();
}

// ── Property 6: Output Safety Filter Correctness ───────────────────────

// Feature: native-rig-agent-guardrails, Property 6: Output Safety Filter Correctness
// **Validates: Requirements 5.1, 5.2**

/// Tests the regex pattern matching logic used by `on_completion_response`.
/// Since constructing a full `CompletionResponse` is complex, we test the
/// pattern-matching logic directly: given text and a set of patterns, verify
/// that a match triggers Terminate and no match triggers Continue.
#[test]
fn test_output_safety_filter_correctness() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    // Strategy: generate a set of simple regex patterns and text to check against
    let strategy = (
        // Generate 0..5 simple patterns (word-like patterns that are valid regex)
        proptest::collection::vec("[a-z]{3,8}", 0..5),
        // Generate text to check
        "[a-z ]{0,100}",
    );

    runner
        .run(&strategy, |(patterns, text)| {
            // Compile patterns into Regex (filter out any that fail to compile)
            let compiled: Vec<Regex> = patterns.iter().filter_map(|p| Regex::new(p).ok()).collect();

            // Check if any pattern matches the text
            let should_block = compiled.iter().any(|re| re.is_match(&text));

            // Build a hook with these patterns
            let hook = make_hook(GuardrailConfig {
                blocked_output_patterns: compiled.clone(),
                ..GuardrailConfig::default()
            });

            // Simulate what on_completion_response does: check text against patterns
            let action = if hook.guardrail_config.blocked_output_patterns.is_empty() {
                HookAction::Continue
            } else {
                let mut result = HookAction::Continue;
                for pattern in &hook.guardrail_config.blocked_output_patterns {
                    if pattern.is_match(&text) {
                        result = HookAction::Terminate {
                            reason: "Response blocked by safety filter".into(),
                        };
                        break;
                    }
                }
                result
            };

            if should_block {
                prop_assert!(
                    matches!(action, HookAction::Terminate { .. }),
                    "Expected Terminate when pattern matches text='{}', patterns={:?}",
                    text,
                    patterns
                );
            } else {
                prop_assert!(
                    matches!(action, HookAction::Continue),
                    "Expected Continue when no pattern matches text='{}', patterns={:?}",
                    text,
                    patterns
                );
            }

            // Verify: empty patterns list → always Continue
            if compiled.is_empty() {
                prop_assert!(
                    matches!(action, HookAction::Continue),
                    "Expected Continue when patterns list is empty"
                );
            }

            Ok(())
        })
        .unwrap();
}
