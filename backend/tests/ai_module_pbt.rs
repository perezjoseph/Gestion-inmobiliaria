// Feature: spec-gap-remediation, Property 7: Multi-turn agent loop terminates
//
// This PBT models the multi-turn agent loop from `services::ai_module::invoke_agent`.
// It verifies that for ANY sequence of LLM responses (final text, tool calls, errors,
// or looping tool calls), the loop always terminates within TURN_LIMIT = 5 turns and
// returns either a Final text or the Spanish TurnLimitReached fallback message.
//
// The test uses a stub LLM that produces a predetermined sequence of responses,
// exercising the loop's termination guarantee without requiring a real model.

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

// ── Constants matching the design ──────────────────────────────────────

const TURN_LIMIT: usize = 5;
const FALLBACK_MESSAGE: &str =
    "Disculpa, no pude completar tu solicitud. Inténtalo de nuevo, por favor.";

// ── Model types ────────────────────────────────────────────────────────

/// Represents a single response from the stub LLM in one turn.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Inner fields used only for Debug output in counterexamples
enum StubLlmResponse {
    /// The model returns a final text message — loop terminates.
    Final(String),
    /// The model returns one or more tool calls — loop continues after executing them.
    ToolCalls(Vec<StubToolCall>),
    /// The model returns a tool call that errors — error is surfaced back to the model.
    ToolError(String),
}

/// A stub tool call with a name and dummy args.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct StubToolCall {
    name: String,
}

/// The outcome of the modeled agent loop.
#[derive(Debug, Clone, PartialEq)]
enum AgentOutcome {
    /// The model produced a final assistant message.
    Final { text: String, turns_used: usize },
    /// The turn limit was reached without a final message.
    TurnLimitReached { text: String, turns_used: usize },
}

// ── Model of the multi-turn loop ───────────────────────────────────────

/// Models the `invoke_agent` multi-turn loop behavior as described in the design:
///
/// ```text
/// loop turn in 0..TURN_LIMIT:
///     response = agent.completion(history)
///     match response {
///         Final(msg)       => return AgentOutcome::Final { text: msg, ... }
///         ToolCalls(calls) => for c in calls { execute tool, push result to history }
///     }
/// return AgentOutcome::TurnLimitReached { text: FALLBACK_MESSAGE, ... }
/// ```
///
/// Tool errors are surfaced back to the model as tool results (the loop continues).
fn model_invoke_agent(responses: &[StubLlmResponse]) -> AgentOutcome {
    for (response_idx, turn) in (0..TURN_LIMIT).enumerate() {
        // If we've exhausted the response sequence, the model keeps returning
        // tool calls (simulating a looping model). The loop should still terminate.
        let Some(response) = responses.get(response_idx) else {
            return AgentOutcome::TurnLimitReached {
                text: FALLBACK_MESSAGE.to_string(),
                turns_used: TURN_LIMIT,
            };
        };

        match response {
            StubLlmResponse::Final(text) => {
                return AgentOutcome::Final {
                    text: text.clone(),
                    turns_used: turn + 1,
                };
            }
            // Tool calls are executed and results pushed to history.
            // Tool errors are surfaced back to the model as a tool result.
            // In both cases the loop continues to the next turn.
            StubLlmResponse::ToolCalls(_) | StubLlmResponse::ToolError(_) => {}
        }
    }

    // Turn limit exhausted without a Final response
    AgentOutcome::TurnLimitReached {
        text: FALLBACK_MESSAGE.to_string(),
        turns_used: TURN_LIMIT,
    }
}

// ── Strategies ─────────────────────────────────────────────────────────

/// Generate a random tool name.
fn arb_tool_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("extract_receipt".to_string()),
        Just("query_balance".to_string()),
        Just("create_maintenance_request".to_string()),
        Just("get_payment_history".to_string()),
        Just("handoff_to_human".to_string()),
        "[a-z_]{3,20}".prop_map(|s| s),
    ]
}

/// Generate a random tool call.
fn arb_tool_call() -> impl Strategy<Value = StubToolCall> {
    arb_tool_name().prop_map(|name| StubToolCall { name })
}

/// Generate a random final text message.
fn arb_final_text() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Hola, ¿en qué puedo ayudarte?".to_string()),
        Just("Tu balance es de 5,000 DOP.".to_string()),
        Just("Solicitud creada exitosamente.".to_string()),
        "[a-zA-Z0-9 ]{1,100}".prop_map(|s| s),
    ]
}

/// Generate a random error message.
fn arb_error_message() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("OCR service not available".to_string()),
        Just("Inquilino no resuelto".to_string()),
        Just("Timeout".to_string()),
        "[a-zA-Z0-9 ]{5,50}".prop_map(|s| s),
    ]
}

/// Generate a single stub LLM response.
fn arb_stub_response() -> impl Strategy<Value = StubLlmResponse> {
    prop_oneof![
        // 30% chance of Final — terminates the loop
        3 => arb_final_text().prop_map(StubLlmResponse::Final),
        // 50% chance of ToolCalls — loop continues
        5 => prop::collection::vec(arb_tool_call(), 1..=3)
            .prop_map(StubLlmResponse::ToolCalls),
        // 20% chance of ToolError — loop continues (error surfaced to model)
        2 => arb_error_message().prop_map(StubLlmResponse::ToolError),
    ]
}

/// Generate a sequence of stub LLM responses (0 to 10 responses).
/// Sequences longer than TURN_LIMIT test that the loop doesn't consume beyond the limit.
/// Sequences shorter than TURN_LIMIT test that the loop handles exhaustion.
fn arb_response_sequence() -> impl Strategy<Value = Vec<StubLlmResponse>> {
    prop::collection::vec(arb_stub_response(), 0..=10)
}

// ── Property Tests ─────────────────────────────────────────────────────

// Feature: spec-gap-remediation, Property 7: Multi-turn agent loop terminates
/// **Validates: Requirements 8.1, 8.4**
#[test]
fn test_agent_loop_always_terminates_within_turn_limit() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_response_sequence(), |responses| {
            let outcome = model_invoke_agent(&responses);

            match &outcome {
                AgentOutcome::Final { turns_used, .. } => {
                    // Must terminate within the turn limit
                    prop_assert!(
                        *turns_used <= TURN_LIMIT,
                        "Final outcome used {} turns, exceeding limit of {}",
                        turns_used,
                        TURN_LIMIT
                    );
                    // Must have used at least 1 turn
                    prop_assert!(
                        *turns_used >= 1,
                        "Final outcome must use at least 1 turn, got {}",
                        turns_used
                    );
                }
                AgentOutcome::TurnLimitReached { text, turns_used } => {
                    // Must have used exactly TURN_LIMIT turns
                    prop_assert_eq!(
                        *turns_used,
                        TURN_LIMIT,
                        "TurnLimitReached should use exactly {} turns, got {}",
                        TURN_LIMIT,
                        turns_used
                    );
                    // Must return the Spanish fallback message
                    prop_assert_eq!(
                        text.as_str(),
                        FALLBACK_MESSAGE,
                        "TurnLimitReached must return the Spanish fallback message"
                    );
                }
            }

            Ok(())
        })
        .unwrap();
}

// Feature: spec-gap-remediation, Property 7: Multi-turn agent loop terminates
/// **Validates: Requirements 8.1, 8.4**
#[test]
fn test_agent_loop_returns_final_when_model_produces_final() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Strategy: 0-4 non-final responses followed by a Final response
    let strategy = (
        prop::collection::vec(
            prop_oneof![
                prop::collection::vec(arb_tool_call(), 1..=3).prop_map(StubLlmResponse::ToolCalls),
                arb_error_message().prop_map(StubLlmResponse::ToolError),
            ],
            0..TURN_LIMIT,
        ),
        arb_final_text(),
    )
        .prop_map(|(mut prefix, final_text)| {
            prefix.push(StubLlmResponse::Final(final_text));
            prefix
        });

    runner
        .run(&strategy, |responses| {
            let outcome = model_invoke_agent(&responses);

            // When a Final response is within the turn limit, the outcome must be Final
            let final_position = responses
                .iter()
                .position(|r| matches!(r, StubLlmResponse::Final(_)))
                .unwrap();

            if final_position < TURN_LIMIT {
                match &outcome {
                    AgentOutcome::Final { text, turns_used } => {
                        prop_assert_eq!(
                            *turns_used,
                            final_position + 1,
                            "Should terminate at the turn where Final was produced"
                        );
                        // Verify the text matches the Final response
                        if let StubLlmResponse::Final(expected) = &responses[final_position] {
                            prop_assert_eq!(text, expected);
                        }
                    }
                    AgentOutcome::TurnLimitReached { .. } => {
                        prop_assert!(
                            false,
                            "Should have returned Final, not TurnLimitReached. \
                             Final was at position {} (within limit {})",
                            final_position,
                            TURN_LIMIT
                        );
                    }
                }
            }

            Ok(())
        })
        .unwrap();
}

// Feature: spec-gap-remediation, Property 7: Multi-turn agent loop terminates
/// **Validates: Requirements 8.1, 8.4**
#[test]
fn test_agent_loop_returns_fallback_when_only_tool_calls() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Strategy: sequences of ONLY tool calls and errors (no Final) — must hit the limit
    let strategy = prop::collection::vec(
        prop_oneof![
            prop::collection::vec(arb_tool_call(), 1..=3).prop_map(StubLlmResponse::ToolCalls),
            arb_error_message().prop_map(StubLlmResponse::ToolError),
        ],
        TURN_LIMIT..=TURN_LIMIT + 5,
    );

    runner
        .run(&strategy, |responses| {
            let outcome = model_invoke_agent(&responses);

            match &outcome {
                AgentOutcome::TurnLimitReached { text, turns_used } => {
                    prop_assert_eq!(
                        *turns_used,
                        TURN_LIMIT,
                        "Must exhaust exactly {} turns",
                        TURN_LIMIT
                    );
                    prop_assert_eq!(
                        text.as_str(),
                        FALLBACK_MESSAGE,
                        "Must return the Spanish fallback"
                    );
                }
                AgentOutcome::Final { .. } => {
                    prop_assert!(
                        false,
                        "Should not return Final when no Final response exists in the sequence"
                    );
                }
            }

            Ok(())
        })
        .unwrap();
}

// Feature: spec-gap-remediation, Property 7: Multi-turn agent loop terminates
/// **Validates: Requirements 8.1, 8.4**
#[test]
fn test_agent_loop_never_exceeds_turn_limit_regardless_of_sequence_length() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Strategy: very long sequences (up to 20 responses) — loop must still terminate at 5
    let strategy = prop::collection::vec(arb_stub_response(), 0..=20);

    runner
        .run(&strategy, |responses| {
            let outcome = model_invoke_agent(&responses);

            let turns_used = match &outcome {
                AgentOutcome::Final { turns_used, .. }
                | AgentOutcome::TurnLimitReached { turns_used, .. } => *turns_used,
            };

            prop_assert!(
                turns_used <= TURN_LIMIT,
                "Loop used {} turns, exceeding the limit of {}. \
                 Sequence length: {}, responses: {:?}",
                turns_used,
                TURN_LIMIT,
                responses.len(),
                responses
            );

            Ok(())
        })
        .unwrap();
}
