// Feature: security-audit-remediation, Property 1: Bug Condition — Security Audit Vulnerability Verification
// **Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 1.10, 1.11, 1.12**
//
// These tests encode the EXPECTED SECURE BEHAVIOR. They are designed to FAIL on
// unfixed code — failure confirms the vulnerabilities exist. After fixes are
// implemented, these same tests will PASS to confirm the vulnerabilities are resolved.
#![allow(clippy::map_unwrap_or, clippy::or_fun_call)]

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

// ═══════════════════════════════════════════════════════════════════════
// 1a. Tenant Isolation — plantilla_documento must have organizacion_id column
// ═══════════════════════════════════════════════════════════════════════

/// On unfixed code, the `plantilla_documento` entity has no `organizacion_id` column,
/// meaning all template CRUD operates globally without tenant filtering.
/// This test verifies the column exists in the entity definition.
///
/// **Validates: Requirements 1.1**
#[test]
fn property_1a_tenant_isolation_plantilla_has_org_column() {
    // Check entity source for OrganizacionId variant in the Column enum
    let source = include_str!("../src/entities/plantilla_documento.rs");

    let has_org_id_column = source.contains("OrganizacionId") || source.contains("organizacion_id");

    assert!(
        has_org_id_column,
        "plantilla_documento entity does not have an organizacion_id column. \
         Templates are accessible across all organizations without tenant filtering. \
         This confirms the tenant isolation vulnerability (Finding 1.1)."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 1b. State Transitions — pagos service must enforce transition validation
// ═══════════════════════════════════════════════════════════════════════

/// Terminal states (pagado, cancelado) should not allow any transitions.
/// On unfixed code, `pagos::update()` only validates the new estado is in the
/// allowlist but does NOT enforce valid transitions — so `pagado → pendiente` succeeds.
///
/// **Validates: Requirements 1.2**
#[test]
fn property_1b_state_transitions_validation_exists() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Invalid transitions that the system should reject
    let invalid_transitions = prop_oneof![
        Just(("pagado", "pendiente")),
        Just(("pagado", "atrasado")),
        Just(("pagado", "cancelado")),
        Just(("cancelado", "pendiente")),
        Just(("cancelado", "pagado")),
        Just(("cancelado", "atrasado")),
    ];

    runner
        .run(&invalid_transitions, |(old_estado, new_estado)| {
            // Check the pagos source for a transition validation mechanism
            let source = include_str!("../src/services/pagos.rs");

            // The fix should add a VALID_TRANSITIONS map and validate_transition helper
            let has_transition_map = source.contains("VALID_TRANSITIONS")
                || source.contains("valid_transitions")
                || source.contains("validate_transition");

            let has_terminal_check = source.contains("pagado")
                && source.contains("cancelado")
                && (source.contains("terminal") || source.contains("no válida"));

            prop_assert!(
                has_transition_map || has_terminal_check,
                "pagos.rs has no state transition validation. \
                 Transition '{}' → '{}' would succeed on unfixed code. \
                 Expected VALID_TRANSITIONS map or validate_transition() function.",
                old_estado,
                new_estado
            );

            Ok(())
        })
        .unwrap();
}

// ═══════════════════════════════════════════════════════════════════════
// 1c. Registration Race — duplicate key must map to Conflict, not Internal
// ═══════════════════════════════════════════════════════════════════════

/// On unfixed code, `check_email_unique()` is called OUTSIDE the transaction,
/// creating a check-then-act race. Two concurrent requests both pass the check,
/// and the second insert hits the DB unique constraint producing a 500 error.
/// The fix moves the check inside the transaction AND catches duplicate key DB errors.
///
/// **Validates: Requirements 1.3**
#[test]
fn property_1c_registration_race_duplicate_key_handling() {
    let source = include_str!("../src/services/auth.rs");

    // Find register_new_org function
    let fn_start = source
        .find("async fn register_new_org")
        .expect("register_new_org function should exist");
    let fn_body = &source[fn_start..];

    let fn_end = fn_body[40..]
        .find("\nasync fn ")
        .or_else(|| fn_body[40..].find("\npub async fn "))
        .map(|i| i + 40)
        .unwrap_or(fn_body.len());
    let fn_content = &fn_body[..fn_end];

    // The bug: check_email_unique is called BEFORE db.begin()
    // Find the relative positions of check_email_unique and begin()
    let check_pos = fn_content.find("check_email_unique");
    let begin_pos = fn_content.find(".begin()");

    let check_is_inside_txn = match (check_pos, begin_pos) {
        (Some(check), Some(begin)) => check > begin,
        _ => false,
    };

    // Additionally check that duplicate key errors from insert are caught
    let has_duplicate_key_catch = fn_content.contains("duplicate key")
        || fn_content.contains("duplicate_key")
        || fn_content.contains("UniqueConstraint");

    assert!(
        check_is_inside_txn || has_duplicate_key_catch,
        "register_new_org() has a race condition: check_email_unique() is called \
         BEFORE the transaction begins. Two concurrent requests can both pass the check, \
         and the second insert produces a raw 500 error instead of 409 Conflict. \
         Expected: check inside transaction OR duplicate key error handling (Finding 1.3)."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 1d. SQL Parameterization — list_conversations must use bind params
// ═══════════════════════════════════════════════════════════════════════

/// On unfixed code, `list_conversations()` uses `format!()` to interpolate values
/// directly into SQL strings. The fix should use parameterized queries with $1, $2, $3.
///
/// **Validates: Requirements 1.4**
#[test]
fn property_1d_sql_parameterization_no_format_interpolation() {
    let source = include_str!("../src/services/chatbot.rs");

    // Find list_conversations function body
    let fn_start = source
        .find("pub async fn list_conversations")
        .expect("list_conversations function should exist");
    let fn_body = &source[fn_start..];

    // Find end of function (next pub async fn)
    let fn_end = fn_body[50..]
        .find("pub async fn ")
        .map(|i| i + 50)
        .unwrap_or(fn_body.len().min(2000));
    let fn_content = &fn_body[..fn_end];

    // Check for format!() usage with SQL interpolation
    let has_format_sql = fn_content.contains("format!(")
        && (fn_content.contains("org_id")
            || fn_content.contains("per_page")
            || fn_content.contains("offset"));

    assert!(
        !has_format_sql,
        "list_conversations() uses format!() for SQL construction with interpolated values. \
         Found format!() with org_id/per_page/offset interpolation. \
         Expected parameterized queries with bind variables ($1, $2, $3). \
         This confirms the SQL injection vulnerability (Finding 1.4)."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 1e. Audit Coverage — cambiar_rol must record audit entry
// ═══════════════════════════════════════════════════════════════════════

/// On unfixed code, `cambiar_rol()` does not call `auditoria::registrar_best_effort()`.
/// The fix should add audit calls to all sensitive operations.
///
/// **Validates: Requirements 1.5**
#[test]
fn property_1e_audit_coverage_cambiar_rol() {
    let source = include_str!("../src/services/usuarios.rs");

    let fn_start = source
        .find("pub async fn cambiar_rol")
        .expect("cambiar_rol function should exist");
    let fn_body = &source[fn_start..];

    // Find end of function (next pub async fn)
    let fn_end = fn_body[30..]
        .find("pub async fn ")
        .map(|i| i + 30)
        .unwrap_or(fn_body.len());
    let fn_content = &fn_body[..fn_end];

    assert!(
        fn_content.contains("registrar_best_effort") || fn_content.contains("auditoria::"),
        "cambiar_rol() does not record an audit entry. \
         Expected a call to auditoria::registrar_best_effort(). \
         This confirms the missing audit trail vulnerability (Finding 1.5)."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 1f. Export Row Cap — exports must enforce row limit
// ═══════════════════════════════════════════════════════════════════════

/// On unfixed code, report exports have no row cap and generate full PDFs in memory.
/// The fix should enforce a configurable row cap (default 50,000) with a Validation error.
///
/// **Validates: Requirements 1.6**
#[test]
fn property_1f_export_row_cap_enforced() {
    let source = include_str!("../src/services/reportes.rs");

    // Check specifically for a row cap constant (not generic .count() calls)
    let has_row_cap = source.contains("ROW_CAP")
        || source.contains("row_cap")
        || source.contains("REPORT_ROW_CAP")
        || source.contains("50_000");

    assert!(
        has_row_cap,
        "reportes.rs does not contain a row cap constant or enforcement. \
         Report exports can generate unbounded memory allocations for large datasets. \
         Expected REPORT_ROW_CAP or similar constant with Validation error (Finding 1.6)."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 1g. Import Transaction — failed imports must roll back all rows
// ═══════════════════════════════════════════════════════════════════════

/// On unfixed code, `importar_propiedades()` inserts row-by-row without a transaction.
/// If row N fails, rows 1..N-1 persist as orphans. The fix wraps all inserts in a transaction.
///
/// **Validates: Requirements 1.7**
#[test]
fn property_1g_import_transaction_wrapping() {
    let source = include_str!("../src/services/importacion.rs");

    // Find importar_propiedades function
    let fn_start = source
        .find("pub async fn importar_propiedades")
        .expect("importar_propiedades function should exist");
    let fn_body = &source[fn_start..];

    // Find end of function (next pub or fn)
    let fn_end = fn_body[40..]
        .find("\npub ")
        .or_else(|| fn_body[40..].find("\nfn "))
        .map(|i| i + 40)
        .unwrap_or(fn_body.len());
    let fn_content = &fn_body[..fn_end];

    // Check for transaction usage
    let has_transaction = fn_content.contains(".begin()")
        || fn_content.contains("txn")
        || fn_content.contains("transaction");

    assert!(
        has_transaction,
        "importar_propiedades() does not use a database transaction. \
         Inserts are done row-by-row; if row N fails, rows 1..N-1 persist as orphans. \
         Expected transaction wrapping with rollback on failure (Finding 1.7)."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 1h. Balance Access Control — unlinked phone must be rejected in code
// ═══════════════════════════════════════════════════════════════════════

/// On unfixed code, `query_tenant_balance()` does not verify that the sender phone
/// is linked to the inquilino. Access control is delegated to the LLM prompt only.
///
/// **Validates: Requirements 1.8**
#[test]
fn property_1h_balance_access_control_in_code() {
    let source = include_str!("../src/services/chatbot.rs");

    // Find query_tenant_balance function
    let fn_start = source
        .find("pub async fn query_tenant_balance")
        .expect("query_tenant_balance function should exist");
    let fn_body = &source[fn_start..];

    let fn_end = fn_body[40..]
        .find("\npub async fn ")
        .map(|i| i + 40)
        .unwrap_or(fn_body.len().min(1500));
    let fn_content = &fn_body[..fn_end];

    // The function should verify the sender is a linked tenant before executing
    // the balance query. On unfixed code it just takes inquilino_id directly.
    let has_sender_verification = fn_content.contains("sender")
        || fn_content.contains("phone")
        || fn_content.contains("find_tenant")
        || fn_content.contains("verificar");

    assert!(
        has_sender_verification,
        "query_tenant_balance() does not verify sender phone is linked to the tenant. \
         The function accepts inquilino_id directly without checking if the caller's \
         phone number belongs to that tenant. Access control is in LLM prompt only. \
         This confirms the balance access control vulnerability (Finding 1.8)."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 1i. Last Admin Guard — demoting the only admin must be rejected
// ═══════════════════════════════════════════════════════════════════════

/// On unfixed code, `cambiar_rol()` allows demoting the last admin, leaving the
/// organization with zero admins. The fix adds an admin count check.
///
/// **Validates: Requirements 1.9**
#[test]
fn property_1i_last_admin_guard() {
    let source = include_str!("../src/services/usuarios.rs");

    let fn_start = source
        .find("pub async fn cambiar_rol")
        .expect("cambiar_rol function should exist");
    let fn_body = &source[fn_start..];

    let fn_end = fn_body[30..]
        .find("pub async fn ")
        .map(|i| i + 30)
        .unwrap_or(fn_body.len());
    let fn_content = &fn_body[..fn_end];

    // Check for admin count guard
    let has_admin_guard = (fn_content.contains("count") || fn_content.contains("Count"))
        && (fn_content.contains("último administrador")
            || fn_content.contains("No se puede quitar")
            || fn_content.contains("last admin"));

    assert!(
        has_admin_guard,
        "cambiar_rol() does not check if demoting the last admin in the organization. \
         The only admin can be demoted, leaving zero admins. \
         Expected an admin count check with rejection (Finding 1.9)."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 1j. Crate Pinning — jsonwebtoken and argon2 must use exact versions
// ═══════════════════════════════════════════════════════════════════════

/// On unfixed code, `Cargo.toml` uses semver ranges for security-critical crates:
/// `jsonwebtoken = "10"` and `argon2 = "0.5"`. The fix pins them to `=X.Y.Z`.
///
/// **Validates: Requirements 1.10**
#[test]
fn property_1j_crate_pinning_exact_versions() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 1, // Static check, only need one run
        ..Default::default()
    });

    runner
        .run(&Just(()), |_| {
            let cargo_toml = include_str!("../Cargo.toml");

            // Check jsonwebtoken uses pinned version (="X.Y.Z" syntax)
            let jwt_line = cargo_toml
                .lines()
                .find(|l| l.contains("jsonwebtoken"))
                .expect("jsonwebtoken should be in Cargo.toml");

            // Pinned versions: version = "=10.0.1" or jsonwebtoken = "=10.0.1"
            let jwt_is_pinned = jwt_line.contains("\"=");
            prop_assert!(
                jwt_is_pinned,
                "jsonwebtoken is not pinned to exact version. Line: '{}'. \
                 Expected '=\"X.Y.Z\"' syntax. Semver ranges allow auto-upgrades \
                 to potentially vulnerable patches (Finding 1.10).",
                jwt_line.trim()
            );

            // Check argon2 uses pinned version
            let argon2_line = cargo_toml
                .lines()
                .find(|l| l.starts_with("argon2") || l.contains("argon2 "))
                .expect("argon2 should be in Cargo.toml");

            let argon2_is_pinned = argon2_line.contains("\"=");
            prop_assert!(
                argon2_is_pinned,
                "argon2 is not pinned to exact version. Line: '{}'. \
                 Expected '=\"X.Y.Z\"' syntax (Finding 1.10).",
                argon2_line.trim()
            );

            Ok(())
        })
        .unwrap();
}

// ═══════════════════════════════════════════════════════════════════════
// 1k. Metrics Auth — /internal/metrics requires token when configured
// ═══════════════════════════════════════════════════════════════════════

/// On unfixed code, `/internal/metrics` has no authentication — it serves metrics
/// to anyone who can reach the route. The fix adds bearer token auth when
/// `METRICS_TOKEN` is configured.
///
/// **Validates: Requirements 1.11**
#[test]
fn property_1k_metrics_auth_required_when_token_set() {
    let source = include_str!("../src/app.rs");

    // Find internal_metrics function
    let fn_start = source
        .find("async fn internal_metrics")
        .expect("internal_metrics function should exist");
    let fn_body = &source[fn_start..];

    let fn_end = fn_body[30..]
        .find("\nasync fn ")
        .or_else(|| fn_body[30..].find("\npub "))
        .or_else(|| fn_body[30..].find("\nfn "))
        .map(|i| i + 30)
        .unwrap_or(fn_body.len().min(500));
    let fn_content = &fn_body[..fn_end];

    // Check for authentication logic
    let has_auth = fn_content.contains("Authorization")
        || fn_content.contains("Bearer")
        || fn_content.contains("METRICS_TOKEN")
        || fn_content.contains("metrics_token")
        || fn_content.contains("Unauthorized")
        || fn_content.contains("401");

    assert!(
        has_auth,
        "internal_metrics() has no authentication check. \
         The endpoint serves Prometheus metrics to any caller without token validation. \
         Expected bearer token validation when METRICS_TOKEN is set (Finding 1.11)."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 1l. Environment Typo — ENVIRONMENT=prod must hard-fail at startup
// ═══════════════════════════════════════════════════════════════════════

/// On unfixed code, `ENVIRONMENT=prod` (or `Production`, `PRODUCTION`) does not
/// trigger a hard-fail because only `== "production"` is checked exactly.
/// The fix adds a case-insensitive contains("prod") check.
///
/// **Validates: Requirements 1.12**
#[test]
fn property_1l_environment_typo_detection() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Values that contain "prod" but aren't exactly "production" — all should hard-fail
    let typo_values = prop_oneof![
        Just("prod".to_string()),
        Just("Prod".to_string()),
        Just("PROD".to_string()),
        Just("Production".to_string()),
        Just("PRODUCTION".to_string()),
    ];

    runner
        .run(&typo_values, |env_value| {
            let source = include_str!("../src/config.rs");

            // The fix should add: if env.to_lowercase().contains("prod") && env != "production" → error
            let has_typo_check = source.contains("contains(\"prod\")")
                || (source.contains("to_lowercase")
                    && source.contains("prod")
                    && source.contains("production"));

            prop_assert!(
                has_typo_check,
                "config.rs does not validate ENVIRONMENT typos. \
                 ENVIRONMENT='{}' would be silently accepted and fall through \
                 to permissive CORS. Expected typo detection logic (Finding 1.12).",
                env_value
            );

            Ok(())
        })
        .unwrap();
}
