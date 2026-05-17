//! Architecture fitness tests — computational sensors for layer boundary enforcement.
//!
//! These tests scan source files to verify the layered architecture:
//!   handlers → services → entities
//!
//! Handlers must NOT import directly from `entities::` (bypass services layer).
//! Handlers must NOT use `sea_orm` directly (database is the services' concern).
//! Services must NOT import from `handlers::` (no upward dependency).

use std::fs;
use std::path::Path;

/// Scans a directory for `.rs` files (non-recursive into subdirs, just the module files).
fn read_rust_files_in(dir: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                if let Ok(content) = fs::read_to_string(&path) {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    files.push((name, content));
                }
            }
        }
    }
    files
}

/// Returns lines that contain a `use` statement matching the given pattern.
/// Ignores comments and lines inside `#[cfg(test)]` blocks.
fn find_use_statements(content: &str, pattern: &str) -> Vec<(usize, String)> {
    let mut violations = Vec::new();
    let mut in_test_cfg = false;
    let mut brace_depth: u32 = 0;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Track #[cfg(test)] module boundaries
        if trimmed == "#[cfg(test)]" {
            in_test_cfg = true;
            brace_depth = 0;
            continue;
        }
        if in_test_cfg {
            brace_depth = brace_depth
                .saturating_add(
                    u32::try_from(line.chars().filter(|&c| c == '{').count()).unwrap_or(0),
                )
                .saturating_sub(
                    u32::try_from(line.chars().filter(|&c| c == '}').count()).unwrap_or(0),
                );
            continue;
        }

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }

        // Check for use statements with the pattern
        if trimmed.starts_with("use ") && trimmed.contains(pattern) {
            violations.push((line_num + 1, line.to_string()));
        }
    }
    violations
}

#[test]
fn handlers_must_not_import_entities_directly() {
    let handlers_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/handlers");
    let files = read_rust_files_in(&handlers_dir);

    let mut all_violations = Vec::new();
    for (filename, content) in &files {
        if filename == "mod.rs" {
            continue;
        }
        let violations = find_use_statements(content, "crate::entities::");
        for (line, text) in &violations {
            let trimmed = text.trim();
            all_violations.push(format!("  handlers/{filename}:{line}: {trimmed}"));
        }
    }

    assert!(
        all_violations.is_empty(),
        "Architecture violation: handlers must not import directly from entities.\n\
         Handlers should use services as intermediaries.\n\
         Violations found:\n{}",
        all_violations.join("\n")
    );
}

#[test]
fn handlers_must_not_use_sea_orm_directly() {
    let handlers_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/handlers");
    let files = read_rust_files_in(&handlers_dir);

    let mut all_violations = Vec::new();
    for (filename, content) in &files {
        if filename == "mod.rs" {
            continue;
        }
        // Allow sea_orm::DatabaseConnection (needed for handler params) and TransactionTrait
        let violations = find_use_statements(content, "sea_orm::");
        for (line, text) in &violations {
            let trimmed = text.trim();
            // These are acceptable — handlers receive DbConn and may start transactions
            if trimmed.contains("DatabaseConnection")
                || trimmed.contains("TransactionTrait")
                || trimmed.contains("DbErr")
            {
                continue;
            }
            all_violations.push(format!("  handlers/{filename}:{line}: {trimmed}"));
        }
    }

    assert!(
        all_violations.is_empty(),
        "Architecture violation: handlers must not use sea_orm query/entity operations directly.\n\
         Only DatabaseConnection, TransactionTrait, and DbErr are allowed.\n\
         Move query logic to services.\n\
         Violations found:\n{}",
        all_violations.join("\n")
    );
}

#[test]
fn services_must_not_import_handlers() {
    let services_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services");
    let files = read_rust_files_in(&services_dir);

    let mut all_violations = Vec::new();
    for (filename, content) in &files {
        if filename == "mod.rs" {
            continue;
        }
        let violations = find_use_statements(content, "crate::handlers::");
        for (line, text) in &violations {
            let trimmed = text.trim();
            all_violations.push(format!("  services/{filename}:{line}: {trimmed}"));
        }
    }

    assert!(
        all_violations.is_empty(),
        "Architecture violation: services must not import from handlers (no upward dependency).\n\
         Violations found:\n{}",
        all_violations.join("\n")
    );
}

#[test]
fn entities_must_not_import_services_or_handlers() {
    let entities_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/entities");
    let files = read_rust_files_in(&entities_dir);

    let mut all_violations = Vec::new();
    for (filename, content) in &files {
        if filename == "mod.rs" || filename == "prelude.rs" {
            continue;
        }
        let svc_violations = find_use_statements(content, "crate::services::");
        let handler_violations = find_use_statements(content, "crate::handlers::");
        for (line, text) in svc_violations.iter().chain(handler_violations.iter()) {
            let trimmed = text.trim();
            all_violations.push(format!("  entities/{filename}:{line}: {trimmed}"));
        }
    }

    assert!(
        all_violations.is_empty(),
        "Architecture violation: entities must not import from services or handlers.\n\
         Entities are the innermost layer.\n\
         Violations found:\n{}",
        all_violations.join("\n")
    );
}

#[test]
fn models_must_not_import_sea_orm_entities() {
    let models_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/models");
    let files = read_rust_files_in(&models_dir);

    let mut all_violations = Vec::new();
    for (filename, content) in &files {
        if filename == "mod.rs" {
            continue;
        }
        let violations = find_use_statements(content, "crate::entities::");
        for (line, text) in &violations {
            let trimmed = text.trim();
            all_violations.push(format!("  models/{filename}:{line}: {trimmed}"));
        }
    }

    assert!(
        all_violations.is_empty(),
        "Architecture violation: models (DTOs) must not import from entities.\n\
         DTOs are the API boundary; entities are the DB boundary.\n\
         Conversion between them belongs in services.\n\
         Violations found:\n{}",
        all_violations.join("\n")
    );
}

#[test]
fn middleware_must_not_import_services() {
    let middleware_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/middleware");
    let files = read_rust_files_in(&middleware_dir);

    let mut all_violations = Vec::new();
    for (filename, content) in &files {
        if filename == "mod.rs" {
            continue;
        }
        let violations = find_use_statements(content, "crate::services::");
        for (line, text) in &violations {
            let trimmed = text.trim();
            // Allow auth service import in auth middleware
            if filename == "auth.rs" && trimmed.contains("auth::") {
                continue;
            }
            all_violations.push(format!("  middleware/{filename}:{line}: {trimmed}"));
        }
    }

    assert!(
        all_violations.is_empty(),
        "Architecture violation: middleware must not import from services (except auth middleware → auth service).\n\
         Middleware handles cross-cutting concerns, not business logic.\n\
         Violations found:\n{}",
        all_violations.join("\n")
    );
}
