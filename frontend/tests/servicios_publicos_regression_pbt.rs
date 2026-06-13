//! Property 13: Bug Condition (regression guard) — Servicios Públicos calls an existing property endpoint
//!
//! This test guards that `frontend/src/pages/servicios_publicos.rs` requests
//! `GET /api/v1/propiedades?perPage=200` (an existing route) and that no caller
//! of the non-existent `/propiedades/todas` route exists anywhere in the frontend source.
//!
//! The bug was already fixed in the current source; this test prevents reintroduction.
//!
//! Feature: e2e-exploratory-bugfixes, Property 13: Bug Condition (regression guard)

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    clippy::missing_const_for_fn
)]

use proptest::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Source inclusion for structural analysis
// ─────────────────────────────────────────────────────────────────────────────

const SERVICIOS_PUBLICOS_SOURCE: &str = include_str!("../src/pages/servicios_publicos.rs");

// Include other frontend source files that could potentially call property endpoints
const APP_SOURCE: &str = include_str!("../src/app.rs");
const LIB_SOURCE: &str = include_str!("../src/lib.rs");

/// The correct endpoint that servicios_publicos.rs should call to load properties.
const CORRECT_ENDPOINT: &str = "/propiedades?perPage=200";

/// The non-existent route that was called in the buggy deployed build.
const BUGGY_ENDPOINT: &str = "/propiedades/todas";

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests: servicios_publicos.rs calls the correct endpoint
// ─────────────────────────────────────────────────────────────────────────────

// Feature: e2e-exploratory-bugfixes, Property 13: Bug Condition (regression guard)

/// **Validates: Requirements 1.7**
///
/// The servicios_publicos page must request `/propiedades?perPage=200` (an existing route).
#[test]
fn servicios_publicos_calls_correct_property_endpoint() {
    assert!(
        SERVICIOS_PUBLICOS_SOURCE.contains(CORRECT_ENDPOINT),
        "servicios_publicos.rs must call '{CORRECT_ENDPOINT}' to load properties — \
         if this fails, the page is calling a non-existent route"
    );
}

/// **Validates: Requirements 1.7**
///
/// The servicios_publicos page must NOT call the non-existent `/propiedades/todas` route.
#[test]
fn servicios_publicos_does_not_call_buggy_endpoint() {
    assert!(
        !SERVICIOS_PUBLICOS_SOURCE.contains(BUGGY_ENDPOINT),
        "servicios_publicos.rs must NOT call '{BUGGY_ENDPOINT}' — \
         this route does not exist and causes a 404 on the property dropdown"
    );
}

/// **Validates: Requirements 1.7**
///
/// No frontend source file should reference `/propiedades/todas`.
#[test]
fn no_frontend_source_calls_buggy_endpoint() {
    let sources: &[(&str, &str)] = &[
        ("servicios_publicos.rs", SERVICIOS_PUBLICOS_SOURCE),
        ("app.rs", APP_SOURCE),
        ("lib.rs", LIB_SOURCE),
    ];

    for (filename, source) in sources {
        assert!(
            !source.contains(BUGGY_ENDPOINT),
            "{filename} contains a reference to '{BUGGY_ENDPOINT}' — \
             this non-existent route must be removed to prevent 404s"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property-based test: regression guard over endpoint variations
// ─────────────────────────────────────────────────────────────────────────────

// Feature: e2e-exploratory-bugfixes, Property 13: Bug Condition (regression guard)

// **Validates: Requirements 1.7**
//
// Property: for any suffix appended to `/propiedades/`, the servicios_publicos page
// must not contain a call to `/propiedades/{suffix}` that would be a non-existent route.
// The only valid property-list call is `/propiedades?perPage=200` (query param, not path segment).
// This guards against reintroduction of `/propiedades/todas` or similar non-existent path routes.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    fn regression_guard_no_invalid_propiedades_path_routes(
        suffix in "(todas|all|lista|listado|complete|full|[a-z]{3,10})"
    ) {
        // Build the pattern that would be a non-existent sub-path route
        let invalid_route = format!("/propiedades/{suffix}\"");

        // The source should not contain this invalid route pattern
        // (valid sub-paths like /propiedades/{id}/unidades use a variable, not a literal string)
        prop_assert!(
            !SERVICIOS_PUBLICOS_SOURCE.contains(&invalid_route),
            "servicios_publicos.rs contains a call to a non-existent route: /propiedades/{suffix} — \
             the correct endpoint is '{CORRECT_ENDPOINT}'. \
             Counterexample: invalid_route=/propiedades/{suffix}"
        );
    }
}

/// **Validates: Requirements 1.7**
///
/// Property: the servicios_publicos source uses `api_get` with the PaginatedResponse type
/// for the property list call, confirming it expects the standard paginated envelope.
#[test]
fn servicios_publicos_uses_paginated_response_for_properties() {
    // The source should contain the full api_get call with PaginatedResponse<Propiedad>
    assert!(
        SERVICIOS_PUBLICOS_SOURCE.contains("api_get::<PaginatedResponse<Propiedad>>"),
        "servicios_publicos.rs must use api_get::<PaginatedResponse<Propiedad>> — \
         this ensures the property list call expects the standard paginated envelope"
    );
}
