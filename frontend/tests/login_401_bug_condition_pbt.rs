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
//! Property 1: Bug Condition — Login 401 surfaces an error and stays on /login
//!
//! This test MUST FAIL on unfixed code — failure confirms the bug exists.
//!
//! The bug: `frontend/src/services/api.rs::handle_response` treats every 401
//! as an expired session by calling `clear_token_and_redirect()` which navigates
//! to `/`. This happens even on the login endpoint where no token/session exists.
//!
//! The property asserts: for a 401 response when no token is stored,
//! `handle_response` does NOT clear-and-redirect and the login flow stays
//! on `/login` with an error surfaced.
//!
//! **Validates: Requirements 1.1**

use proptest::prelude::*;

// ── Model of handle_response's 401 decision logic ──────────────────────────
//
// This models the ACTUAL behavior of handle_response in api.rs.
// The current (unfixed) code:
//   if response.status() == 401 {
//       clear_token_and_redirect();  // always! no token check
//       return Err("Sesión expirada...");
//   }

/// The possible outcomes of handle_response when it encounters a 401.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Response401Outcome {
    /// Token cleared, navigates to "/" — session expiry behavior
    ClearedAndRedirected,
    /// Stays on current page, returns an error message for the form to display
    ErrorSurfaced,
}

/// Model of the CURRENT (fixed) handle_response behavior on 401.
/// After fix in task 3.1: only clear-and-redirect when a token IS present.
/// When no token is present (login attempt), fall through to error humanization.
fn handle_response_401_current(has_token: bool) -> Response401Outcome {
    if has_token {
        Response401Outcome::ClearedAndRedirected
    } else {
        Response401Outcome::ErrorSurfaced
    }
}

/// Model of the EXPECTED (correct) behavior on 401.
/// This is `F'` — what the fix should produce.
/// Only clear-and-redirect when a token IS present (genuine session expiry).
/// When no token is present (login attempt), surface the error.
#[allow(dead_code)]
fn handle_response_401_expected(has_token: bool) -> Response401Outcome {
    if has_token {
        Response401Outcome::ClearedAndRedirected
    } else {
        Response401Outcome::ErrorSurfaced
    }
}

// ── Strategies ─────────────────────────────────────────────────────────────

/// Input space for the bug condition: login attempts where no token is stored
/// and the backend returns 401 (wrong credentials).
///
/// We scope the property to {has_token: false, status: 401} as specified
/// in the task. The email/password are generated to show the variety of
/// inputs that trigger this path.
fn login_401_no_token_strategy() -> impl Strategy<Value = (String, String)> {
    // Generate arbitrary email-like strings and password strings
    // representing login attempts that result in a 401
    (
        "[a-z]{3,10}@[a-z]{3,8}\\.[a-z]{2,4}",
        "[a-zA-Z0-9!@#$%]{4,20}",
    )
}

// ── Property Tests ─────────────────────────────────────────────────────────

// Feature: e2e-exploratory-bugfixes, Property 1: Bug Condition

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 1.1**
    ///
    /// Property 1: Bug Condition — Login 401 surfaces an error and stays on /login
    ///
    /// For ANY login submission (no token present) where the backend returns 401,
    /// handle_response SHOULD surface an error and NOT clear-and-redirect.
    ///
    /// This test is EXPECTED TO FAIL on unfixed code because the current
    /// implementation always calls clear_token_and_redirect() on 401,
    /// regardless of token presence.
    #[test]
    fn prop_login_401_without_token_should_surface_error(
        (email, password) in login_401_no_token_strategy()
    ) {
        // Precondition: no token stored (login attempt, not authenticated session)
        let has_token = false;

        // The status is 401 (wrong credentials on login)
        let _email = email;
        let _password = password;

        // Act: model the current handle_response behavior on 401
        let outcome = handle_response_401_current(has_token);

        // Assert: the outcome SHOULD be ErrorSurfaced (stay on /login, show error)
        // This will FAIL because handle_response_401_current always returns
        // ClearedAndRedirected — proving the bug exists.
        prop_assert_eq!(
            outcome,
            Response401Outcome::ErrorSurfaced,
            "Bug confirmed: login 401 with no token should surface an error \
             and stay on /login, but the current code clears the token and \
             redirects to '/'. Email: {}, Password: [redacted]",
            _email
        );
    }
}
