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

fn route_variant_name(index: usize) -> &'static str {
    match index {
        0 => "Login",
        1 => "Dashboard",
        2 => "Propiedades",
        3 => "Inquilinos",
        4 => "Contratos",
        5 => "Pagos",
        6 => "Registro",
        7 => "Reportes",
        8 => "UsuariosPage",
        9 => "Perfil",
        10 => "AuditoriaPage",
        11 => "Importar",
        12 => "Mantenimiento",
        13 => "NotFound",
        14 => "Gastos",
        15 => "CategoriasGastos",
        16 => "Notificaciones",
        17 => "Configuracion",
        18 => "ConfiguracionChatbot",
        19 => "Plantillas",
        20 => "DocumentosPorVencer",
        21 => "DocumentoEditor",
        22 => "DocumentoEditorExisting",
        23 => "FirmaPublica",
        _ => unreachable!(),
    }
}

const ROUTE_COUNT: usize = 24;

const SWITCH_FN_SOURCE: &str = include_str!("../src/app.rs");

// Debug logging tests removed — diagnostic console output is no longer
// part of the switch function (it was causing unnecessary overhead).

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

#[cfg(test)]
mod structural_checks {
    use super::*;

    #[test]
    fn switch_function_exists_in_source() {
        assert!(
            SWITCH_FN_SOURCE.contains("fn switch("),
            "The switch function must exist in app.rs"
        );
    }

    #[test]
    fn all_route_variants_covered_in_switch() {
        let switch_fn = extract_switch_fn(SWITCH_FN_SOURCE);
        for idx in 0..ROUTE_COUNT {
            let variant = route_variant_name(idx);
            assert!(
                switch_fn.contains(&format!("Route::{variant}")),
                "switch() must handle Route::{variant}",
            );
        }
    }
}

// These tests verify baseline behavior on UNFIXED code. They must PASS,
// confirming the behavior that the bugfix must preserve.

const INDEX_HTML_SOURCE: &str = include_str!("../index.html");

fn extract_protected_route_fn(source: &str) -> &str {
    let fn_start = source.find("fn ProtectedRoute(");
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

const PUBLIC_ROUTES: [&str; 2] = ["Login", "Registro"];

const PROTECTED_ROUTES: [&str; 11] = [
    "Dashboard",
    "Propiedades",
    "Inquilinos",
    "Contratos",
    "Pagos",
    "Reportes",
    "UsuariosPage",
    "Perfil",
    "AuditoriaPage",
    "Importar",
    "Mantenimiento",
];

// **Validates: Requirements 3.1**
//
// Property 2a: Routing Preservation — switch() handles all Route variants
// and wraps protected routes in ProtectedRoute while rendering public
// routes directly.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(14))]
    #[test]
    fn preservation_routing_all_variants_handled(route_idx in 0..ROUTE_COUNT) {
        let variant = route_variant_name(route_idx);
        let switch_fn = extract_switch_fn(SWITCH_FN_SOURCE);

        prop_assert!(
            switch_fn.contains(&format!("Route::{variant}")),
            "switch() must handle Route::{} — routing preservation violated",
            variant,
        );

        if PUBLIC_ROUTES.contains(&variant) {
            prop_assert!(
                !switch_fn.contains(&format!("Route::{variant} => html! {{ <ProtectedRoute>")),
                "Route::{} should render directly without ProtectedRoute wrapper",
                variant,
            );
        }
    }
}

#[cfg(test)]
mod preservation_routing {
    use super::*;

    // **Validates: Requirements 3.1**
    #[test]
    fn public_routes_render_directly() {
        let switch_fn = extract_switch_fn(SWITCH_FN_SOURCE);
        for variant in &PUBLIC_ROUTES {
            let arm = format!("Route::{variant}");
            let arm_start = switch_fn
                .find(&arm)
                .unwrap_or_else(|| panic!("switch() must contain {arm}"));
            let arm_rest = &switch_fn[arm_start..];
            let arm_end = arm_rest
                .find('}')
                .unwrap_or_else(|| panic!("Could not find end of arm for {arm}"));
            let arm_text = &arm_rest[..arm_end + 1];
            assert!(
                !arm_text.contains("ProtectedRoute"),
                "Route::{variant} must render directly without ProtectedRoute wrapper",
            );
        }
    }

    // **Validates: Requirements 3.1**
    #[test]
    fn protected_routes_wrapped_in_protected_route() {
        let switch_fn = extract_switch_fn(SWITCH_FN_SOURCE);
        for variant in &PROTECTED_ROUTES {
            let arm = format!("Route::{variant}");
            let arm_start = switch_fn
                .find(&arm)
                .unwrap_or_else(|| panic!("switch() must contain {arm}"));
            let arm_rest = &switch_fn[arm_start..];
            let next_route = arm_rest[1..].find("Route::");
            let arm_text = match next_route {
                Some(end) => &arm_rest[..end + 1],
                None => arm_rest,
            };
            assert!(
                arm_text.contains("ProtectedRoute"),
                "Route::{variant} must be wrapped in <ProtectedRoute>",
            );
        }
    }

    // **Validates: Requirements 3.1**
    #[test]
    fn not_found_route_renders_404() {
        let switch_fn = extract_switch_fn(SWITCH_FN_SOURCE);
        let arm = "Route::NotFound";
        let arm_start = switch_fn
            .find(arm)
            .expect("switch() must contain Route::NotFound");
        let arm_rest = &switch_fn[arm_start..];
        assert!(
            arm_rest.contains("404"),
            "Route::NotFound must render a 404 page with '404' text",
        );
    }
}

// **Validates: Requirements 3.2**
#[cfg(test)]
mod preservation_auth_redirect {
    use super::*;

    #[test]
    fn protected_route_redirects_unauthenticated_to_login() {
        let pr_fn = extract_protected_route_fn(SWITCH_FN_SOURCE);
        assert!(
            !pr_fn.is_empty(),
            "ProtectedRoute function must exist in app.rs",
        );
        assert!(
            pr_fn.contains("navigator.push(&Route::Login)"),
            "ProtectedRoute must redirect unauthenticated users to Login via navigator.push(&Route::Login)",
        );
    }

    #[test]
    fn protected_route_checks_auth_state() {
        let pr_fn = extract_protected_route_fn(SWITCH_FN_SOURCE);
        assert!(
            pr_fn.contains("is_authed"),
            "ProtectedRoute must check is_authed to determine authentication state",
        );
    }

    #[test]
    fn protected_route_renders_layout_for_authenticated() {
        let pr_fn = extract_protected_route_fn(SWITCH_FN_SOURCE);
        assert!(
            pr_fn.contains("Navbar"),
            "ProtectedRoute must render Navbar for authenticated users",
        );
        assert!(
            pr_fn.contains("Sidebar"),
            "ProtectedRoute must render Sidebar for authenticated users",
        );
        assert!(
            pr_fn.contains("Footer"),
            "ProtectedRoute must render Footer for authenticated users",
        );
        assert!(
            pr_fn.contains("OfflineBanner"),
            "ProtectedRoute must render OfflineBanner for authenticated users",
        );
    }
}

// **Validates: Requirements 3.4**
#[cfg(test)]
mod preservation_error_handlers {
    use super::*;

    #[test]
    fn index_html_has_global_error_handler() {
        assert!(
            INDEX_HTML_SOURCE.contains("addEventListener('error'")
                || INDEX_HTML_SOURCE.contains("addEventListener(\"error\""),
            "index.html must contain a global 'error' event handler",
        );
    }

    #[test]
    fn index_html_has_unhandled_rejection_handler() {
        assert!(
            INDEX_HTML_SOURCE.contains("unhandledrejection"),
            "index.html must contain an 'unhandledrejection' handler",
        );
    }

    #[test]
    fn index_html_error_overlay_displays_error() {
        assert!(
            INDEX_HTML_SOURCE.contains("Error loading app") || INDEX_HTML_SOURCE.contains("error"),
            "index.html error handlers must display an error overlay",
        );
        assert!(
            INDEX_HTML_SOURCE.contains("innerHTML"),
            "index.html error handlers must update innerHTML to show error details",
        );
    }
}

// **Validates: Requirements 3.5**
#[cfg(test)]
mod preservation_spinner_removal {
    use super::*;

    #[test]
    fn index_html_has_trunk_application_started_listener() {
        assert!(
            INDEX_HTML_SOURCE.contains("TrunkApplicationStarted"),
            "index.html must listen for the TrunkApplicationStarted event",
        );
    }

    #[test]
    fn index_html_removes_loading_element() {
        let has_loading_removal = INDEX_HTML_SOURCE.contains("getElementById(\"loading\")")
            || INDEX_HTML_SOURCE.contains("getElementById('loading')");
        assert!(
            has_loading_removal,
            "index.html must reference the #loading element for removal",
        );

        let has_remove = INDEX_HTML_SOURCE.contains(".remove()");
        assert!(
            has_remove,
            "index.html must call .remove() on the loading element",
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Feature: spec-gap-remediation, Bug_Condition_PBT: All [INIT] markers present at boot
// ─────────────────────────────────────────────────────────────────────────────
//
// This PBT verifies that the six [INIT] bootstrap markers are emitted for every
// (route, auth_state) permutation. Since WASM browser tests require a headless
// browser environment (CI), we verify the property via source-level analysis:
// the markers must be unconditionally present in the code paths that execute
// for each route/auth combination.
//
// The six required markers:
//   1. "[INIT] pre-renderer"       — in main.rs before renderer starts
//   2. "[INIT] app mounted"        — in App component body
//   3. "[INIT] route resolution"   — in Switch<Route> render closure
//   4. "[INIT] switch"             — in switch() function body
//   5. "[INIT] auth check"         — in ProtectedRoute component body
//   6. "[INIT] first route rendered" — in ProtectedRoute auth-success path

const MAIN_RS_SOURCE: &str = include_str!("../src/main.rs");

const INIT_MARKERS: [&str; 6] = [
    "[INIT] pre-renderer",
    "[INIT] app mounted",
    "[INIT] route resolution",
    "[INIT] switch",
    "[INIT] auth check",
    "[INIT] first route rendered",
];

/// Markers that fire unconditionally on every boot (before auth branching).
const UNCONDITIONAL_MARKERS: [&str; 4] = [
    "[INIT] pre-renderer",
    "[INIT] app mounted",
    "[INIT] route resolution",
    "[INIT] switch",
];

/// Markers that fire only for protected (authenticated) routes.
#[allow(dead_code)]
const PROTECTED_ONLY_MARKERS: [&str; 2] = ["[INIT] auth check", "[INIT] first route rendered"];

/// Routes that go through ProtectedRoute (authenticated).
const AUTH_PROTECTED_ROUTES: [&str; 20] = [
    "Dashboard",
    "Propiedades",
    "Inquilinos",
    "Contratos",
    "Pagos",
    "Gastos",
    "CategoriasGastos",
    "Reportes",
    "UsuariosPage",
    "Perfil",
    "AuditoriaPage",
    "Importar",
    "Mantenimiento",
    "Notificaciones",
    "Configuracion",
    "ConfiguracionChatbot",
    "Plantillas",
    "DocumentosPorVencer",
    "DocumentoEditor",
    "DocumentoEditorExisting",
];

/// Routes that render directly without ProtectedRoute (public).
#[allow(dead_code)]
const PUBLIC_BOOT_ROUTES: [&str; 4] = ["Login", "Registro", "FirmaPublica", "NotFound"];

/// Auth states for permutation testing.
#[derive(Debug, Clone, Copy)]
enum AuthState {
    Authenticated,
    Unauthenticated,
}

fn auth_state_from_index(idx: usize) -> AuthState {
    match idx % 2 {
        0 => AuthState::Authenticated,
        _ => AuthState::Unauthenticated,
    }
}

/// Returns the set of [INIT] markers expected for a given (route, auth_state) pair.
fn expected_markers_for(route: &str, auth: AuthState) -> Vec<&'static str> {
    let mut markers: Vec<&str> = UNCONDITIONAL_MARKERS.to_vec();

    let is_protected = AUTH_PROTECTED_ROUTES.contains(&route);

    if is_protected {
        // Protected routes always hit ProtectedRoute, which emits "auth check".
        markers.push("[INIT] auth check");
        // "first route rendered" only fires when authenticated.
        if matches!(auth, AuthState::Authenticated) {
            markers.push("[INIT] first route rendered");
        }
    }

    markers
}

/// Checks that a marker is present in the correct source file.
fn marker_present_in_source(marker: &str) -> bool {
    match marker {
        "[INIT] pre-renderer" => MAIN_RS_SOURCE.contains(marker),
        "[INIT] app mounted" => SWITCH_FN_SOURCE.contains(marker),
        "[INIT] route resolution" => SWITCH_FN_SOURCE.contains(marker),
        "[INIT] switch" => SWITCH_FN_SOURCE.contains(marker),
        "[INIT] auth check" => SWITCH_FN_SOURCE.contains(marker),
        "[INIT] first route rendered" => SWITCH_FN_SOURCE.contains(marker),
        _ => false,
    }
}

/// Verifies that a marker is NOT gated behind cfg(debug_assertions).
fn marker_not_debug_gated(marker: &str) -> bool {
    let source = if marker == "[INIT] pre-renderer" {
        MAIN_RS_SOURCE
    } else {
        SWITCH_FN_SOURCE
    };

    let Some(pos) = source.find(marker) else {
        return false;
    };

    // Check the 200 chars before the marker for a debug gate
    let start = pos.saturating_sub(200);
    let preceding = &source[start..pos];
    !preceding.contains("cfg(debug_assertions)") && !preceding.contains("#[cfg(debug_assertions)]")
}

// **Validates: Requirements 7.1, 7.2, 7.3, 7.4**
#[cfg(test)]
mod bug_condition_pbt {
    use super::*;

    /// Structural pre-check: all six markers exist in source and are not debug-gated.
    #[test]
    fn all_init_markers_present_in_source() {
        for marker in &INIT_MARKERS {
            assert!(
                marker_present_in_source(marker),
                "Missing [INIT] marker in source: {marker}",
            );
            assert!(
                marker_not_debug_gated(marker),
                "[INIT] marker must not be gated behind cfg(debug_assertions): {marker}",
            );
        }
    }

    /// Verify pre-renderer fires before the renderer call.
    #[test]
    fn pre_renderer_fires_before_render() {
        let marker_pos = MAIN_RS_SOURCE
            .find("[INIT] pre-renderer")
            .expect("[INIT] pre-renderer must exist in main.rs");
        let render_pos = MAIN_RS_SOURCE
            .find("Renderer::<")
            .or_else(|| MAIN_RS_SOURCE.find("yew::Renderer"))
            .expect("yew::Renderer call must exist in main.rs");
        assert!(
            marker_pos < render_pos,
            "[INIT] pre-renderer must fire BEFORE yew::Renderer is called",
        );
    }

    /// Verify switch marker fires unconditionally at the top of switch().
    #[test]
    fn switch_marker_at_top_of_switch_fn() {
        let switch_fn = extract_switch_fn(SWITCH_FN_SOURCE);
        assert!(
            !switch_fn.is_empty(),
            "switch() function must exist in app.rs",
        );
        let marker_pos = switch_fn
            .find("[INIT] switch")
            .expect("[INIT] switch must exist inside switch()");
        let match_pos = switch_fn
            .find("match")
            .expect("switch() must contain a match expression");
        assert!(
            marker_pos < match_pos,
            "[INIT] switch must fire BEFORE the match expression in switch()",
        );
    }

    /// Verify auth check and first route rendered are inside ProtectedRoute.
    #[test]
    fn auth_markers_inside_protected_route() {
        let pr_fn = extract_protected_route_fn(SWITCH_FN_SOURCE);
        assert!(
            !pr_fn.is_empty(),
            "ProtectedRoute function must exist in app.rs",
        );
        assert!(
            pr_fn.contains("[INIT] auth check"),
            "[INIT] auth check must be inside ProtectedRoute",
        );
        assert!(
            pr_fn.contains("[INIT] first route rendered"),
            "[INIT] first route rendered must be inside ProtectedRoute",
        );
    }

    /// Verify "first route rendered" fires only after auth succeeds (after is_authed check).
    #[test]
    fn first_route_rendered_after_auth_check() {
        let pr_fn = extract_protected_route_fn(SWITCH_FN_SOURCE);
        let auth_check_pos = pr_fn
            .find("[INIT] auth check")
            .expect("[INIT] auth check must exist in ProtectedRoute");
        let first_rendered_pos = pr_fn
            .find("[INIT] first route rendered")
            .expect("[INIT] first route rendered must exist in ProtectedRoute");
        let is_authed_pos = pr_fn
            .find("is_authed")
            .expect("ProtectedRoute must check is_authed");
        assert!(
            auth_check_pos < is_authed_pos,
            "[INIT] auth check must fire before is_authed branching",
        );
        assert!(
            first_rendered_pos > is_authed_pos,
            "[INIT] first route rendered must fire after is_authed check (auth-success path only)",
        );
    }
}

// **Validates: Requirements 7.1, 7.2, 7.3, 7.4**
//
// Bug_Condition_PBT: For every (route, auth_state) permutation, all expected
// [INIT] markers are present in the code paths that will execute at boot.
// On failure, the counterexample identifies the missing stage.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]
    #[test]
    fn bug_condition_all_init_markers_present_at_boot(
        route_idx in 0..ROUTE_COUNT,
        auth_idx in 0..2usize,
    ) {
        let route = route_variant_name(route_idx);
        let auth = auth_state_from_index(auth_idx);
        let expected = expected_markers_for(route, auth);
        let switch_fn = extract_switch_fn(SWITCH_FN_SOURCE);

        // Verify the route is handled in switch()
        prop_assert!(
            switch_fn.contains(&format!("Route::{route}"))
                || switch_fn.contains(route),
            "Route::{} not found in switch() — boot path broken",
            route,
        );

        // Verify each expected marker is present in source
        for marker in &expected {
            prop_assert!(
                marker_present_in_source(marker),
                "Missing [INIT] stage '{}' for route={}, auth={:?} — \
                 counterexample: (route={}, auth_state={:?}, missing_stage='{}')",
                marker, route, auth, route, auth, marker,
            );

            // Verify not debug-gated (Requirement 7.4)
            prop_assert!(
                marker_not_debug_gated(marker),
                "[INIT] stage '{}' is gated behind cfg(debug_assertions) — \
                 must be present in production builds. \
                 counterexample: (route={}, auth_state={:?}, debug_gated_stage='{}')",
                marker, route, auth, marker,
            );
        }

        // For protected routes, verify ProtectedRoute wraps the route
        let is_protected = AUTH_PROTECTED_ROUTES.contains(&route);
        if is_protected {
            let arm = format!("Route::{route}");
            if let Some(arm_start) = switch_fn.find(&arm) {
                let arm_rest = &switch_fn[arm_start..];
                let next_route = arm_rest[1..].find("Route::");
                let arm_text = match next_route {
                    Some(end) => &arm_rest[..end + 1],
                    None => arm_rest,
                };
                prop_assert!(
                    arm_text.contains("ProtectedRoute"),
                    "Route::{} must be wrapped in <ProtectedRoute> to emit auth markers — \
                     counterexample: (route={}, missing ProtectedRoute wrapper)",
                    route, route,
                );
            }
        }
    }
}
