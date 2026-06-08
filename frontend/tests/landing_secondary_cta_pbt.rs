#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    unused_doc_comments
)]
//! Property 3: Secondary CTA styling consistency
//!
//! For any action button on the landing page not labeled "Registrarse gratis",
//! it SHALL use secondary styling (outline border) and SHALL NOT use #3d8b8b background.

use proptest::prelude::*;

// Feature: landing-page, Property 3: Secondary CTA styling consistency

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
    "Ya tengo cuenta",
    "Ver demo en vivo",
    "Ver código en GitHub",
];

/// The primary CTA label — the only one allowed to use #3d8b8b background.
const PRIMARY_CTA_LABEL: &str = "Registrarse gratis";

/// Returns the source file content containing the given CTA label.
fn source_for_cta(label: &str) -> &'static str {
    match label {
        "Registrarse gratis" | "Ya tengo cuenta" => HERO_SOURCE,
        "Ver demo en vivo" => PREVIEW_SOURCE,
        "Ver código en GitHub" => TRANSPARENCY_SOURCE,
        _ => "",
    }
}

/// Extracts the styling context (class + style attributes) around a CTA label
/// by finding the enclosing element's attributes. Returns the text of the
/// element block (up to 15 lines before the label) for analysis.
fn extract_cta_context(source: &str, label: &str) -> String {
    let label_quoted = format!("{{\"{label}\"}}");
    let label_pos = match source.find(&label_quoted) {
        Some(pos) => pos,
        None => return String::new(),
    };

    // Look backwards from the label to capture the element's opening tag and attributes
    let start = source[..label_pos]
        .rfind('<')
        .unwrap_or(label_pos.saturating_sub(500));
    source[start..label_pos + label_quoted.len()].to_string()
}

/// Checks whether a CTA context contains the primary accent background color.
fn has_primary_background(context: &str) -> bool {
    // Check for inline style with the accent color
    context.contains("#3d8b8b")
        // Check for the primary CTA class
        || context.contains("gi-landing-cta-primary")
}

/// Checks whether a CTA context uses secondary/outline styling.
fn has_secondary_styling(context: &str) -> bool {
    // Has border (inline or via class) and no filled accent background
    context.contains("border") || context.contains("gi-landing-cta-secondary")
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
        !has_primary_background(&context),
        "\"Ya tengo cuenta\" must NOT use #3d8b8b background or gi-landing-cta-primary class. \
         Context: {context}"
    );
    assert!(
        has_secondary_styling(&context),
        "\"Ya tengo cuenta\" must use secondary styling (border outline). Context: {context}"
    );
}

/// **Validates: Requirements 9.1, 9.3**
#[test]
fn test_ver_demo_uses_secondary_styling() {
    let context = extract_cta_context(PREVIEW_SOURCE, "Ver demo en vivo");
    assert!(
        !context.is_empty(),
        "\"Ver demo en vivo\" CTA must exist in preview.rs"
    );
    assert!(
        !has_primary_background(&context),
        "\"Ver demo en vivo\" must NOT use #3d8b8b background or gi-landing-cta-primary class. \
         Context: {context}"
    );
    assert!(
        has_secondary_styling(&context),
        "\"Ver demo en vivo\" must use secondary styling (border outline). Context: {context}"
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
        !has_primary_background(&context),
        "\"Ver código en GitHub\" must NOT use #3d8b8b background or gi-landing-cta-primary class. \
         Context: {context}"
    );
    assert!(
        has_secondary_styling(&context),
        "\"Ver código en GitHub\" must use secondary styling (border outline). Context: {context}"
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
        has_primary_background(&context),
        "\"Registrarse gratis\" must use #3d8b8b background or gi-landing-cta-primary class. \
         Context: {context}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Property-based test: All non-primary CTAs use secondary styling
// ─────────────────────────────────────────────────────────────────────────────

/// **Validates: Requirements 9.1, 9.3**
///
/// Property 3: For any CTA label that is NOT "Registrarse gratis", its styling
/// context SHALL contain border/outline styling and SHALL NOT contain #3d8b8b
/// background color or the gi-landing-cta-primary class.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_secondary_ctas_never_use_primary_background(
        cta_idx in 0..ALL_CTA_LABELS.len()
    ) {
        let label = ALL_CTA_LABELS[cta_idx];

        // Skip the primary CTA — it is expected to have primary styling
        if label == PRIMARY_CTA_LABEL {
            return Ok(());
        }

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

        // Property: secondary CTAs must NOT use primary background
        prop_assert!(
            !has_primary_background(&context),
            "CTA \"{}\" must NOT use #3d8b8b background or gi-landing-cta-primary class. \
             Counterexample: label=\"{}\", context=\"{}\"",
            label, label, context
        );

        // Property: secondary CTAs must have border/outline styling
        prop_assert!(
            has_secondary_styling(&context),
            "CTA \"{}\" must use secondary styling (border outline or gi-landing-cta-secondary). \
             Counterexample: label=\"{}\", context=\"{}\"",
            label, label, context
        );
    }
}
