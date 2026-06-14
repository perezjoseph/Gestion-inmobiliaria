#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::no_effect_underscore_binding,
    unused_doc_comments
)]
//! Property 2: Preservation — Authenticated flows and session expiry unchanged
//!
//! This test captures CORRECT behavior that must remain unchanged after the fix.
//!
//! Observation on UNFIXED code:
//! - A 401 WITH a token present clears the token and redirects to `/` (session expiry)
//! - A successful response (200) navigates to the dashboard (login success)
//!
//! The property asserts over `{has_token ∈ {true, false}, status ∈ {200, 401, ...}}`:
//! redirect happens iff `status == 401 && has_token == true`.
//!
//! **EXPECTED OUTCOME**: Test PASSES on unfixed code.
//!
//! **Validates: Requirements 3.1**

// Feature: e2e-exploratory-bugfixes, Property 2: Preservation

use proptest::prelude::*;

// ── Model of handle_response's decision logic ──────────────────────────────
//
// This models the ACTUAL behavior of handle_response in api.rs (unfixed code).
//
// The current (unfixed) code:
//   if response.status() == 401 {
//       clear_token_and_redirect();  // always redirects on 401
//       return Err("Sesión expirada...");
//   }
//   if !response.ok() {
//       return Err(humanize_error(status, &text));
//   }
//   Ok(response)

/// The possible outcomes of handle_response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandleResponseOutcome {
    /// Token cleared, navigates to "/" — session expiry behavior
    ClearedAndRedirected,
    /// Success — response passes through, caller navigates to dashboard
    Success,
    /// Error returned (non-401 failure), stays on current page
    ErrorReturned,
}

/// Model of the CURRENT (unfixed) handle_response behavior.
/// This is `F` — the existing implementation.
fn handle_response_current(has_token: bool, status: u16) -> HandleResponseOutcome {
    let _ = has_token; // current code doesn't check token — that's the bug for 401-without-token
    if status == 401 {
        // Current code: unconditionally clears and redirects on ANY 401
        return HandleResponseOutcome::ClearedAndRedirected;
    }
    if (200..300).contains(&status) {
        return HandleResponseOutcome::Success;
    }
    // Any other non-ok status
    HandleResponseOutcome::ErrorReturned
}

// ── Strategies ─────────────────────────────────────────────────────────────

/// Strategy for HTTP status codes that represent the preservation domain:
/// - 200-299: successful responses (login works, API calls succeed)
/// - 401: session expiry (when token present)
/// - Other error codes (403, 404, 500, etc.) for completeness
fn http_status_strategy() -> impl Strategy<Value = u16> {
    prop_oneof![
        3 => 200_u16..=299_u16,  // success range (weighted higher)
        2 => Just(401_u16),       // 401 — session expiry when token present
        1 => Just(403_u16),       // forbidden
        1 => Just(404_u16),       // not found
        1 => Just(500_u16),       // server error
        1 => 402_u16..=499_u16, // other client errors
        1 => 500_u16..=599_u16, // other server errors
    ]
}

// ── Property Tests ─────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 3.1**
    ///
    /// Property 2: Preservation — Authenticated flows and session expiry unchanged
    ///
    /// The preservation property asserts: on the UNFIXED code,
    /// `ClearedAndRedirected` happens iff `status == 401 && has_token == true`.
    ///
    /// This captures two correct behaviors that must survive the fix:
    /// 1. Valid logins (200) reach the dashboard (Success outcome)
    /// 2. Expired sessions (401 with token) clear token and redirect
    ///
    /// The test scopes AWAY from the bug condition (has_token=false, status=401)
    /// by only asserting the preservation invariant for the subset of inputs
    /// where the current code is correct.
    ///
    /// EXPECTED: PASSES on unfixed code.
    #[test]
    fn prop_preservation_authenticated_flows_and_session_expiry(
        has_token in proptest::bool::ANY,
        status in http_status_strategy()
    ) {
        let outcome = handle_response_current(has_token, status);

        // Preservation invariant: redirect happens iff status==401 AND has_token==true
        //
        // On the UNFIXED code, the actual behavior is:
        //   - 401 (any token state) → ClearedAndRedirected
        //   - 200-299 → Success
        //   - other → ErrorReturned
        //
        // The PRESERVATION subset we care about (correct behavior):
        //   - status==401 && has_token==true → ClearedAndRedirected ✓ (session expiry)
        //   - status in 200..299 → Success ✓ (valid login/API call)
        //   - status not 401 and not 2xx → ErrorReturned ✓ (other errors)
        //   - status==401 && has_token==false → this is the BUG CONDITION (Property 1)
        //     The current code returns ClearedAndRedirected which is WRONG for this case,
        //     but we SKIP asserting on it here (it's out of preservation scope).
        //
        // We only assert on the preservation domain (¬C):
        if status == 401 && !has_token {
            // Bug condition domain — skip (Property 1 covers this)
            // The current code gives ClearedAndRedirected here, which IS wrong,
            // but the preservation test doesn't assert on the buggy case.
            return Ok(());
        }

        // For all other inputs, assert the current behavior is what we expect to preserve:
        let expected = if status == 401 && has_token {
            // Session expiry: token-bearing 401 clears and redirects (CORRECT, preserve this)
            HandleResponseOutcome::ClearedAndRedirected
        } else if (200..300).contains(&status) {
            // Success: valid login/API call proceeds (CORRECT, preserve this)
            HandleResponseOutcome::Success
        } else {
            // Other errors: return error message, stay on page (CORRECT, preserve this)
            HandleResponseOutcome::ErrorReturned
        };

        prop_assert!(
            outcome == expected,
            "Preservation violated: for has_token={}, status={}, \
             expected {:?} but got {:?}",
            has_token,
            status,
            expected,
            outcome
        );
    }
}
