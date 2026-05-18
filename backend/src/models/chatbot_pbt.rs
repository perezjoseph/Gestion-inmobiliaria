//! Property-based tests for `AgentConfig::resolve()`.
//!
//! Feature: native-rig-agent-guardrails, Property 7: AgentConfig Resolution Validity
//! **Validates: Requirements 6.2, 6.3, 6.4, 6.5, 6.6, 6.7**

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

use super::chatbot::{AgentConfig, GuardrailOverrides, ToolRegistrationStrategy};

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate an arbitrary `ToolRegistrationStrategy`.
fn arb_tool_registration_strategy() -> impl Strategy<Value = ToolRegistrationStrategy> {
    prop_oneof![
        Just(ToolRegistrationStrategy::Selective),
        Just(ToolRegistrationStrategy::AllWithHookGating),
    ]
}

/// Generate arbitrary `GuardrailOverrides`.
fn arb_guardrail_overrides() -> impl Strategy<Value = GuardrailOverrides> {
    (
        prop::option::of(any::<f64>()),
        prop::option::of(any::<f64>()),
        prop::option::of(prop::collection::vec(".*", 0..=5)),
        prop::option::of(any::<bool>()),
    )
        .prop_map(
            |(max_dop, max_usd, blocked_patterns, output_safety)| GuardrailOverrides {
                max_receipt_amount_dop: max_dop,
                max_receipt_amount_usd: max_usd,
                blocked_patterns,
                output_safety_enabled: output_safety,
            },
        )
}

/// Generate an arbitrary `AgentConfig` with full range of possible values.
fn arb_agent_config() -> impl Strategy<Value = AgentConfig> {
    (
        prop::option::of(any::<u8>()),
        prop::option::of(any::<f64>()),
        prop::option::of(any::<u64>()),
        prop::option::of(arb_tool_registration_strategy()),
        prop::option::of(arb_guardrail_overrides()),
    )
        .prop_map(
            |(max_turns, temperature, max_tokens, tool_registration, guardrails)| AgentConfig {
                max_turns,
                temperature,
                max_tokens,
                tool_registration,
                guardrails,
            },
        )
}

// ── Property Tests ─────────────────────────────────────────────────────

// Feature: native-rig-agent-guardrails, Property 7: AgentConfig Resolution Validity
// **Validates: Requirements 6.2, 6.3, 6.4, 6.5, 6.6, 6.7**

/// Property 7.1: resolved.max_turns is always in range 1–15.
/// **Validates: Requirements 6.2, 6.3**
#[test]
fn agent_config_resolve_max_turns_always_in_range() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_agent_config(), |config| {
            let resolved = config.resolve();
            prop_assert!(
                (1..=15).contains(&resolved.max_turns),
                "max_turns {} is outside range 1–15 for input {:?}",
                resolved.max_turns,
                config.max_turns
            );
            Ok(())
        })
        .unwrap();
}

/// Property 7.2: If config.max_turns is None, resolved is 5.
/// **Validates: Requirements 6.3**
#[test]
fn agent_config_resolve_max_turns_defaults_to_5() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    // Generate configs where max_turns is always None
    let strategy = arb_agent_config().prop_map(|mut config| {
        config.max_turns = None;
        config
    });

    runner
        .run(&strategy, |config| {
            let resolved = config.resolve();
            prop_assert_eq!(
                resolved.max_turns,
                5,
                "max_turns should default to 5 when None"
            );
            Ok(())
        })
        .unwrap();
}

/// Property 7.3: resolved.temperature is always None or within 0.0–2.0.
/// **Validates: Requirements 6.4, 6.5**
#[test]
fn agent_config_resolve_temperature_always_valid_or_none() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_agent_config(), |config| {
            let resolved = config.resolve();
            match resolved.temperature {
                Some(t) => prop_assert!(
                    (0.0..=2.0).contains(&t),
                    "temperature {} is outside range 0.0–2.0",
                    t
                ),
                None => {} // None is always valid
            }
            Ok(())
        })
        .unwrap();
}

/// Property 7.4: If config.temperature is Some but outside 0.0–2.0, resolved is None.
/// **Validates: Requirements 6.5**
#[test]
fn agent_config_resolve_temperature_outside_range_becomes_none() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    // Generate configs with temperature outside valid range
    let strategy = arb_agent_config().prop_flat_map(|config| {
        // Generate f64 values that are outside 0.0..=2.0
        prop_oneof![
            // Negative values
            (-1000.0f64..0.0f64).prop_map(|t| t - f64::EPSILON),
            // Values above 2.0
            (2.0f64..1000.0f64).prop_map(|t| t + f64::EPSILON),
        ]
        .prop_map(move |temp| {
            let mut c = config.clone();
            c.temperature = Some(temp);
            c
        })
    });

    runner
        .run(&strategy, |config| {
            let resolved = config.resolve();
            prop_assert_eq!(
                resolved.temperature,
                None,
                "temperature {:?} outside 0.0–2.0 should resolve to None",
                config.temperature
            );
            Ok(())
        })
        .unwrap();
}

/// Property 7.5: resolved.max_tokens is always None or within 1–4096.
/// **Validates: Requirements 6.6, 6.7**
#[test]
fn agent_config_resolve_max_tokens_always_valid_or_none() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arb_agent_config(), |config| {
            let resolved = config.resolve();
            match resolved.max_tokens {
                Some(t) => prop_assert!(
                    (1..=4096).contains(&t),
                    "max_tokens {} is outside range 1–4096",
                    t
                ),
                None => {} // None is always valid
            }
            Ok(())
        })
        .unwrap();
}

/// Property 7.6: If config.max_tokens is Some but outside 1–4096, resolved is None.
/// **Validates: Requirements 6.7**
#[test]
fn agent_config_resolve_max_tokens_outside_range_becomes_none() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    // Generate configs with max_tokens outside valid range
    let strategy = arb_agent_config().prop_flat_map(|config| {
        prop_oneof![
            // Zero (below minimum)
            Just(0u64),
            // Above maximum
            (4097u64..=u64::MAX),
        ]
        .prop_map(move |tokens| {
            let mut c = config.clone();
            c.max_tokens = Some(tokens);
            c
        })
    });

    runner
        .run(&strategy, |config| {
            let resolved = config.resolve();
            prop_assert_eq!(
                resolved.max_tokens,
                None,
                "max_tokens {:?} outside 1–4096 should resolve to None",
                config.max_tokens
            );
            Ok(())
        })
        .unwrap();
}

/// Property 7.7: resolved.tool_registration defaults to Selective when None.
/// **Validates: Requirements 6.2**
#[test]
fn agent_config_resolve_tool_registration_defaults_to_selective() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    // Generate configs where tool_registration is always None
    let strategy = arb_agent_config().prop_map(|mut config| {
        config.tool_registration = None;
        config
    });

    runner
        .run(&strategy, |config| {
            let resolved = config.resolve();
            prop_assert_eq!(
                resolved.tool_registration,
                ToolRegistrationStrategy::Selective,
                "tool_registration should default to Selective when None"
            );
            Ok(())
        })
        .unwrap();
}
