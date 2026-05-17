#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::case_sensitive_file_extension_comparisons
)]

use proptest::prelude::*;

use super::dispatch_formatter;

// -- Strategies --

/// Generate a random directory prefix (may or may not include baileys-service/ or android/).
fn directory_prefix() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("baileys-service/".to_string()),
        Just("baileys-service/src/".to_string()),
        Just("baileys-service/components/".to_string()),
        Just("android/".to_string()),
        Just("android/app/src/main/".to_string()),
        Just("android/core/data/".to_string()),
        Just("src/".to_string()),
        Just("backend/src/".to_string()),
        Just("frontend/src/".to_string()),
        Just("infra/".to_string()),
        Just("scripts/".to_string()),
        Just(String::new()),
    ]
}

/// Generate a random file stem (the name without extension).
fn file_stem() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{1,15}"
}

/// Generate a file extension from the set we care about.
fn file_extension() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(".rs".to_string()),
        Just(".ts".to_string()),
        Just(".tsx".to_string()),
        Just(".kt".to_string()),
        Just(".py".to_string()),
        Just(".md".to_string()),
        Just(".yml".to_string()),
        Just(".json".to_string()),
        Just(".yaml".to_string()),
        Just(".toml".to_string()),
    ]
}

/// Generate a complete file path by combining prefix + stem + extension.
fn file_path() -> impl Strategy<Value = String> {
    (directory_prefix(), file_stem(), file_extension())
        .prop_map(|(prefix, stem, ext)| format!("{prefix}{stem}{ext}"))
}

// -- Property Tests --

// Feature: autofix-harness-polish, Property 1: Hook dispatch produces correct formatter command
proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    /// **Validates: Requirements 3.2, 3.3, 3.4**
    #[test]
    fn dispatch_formatter_returns_correct_command(path in file_path()) {
        let result = dispatch_formatter(&path);

        if path.ends_with(".rs") {
            // Rust files always get cargo fmt
            prop_assert_eq!(result, Some(format!("cargo fmt -- {path}")));
        } else if (path.ends_with(".ts") || path.ends_with(".tsx"))
            && path.contains("baileys-service/")
        {
            // TS/TSX under baileys-service get eslint --fix
            prop_assert_eq!(result, Some(format!("npx eslint --fix {path}")));
        } else if path.ends_with(".kt") && path.contains("android/") {
            // Kotlin under android gets gradlew spotlessApply
            prop_assert_eq!(result, Some("./gradlew spotlessApply".to_string()));
        } else if (path.ends_with(".ts") || path.ends_with(".tsx"))
            && !path.contains("baileys-service/")
        {
            // TS/TSX NOT under baileys-service -> None
            prop_assert_eq!(result, None);
        } else if path.ends_with(".kt") && !path.contains("android/") {
            // Kotlin NOT under android -> None
            prop_assert_eq!(result, None);
        } else {
            // All other extensions (.py, .md, .yml, .json, .yaml, .toml) -> None
            prop_assert_eq!(result, None);
        }
    }
}
