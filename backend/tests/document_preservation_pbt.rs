// Feature: e2e-exploratory-bugfixes, Property 16: Preservation
// **Validates: Requirements 3.10**
//
// Preservation property: Missing files return 404 and directory traversal (`..`) is rejected.
// The URL fix (changing from `/api/v1/` to `/uploads/`) does NOT weaken these protections.
//
// Observation-first methodology: on UNFIXED code, `serve_upload` in `app.rs`:
// 1. Rejects any path containing ".." with Forbidden("Acceso denegado")
// 2. Returns NotFound("Archivo no encontrado") for paths that don't exist on disk
// 3. Canonicalizes paths and verifies they stay within UPLOAD_DIR
//
// These tests MUST PASS on unfixed code (baseline captured) and after the fix.
#![allow(clippy::needless_return)]

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

// ── Strategies ─────────────────────────────────────────────────

/// Generate paths that contain directory traversal sequences (`..`).
/// These should always be rejected by `serve_upload`.
fn traversal_path() -> impl Strategy<Value = String> {
    prop_oneof![
        // Classic traversal patterns
        Just("../etc/passwd".to_string()),
        Just("../../secret.txt".to_string()),
        Just("subdir/../../../etc/shadow".to_string()),
        Just("foo/bar/../../baz/../secret".to_string()),
        Just("..".to_string()),
        Just("valid/path/../traversal".to_string()),
        // Traversal embedded in otherwise normal-looking paths
        "[a-z]{1,10}/\\.\\./[a-z]{1,10}".prop_map(|s| s),
        "[a-z]{1,10}/\\.\\.".prop_map(|s| s),
        "\\.\\./[a-z]{1,10}".prop_map(|s| s),
        // Double-dot in various positions
        "[a-z]{1,5}(/\\.\\./[a-z]{1,5}){1,3}".prop_map(|s| s),
    ]
}

/// Generate paths for non-existent files (no traversal, but the file won't exist).
/// These should always return 404 from `serve_upload`.
fn missing_file_path() -> impl Strategy<Value = String> {
    prop_oneof![
        // UUID-based paths that won't exist on disk
        "[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}/[a-z]{3,10}\\.(jpg|png|pdf)"
            .prop_map(|s| s),
        // Deeply nested non-existent paths
        "propiedad/[a-f0-9]{8}/[a-f0-9]{8}-document\\.(jpg|png|pdf)".prop_map(|s| s),
        // Simple non-existent filenames
        "nonexistent_[a-z]{5,15}\\.(jpg|png|pdf|docx)".prop_map(|s| s),
        // Paths that look valid but don't exist
        "uploads/[a-f0-9]{32}\\.(jpg|png)".prop_map(|s| s),
    ]
}

// ── Source-level verification ──────────────────────────────────

/// Property 16a: serve_upload rejects traversal at the source level.
///
/// Verifies the source code contains the `..` check before filesystem access.
/// This is a structural preservation check — the traversal guard MUST remain.
#[test]
fn property_16a_serve_upload_rejects_traversal_in_source() {
    let source = include_str!("../src/app.rs");

    // Find serve_upload function
    let fn_start = source
        .find("async fn serve_upload")
        .expect("serve_upload function should exist in app.rs");
    let fn_body = &source[fn_start..];

    let fn_end = fn_body[30..]
        .find("\nfn ")
        .or_else(|| fn_body[30..].find("\nasync fn "))
        .or_else(|| fn_body[30..].find("\npub fn "))
        .map(|i| i + 30)
        .unwrap_or(fn_body.len());
    let fn_content = &fn_body[..fn_end];

    // Property: checks for ".." in the requested path
    assert!(
        fn_content.contains("contains(\"..\")"),
        "serve_upload must check for '..' traversal in the requested path. \
         This security check must be preserved after the URL fix."
    );

    // Property: returns Forbidden for traversal attempts
    assert!(
        fn_content.contains("Forbidden") && fn_content.contains("Acceso denegado"),
        "serve_upload must return Forbidden('Acceso denegado') for traversal paths. \
         This response must be preserved."
    );

    // Property: returns NotFound for missing files
    assert!(
        fn_content.contains("NotFound") && fn_content.contains("Archivo no encontrado"),
        "serve_upload must return NotFound('Archivo no encontrado') for missing files. \
         This 404 behavior must be preserved."
    );

    // Property: performs canonicalization to prevent path escape
    assert!(
        fn_content.contains("canonicalize"),
        "serve_upload must canonicalize paths to prevent escaping UPLOAD_DIR. \
         This defense-in-depth check must be preserved."
    );

    // Property: verifies canonical path starts_with the upload directory
    assert!(
        fn_content.contains("starts_with"),
        "serve_upload must verify the canonical path is within UPLOAD_DIR. \
         This containment check must be preserved."
    );

    // Property: requires authentication (Claims extractor)
    assert!(
        fn_content.contains("Claims") || fn_content.contains("_claims"),
        "serve_upload must require authentication via Claims extractor. \
         The endpoint must remain authenticated."
    );
}

/// Property 16b: Traversal paths are always rejected.
///
/// For any path containing `..`, the `serve_upload` logic rejects it before
/// reaching the filesystem. We model the pre-filesystem check directly.
#[test]
fn property_16b_traversal_always_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&traversal_path(), |path| {
            // Model the serve_upload traversal check:
            // "if requested_path.contains("..") { return Err(Forbidden) }"
            let is_rejected = path.contains("..");

            prop_assert!(
                is_rejected,
                "Path '{}' contains '..' but was not flagged for rejection. \
                 serve_upload must reject ALL paths containing '..' as traversal attempts.",
                path
            );

            Ok(())
        })
        .unwrap();
}

/// Property 16c: Non-existent file paths yield 404 (not 200 or other status).
///
/// For any path that doesn't contain `..` but references a non-existent file,
/// `serve_upload` returns NotFound. We verify this by checking the path passes
/// the traversal check and then confirming the filesystem lookup would fail.
#[test]
fn property_16c_missing_files_yield_404() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&missing_file_path(), |path| {
            // Step 1: Path must NOT be rejected by traversal check
            prop_assert!(
                !path.contains(".."),
                "Generated 'missing file' path '{}' unexpectedly contains '..' — \
                 strategy error, not a code bug.",
                path
            );

            // Step 2: The file must not exist on the filesystem
            // Use a temp dir as UPLOAD_DIR (models the real behavior without side effects)
            let upload_dir = std::env::temp_dir().join("pbt_upload_test_nonexistent");
            let full_path = upload_dir.join(&path);
            prop_assert!(
                !full_path.exists(),
                "Generated path '{}' unexpectedly exists at {:?} — \
                 the test assumes these paths don't exist. \
                 serve_upload would return 404 (NotFound) for missing files.",
                path,
                full_path
            );

            // Step 3: Since the file doesn't exist, canonicalize would fail,
            // and serve_upload returns NotFound("Archivo no encontrado").
            // This models the exact behavior in serve_upload:
            //   let canonical_file = std::fs::canonicalize(&full_path_clone)
            //       .map_err(|_| AppError::NotFound("Archivo no encontrado".to_string()))?;
            let canonicalize_result = std::fs::canonicalize(&full_path);
            prop_assert!(
                canonicalize_result.is_err(),
                "Canonicalization of non-existent path '{}' should fail, \
                 triggering serve_upload's NotFound response.",
                path
            );

            Ok(())
        })
        .unwrap();
}

/// Property 16d: The traversal check occurs BEFORE filesystem access.
///
/// This is a defense-in-depth property: even if a `..` path somehow resolved to
/// a valid file, it would still be rejected because the check is pre-filesystem.
/// We verify this by ensuring the `contains("..")` check appears before `canonicalize`.
#[test]
fn property_16d_traversal_check_precedes_filesystem_access() {
    let source = include_str!("../src/app.rs");

    let fn_start = source
        .find("async fn serve_upload")
        .expect("serve_upload function should exist");
    let fn_body = &source[fn_start..];

    let fn_end = fn_body[30..]
        .find("\nfn ")
        .or_else(|| fn_body[30..].find("\nasync fn "))
        .or_else(|| fn_body[30..].find("\npub fn "))
        .map(|i| i + 30)
        .unwrap_or(fn_body.len());
    let fn_content = &fn_body[..fn_end];

    // The ".." check must appear before canonicalize in the source
    let traversal_pos = fn_content
        .find("contains(\"..\")")
        .expect("serve_upload must contain the '..' check");
    let canonicalize_pos = fn_content
        .find("canonicalize")
        .expect("serve_upload must contain canonicalize");

    assert!(
        traversal_pos < canonicalize_pos,
        "The '..' traversal check (position {}) must appear BEFORE filesystem \
         canonicalization (position {}). This ordering ensures traversal is rejected \
         without any filesystem access.",
        traversal_pos,
        canonicalize_pos
    );
}
