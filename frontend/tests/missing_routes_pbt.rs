//! Bug Condition Exploration Test: Missing Frontend Routes
//!
//! This test verifies that the 7 missing route variants exist in the Route enum
//! and that sidebar links reference them. On UNFIXED code, this test MUST FAIL
//! because the variants do not exist yet — failure confirms the bug.
//!
//! **Property 1: Bug Condition** — Missing Routes Render 404
//!
//! The bug: navigating to /desahucios, /ncf, /tareas, /invitaciones,
//! /organizacion, /dgii, or /servicios-publicos results in 404 because
//! the Route enum has no variants for these paths and the sidebar has no links.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::ignore_without_reason,
    clippy::option_if_let_else,
    clippy::range_plus_one,
    clippy::if_not_else,
    clippy::manual_let_else
)]

use proptest::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Source inclusion for structural analysis
// ─────────────────────────────────────────────────────────────────────────────

const APP_RS_SOURCE: &str = include_str!("../src/app.rs");
const SIDEBAR_RS_SOURCE: &str = include_str!("../src/components/layout/sidebar.rs");

// ─────────────────────────────────────────────────────────────────────────────
// Missing route definitions
// ─────────────────────────────────────────────────────────────────────────────

/// The 7 route variants that SHOULD exist but are currently missing.
const MISSING_ROUTE_VARIANTS: [&str; 7] = [
    "Desahucios",
    "Ncf",
    "Tareas",
    "Invitaciones",
    "Organizacion",
    "Dgii",
    "ServiciosPublicos",
];

/// The 7 URL paths that SHOULD be routable but currently produce 404.
const MISSING_ROUTE_PATHS: [&str; 7] = [
    "/desahucios",
    "/ncf",
    "/tareas",
    "/invitaciones",
    "/organizacion",
    "/dgii",
    "/servicios-publicos",
];

/// Sidebar link labels that SHOULD exist for the new routes.
const MISSING_SIDEBAR_LABELS: [&str; 7] = [
    "Desahucios",
    "NCF",
    "Tareas",
    "Invitaciones",
    "Organizaci", // matches both "Organización" and "Organizacion"
    "DGII",
    "Servicios P", // matches "Servicios Públicos" or "Servicios Publicos"
];

fn variant_name_from_index(idx: usize) -> &'static str {
    MISSING_ROUTE_VARIANTS[idx]
}

fn route_path_from_index(idx: usize) -> &'static str {
    MISSING_ROUTE_PATHS[idx]
}

fn sidebar_label_from_index(idx: usize) -> &'static str {
    MISSING_SIDEBAR_LABELS[idx]
}

/// Extracts the Route enum definition from app.rs source.
fn extract_route_enum(source: &str) -> &str {
    let enum_start = source.find("pub enum Route");
    let enum_start = match enum_start {
        Some(pos) => pos,
        None => return "",
    };

    let rest = &source[enum_start..];
    let mut depth: i32 = 0;
    let mut found_first_brace = false;
    for (idx, ch) in rest.char_indices() {
        match ch {
            '{' => {
                depth += 1;
                found_first_brace = true;
            }
            '}' => {
                depth -= 1;
                if found_first_brace && depth == 0 {
                    return &rest[..idx + 1];
                }
            }
            _ => {}
        }
    }

    rest
}

/// Extracts the switch() function from app.rs source.
fn extract_switch_fn(source: &str) -> &str {
    let fn_start = source.find("fn switch(");
    let fn_start = match fn_start {
        Some(pos) => pos,
        None => return "",
    };

    let rest = &source[fn_start..];
    let mut depth: i32 = 0;
    let mut found_first_brace = false;
    for (idx, ch) in rest.char_indices() {
        match ch {
            '{' => {
                depth += 1;
                found_first_brace = true;
            }
            '}' => {
                depth -= 1;
                if found_first_brace && depth == 0 {
                    return &rest[..idx + 1];
                }
            }
            _ => {}
        }
    }

    rest
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests: Route enum variant existence
// ─────────────────────────────────────────────────────────────────────────────

// **Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7**
#[cfg(test)]
mod route_variant_tests {
    use super::*;

    #[test]
    fn route_enum_contains_desahucios_variant() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains("Desahucios"),
            "Route enum must contain Desahucios variant — \
             currently missing, proving /desahucios renders 404"
        );
    }

    #[test]
    fn route_enum_contains_ncf_variant() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains("Ncf"),
            "Route enum must contain Ncf variant — \
             currently missing, proving /ncf renders 404"
        );
    }

    #[test]
    fn route_enum_contains_tareas_variant() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains("Tareas"),
            "Route enum must contain Tareas variant — \
             currently missing, proving /tareas renders 404"
        );
    }

    #[test]
    fn route_enum_contains_invitaciones_variant() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains("Invitaciones"),
            "Route enum must contain Invitaciones variant — \
             currently missing, proving /invitaciones renders 404"
        );
    }

    #[test]
    fn route_enum_contains_organizacion_variant() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains("Organizacion"),
            "Route enum must contain Organizacion variant — \
             currently missing, proving /organizacion renders 404"
        );
    }

    #[test]
    fn route_enum_contains_dgii_variant() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains("Dgii"),
            "Route enum must contain Dgii variant — \
             currently missing, proving /dgii renders 404"
        );
    }

    #[test]
    fn route_enum_contains_servicios_publicos_variant() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains("ServiciosPublicos"),
            "Route enum must contain ServiciosPublicos variant — \
             currently missing, proving /servicios-publicos renders 404"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests: #[at(...)] path annotations
// ─────────────────────────────────────────────────────────────────────────────

// **Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7**
#[cfg(test)]
mod route_path_tests {
    use super::*;

    #[test]
    fn route_enum_has_desahucios_path() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains(r#"#[at("/desahucios")]"#),
            "Route enum must have #[at(\"/desahucios\")] annotation — \
             currently missing, Route::recognize(\"/desahucios\") returns None"
        );
    }

    #[test]
    fn route_enum_has_ncf_path() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains(r#"#[at("/ncf")]"#),
            "Route enum must have #[at(\"/ncf\")] annotation — \
             currently missing, Route::recognize(\"/ncf\") returns None"
        );
    }

    #[test]
    fn route_enum_has_tareas_path() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains(r#"#[at("/tareas")]"#),
            "Route enum must have #[at(\"/tareas\")] annotation — \
             currently missing, Route::recognize(\"/tareas\") returns None"
        );
    }

    #[test]
    fn route_enum_has_invitaciones_path() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains(r#"#[at("/invitaciones")]"#),
            "Route enum must have #[at(\"/invitaciones\")] annotation — \
             currently missing, Route::recognize(\"/invitaciones\") returns None"
        );
    }

    #[test]
    fn route_enum_has_organizacion_path() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains(r#"#[at("/organizacion")]"#),
            "Route enum must have #[at(\"/organizacion\")] annotation — \
             currently missing, Route::recognize(\"/organizacion\") returns None"
        );
    }

    #[test]
    fn route_enum_has_dgii_path() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains(r#"#[at("/dgii")]"#),
            "Route enum must have #[at(\"/dgii\")] annotation — \
             currently missing, Route::recognize(\"/dgii\") returns None"
        );
    }

    #[test]
    fn route_enum_has_servicios_publicos_path() {
        let route_enum = extract_route_enum(APP_RS_SOURCE);
        assert!(
            route_enum.contains(r#"#[at("/servicios-publicos")]"#),
            "Route enum must have #[at(\"/servicios-publicos\")] annotation — \
             currently missing, Route::recognize(\"/servicios-publicos\") returns None"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests: switch() function handles new routes
// ─────────────────────────────────────────────────────────────────────────────

// **Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7**
#[cfg(test)]
mod switch_arm_tests {
    use super::*;

    #[test]
    fn switch_handles_all_missing_routes() {
        let switch_fn = extract_switch_fn(APP_RS_SOURCE);
        assert!(
            !switch_fn.is_empty(),
            "switch() function must exist in app.rs"
        );

        for variant in &MISSING_ROUTE_VARIANTS {
            let arm = format!("Route::{variant}");
            assert!(
                switch_fn.contains(&arm),
                "switch() must handle {arm} — currently missing, \
                 navigation to the route renders 404 via NotFound fallback"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests: Sidebar links for new routes
// ─────────────────────────────────────────────────────────────────────────────

// **Validates: Requirements 1.8**
#[cfg(test)]
mod sidebar_link_tests {
    use super::*;

    #[test]
    fn sidebar_contains_desahucios_link() {
        assert!(
            SIDEBAR_RS_SOURCE.contains("Route::Desahucios"),
            "Sidebar must contain a Link<Route> to Route::Desahucios — \
             currently missing, users have no way to navigate to /desahucios"
        );
    }

    #[test]
    fn sidebar_contains_ncf_link() {
        assert!(
            SIDEBAR_RS_SOURCE.contains("Route::Ncf"),
            "Sidebar must contain a Link<Route> to Route::Ncf — \
             currently missing, users have no way to navigate to /ncf"
        );
    }

    #[test]
    fn sidebar_contains_tareas_link() {
        assert!(
            SIDEBAR_RS_SOURCE.contains("Route::Tareas"),
            "Sidebar must contain a Link<Route> to Route::Tareas — \
             currently missing, users have no way to navigate to /tareas"
        );
    }

    #[test]
    fn sidebar_contains_invitaciones_link() {
        assert!(
            SIDEBAR_RS_SOURCE.contains("Route::Invitaciones"),
            "Sidebar must contain a Link<Route> to Route::Invitaciones — \
             currently missing, users have no way to navigate to /invitaciones"
        );
    }

    #[test]
    fn sidebar_contains_organizacion_link() {
        assert!(
            SIDEBAR_RS_SOURCE.contains("Route::Organizacion"),
            "Sidebar must contain a Link<Route> to Route::Organizacion — \
             currently missing, users have no way to navigate to /organizacion"
        );
    }

    #[test]
    fn sidebar_contains_dgii_link() {
        assert!(
            SIDEBAR_RS_SOURCE.contains("Route::Dgii"),
            "Sidebar must contain a Link<Route> to Route::Dgii — \
             currently missing, users have no way to navigate to /dgii"
        );
    }

    #[test]
    fn sidebar_contains_servicios_publicos_link() {
        assert!(
            SIDEBAR_RS_SOURCE.contains("Route::ServiciosPublicos"),
            "Sidebar must contain a Link<Route> to Route::ServiciosPublicos — \
             currently missing, users have no way to navigate to /servicios-publicos"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property-based test: All 7 missing routes fail recognition
// ─────────────────────────────────────────────────────────────────────────────

// **Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8**
//
// Property 1: Bug Condition — For each of the 7 missing routes, the Route enum
// must contain the variant, the #[at(...)] path annotation, a switch() arm,
// and a corresponding sidebar link. On UNFIXED code, ALL of these are absent,
// proving the bug exists.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(7))]
    #[test]
    fn bug_condition_missing_routes_not_recognized(route_idx in 0..7usize) {
        let variant = variant_name_from_index(route_idx);
        let path = route_path_from_index(route_idx);
        let sidebar_label = sidebar_label_from_index(route_idx);

        let route_enum = extract_route_enum(APP_RS_SOURCE);
        let switch_fn = extract_switch_fn(APP_RS_SOURCE);

        // Check 1: Route enum must contain the variant
        let at_annotation = format!("#[at(\"{path}\")]");
        prop_assert!(
            route_enum.contains(&at_annotation),
            "Route enum missing #[at(\"{path}\")] — \
             Route::recognize(\"{path}\") returns None (404). \
             Counterexample: path={path}, variant={variant}",
        );

        // Check 2: Route enum must have the variant name
        prop_assert!(
            route_enum.contains(variant),
            "Route enum missing variant {variant} — \
             cannot match Route::{variant}. \
             Counterexample: variant={variant}, path={path}",
        );

        // Check 3: switch() must handle the route
        let arm = format!("Route::{variant}");
        prop_assert!(
            switch_fn.contains(&arm),
            "switch() missing arm for {arm} — \
             even if route resolves, it won't render a page. \
             Counterexample: variant={variant}, path={path}",
        );

        // Check 4: Sidebar must have a link for this route
        prop_assert!(
            SIDEBAR_RS_SOURCE.contains(&format!("Route::{variant}")),
            "Sidebar missing Link<Route> to Route::{variant} — \
             users cannot navigate to {path} from the UI. \
             Counterexample: variant={variant}, sidebar_label={sidebar_label}",
        );
    }
}
