#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    unused_doc_comments
)]
//! Landing page route mapping tests
//!
//! Verifies the route restructuring: "/" → Landing, "/login" → Login, "/registro" → Registro
//!
//! NOTE: The auth redirect test (authenticated user at "/" redirects to "/dashboard") cannot
//! be easily tested without a WASM runtime and browser APIs (localStorage, navigator).
//! This is documented as a manual test case:
//!
//! ## Manual Test: Auth Redirect
//! 1. Open the app in a browser with a valid JWT in localStorage
//! 2. Navigate to "/"
//! 3. Verify the browser redirects to "/dashboard"
//! 4. Clear localStorage and navigate to "/"
//! 5. Verify the Landing page renders (no redirect)

use realestate_frontend::app::Route;
use yew_router::Routable;

/// **Validates: Requirements 1.1**
///
/// The root path "/" must resolve to Route::Landing (the new landing page).
#[test]
fn test_root_resolves_to_landing() {
    assert_eq!(Route::recognize("/"), Some(Route::Landing));
}

/// **Validates: Requirements 1.2**
///
/// The "/login" path must resolve to Route::Login (moved from "/" to "/login").
#[test]
fn test_login_resolves_to_login() {
    assert_eq!(Route::recognize("/login"), Some(Route::Login));
}

/// **Validates: Requirements 1.4**
///
/// The "/registro" path must still resolve to Route::Registro (unchanged).
#[test]
fn test_registro_still_resolves() {
    assert_eq!(Route::recognize("/registro"), Some(Route::Registro));
}

/// Verify that "/dashboard" continues to resolve to Route::Dashboard.
#[test]
fn test_dashboard_still_resolves() {
    assert_eq!(Route::recognize("/dashboard"), Some(Route::Dashboard));
}

/// **Validates: Requirements 1.1**
///
/// Route::Landing.to_path() must return "/" (round-trip consistency).
#[test]
fn test_landing_route_to_path() {
    assert_eq!(Route::Landing.to_path(), "/");
}

/// **Validates: Requirements 1.2**
///
/// Route::Login.to_path() must return "/login" (round-trip consistency).
#[test]
fn test_login_route_to_path() {
    assert_eq!(Route::Login.to_path(), "/login");
}
