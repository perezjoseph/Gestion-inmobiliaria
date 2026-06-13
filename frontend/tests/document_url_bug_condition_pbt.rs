#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    unused_doc_comments
)]
//! Property 15: Bug Condition — Stored document image is served
//!
//! **GOAL**: Show that the gallery builds an unroutable `/api/v1/{file_path}` URL because
//! `trim_end_matches("/api")` is a no-op on `"/api/v1"` (the string ends with `"/v1"`,
//! not `"/api"`).
//!
//! The correct route for serving stored documents is `GET /uploads/{path:.*}`.
//! The bug causes the URL to be `/api/v1/{file_path}` which has no matching route → 404.
//!
//! **CRITICAL**: This test MUST FAIL on unfixed code — failure confirms the bug exists.
//!
//! **Validates: Requirements 1.8**

use proptest::prelude::*;

// Feature: e2e-exploratory-bugfixes, Property 15: Bug Condition

// ── Constants matching the source ──────────────────────────────────────────

/// The BASE_URL constant from `frontend/src/services/api.rs`.
const BASE_URL: &str = "/api/v1";

// ── URL builder logic (mirrors document_gallery.rs::DocumentCard) ──────────

/// Reproduces the CURRENT (buggy) URL builder from `DocumentCard`:
/// ```rust
/// let file_url = format!("{}/{}", BASE_URL.trim_end_matches("/api"), doc.file_path);
/// ```
fn build_document_url_current(file_path: &str) -> String {
    format!("{}/{}", BASE_URL.trim_end_matches("/api"), file_path)
}

// ── Strategies ─────────────────────────────────────────────────────────────

/// Generates realistic `entity_id` values (UUID-like strings).
fn entity_id_strategy() -> impl Strategy<Value = String> {
    "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
}

/// Generates realistic filenames that match real document uploads.
/// Includes edge cases: spaces, parentheses, unicode, long names.
fn filename_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple filenames
        "[a-zA-Z0-9_-]{3,20}\\.(jpg|png|pdf|docx)",
        // Filenames with spaces (like the real counterexample)
        Just("Eg3tKKlWsAA0v_w (1).jpg".to_string()),
        Just("contrato firmado (2).pdf".to_string()),
        Just("foto casa.png".to_string()),
        // UUID-prefixed filenames (as stored by the upload service)
        "[0-9a-f]{8}-[a-zA-Z0-9_-]{3,15}\\.(jpg|png|pdf)",
    ]
}

/// Generates realistic `file_path` values as stored by `services::documentos::upload`.
/// Format: `propiedad/{entity_id}/{uuid}-{filename}`
fn file_path_strategy() -> impl Strategy<Value = String> {
    (entity_id_strategy(), filename_strategy())
        .prop_map(|(entity_id, filename)| format!("propiedad/{entity_id}/{filename}"))
}

// ── Property Tests ─────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 1.8**
    ///
    /// Property 15: Bug Condition — For any stored document with a valid `file_path`,
    /// the URL builder in `DocumentCard` MUST produce a URL starting with `/uploads/`
    /// (the route that actually serves files).
    ///
    /// This test FAILS on unfixed code because `trim_end_matches("/api")` is a no-op
    /// on `"/api/v1"`, producing `/api/v1/{file_path}` instead of `/uploads/{file_path}`.
    #[test]
    fn prop_document_url_resolves_to_uploads_route(
        file_path in file_path_strategy()
    ) {
        let built_url = build_document_url_current(&file_path);

        // The URL MUST start with "/uploads/" to match the file-serving route
        prop_assert!(
            built_url.starts_with("/uploads/"),
            "Bug 8 confirmed: document URL builder produces '{}' which does NOT start \
             with '/uploads/'. The trim_end_matches(\"/api\") on BASE_URL=\"/api/v1\" is \
             a no-op (string ends with \"/v1\", not \"/api\"). \
             Expected: '/uploads/{}', Got: '{}'",
            built_url,
            file_path,
            built_url
        );
    }

    /// **Validates: Requirements 1.8**
    ///
    /// Additional property: the built URL must NOT start with `/api/v1/` because
    /// there is no route matching `GET /api/v1/propiedad/{id}/{filename}`.
    #[test]
    fn prop_document_url_does_not_use_api_v1_prefix(
        file_path in file_path_strategy()
    ) {
        let built_url = build_document_url_current(&file_path);

        // The URL MUST NOT use the API prefix — there is no matching route
        prop_assert!(
            !built_url.starts_with("/api/v1/"),
            "Bug 8 confirmed: document URL builder produces '{}' which starts with \
             '/api/v1/' — a prefix with no file-serving route. The real route is \
             GET /uploads/{{path:.*}}. Root cause: trim_end_matches(\"/api\") on \
             \"/api/v1\" is a no-op.",
            built_url
        );
    }
}
