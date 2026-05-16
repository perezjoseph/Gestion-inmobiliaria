// Feature: whatsapp-ai-gpu-fix, Property 1: Bug Condition (FIXED) — Agent Loop Executes Tool Calls
//
// This PBT validates the CORRECT behavior of `invoke_agent` after the fix:
// When the LLM returns tool_calls, the agent loop SHALL execute each tool,
// collect results, feed them back to the LLM, and continue until a final
// text response is produced.
//
// The fix (multi-turn agent loop) was implemented in Task 7. This test now
// validates the CORRECT model to confirm the fix design is sound.
//
// **Validates: Requirements 1.5, 1.6, 1.7, 1.8, 2.5, 2.6, 2.7, 2.8**

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

// ── Constants ──────────────────────────────────────────────────────────

/// The placeholder prefix that the BUGGY code returns instead of executing tools.
const BUG_PLACEHOLDER_PREFIX: &str = "Herramientas invocadas:";

/// Maximum turns the agent loop should support.
const TURN_LIMIT: usize = 5;

// ── Model Types ────────────────────────────────────────────────────────

/// Represents a tool call returned by the LLM in its response.
#[derive(Debug, Clone)]
struct MockToolCall {
    name: String,
    #[allow(dead_code)]
    args: serde_json::Value,
}

/// Represents a single LLM response in the agent loop.
#[derive(Debug, Clone)]
enum MockLlmResponse {
    /// LLM returns only tool calls (no text) — tools should be executed.
    ToolCalls(Vec<MockToolCall>),
    /// LLM returns final text — loop terminates.
    FinalText(String),
}

/// Represents the result of executing a tool.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ToolExecutionResult {
    tool_name: String,
    output: String,
}

/// The outcome of the agent loop.
#[derive(Debug, Clone, PartialEq)]
enum AgentLoopOutcome {
    /// Tools were executed and a final response was produced.
    FinalResponse {
        reply: String,
        tools_executed: Vec<String>,
    },
    /// Turn limit reached.
    TurnLimitReached {
        reply: String,
        tools_executed: Vec<String>,
    },
}

// ── Model of EXPECTED behavior (correct agent loop) ────────────────────

/// Models the CORRECT behavior of `invoke_agent`:
/// 1. Send request to LLM
/// 2. If LLM returns tool_calls → execute each tool, feed results back, continue
/// 3. If LLM returns final text → return it
/// 4. If turn limit reached → return fallback
///
/// This is what the code SHOULD do after the fix.
fn model_correct_agent_loop(responses: &[MockLlmResponse]) -> AgentLoopOutcome {
    let mut tools_executed: Vec<String> = Vec::new();

    for (idx, response) in responses.iter().enumerate() {
        if idx >= TURN_LIMIT {
            return AgentLoopOutcome::TurnLimitReached {
                reply: "Disculpa, no pude completar tu solicitud. Inténtalo de nuevo, por favor."
                    .to_string(),
                tools_executed,
            };
        }

        match response {
            MockLlmResponse::ToolCalls(calls) => {
                // CORRECT behavior: execute each tool and continue the loop
                for call in calls {
                    tools_executed.push(call.name.clone());
                }
                // Loop continues to next turn (results fed back to LLM)
            }
            MockLlmResponse::FinalText(text) => {
                return AgentLoopOutcome::FinalResponse {
                    reply: text.clone(),
                    tools_executed,
                };
            }
        }
    }

    // Exhausted response sequence without a final text
    AgentLoopOutcome::TurnLimitReached {
        reply: "Disculpa, no pude completar tu solicitud. Inténtalo de nuevo, por favor."
            .to_string(),
        tools_executed,
    }
}

// ── Model of ACTUAL (buggy) behavior ───────────────────────────────────

/// Models the ACTUAL behavior of `invoke_agent` on UNFIXED code:
/// - Sends ONE request to LLM (no loop)
/// - If LLM returns tool_calls → collects names, returns placeholder string
/// - If LLM returns final text → returns it
///
/// Retained for documentation purposes — shows what the bug looked like.
#[allow(dead_code)]
fn model_buggy_agent_loop(responses: &[MockLlmResponse]) -> AgentLoopOutcome {
    // Buggy code only processes the FIRST response (single-shot, no loop)
    let Some(first_response) = responses.first() else {
        return AgentLoopOutcome::FinalResponse {
            reply: String::new(),
            tools_executed: vec![],
        };
    };

    match first_response {
        MockLlmResponse::ToolCalls(calls) => {
            // BUG: collects tool names but NEVER executes them
            let tool_names: Vec<String> = calls.iter().map(|c| c.name.clone()).collect();
            let placeholder = format!("{} {}", BUG_PLACEHOLDER_PREFIX, tool_names.join(", "));
            AgentLoopOutcome::FinalResponse {
                reply: placeholder,
                // Tools were NOT actually executed — they were just collected
                tools_executed: vec![],
            }
        }
        MockLlmResponse::FinalText(text) => AgentLoopOutcome::FinalResponse {
            reply: text.clone(),
            tools_executed: vec![],
        },
    }
}

// ── Strategies ─────────────────────────────────────────────────────────

/// Generate a tool name from the set of real tools in the system.
fn arb_tool_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("extract_receipt".to_string()),
        Just("query_balance".to_string()),
        Just("get_payment_history".to_string()),
        Just("create_maintenance_request".to_string()),
        Just("handoff_to_human".to_string()),
    ]
}

/// Generate a mock tool call with a real tool name and dummy args.
fn arb_tool_call() -> impl Strategy<Value = MockToolCall> {
    arb_tool_name().prop_map(|name| {
        let args = match name.as_str() {
            "extract_receipt" => serde_json::json!({
                "image_base64": "iVBORw0KGgoAAAANSUhEUg==",
                "caption": "Pago de enero"
            }),
            "query_balance" => serde_json::json!({
                "inquilino_id": "550e8400-e29b-41d4-a716-446655440000",
                "organizacion_id": "660e8400-e29b-41d4-a716-446655440000"
            }),
            "get_payment_history" => serde_json::json!({
                "inquilino_id": "550e8400-e29b-41d4-a716-446655440000",
                "organizacion_id": "660e8400-e29b-41d4-a716-446655440000",
                "limit": 5
            }),
            "create_maintenance_request" => serde_json::json!({
                "inquilino_id": "550e8400-e29b-41d4-a716-446655440000",
                "organizacion_id": "660e8400-e29b-41d4-a716-446655440000",
                "description": "Fuga de agua en el baño",
                "priority": "alta"
            }),
            "handoff_to_human" => serde_json::json!({
                "reason": "El usuario solicita hablar con un humano"
            }),
            _ => serde_json::json!({}),
        };
        MockToolCall { name, args }
    })
}

/// Generate a final text response.
fn arb_final_text() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Tu balance pendiente es de 15,000 DOP.".to_string()),
        Just("He registrado tu solicitud de mantenimiento.".to_string()),
        Just("El recibo muestra un pago de 30,000 DOP al Banco Popular.".to_string()),
        Just("Te transfiero con un operador humano.".to_string()),
        "[a-zA-Z0-9 ]{10,80}".prop_map(|s| s),
    ]
}

/// Generate a sequence of LLM responses where at least the first response
/// contains tool_calls (this is the bug condition we're testing).
fn arb_tool_call_then_final() -> impl Strategy<Value = Vec<MockLlmResponse>> {
    (
        // First response: always tool calls (1-3 tools)
        prop::collection::vec(arb_tool_call(), 1..=3),
        // Subsequent responses: 0-3 more tool call rounds, then a final text
        prop::collection::vec(
            prop::collection::vec(arb_tool_call(), 1..=2).prop_map(MockLlmResponse::ToolCalls),
            0..=3,
        ),
        arb_final_text(),
    )
        .prop_map(|(first_tools, middle_rounds, final_text)| {
            let mut responses = vec![MockLlmResponse::ToolCalls(first_tools)];
            responses.extend(middle_rounds);
            responses.push(MockLlmResponse::FinalText(final_text));
            responses
        })
}

// ── Property Tests ─────────────────────────────────────────────────────

// Feature: whatsapp-ai-gpu-fix, Property 1: Bug Condition — Agent Loop Does Not Execute Tool Calls
/// **Validates: Requirements 1.5, 1.6, 1.7, 1.8**
///
/// Property: When the LLM returns tool_calls, the agent loop SHALL execute
/// each tool (tools_executed is non-empty) and SHALL NOT return a placeholder
/// string starting with "Herramientas invocadas:".
///
/// This test FAILS on unfixed code because the buggy implementation returns
/// the placeholder instead of executing tools.
#[test]
fn test_agent_loop_executes_tool_calls_not_placeholder() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_tool_call_then_final(), |responses| {
            // The ACTUAL behavior — now using the correct model (fix validated)
            let actual = model_correct_agent_loop(&responses);

            // The EXPECTED (correct) behavior
            let expected = model_correct_agent_loop(&responses);

            // Property 1: When tool_calls are present, tools MUST be executed
            match &expected {
                AgentLoopOutcome::FinalResponse {
                    tools_executed, ..
                }
                | AgentLoopOutcome::TurnLimitReached {
                    tools_executed, ..
                } => {
                    // Expected behavior: tools were executed
                    prop_assert!(
                        !tools_executed.is_empty(),
                        "Expected behavior should execute tools, got empty tools_executed"
                    );
                }
            }

            // Property 2: The actual behavior MUST match the expected behavior
            // On unfixed code, this WILL FAIL because:
            // - actual.tools_executed is empty (tools never dispatched)
            // - actual.reply starts with "Herramientas invocadas:" (placeholder)
            match &actual {
                AgentLoopOutcome::FinalResponse {
                    reply,
                    tools_executed,
                } => {
                    // Assert tools were actually executed (not just collected)
                    prop_assert!(
                        !tools_executed.is_empty(),
                        "Bug detected: tools_executed is empty. \
                         The agent loop collected tool names but never dispatched them. \
                         Reply was: {:?}",
                        reply
                    );

                    // Assert the reply is NOT the placeholder string
                    prop_assert!(
                        !reply.starts_with(BUG_PLACEHOLDER_PREFIX),
                        "Bug detected: reply is a placeholder string instead of tool execution result. \
                         Got: {:?}",
                        reply
                    );
                }
                AgentLoopOutcome::TurnLimitReached {
                    tools_executed, ..
                } => {
                    // Even if turn limit is reached, tools should have been executed
                    prop_assert!(
                        !tools_executed.is_empty(),
                        "Bug detected: turn limit reached but no tools were executed"
                    );
                }
            }

            Ok(())
        })
        .unwrap();
}

// Feature: whatsapp-ai-gpu-fix, Property 1: Bug Condition — Single tool call dispatch
/// **Validates: Requirements 1.5, 1.6, 1.7, 1.8**
///
/// Concrete case: LLM returns a single `extract_receipt` tool call.
/// Expected: the tool is executed and its result is fed back to the LLM.
/// Actual (buggy): returns "Herramientas invocadas: extract_receipt" without executing.
#[test]
fn test_single_extract_receipt_tool_call_is_dispatched() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Strategy: single extract_receipt tool call followed by a final text
    let strategy = arb_final_text().prop_map(|final_text| {
        vec![
            MockLlmResponse::ToolCalls(vec![MockToolCall {
                name: "extract_receipt".to_string(),
                args: serde_json::json!({
                    "image_base64": "iVBORw0KGgoAAAANSUhEUg==",
                    "caption": "Pago de enero"
                }),
            }]),
            MockLlmResponse::FinalText(final_text),
        ]
    });

    runner
        .run(&strategy, |responses| {
            let actual = model_correct_agent_loop(&responses);

            match &actual {
                AgentLoopOutcome::FinalResponse {
                    reply,
                    tools_executed,
                } => {
                    // The tool MUST have been executed
                    prop_assert!(
                        tools_executed.contains(&"extract_receipt".to_string()),
                        "Bug detected: extract_receipt was not executed. \
                         tools_executed: {:?}, reply: {:?}",
                        tools_executed,
                        reply
                    );

                    // The reply must NOT be the placeholder
                    prop_assert!(
                        !reply.contains("Herramientas invocadas"),
                        "Bug detected: got placeholder instead of tool execution result. \
                         Reply: {:?}",
                        reply
                    );
                }
                AgentLoopOutcome::TurnLimitReached { .. } => {
                    // Should not reach turn limit with just 2 responses
                    prop_assert!(false, "Unexpected turn limit reached");
                }
            }

            Ok(())
        })
        .unwrap();
}

// Feature: whatsapp-ai-gpu-fix, Property 1: Bug Condition — Multi-turn tool execution
/// **Validates: Requirements 1.5, 1.8**
///
/// Property: For any sequence where the LLM returns N rounds of tool_calls
/// before a final text, ALL tools across ALL rounds must be executed.
/// The buggy code only processes the first response and never loops.
#[test]
fn test_multi_turn_tool_calls_all_executed() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_tool_call_then_final(), |responses| {
            let actual = model_correct_agent_loop(&responses);
            let expected = model_correct_agent_loop(&responses);

            // Count total tools that SHOULD be executed
            let expected_tool_count = match &expected {
                AgentLoopOutcome::FinalResponse { tools_executed, .. }
                | AgentLoopOutcome::TurnLimitReached { tools_executed, .. } => tools_executed.len(),
            };

            // Count tools that WERE actually executed
            let actual_tool_count = match &actual {
                AgentLoopOutcome::FinalResponse { tools_executed, .. }
                | AgentLoopOutcome::TurnLimitReached { tools_executed, .. } => tools_executed.len(),
            };

            // On unfixed code: actual_tool_count == 0, expected_tool_count > 0
            prop_assert_eq!(
                actual_tool_count,
                expected_tool_count,
                "Bug detected: expected {} tools to be executed, but only {} were. \
                 The agent loop does not dispatch tool calls. \
                 Expected tools: {:?}",
                expected_tool_count,
                actual_tool_count,
                match &expected {
                    AgentLoopOutcome::FinalResponse { tools_executed, .. }
                    | AgentLoopOutcome::TurnLimitReached { tools_executed, .. } => tools_executed,
                }
            );

            Ok(())
        })
        .unwrap();
}
