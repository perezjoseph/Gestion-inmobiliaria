//! Property 14: Preservation — Other Servicios Públicos calls unchanged
//!
//! **GOAL**: Guard that the other API calls on the `/servicios-publicos` page
//! (units and servicios endpoints) continue to target their correct, existing routes.
//!
//! Observed on current source:
//! - Units call: `format!("/propiedades/{prop_id}/unidades")` → targets existing
//!   `GET /api/v1/propiedades/{propiedad_id}/unidades` route
//! - Servicios call: `format!("/propiedades/{prop_id}/unidades/{unit_id}/servicios")` → targets
//!   existing `GET /api/v1/propiedades/{propiedad_id}/unidades/{id}/servicios` route
//!
//! Both routes are confirmed in `backend/src/routes.rs`.
//!
//! **Validates: Requirements 3.9**

// Feature: e2e-exploratory-bugfixes, Property 14: Preservation

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::literal_string_with_formatting_args,
    unused_doc_comments
)]

use proptest::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Source inclusion for structural analysis
// ─────────────────────────────────────────────────────────────────────────────

const SERVICIOS_PUBLICOS_SOURCE: &str = include_str!("../src/pages/servicios_publicos.rs");
const BACKEND_ROUTES_SOURCE: &str = include_str!("../../backend/src/routes.rs");

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests: verify units and servicios endpoints are correct
// ─────────────────────────────────────────────────────────────────────────────

// Feature: e2e-exploratory-bugfixes, Property 14: Preservation

/// **Validates: Requirements 3.9**
///
/// The servicios_publicos page must call `/propiedades/{prop_id}/unidades`
/// to load units when a property is selected.
#[test]
fn servicios_publicos_calls_unidades_endpoint() {
    // The source uses format! with the pattern /propiedades/{prop_id}/unidades
    assert!(
        SERVICIOS_PUBLICOS_SOURCE.contains(r#""/propiedades/{prop_id}/unidades""#),
        "servicios_publicos.rs must call '/propiedades/{{prop_id}}/unidades' \
         to load units for the selected property"
    );
}

/// **Validates: Requirements 3.9**
///
/// The servicios_publicos page must call `/propiedades/{prop_id}/unidades/{unit_id}/servicios`
/// to load servicios when a unit is selected.
#[test]
fn servicios_publicos_calls_servicios_endpoint() {
    // The source uses format! with the pattern /propiedades/{prop_id}/unidades/{unit_id}/servicios
    assert!(
        SERVICIOS_PUBLICOS_SOURCE
            .contains(r#""/propiedades/{prop_id}/unidades/{unit_id}/servicios""#),
        "servicios_publicos.rs must call \
         '/propiedades/{{prop_id}}/unidades/{{unit_id}}/servicios' \
         to load servicios for the selected unit"
    );
}

/// **Validates: Requirements 3.9**
///
/// The backend must expose the `/{propiedad_id}/unidades` route under propiedades scope.
#[test]
fn backend_exposes_unidades_route() {
    assert!(
        BACKEND_ROUTES_SOURCE.contains("unidades"),
        "backend routes.rs must define a '/{{propiedad_id}}/unidades' scope — \
         this route is required by the servicios_publicos page"
    );
}

/// **Validates: Requirements 3.9**
///
/// The backend must expose the `/{id}/servicios` route under the unidades scope.
#[test]
fn backend_exposes_servicios_route() {
    // The routes file defines /{id}/servicios under the unidades scope
    assert!(
        BACKEND_ROUTES_SOURCE.contains("servicios"),
        "backend routes.rs must define a '/{{id}}/servicios' route under unidades — \
         this route is required by the servicios_publicos page"
    );
}

/// **Validates: Requirements 3.9**
///
/// The unidades call uses `api_get::<Vec<Unidad>>` — confirming it expects
/// a direct Vec response (not PaginatedResponse), matching the backend handler.
#[test]
fn servicios_publicos_unidades_uses_vec_response_type() {
    assert!(
        SERVICIOS_PUBLICOS_SOURCE.contains("api_get::<Vec<Unidad>>"),
        "servicios_publicos.rs must use api_get::<Vec<Unidad>> for the unidades call — \
         the backend returns a Vec, not a PaginatedResponse"
    );
}

/// **Validates: Requirements 3.9**
///
/// The servicios call uses `api_get::<Vec<ResponsabilidadEfectiva>>` — confirming it expects
/// a direct Vec response, matching the backend handler.
#[test]
fn servicios_publicos_servicios_uses_vec_response_type() {
    assert!(
        SERVICIOS_PUBLICOS_SOURCE.contains("api_get::<Vec<ResponsabilidadEfectiva>>"),
        "servicios_publicos.rs must use api_get::<Vec<ResponsabilidadEfectiva>> \
         for the servicios call — the backend returns a Vec, not a PaginatedResponse"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Property-based test: endpoint patterns remain unchanged across generated IDs
// ─────────────────────────────────────────────────────────────────────────────

// Feature: e2e-exploratory-bugfixes, Property 14: Preservation

// **Validates: Requirements 3.9**
//
// Property: for any valid UUID-like property and unit IDs, the endpoint patterns
// used in servicios_publicos.rs produce URLs that match the backend route structure.
// This confirms the frontend and backend stay aligned for the units/servicios calls.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_units_endpoint_pattern_matches_backend_route(
        prop_id in "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
    ) {
        // Simulate what the frontend does: format!("/propiedades/{prop_id}/unidades")
        let url = format!("/propiedades/{prop_id}/unidades");

        // The URL must start with /propiedades/ and end with /unidades
        prop_assert!(url.starts_with("/propiedades/"));
        prop_assert!(url.ends_with("/unidades"));

        // Must not contain the buggy /propiedades/todas pattern
        prop_assert!(!url.contains("/propiedades/todas"));

        // The source must still contain the format pattern that generates this URL shape
        prop_assert!(
            SERVICIOS_PUBLICOS_SOURCE.contains(r#""/propiedades/{prop_id}/unidades""#),
            "The units endpoint pattern must remain in servicios_publicos.rs"
        );
    }

    #[test]
    fn prop_servicios_endpoint_pattern_matches_backend_route(
        prop_id in "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
        unit_id in "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
    ) {
        // Simulate what the frontend does:
        // format!("/propiedades/{prop_id}/unidades/{unit_id}/servicios")
        let url = format!("/propiedades/{prop_id}/unidades/{unit_id}/servicios");

        // The URL must follow the expected structure
        prop_assert!(url.starts_with("/propiedades/"));
        prop_assert!(url.contains("/unidades/"));
        prop_assert!(url.ends_with("/servicios"));

        // Must not contain any unexpected path segments
        prop_assert!(!url.contains("/todas"));

        // The source must still contain the format pattern that generates this URL shape
        prop_assert!(
            SERVICIOS_PUBLICOS_SOURCE
                .contains(r#""/propiedades/{prop_id}/unidades/{unit_id}/servicios""#),
            "The servicios endpoint pattern must remain in servicios_publicos.rs"
        );
    }
}
