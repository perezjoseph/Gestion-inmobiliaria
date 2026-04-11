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
        12 => "NotFound",
        _ => unreachable!(),
    }
}

const ROUTE_COUNT: usize = 13;

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
                switch_fn.contains(&format!("Route::{}", variant)),
                "switch() must handle Route::{}",
                variant,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Task 2: Preservation Property Tests
// ---------------------------------------------------------------------------
//
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

const PROTECTED_ROUTES: [&str; 10] = [
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
];

// **Validates: Requirements 3.1**
//
// Property 2a: Routing Preservation — switch() handles all Route variants
// and wraps protected routes in ProtectedRoute while rendering public
// routes directly.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(13))]
    #[test]
    fn preservation_routing_all_variants_handled(route_idx in 0..ROUTE_COUNT) {
        let variant = route_variant_name(route_idx);
        let switch_fn = extract_switch_fn(SWITCH_FN_SOURCE);

        prop_assert!(
            switch_fn.contains(&format!("Route::{}", variant)),
            "switch() must handle Route::{} — routing preservation violated",
            variant,
        );

        if PUBLIC_ROUTES.contains(&variant) {
            prop_assert!(
                !switch_fn.contains(&format!("Route::{} => html! {{ <ProtectedRoute>", variant)),
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
            let arm = format!("Route::{}", variant);
            let arm_start = switch_fn
                .find(&arm)
                .unwrap_or_else(|| panic!("switch() must contain {}", arm));
            let arm_rest = &switch_fn[arm_start..];
            let arm_end = arm_rest
                .find('}')
                .unwrap_or_else(|| panic!("Could not find end of arm for {}", arm));
            let arm_text = &arm_rest[..arm_end + 1];
            assert!(
                !arm_text.contains("ProtectedRoute"),
                "Route::{} must render directly without ProtectedRoute wrapper",
                variant,
            );
        }
    }

    // **Validates: Requirements 3.1**
    #[test]
    fn protected_routes_wrapped_in_protected_route() {
        let switch_fn = extract_switch_fn(SWITCH_FN_SOURCE);
        for variant in &PROTECTED_ROUTES {
            let arm = format!("Route::{}", variant);
            let arm_start = switch_fn
                .find(&arm)
                .unwrap_or_else(|| panic!("switch() must contain {}", arm));
            let arm_rest = &switch_fn[arm_start..];
            let next_route = arm_rest[1..].find("Route::");
            let arm_text = match next_route {
                Some(end) => &arm_rest[..end + 1],
                None => arm_rest,
            };
            assert!(
                arm_text.contains("ProtectedRoute"),
                "Route::{} must be wrapped in <ProtectedRoute>",
                variant,
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
