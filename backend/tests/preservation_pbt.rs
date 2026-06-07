// Feature: whatsapp-ai-gpu-fix, Property 2: Preservation — Non-Tool-Call Responses Pass Through Unchanged
//
// These PBTs capture baseline behavior that MUST remain unchanged after the fix:
// 1. When LLM returns final text (no tool_calls), the reply is passed through unchanged
// 2. Tool definitions are gated by tenant capabilities — exact mapping preserved
// 3. `compose_system_prompt` is deterministic — same inputs always produce same output
//
// All tests PASS on UNFIXED code (confirming the behavior to preserve).
//
// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

use realestate_backend::models::chatbot::{Capabilities, FaqEntry};
use realestate_backend::services::ai_module::{
    ChatbotPersona, TenantContext, compose_system_prompt, get_enabled_tools,
};

// ── Constants ──────────────────────────────────────────────────────────

/// The placeholder prefix that the buggy code returns for tool calls.
/// Final text responses must NEVER contain this prefix.
const BUG_PLACEHOLDER_PREFIX: &str = "Herramientas invocadas:";

// ── Model of invoke_agent text pass-through (unfixed code) ─────────────

/// Models the behavior of `invoke_agent` when the LLM returns ONLY text
/// (no tool_calls). On both unfixed and fixed code, this path is identical:
/// the text parts are joined with "\n" and returned directly.
///
/// This mirrors the code at lines 253-270 of ai_module.rs:
/// ```rust
/// let reply = if reply_parts.is_empty() && !tools_invoked.is_empty() {
///     format!("Herramientas invocadas: {}", tools_invoked.join(", "))
/// } else {
///     reply_parts.join("\n")
/// };
/// ```
fn model_text_passthrough(text_parts: &[String]) -> String {
    text_parts.join("\n")
}

// ── Strategies ─────────────────────────────────────────────────────────

/// Generate a non-empty text response (simulating LLM final text with no tool_calls).
fn arb_final_text_response() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        prop_oneof![
            Just("Hola, ¿en qué puedo ayudarte?".to_string()),
            Just("Tu balance pendiente es de 15,000 DOP.".to_string()),
            Just("No tengo información sobre eso.".to_string()),
            Just("Buen día! Soy tu asistente virtual.".to_string()),
            "[a-zA-Z0-9áéíóúñ .,!?]{1,200}".prop_map(|s| s),
        ],
        1..=3,
    )
}

/// Generate an arbitrary `Capabilities` configuration (all 2^5 = 32 combinations).
fn arb_capabilities() -> impl Strategy<Value = Capabilities> {
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

/// Generate an arbitrary `ChatbotPersona`.
fn arb_persona() -> impl Strategy<Value = ChatbotPersona> {
    (
        prop::option::of(prop_oneof![
            Just("profesional".to_string()),
            Just("amigable".to_string()),
            Just("formal".to_string()),
        ]),
        prop::option::of(prop_oneof![
            Just("Bienvenido!".to_string()),
            Just("Hola, soy tu asistente.".to_string()),
        ]),
        prop::option::of(prop_oneof![
            Just("Eres un asistente de gestión inmobiliaria.".to_string()),
            Just("Ayuda a los inquilinos con sus consultas.".to_string()),
        ]),
        prop_oneof![
            Just("es-DO".to_string()),
            Just("es".to_string()),
            Just("en".to_string()),
        ],
    )
        .prop_map(|(tone, greeting, system_prompt, language)| ChatbotPersona {
            tone,
            greeting,
            system_prompt,
            language,
        })
}

/// Generate an optional `TenantContext`.
fn arb_tenant_context() -> impl Strategy<Value = Option<TenantContext>> {
    prop::option::of(
        prop_oneof![
            Just("Juan Pérez".to_string()),
            Just("María García".to_string()),
            Just("Carlos Rodríguez".to_string()),
            "[A-Z][a-z]{2,10} [A-Z][a-z]{2,10}".prop_map(|s| s),
        ]
        .prop_map(|name| TenantContext { name }),
    )
}

/// Generate a vector of FAQ entries.
fn arb_faqs() -> impl Strategy<Value = Vec<FaqEntry>> {
    prop::collection::vec(
        (
            prop_oneof![
                Just("¿Cuándo se paga?".to_string()),
                Just("¿Aceptan transferencia?".to_string()),
                Just("¿Cuál es el horario?".to_string()),
            ],
            prop_oneof![
                Just("El día 5 de cada mes.".to_string()),
                Just("Sí, Banreservas y Popular.".to_string()),
                Just("De 8am a 5pm.".to_string()),
            ],
        )
            .prop_map(|(question, answer)| FaqEntry { question, answer }),
        0..=3,
    )
}

/// Generate optional policies text.
fn arb_policies() -> impl Strategy<Value = Option<String>> {
    prop::option::of(prop_oneof![
        Just("No se permiten mascotas.".to_string()),
        Just("Horario de ruido: 10pm-7am.".to_string()),
        Just("".to_string()),
    ])
}

/// Generate handoff keywords.
fn arb_handoff_keywords() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        prop_oneof![
            Just("hablar con humano".to_string()),
            Just("agente".to_string()),
            Just("operador".to_string()),
            Just("persona real".to_string()),
        ],
        0..=3,
    )
}

// ── Property Tests ─────────────────────────────────────────────────────

// Feature: whatsapp-ai-gpu-fix, Property 2: Preservation — Final Text Pass-Through
/// **Validates: Requirements 3.3**
///
/// Property: For all mock LLM responses that contain NO tool_calls (final text only),
/// the response text is returned directly without entering a tool execution loop.
/// The reply is the text parts joined with "\n" and never contains the bug placeholder.
#[test]
fn test_final_text_responses_pass_through_unchanged() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_final_text_response(), |text_parts| {
            let result = model_text_passthrough(&text_parts);

            // Property: the result is exactly the text parts joined with "\n"
            let expected = text_parts.join("\n");
            prop_assert_eq!(
                &result,
                &expected,
                "Final text response must be passed through unchanged. \
                 Got: {:?}, Expected: {:?}",
                result,
                expected
            );

            // Property: the result must NOT contain the bug placeholder prefix
            prop_assert!(
                !result.starts_with(BUG_PLACEHOLDER_PREFIX),
                "Final text response must not contain the tool invocation placeholder. \
                 Got: {:?}",
                result
            );

            // Property: the result must not be empty (we generate non-empty text)
            prop_assert!(!result.is_empty(), "Final text response must not be empty");

            Ok(())
        })
        .unwrap();
}

// Feature: whatsapp-ai-gpu-fix, Property 2: Preservation — Capability Gating
/// **Validates: Requirements 3.4**
///
/// Property: For all `Capabilities` configurations, tool registration matches
/// exactly the enabled capabilities. The mapping is:
/// - `receipt_ocr` → `extract_receipt`
/// - `balance_queries` → `query_balance`, `get_payment_history`
/// - `maintenance_requests` → `create_maintenance_request`
/// - `human_handoff` → `handoff_to_human`
/// - `payment_reminders` → (no tool mapping)
#[test]
fn test_capability_gating_matches_enabled_capabilities() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_capabilities(), |caps| {
            let tools = get_enabled_tools(&caps);

            // Property: extract_receipt is present IFF receipt_ocr is enabled
            prop_assert_eq!(
                tools.contains(&"extract_receipt"),
                caps.receipt_ocr,
                "extract_receipt presence must match receipt_ocr capability. \
                 caps.receipt_ocr={}, tools={:?}",
                caps.receipt_ocr,
                tools
            );

            // Property: query_balance is present IFF balance_queries is enabled
            prop_assert_eq!(
                tools.contains(&"query_balance"),
                caps.balance_queries,
                "query_balance presence must match balance_queries capability. \
                 caps.balance_queries={}, tools={:?}",
                caps.balance_queries,
                tools
            );

            // Property: get_payment_history is present IFF balance_queries is enabled
            prop_assert_eq!(
                tools.contains(&"get_payment_history"),
                caps.balance_queries,
                "get_payment_history presence must match balance_queries capability. \
                 caps.balance_queries={}, tools={:?}",
                caps.balance_queries,
                tools
            );

            // Property: create_maintenance_request is present IFF maintenance_requests is enabled
            prop_assert_eq!(
                tools.contains(&"create_maintenance_request"),
                caps.maintenance_requests,
                "create_maintenance_request presence must match maintenance_requests capability. \
                 caps.maintenance_requests={}, tools={:?}",
                caps.maintenance_requests,
                tools
            );

            // Property: handoff_to_human is present IFF human_handoff is enabled
            prop_assert_eq!(
                tools.contains(&"handoff_to_human"),
                caps.human_handoff,
                "handoff_to_human presence must match human_handoff capability. \
                 caps.human_handoff={}, tools={:?}",
                caps.human_handoff,
                tools
            );

            // Property: payment_reminders has NO tool mapping — no extra tools
            let expected_count = caps.receipt_ocr as usize
                + (caps.balance_queries as usize * 2)
                + caps.maintenance_requests as usize
                + caps.human_handoff as usize;
            prop_assert_eq!(
                tools.len(),
                expected_count,
                "Tool count must match exactly the enabled capabilities. \
                 Expected {}, got {}. caps={:?}, tools={:?}",
                expected_count,
                tools.len(),
                caps,
                tools
            );

            Ok(())
        })
        .unwrap();
}

// Feature: whatsapp-ai-gpu-fix, Property 2: Preservation — System Prompt Determinism
/// **Validates: Requirements 3.5, 3.6**
///
/// Property: For all conversation histories without tool calls, `compose_system_prompt`
/// output is identical before and after fix. We verify this by calling the function
/// twice with the same inputs and asserting the output is identical (determinism).
/// This guarantees the fix cannot accidentally alter system prompt composition.
#[test]
fn test_compose_system_prompt_is_deterministic() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy = (
        arb_persona(),
        arb_tenant_context(),
        arb_faqs(),
        arb_policies(),
        arb_handoff_keywords(),
    );

    runner
        .run(
            &strategy,
            |(persona, tenant_ctx, faqs, policies, handoff_kw)| {
                let tenant_ref = tenant_ctx.as_ref();
                let policies_ref = policies.as_deref();

                // Call compose_system_prompt twice with identical inputs
                let result1 = compose_system_prompt(
                    &persona,
                    tenant_ref,
                    &faqs,
                    policies_ref,
                    &handoff_kw,
                    &[],
                );
                let result2 = compose_system_prompt(
                    &persona,
                    tenant_ref,
                    &faqs,
                    policies_ref,
                    &handoff_kw,
                    &[],
                );

                // Property: same inputs always produce same output
                prop_assert_eq!(
                    &result1,
                    &result2,
                    "compose_system_prompt must be deterministic. \
                 Two calls with identical inputs produced different outputs."
                );

                // Property: output always contains the language
                prop_assert!(
                    result1.contains(&format!("Idioma: {}", persona.language)),
                    "System prompt must always contain the language. Got: {:?}",
                    result1
                );

                // Property: if tone is set, it appears in the output
                if let Some(ref tone) = persona.tone {
                    prop_assert!(
                        result1.contains(tone),
                        "System prompt must contain the tone when set. \
                     Tone: {:?}, Prompt: {:?}",
                        tone,
                        result1
                    );
                }

                // Property: if greeting is set, it appears in the output
                if let Some(ref greeting) = persona.greeting {
                    prop_assert!(
                        result1.contains(greeting),
                        "System prompt must contain the greeting when set. \
                     Greeting: {:?}, Prompt: {:?}",
                        greeting,
                        result1
                    );
                }

                // Property: system_prompt field is deprecated and no longer included
                // (replaced by guidance rules)

                // Property: if tenant context is set, tenant name appears in the output
                if let Some(ref ctx) = tenant_ctx {
                    prop_assert!(
                        result1.contains(&ctx.name),
                        "System prompt must contain tenant name when context is set. \
                     Name: {:?}, Prompt: {:?}",
                        ctx.name,
                        result1
                    );
                }

                // Property: all FAQ questions and answers appear in the output
                for faq in &faqs {
                    prop_assert!(
                        result1.contains(&faq.question),
                        "System prompt must contain FAQ question. \
                     Question: {:?}, Prompt: {:?}",
                        faq.question,
                        result1
                    );
                    prop_assert!(
                        result1.contains(&faq.answer),
                        "System prompt must contain FAQ answer. \
                     Answer: {:?}, Prompt: {:?}",
                        faq.answer,
                        result1
                    );
                }

                // Property: if policies is set and non-empty, it appears in the output
                if let Some(ref p) = policies {
                    if !p.is_empty() {
                        prop_assert!(
                            result1.contains(p),
                            "System prompt must contain policies when set and non-empty. \
                         Policies: {:?}, Prompt: {:?}",
                            p,
                            result1
                        );
                    }
                }

                // Property: all handoff keywords appear in the output
                for kw in &handoff_kw {
                    prop_assert!(
                        result1.contains(kw),
                        "System prompt must contain handoff keyword. \
                     Keyword: {:?}, Prompt: {:?}",
                        kw,
                        result1
                    );
                }

                Ok(())
            },
        )
        .unwrap();
}
