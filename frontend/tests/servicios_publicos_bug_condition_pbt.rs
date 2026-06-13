#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    unused_doc_comments
)]
//! Property 13: Bug Condition — Servicios Públicos calls an existing property endpoint
//!
//! **GOAL**: Guard that no caller of the non-existent `/propiedades/todas` route exists.
//!
//! The bug: the deployed build requested `GET /api/v1/propiedades/todas` (non-existent route),
//! causing the property dropdown to be empty. The current source is already fixed to call
//! `/propiedades?perPage=200`.
//!
//! This test acts as a **regression guard** — it PASSES on the current source (the bug is
//! already resolved) and would FAIL if someone reintroduced the `/propiedades/todas` caller.
//!
//! **Validates: Requirements 1.7**

use proptest::prelude::*;
use std::fs;
use std::path::Path;

// ── Source-code assertion helpers ──────────────────────────────────────────

/// Reads a source file and returns its content.
fn read_source_file(relative_path: &str) -> String {
    // Try multiple base paths to handle different working directories
    let candidates = [
        Path::new(relative_path).to_path_buf(),
        Path::new("..").join(relative_path),
        Path::new("../..").join(relative_path),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return fs::read_to_string(candidate)
                .unwrap_or_else(|e| panic!("Failed to read {}: {e}", candidate.display()));
        }
    }

    panic!(
        "Could not find source file '{}' from any base path. Tried: {:?}",
        relative_path,
        candidates
            .iter()
            .map(|c| c.display().to_string())
            .collect::<Vec<_>>()
    );
}

/// Recursively collects all `.rs` files under a directory.
fn collect_rs_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_rs_files(&path));
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files
}

/// Finds the frontend source root by checking candidate paths.
fn find_frontend_src() -> std::path::PathBuf {
    let candidates = [
        Path::new("frontend/src").to_path_buf(),
        Path::new("../frontend/src").to_path_buf(),
        Path::new("src").to_path_buf(),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    panic!("Could not locate frontend/src from any base path");
}

// ── Tests ──────────────────────────────────────────────────────────────────

// Feature: e2e-exploratory-bugfixes, Property 13: Bug Condition

/// **Validates: Requirements 1.7**
///
/// Assert that `servicios_publicos.rs` requests the existing `/propiedades?perPage=200`
/// endpoint and does NOT reference the non-existent `/propiedades/todas`.
#[test]
fn test_servicios_publicos_calls_existing_propiedades_endpoint() {
    let source = read_source_file("frontend/src/pages/servicios_publicos.rs");

    // The source MUST contain the correct endpoint call
    assert!(
        source.contains("/propiedades?perPage=200"),
        "servicios_publicos.rs MUST call '/propiedades?perPage=200' (the existing route). \
         Bug 7 regression: the deployed build previously called '/propiedades/todas' which \
         does not exist."
    );

    // The source MUST NOT contain the non-existent route
    assert!(
        !source.contains("/propiedades/todas"),
        "servicios_publicos.rs MUST NOT reference '/propiedades/todas' (non-existent route). \
         This would reintroduce Bug 7."
    );
}

/// **Validates: Requirements 1.7**
///
/// Repo-wide regression guard: no `.rs` file in the frontend source tree should
/// reference `/propiedades/todas`. This catches reintroduction in any file, not
/// just `servicios_publicos.rs`.
#[test]
fn test_no_frontend_file_references_propiedades_todas() {
    let src_root = find_frontend_src();
    let rs_files = collect_rs_files(&src_root);

    assert!(
        !rs_files.is_empty(),
        "Should find at least one .rs file under frontend/src"
    );

    let mut violating_files = Vec::new();

    for file_path in &rs_files {
        if let Ok(content) = fs::read_to_string(file_path) {
            if content.contains("/propiedades/todas") {
                violating_files.push(file_path.display().to_string());
            }
        }
    }

    assert!(
        violating_files.is_empty(),
        "Bug 7 regression: the following files reference the non-existent \
         '/propiedades/todas' route and must be repointed to '/propiedades?perPage=200': \
         {:?}",
        violating_files
    );
}

// ── Property-based regression guard ────────────────────────────────────────

/// Strategy that generates variants of the non-existent route pattern.
/// This ensures we catch various forms of the bad endpoint.
fn bad_route_variants() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("/propiedades/todas".to_string()),
        Just("propiedades/todas".to_string()),
        Just("/api/v1/propiedades/todas".to_string()),
        Just("api/v1/propiedades/todas".to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 1.7**
    ///
    /// Property 13: Bug Condition — no variant of the non-existent `/propiedades/todas`
    /// route appears in `servicios_publicos.rs`.
    ///
    /// This property test checks multiple forms of the bad route to ensure none
    /// are present in the source. Acts as a regression guard since the bug is
    /// already fixed in the current source.
    #[test]
    fn prop_servicios_publicos_does_not_call_propiedades_todas(
        bad_route in bad_route_variants()
    ) {
        let source = read_source_file("frontend/src/pages/servicios_publicos.rs");

        prop_assert!(
            !source.contains(&bad_route),
            "Regression guard failed: servicios_publicos.rs contains '{}' which is a \
             non-existent route. The property-list request must use '/propiedades?perPage=200'.",
            bad_route
        );
    }
}
