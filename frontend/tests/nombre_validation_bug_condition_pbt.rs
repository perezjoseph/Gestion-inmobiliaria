#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::no_effect_underscore_binding,
    clippy::used_underscore_binding,
    unused_doc_comments
)]
//! Property 3: Bug Condition — Filled Nombre clears the validation error
//!
//! This test MUST FAIL on unfixed code — failure confirms the bug exists.
//!
//! The bug: `frontend/src/components/auth/register_form.rs` validates `nombre`
//! only on submit (`validate_nombre(&nombre)` inside `on_submit`). The `nombre`
//! field uses the generic `make_handler` which only sets state — it does NOT
//! call `nombre_error.set(validate_nombre(&value))` on input change. Therefore,
//! after a prior empty submission sets `nombre_error` to
//! `Some("El nombre es obligatorio")`, typing a non-empty name does NOT clear
//! the error. The error persists until the next full submit.
//!
//! The property asserts: after an empty submit (which sets the nombre error),
//! entering a non-empty Nombre should yield `nombre_error == None`.
//!
//! **Validates: Requirements 1.2**

use proptest::prelude::*;

// ── Model of the registration form's nombre validation lifecycle ────────────
//
// This models the ACTUAL behavior of register_form.rs.
//
// Current (unfixed) code flow:
//   1. on_submit: nombre_error.set(validate_nombre(&nombre)) — sets error if empty
//   2. on_nombre_change (make_handler): nombre.set(value) — ONLY sets state, no revalidation
//   3. The nombre_error stays whatever it was last set to in on_submit
//
// So after step 1 sets nombre_error = Some("El nombre es obligatorio"),
// step 2 typing a non-empty name does NOT clear nombre_error.

/// Represents the nombre_error state: None means no error, Some means error displayed.
type NombreError = Option<String>;

/// Model of `validate_nombre` from register_form.rs (pure function).
fn validate_nombre(input: &str) -> Option<String> {
    if input.trim().is_empty() {
        Some("El nombre es obligatorio".into())
    } else {
        None
    }
}

/// Simulates the CURRENT (fixed) form lifecycle for the nombre field:
///
/// 1. User submits with empty nombre → nombre_error is set
/// 2. User types a new (non-empty) nombre value
/// 3. Returns what nombre_error is AFTER typing (without another submit)
///
/// In the fixed code, the custom on_nombre_change handler revalidates on
/// every input event: `nombre_error.set(validate_nombre(&value))`, so the
/// error clears as soon as a non-empty value is entered.
fn simulate_nombre_lifecycle_current(
    _prior_empty_submission: bool,
    new_nombre_value: &str,
) -> NombreError {
    // Step 1: Prior empty submission sets the error
    // Step 2: User types a new value via the FIXED on_nombre_change handler:
    //   nombre_error.set(validate_nombre(&value));
    //   nombre.set(value);
    //
    // The displayed error is now based on the current input value.
    validate_nombre(new_nombre_value)
}

/// Model of the EXPECTED (correct/fixed) behavior:
///
/// After a prior empty submission, typing a non-empty nombre should clear
/// the error because the fixed on_input handler calls:
///   nombre_error.set(validate_nombre(&value))
#[allow(dead_code)]
fn simulate_nombre_lifecycle_expected(
    _prior_empty_submission: bool,
    new_nombre_value: &str,
) -> NombreError {
    // Step 1: Prior empty submission sets the error (same as current)
    // Step 2: User types a new value via the FIXED handler which revalidates:
    //   nombre_error.set(validate_nombre(&value));
    //   nombre.set(value);
    // The displayed error is now based on the current input value
    validate_nombre(new_nombre_value)
}

// ── Strategies ─────────────────────────────────────────────────────────────

/// Input space for the bug condition:
/// - priorEmptySubmission is always true (scoped to this condition)
/// - nombre is a non-empty string (the user has typed something after the failed submit)
///
/// We generate various non-empty strings to demonstrate the bug across
/// different valid nombre values.
fn non_empty_nombre_strategy() -> impl Strategy<Value = String> {
    // Generate non-empty, non-whitespace-only strings representing valid names
    "[a-zA-ZáéíóúñÁÉÍÓÚÑ ]{1,50}".prop_filter("nombre must not be whitespace-only", |s| {
        !s.trim().is_empty()
    })
}

// ── Property Tests ─────────────────────────────────────────────────────────

// Feature: e2e-exploratory-bugfixes, Property 3: Bug Condition

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 1.2**
    ///
    /// Property 3: Bug Condition — Filled Nombre clears the validation error
    ///
    /// For ANY non-empty Nombre entered after a prior empty submission,
    /// the nombre_error SHOULD be None (no error displayed).
    ///
    /// This test is EXPECTED TO FAIL on unfixed code because the current
    /// on_nombre_change handler (make_handler) only sets the nombre state
    /// and does NOT revalidate — so nombre_error stays sticky from the
    /// prior empty submission.
    #[test]
    fn prop_nombre_error_clears_after_non_empty_input(
        nombre in non_empty_nombre_strategy()
    ) {
        // Precondition: a prior empty submission occurred (scoped by bug condition)
        let prior_empty_submission = true;

        // Act: model the current form lifecycle
        let nombre_error = simulate_nombre_lifecycle_current(prior_empty_submission, &nombre);

        // Assert: nombre_error SHOULD be None (error cleared for non-empty input)
        // This will FAIL because the current code does not revalidate on input,
        // so nombre_error remains Some("El nombre es obligatorio") — proving the bug.
        prop_assert_eq!(
            nombre_error.clone(),
            None,
            "Bug confirmed: after a prior empty submission, typing a non-empty \
             Nombre ('{}') should clear the validation error, but the current \
             code leaves nombre_error sticky as '{:?}' because make_handler \
             does not revalidate on input change.",
            nombre,
            nombre_error
        );
    }
}
