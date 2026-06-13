#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::no_effect_underscore_binding,
    clippy::used_underscore_binding,
    clippy::redundant_clone,
    clippy::implicit_clone,
    unused_doc_comments
)]
//! Property 4: Preservation — Empty-name error and valid submissions unchanged
//!
//! This test captures the CORRECT baseline behavior of `validate_nombre` that
//! must remain unchanged after the sticky-error fix.
//!
//! Observation on UNFIXED code:
//! - `validate_nombre("")` → Some("El nombre es obligatorio")
//! - `validate_nombre("   ")` → Some("El nombre es obligatorio")
//! - `validate_nombre("Test User")` → None (no error)
//!
//! The property asserts over Nombre values:
//! - empty/whitespace ⇒ error "El nombre es obligatorio"
//! - non-empty (after trim) ⇒ no error (None)
//!
//! **EXPECTED OUTCOME**: Tests PASS on unfixed code (baseline behavior captured).
//!
//! **Validates: Requirements 3.2, 3.3**

// Feature: e2e-exploratory-bugfixes, Property 4: Preservation

use proptest::prelude::*;
use realestate_frontend::components::auth::register_form::validate_nombre;

// ── Strategies ─────────────────────────────────────────────────────────────

/// Strategy for empty/whitespace-only strings that should trigger the error.
/// Covers: empty string, spaces, tabs, mixed whitespace.
fn empty_or_whitespace_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),                   // empty string
        " {0,20}".prop_map(|s| s.to_string()), // spaces only
        "[ \t\n\r]{1,10}",                     // mixed whitespace
    ]
}

/// Strategy for non-empty strings (after trim) that should NOT trigger the error.
/// Covers: plain names, names with leading/trailing spaces, unicode, etc.
fn non_empty_nombre_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple alphabetic names
        "[a-zA-Z]{1,30}",
        // Names with spaces (but not whitespace-only)
        "[a-zA-Z]{1,15} [a-zA-Z]{1,15}",
        // Names with accented characters (Dominican names)
        "[a-zA-ZáéíóúñÁÉÍÓÚÑ]{1,20}",
        // Names with leading/trailing whitespace (but non-empty after trim)
        " {0,5}[a-zA-Z]{1,15} {0,5}",
    ]
    .prop_filter("must not be whitespace-only after trim", |s| {
        !s.trim().is_empty()
    })
}

// ── Property Tests ─────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 3.3**
    ///
    /// Property 4a: Preservation — Empty/whitespace Nombre still produces the error.
    ///
    /// For ANY empty or whitespace-only Nombre value, `validate_nombre` SHALL
    /// return Some("El nombre es obligatorio").
    ///
    /// EXPECTED: PASSES on unfixed code.
    #[test]
    fn prop_empty_or_whitespace_nombre_produces_error(
        nombre in empty_or_whitespace_strategy()
    ) {
        let result = validate_nombre(&nombre);

        prop_assert_eq!(
            result,
            Some("El nombre es obligatorio".into()),
            "Preservation violated: validate_nombre({:?}) should return the error \
             message for empty/whitespace input, but got None",
            nombre
        );
    }

    /// **Validates: Requirements 3.2**
    ///
    /// Property 4b: Preservation — Non-empty Nombre produces no error.
    ///
    /// For ANY non-empty (after trim) Nombre value, `validate_nombre` SHALL
    /// return None (no error), allowing valid form submissions to proceed.
    ///
    /// EXPECTED: PASSES on unfixed code.
    #[test]
    fn prop_non_empty_nombre_produces_no_error(
        nombre in non_empty_nombre_strategy()
    ) {
        let result = validate_nombre(&nombre);

        prop_assert_eq!(
            result.clone(),
            None,
            "Preservation violated: validate_nombre({:?}) should return None \
             for non-empty input, but got {:?}",
            nombre,
            result
        );
    }
}
