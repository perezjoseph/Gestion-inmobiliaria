#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    unused_doc_comments
)]
//! Property 3: CTA styling consistency
//!
//! Every action button on the landing page uses one of two styles:
//! - Primary CTAs ("Registrarse gratis", "Crear cuenta gratis") use the filled
//!   `gi-l-btn-primary` class.
//! - Every other CTA uses the outlined `gi-l-btn-secondary` class and SHALL NOT
//!   use the primary class.

use proptest::prelude::*;

// Feature: landing-page, Property 3: CTA styling consistency

// ─────────────────────────────────────────────────────────────────────────────
// Source inclusion for structural analysis
// ─────────────────────────────────────────────────────────────────────────────

const HERO_SOURCE: &str = include_str!("../src/components/landing/hero.rs");
const PREVIEW_SOURCE: &str = include_str!("../src/components/landing/preview.rs");
const TRANSPARENCY_SOURCE: &str = include_str!("../src/components/landing/transparency.rs");

// ─────────────────────────────────────────────────────────────────────────────
// CTA definitions
// ─────────────────────────────────────────────────────────────────────────────

/// All CTA labels on the landing page.
const ALL_CTA_LABELS: [&str; 4] = [
    "Registrarse gratis",
    "Crear cuenta gratis",
    "Ya tengo cuenta",
    "Ver código en GitHub",
];

/// The primary CTA labels — the only ones allowed to use the filled primary class.
const PRIMARY_CTA_LABELS: [&str; 2] = ["Registrarse gratis", "Crear cuenta gratis"];

/// Whether a label is a primary CTA.
fn is_primary_cta(label: &str) -> bool {
    PRIMARY_CTA_LABELS.contains(&label)
}

/// Returns the source file content containing the given CTA label.
fn source_for_cta(label: &str) -> &'static str {
    match label {
        "Registrarse gratis" | "Ya tengo cuenta" => HERO_SOURCE,
        "Crear cuenta gratis" => PREVIEW_SOURCE,
        "Ver código en GitHub" => TRANSPARENCY_SOURCE,
        _ => "",
    }
}

/// Extracts the styling context (class attributes) around a CTA label by finding
/// the enclosing element's opening tag. Returns the text of the element block
/// (back to the previous `<`) for analysis.
fn extract_cta_context(source: &str, label: &str) -> String {
    let label_quoted = format!("{{\"{label}\"}}");
    let Some(label_pos) = source.find(&label_quoted) else {
        return String::new();
    };

    // Look backwards from the label to capture the element's opening tag and attributes.
    let start = source[..label_pos]
        .rfind('<')
        .unwrap_or_else(|| label_pos.saturating_sub(500));
    source[start..label_pos + label_quoted.len()].to_string()
}

/// Checks whether a CTA context uses the filled primary styling.
fn has_primary_styling(context: &str) -> bool {
    context.contains("gi-l-btn-primary")
}

/// Checks whether a CTA context uses the outlined secondary styling.
fn has_secondary_styling(context: &str) -> bool {
    context.contains("gi-l-btn-secondary")
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests: specific CTA examples
// ─────────────────────────────────────────────────────────────────────────────

/// **Validates: Requirements 9.1, 9.3**
#[test]
fn test_ya_tengo_cuenta_uses_secondary_styling() {
    let context = extract_cta_context(HERO_SOURCE, "Ya tengo cuenta");
    assert!(
        !context.is_empty(),
        "\"Ya tengo cuenta\" CTA must exist in hero.rs"
    );
    assert!(
        !has_primary_styling(&context),
        "\"Ya tengo cuenta\" must NOT use the gi-l-btn-primary class. Context: {context}"
    );
    assert!(
        has_secondary_styling(&context),
        "\"Ya tengo cuenta\" must use secondary styling (gi-l-btn-secondary). Context: {context}"
    );
}

/// **Validates: Requirements 9.1, 9.3**
#[test]
fn test_ver_codigo_uses_secondary_styling() {
    let context = extract_cta_context(TRANSPARENCY_SOURCE, "Ver código en GitHub");
    assert!(
        !context.is_empty(),
        "\"Ver código en GitHub\" CTA must exist in transparency.rs"
    );
    assert!(
        !has_primary_styling(&context),
        "\"Ver código en GitHub\" must NOT use the gi-l-btn-primary class. Context: {context}"
    );
    assert!(
        has_secondary_styling(&context),
        "\"Ver código en GitHub\" must use secondary styling (gi-l-btn-secondary). Context: {context}"
    );
}

/// **Validates: Requirements 9.1**
#[test]
fn test_registrarse_gratis_uses_primary_styling() {
    let context = extract_cta_context(HERO_SOURCE, "Registrarse gratis");
    assert!(
        !context.is_empty(),
        "\"Registrarse gratis\" CTA must exist in hero.rs"
    );
    assert!(
        has_primary_styling(&context),
        "\"Registrarse gratis\" must use the gi-l-btn-primary class. Context: {context}"
    );
}

/// **Validates: Requirements 9.1**
#[test]
fn test_crear_cuenta_uses_primary_styling() {
    let context = extract_cta_context(PREVIEW_SOURCE, "Crear cuenta gratis");
    assert!(
        !context.is_empty(),
        "\"Crear cuenta gratis\" CTA must exist in preview.rs"
    );
    assert!(
        has_primary_styling(&context),
        "\"Crear cuenta gratis\" must use the gi-l-btn-primary class. Context: {context}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Property-based test: CTA styling is consistent with the primary/secondary split
// ─────────────────────────────────────────────────────────────────────────────

/// **Validates: Requirements 9.1, 9.3**
///
/// Property 3: For any CTA label, its styling context SHALL use exactly the class
/// that matches its role: primary CTAs use `gi-l-btn-primary` (and not the
/// secondary class), and every other CTA uses `gi-l-btn-secondary` (and not the
/// primary class).
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_cta_styling_matches_role(
        cta_idx in 0..ALL_CTA_LABELS.len()
    ) {
        let label = ALL_CTA_LABELS[cta_idx];

        let source = source_for_cta(label);
        prop_assert!(
            !source.is_empty(),
            "Source file for CTA \"{}\" must be identified", label
        );

        let context = extract_cta_context(source, label);
        prop_assert!(
            !context.is_empty(),
            "CTA \"{}\" must exist in its source file", label
        );

        if is_primary_cta(label) {
            prop_assert!(
                has_primary_styling(&context),
                "Primary CTA \"{}\" must use the gi-l-btn-primary class. Context: \"{}\"",
                label, context
            );
            prop_assert!(
                !has_secondary_styling(&context),
                "Primary CTA \"{}\" must NOT use the gi-l-btn-secondary class. Context: \"{}\"",
                label, context
            );
        } else {
            prop_assert!(
                !has_primary_styling(&context),
                "Secondary CTA \"{}\" must NOT use the gi-l-btn-primary class. Context: \"{}\"",
                label, context
            );
            prop_assert!(
                has_secondary_styling(&context),
                "Secondary CTA \"{}\" must use the gi-l-btn-secondary class. Context: \"{}\"",
                label, context
            );
        }
    }
}
