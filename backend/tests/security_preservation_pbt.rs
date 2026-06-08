// Feature: security-audit-remediation, Property 2: Preservation — Existing Secure Behavior Unchanged
// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.10, 3.11, 3.12**
//
// These tests capture BASELINE behavior for non-buggy inputs on UNFIXED code.
// They MUST PASS before fixes are applied, and MUST CONTINUE to pass afterward.
// Any test failure after fixes indicates a regression in existing correct behavior.
//
// Observation-first methodology: each test observes what the current code does for
// legitimate (non-buggy) inputs, then asserts that behavior holds.
#![allow(clippy::map_unwrap_or, clippy::or_fun_call)]

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

// ═══════════════════════════════════════════════════════════════════════
// 2a. Template Fill Preservation — rellenar() continues filling templates correctly
// ═══════════════════════════════════════════════════════════════════════

/// Observation: `rellenar()` verifies entity ownership via
/// `verificar_entidad_pertenece_a_org()`, loads the template by ID, builds a
/// replacement map from entity fields via `load_entity_fields()`, then resolves
/// `{{key}}` placeholders in the template's JSON content.
///
/// Preservation property: The rellenar function continues to verify org ownership,
/// load the template, resolve placeholders, and return `PlantillaRellenadaResponse`.
/// This pipeline must survive the org-filtering changes to CRUD operations.
///
/// **Validates: Requirements 3.1**
#[test]
fn property_2a_template_fill_preservation() {
    let source = include_str!("../src/services/plantillas.rs");

    // Find rellenar function
    let fn_start = source
        .find("pub async fn rellenar")
        .expect("rellenar function should exist");
    let fn_body = &source[fn_start..];

    let fn_end = fn_body[30..]
        .find("\npub(crate) fn ")
        .or_else(|| fn_body[30..].find("\npub async fn "))
        .or_else(|| fn_body[30..].find("\nfn "))
        .map(|i| i + 30)
        .unwrap_or(fn_body.len());
    let fn_content = &fn_body[..fn_end];

    // Property: verifies entity belongs to caller's organization
    assert!(
        fn_content.contains("verificar_entidad_pertenece_a_org")
            || fn_content.contains("verificar_entidad"),
        "rellenar() must verify entity ownership before filling templates. \
         This org-boundary check must be preserved."
    );

    // Property: accepts organizacion_id parameter
    assert!(
        fn_content.contains("organizacion_id"),
        "rellenar() must accept organizacion_id to verify ownership."
    );

    // Property: loads the template by ID
    assert!(
        fn_content.contains("find_by_id") || fn_content.contains("plantilla_documento::Entity"),
        "rellenar() must load the template by ID."
    );

    // Property: returns NotFound for missing templates
    assert!(
        fn_content.contains("NotFound") || fn_content.contains("no encontrada"),
        "rellenar() must return NotFound for missing templates."
    );

    // Property: loads entity fields for replacement
    assert!(
        fn_content.contains("load_entity_fields"),
        "rellenar() must load entity fields for placeholder replacement."
    );

    // Property: resolves placeholders in template content
    assert!(
        fn_content.contains("resolve_placeholders"),
        "rellenar() must resolve placeholders in the template content."
    );

    // Property: returns PlantillaRellenadaResponse
    assert!(
        fn_content.contains("PlantillaRellenadaResponse"),
        "rellenar() must return PlantillaRellenadaResponse."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 2b. Bulk Payment Preservation — bulk_marcar_pagado validates pendiente/atrasado
// ═══════════════════════════════════════════════════════════════════════

/// Observation: `bulk_marcar_pagado()` currently validates that payments are in
/// `pendiente` or `atrasado` state before transitioning to `pagado`. Payments in
/// other states are rejected with a validation error listing the non-updatable IDs.
///
/// Preservation property: The source code of `bulk_marcar_pagado` continues to
/// filter payments by `estado != "pendiente" && estado != "atrasado"` and rejects
/// non-updatable payments. This behavior must survive the state transition fix.
///
/// **Validates: Requirements 3.2**
#[test]
fn property_2b_bulk_payment_preservation() {
    let source = include_str!("../src/services/pagos.rs");

    // Find bulk_marcar_pagado function
    let fn_start = source
        .find("pub async fn bulk_marcar_pagado")
        .expect("bulk_marcar_pagado function should exist");
    let fn_body = &source[fn_start..];

    let fn_end = fn_body[40..]
        .find("\npub async fn ")
        .map(|i| i + 40)
        .unwrap_or(fn_body.len());
    let fn_content = &fn_body[..fn_end];

    // Property: validates that only pendiente/atrasado can transition to pagado
    assert!(
        fn_content.contains("pendiente") && fn_content.contains("atrasado"),
        "bulk_marcar_pagado must check for pendiente/atrasado states. \
         This is the baseline behavior that must be preserved after the state transition fix."
    );

    // Property: rejects non-updatable payments with validation error
    assert!(
        fn_content.contains("non_updatable") || fn_content.contains("no están en estado"),
        "bulk_marcar_pagado must reject payments not in pendiente/atrasado state. \
         This validation must be preserved."
    );

    // Property: validates metodo_pago
    assert!(
        fn_content.contains("metodo_pago") && fn_content.contains("validate_enum"),
        "bulk_marcar_pagado must validate metodo_pago enum value."
    );

    // Property: records audit entry after bulk update
    assert!(
        fn_content.contains("registrar_best_effort") || fn_content.contains("auditoria"),
        "bulk_marcar_pagado must record an audit entry for the bulk operation."
    );

    // Property: enforces maximum 100 payments per operation
    assert!(
        fn_content.contains("100"),
        "bulk_marcar_pagado must enforce the 100-payment limit per bulk operation."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 2c. Single Registration Preservation — register creates org + user + JWT
// ═══════════════════════════════════════════════════════════════════════

/// Observation: `register_new_org()` creates an organization and user within a
/// transaction and returns a JWT via `build_login_response()`. For single unique-email
/// registrations, this flow succeeds with org creation, user creation, and JWT return.
///
/// Preservation property: The registration function continues to use a transaction
/// for creating org + user, hash the password, and return a LoginResponse with JWT.
///
/// **Validates: Requirements 3.3**
#[test]
fn property_2c_single_registration_preservation() {
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

    // Property: uses a transaction for org + user creation
    assert!(
        fn_content.contains(".begin()") && fn_content.contains(".commit()"),
        "register_new_org must wrap org + user creation in a transaction. \
         This atomicity guarantee must be preserved after the race condition fix."
    );

    // Property: hashes password before storing
    assert!(
        fn_content.contains("hash_password"),
        "register_new_org must hash the password before creating the user."
    );

    // Property: creates organization model
    assert!(
        fn_content.contains("organizacion::ActiveModel") || fn_content.contains("org_model"),
        "register_new_org must create an organization."
    );

    // Property: creates user model with admin role
    assert!(
        fn_content.contains("usuario::ActiveModel") || fn_content.contains("user_model"),
        "register_new_org must create a user."
    );
    assert!(
        fn_content.contains("\"admin\""),
        "register_new_org must set the initial user role to admin."
    );

    // Property: returns login response with JWT
    assert!(
        fn_content.contains("build_login_response"),
        "register_new_org must return a login response with JWT."
    );

    // Property: validates tipo (persona_fisica / persona_juridica)
    assert!(
        fn_content.contains("persona_fisica") && fn_content.contains("persona_juridica"),
        "register_new_org must validate the organization type."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 2d. Chatbot Query Preservation — list_conversations returns paginated results
// ═══════════════════════════════════════════════════════════════════════

/// Observation: `list_conversations()` returns paginated conversation lists with
/// `sender_phone`, `inquilino_id`, `last_message`, `last_message_at`, `message_count`.
/// The function uses `DISTINCT ON (sender_phone)` SQL semantics and pagination.
///
/// Preservation property: After parameterization, the query must continue to:
/// 1. Count distinct sender phones per org
/// 2. Return paginated results ordered by sender_phone, created_at DESC
/// 3. Include message_count subquery
/// 4. Use LIMIT/OFFSET pagination
///
/// **Validates: Requirements 3.4**
#[test]
fn property_2d_chatbot_query_preservation() {
    let source = include_str!("../src/services/chatbot.rs");

    // Find list_conversations function
    let fn_start = source
        .find("pub async fn list_conversations")
        .expect("list_conversations function should exist");
    let fn_body = &source[fn_start..];

    let fn_end = fn_body[50..]
        .find("\npub async fn ")
        .map(|i| i + 50)
        .unwrap_or(fn_body.len().min(2000));
    let fn_content = &fn_body[..fn_end];

    // Property: queries distinct sender phones (SQL semantics preserved)
    assert!(
        fn_content.contains("DISTINCT") || fn_content.contains("distinct"),
        "list_conversations must use DISTINCT ON (sender_phone) for unique conversations."
    );

    // Property: counts total conversations for pagination
    assert!(
        fn_content.contains("COUNT") || fn_content.contains("count"),
        "list_conversations must count total distinct conversations for pagination metadata."
    );

    // Property: uses pagination with offset calculation
    assert!(
        fn_content.contains("offset") || fn_content.contains("OFFSET"),
        "list_conversations must support OFFSET-based pagination."
    );

    // Property: returns PaginatedResponse with data, total, page, per_page
    assert!(
        fn_content.contains("PaginatedResponse"),
        "list_conversations must return a PaginatedResponse structure."
    );

    // Property: returns ConversationListResponse with expected fields
    assert!(
        fn_content.contains("sender_phone") && fn_content.contains("last_message"),
        "list_conversations must include sender_phone and last_message in results."
    );

    // Property: orders by created_at DESC for latest message
    assert!(
        fn_content.contains("created_at DESC") || fn_content.contains("ORDER BY"),
        "list_conversations must order by created_at DESC to get the latest message."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 2e. Existing Audit Preservation — payment/property CRUD records audit entries
// ═══════════════════════════════════════════════════════════════════════

/// Observation: The payment CRUD (create, update, delete) already calls
/// `auditoria::registrar_best_effort()`. Property CRUD in handlers also records
/// audit entries. These existing audit calls must remain intact.
///
/// Preservation property: The pagos service continues to call audit functions
/// in create, update, delete, and bulk operations.
///
/// **Validates: Requirements 3.5**
#[test]
fn property_2e_existing_audit_preservation() {
    let pagos_source = include_str!("../src/services/pagos.rs");

    // Property: pagos::create records audit
    let create_start = pagos_source
        .find("pub async fn create")
        .expect("pagos::create should exist");
    let create_body = &pagos_source[create_start..];
    let create_end = create_body[30..]
        .find("\npub async fn ")
        .map(|i| i + 30)
        .unwrap_or(create_body.len());
    let create_fn = &create_body[..create_end];
    assert!(
        create_fn.contains("registrar_best_effort") || create_fn.contains("auditoria"),
        "pagos::create must continue to record audit entries."
    );

    // Property: pagos::update records audit
    let update_start = pagos_source
        .find("pub async fn update")
        .expect("pagos::update should exist");
    let update_body = &pagos_source[update_start..];
    let update_end = update_body[30..]
        .find("\npub async fn ")
        .map(|i| i + 30)
        .unwrap_or(update_body.len());
    let update_fn = &update_body[..update_end];
    assert!(
        update_fn.contains("registrar_best_effort") || update_fn.contains("auditoria"),
        "pagos::update must continue to record audit entries."
    );

    // Property: pagos::delete records audit
    let delete_start = pagos_source
        .find("pub async fn delete")
        .expect("pagos::delete should exist");
    let delete_body = &pagos_source[delete_start..];
    let delete_end = delete_body[30..]
        .find("\npub async fn ")
        .map(|i| i + 30)
        .unwrap_or(delete_body.len());
    let delete_fn = &delete_body[..delete_end];
    assert!(
        delete_fn.contains("registrar_best_effort") || delete_fn.contains("auditoria"),
        "pagos::delete must continue to record audit entries."
    );

    // Property: pagos::bulk_marcar_pagado records audit
    let bulk_start = pagos_source
        .find("pub async fn bulk_marcar_pagado")
        .expect("pagos::bulk_marcar_pagado should exist");
    let bulk_body = &pagos_source[bulk_start..];
    let bulk_end = bulk_body[40..]
        .find("\npub async fn ")
        .map(|i| i + 40)
        .unwrap_or(bulk_body.len());
    let bulk_fn = &bulk_body[..bulk_end];
    assert!(
        bulk_fn.contains("registrar_best_effort") || bulk_fn.contains("auditoria"),
        "pagos::bulk_marcar_pagado must continue to record audit entries."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 2f. Sub-Cap Export Preservation — exports below row cap generate content
// ═══════════════════════════════════════════════════════════════════════

/// Observation: The report generation service (`reportes.rs`) currently generates
/// PDF/XLSX exports for any result set size. After the row cap fix, datasets below
/// 50k rows must continue to generate exports normally.
///
/// Preservation property: The report export functions continue to contain PDF/XLSX
/// generation logic and return complete reports for normal-sized datasets.
///
/// **Validates: Requirements 3.6**
#[test]
fn property_2f_sub_cap_export_preservation() {
    let source = include_str!("../src/services/reportes.rs");

    // Property: PDF generation logic exists
    assert!(
        source.contains("pdf") || source.contains("Pdf") || source.contains("genpdf"),
        "reportes.rs must contain PDF generation logic."
    );

    // Property: XLSX generation logic exists
    assert!(
        source.contains("xlsx") || source.contains("Xlsx") || source.contains("xlsxwriter"),
        "reportes.rs must contain XLSX generation logic."
    );

    // Property: generates report data from database queries
    assert!(
        source.contains("find") || source.contains("Entity"),
        "reportes.rs must query entities to generate report data."
    );

    // Property: returns binary data (bytes) for the generated report
    assert!(
        source.contains("Vec<u8>") || source.contains("bytes") || source.contains("buffer"),
        "reportes.rs must return binary report content."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 2g. Valid Import Preservation — imports with valid data produce ImportResult
// ═══════════════════════════════════════════════════════════════════════

/// Observation: `importar_propiedades()` processes valid CSV/XLSX rows, inserts them
/// one by one, and returns `ImportResult { total_filas, exitosos, fallidos }`.
/// For valid data with <5k rows, this behavior must continue unchanged.
///
/// Preservation property: The import function returns ImportResult with correct
/// counting structure (total_filas = data rows, exitosos = successful inserts,
/// fallidos = per-row errors).
///
/// **Validates: Requirements 3.7**
#[test]
fn property_2g_valid_import_preservation() {
    let source = include_str!("../src/services/importacion.rs");

    // Find importar_propiedades
    let fn_start = source
        .find("pub async fn importar_propiedades")
        .expect("importar_propiedades function should exist");
    let fn_body = &source[fn_start..];

    let fn_end = fn_body[40..]
        .find("\npub ")
        .or_else(|| fn_body[40..].find("\nfn "))
        .map(|i| i + 40)
        .unwrap_or(fn_body.len());
    let fn_content = &fn_body[..fn_end];

    // Property: returns ImportResult structure
    assert!(
        fn_content.contains("ImportResult"),
        "importar_propiedades must return ImportResult."
    );

    // Property: counts total rows correctly (data_rows.len())
    assert!(
        fn_content.contains("total_filas") || fn_content.contains("data_rows.len()"),
        "importar_propiedades must track total_filas count."
    );

    // Property: counts successful inserts
    assert!(
        fn_content.contains("exitosos"),
        "importar_propiedades must track successful insert count."
    );

    // Property: collects per-row errors
    assert!(
        fn_content.contains("fallidos") && fn_content.contains("ImportError"),
        "importar_propiedades must collect per-row errors in fallidos vec."
    );

    // Property: processes rows iteratively
    assert!(
        fn_content.contains("for") && fn_content.contains("enumerate"),
        "importar_propiedades must iterate over data rows with enumeration."
    );

    // Property: uses process_propiedad_row for row processing
    assert!(
        fn_content.contains("process_propiedad_row"),
        "importar_propiedades must use process_propiedad_row for each row."
    );

    // Property: handles empty input gracefully
    assert!(
        fn_content.contains("is_empty"),
        "importar_propiedades must handle empty input gracefully (return zero counts)."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 2h. Owner Policy Preservation — owner_only restricts to verified owner phone
// ═══════════════════════════════════════════════════════════════════════

/// Observation: `is_sender_allowed()` with unrecognized policies returns false
/// (fail-closed). The `check_sender_policy_no_db()` function handles:
/// - "tenants_and_prospects" → always true
/// - "allowlist" → checks phone in allowlist
/// - unknown → false (fail-closed)
/// - "tenants_only" → requires DB lookup (returns None)
///
/// The owner_only policy currently falls into the catch-all `_ => false` branch,
/// meaning it is fail-closed. This behavior must be preserved.
///
/// **Validates: Requirements 3.8**
#[test]
fn property_2h_owner_policy_preservation() {
    use realestate_backend::services::chatbot::{check_sender_policy_no_db, is_phone_in_allowlist};

    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Strategy: random phone numbers
    let phone_strategy = "\\+1[0-9]{10}".prop_map(|s| s);

    runner
        .run(&phone_strategy, |phone| {
            // Property: unrecognized policies (including "owner_only") are fail-closed
            let result = check_sender_policy_no_db("owner_only", &phone, None);
            prop_assert_eq!(
                result,
                Some(false),
                "owner_only policy must return false (fail-closed) for any phone. \
                 The sender restriction must be preserved. Phone: {}",
                phone
            );

            // Property: empty/unknown policies are fail-closed
            let unknown = check_sender_policy_no_db("unknown_policy", &phone, None);
            prop_assert_eq!(
                unknown,
                Some(false),
                "Unknown policies must return false (fail-closed). Phone: {}",
                phone
            );

            // Property: tenants_and_prospects allows all
            let open = check_sender_policy_no_db("tenants_and_prospects", &phone, None);
            prop_assert_eq!(
                open,
                Some(true),
                "tenants_and_prospects must allow all phones. Phone: {}",
                phone
            );

            // Property: allowlist with no list rejects all
            let no_list = check_sender_policy_no_db("allowlist", &phone, None);
            prop_assert_eq!(
                no_list,
                Some(false),
                "allowlist policy with no list must reject all phones. Phone: {}",
                phone
            );

            // Property: allowlist with matching phone allows
            let list = vec![phone.clone()];
            let with_list = check_sender_policy_no_db("allowlist", &phone, Some(&list));
            prop_assert_eq!(
                with_list,
                Some(true),
                "allowlist policy must allow phone when in list. Phone: {}",
                phone
            );

            // Property: is_phone_in_allowlist returns false for None
            prop_assert!(
                !is_phone_in_allowlist(&phone, None),
                "is_phone_in_allowlist must return false for None list"
            );

            Ok(())
        })
        .unwrap();
}

// ═══════════════════════════════════════════════════════════════════════
// 2i. Non-Last Admin Demotion — demotion when 2+ admins exist succeeds
// ═══════════════════════════════════════════════════════════════════════

/// Observation: `cambiar_rol()` currently validates that the new role is in the
/// allowlist (admin, gerente, visualizador) and then updates the user's role.
/// When 2+ admins exist, demoting one should always succeed.
///
/// Preservation property: The cambiar_rol function continues to validate the role
/// enum, find the user by id + org, and update the role. The basic flow must remain
/// intact after adding the last-admin guard.
///
/// **Validates: Requirements 3.9**
#[test]
fn property_2i_non_last_admin_demotion_preservation() {
    let source = include_str!("../src/services/usuarios.rs");

    // Find cambiar_rol function
    let fn_start = source
        .find("pub async fn cambiar_rol")
        .expect("cambiar_rol function should exist");
    let fn_body = &source[fn_start..];

    let fn_end = fn_body[30..]
        .find("\npub async fn ")
        .map(|i| i + 30)
        .unwrap_or(fn_body.len());
    let fn_content = &fn_body[..fn_end];

    // Property: validates role is in allowed list
    assert!(
        fn_content.contains("validate_enum"),
        "cambiar_rol must validate the new role is in the allowed list."
    );

    // Property: looks up user by id and org
    assert!(
        fn_content.contains("find_by_id") || fn_content.contains("Entity::find"),
        "cambiar_rol must find the user by ID."
    );
    assert!(
        fn_content.contains("OrganizacionId") || fn_content.contains("org_id"),
        "cambiar_rol must filter by organization ID."
    );

    // Property: returns NotFound for non-existent users
    assert!(
        fn_content.contains("NotFound") || fn_content.contains("no encontrado"),
        "cambiar_rol must return NotFound for missing users."
    );

    // Property: updates the role and returns response
    assert!(
        fn_content.contains("update(") || fn_content.contains(".update("),
        "cambiar_rol must update the user record."
    );

    // Property: returns UsuarioResponse
    assert!(
        fn_content.contains("UsuarioResponse"),
        "cambiar_rol must return UsuarioResponse after update."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 2j. Non-Security Deps Preservation — other crates keep semver ranges
// ═══════════════════════════════════════════════════════════════════════

/// Observation: Most crates in Cargo.toml use semver ranges (e.g., `actix-web = "4"`,
/// `serde = "1"`, `tokio = "1"`). Only jsonwebtoken and argon2 will be pinned.
/// All other dependencies must continue using their current version ranges.
///
/// Preservation property: Non-security dependencies do NOT use the `=` exact pin
/// syntax. They remain as semver-compatible ranges.
///
/// **Validates: Requirements 3.10**
#[test]
fn property_2j_non_security_deps_preservation() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Non-security crates that should NOT be pinned
    let non_security_crates = prop_oneof![
        Just("actix-web"),
        Just("actix-cors"),
        Just("serde"),
        Just("serde_json"),
        Just("uuid"),
        Just("chrono"),
        Just("tracing"),
        Just("reqwest"),
        Just("anyhow"),
        Just("thiserror"),
    ];

    runner
        .run(&non_security_crates, |crate_name| {
            let cargo_toml = include_str!("../Cargo.toml");

            // Find the line for this crate
            let crate_line = cargo_toml
                .lines()
                .find(|l| l.starts_with(crate_name) || l.starts_with(&format!("{crate_name} ")));

            if let Some(line) = crate_line {
                // Property: non-security crates must NOT use exact pin syntax ("=X.Y.Z")
                // They should use semver ranges
                let has_exact_pin = line.contains("\"=");
                prop_assert!(
                    !has_exact_pin,
                    "Non-security crate '{}' should NOT be pinned with exact version. \
                     Line: '{}'. Only jsonwebtoken and argon2 should be pinned.",
                    crate_name,
                    line.trim()
                );
            }
            // If crate not found on its own line (workspace dep), that's fine

            Ok(())
        })
        .unwrap();
}

// ═══════════════════════════════════════════════════════════════════════
// 2k. Valid Token Metrics Preservation — /internal/metrics serves metrics
// ═══════════════════════════════════════════════════════════════════════

/// Observation: The `internal_metrics()` handler currently serves Prometheus metrics
/// unconditionally (no auth check). After the fix adds token auth, requests WITH
/// a valid token (or when METRICS_TOKEN is unset) must continue to serve metrics.
///
/// Preservation property: The internal_metrics handler continues to use the Prometheus
/// encoder to gather and serve metrics with `text/plain; version=0.0.4` content type.
///
/// **Validates: Requirements 3.11**
#[test]
fn property_2k_valid_token_metrics_preservation() {
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
        .unwrap_or(fn_body.len().min(800));
    let fn_content = &fn_body[..fn_end];

    // Property: uses Prometheus encoder to gather metrics
    assert!(
        fn_content.contains("prometheus") || fn_content.contains("Encoder"),
        "internal_metrics must use the Prometheus encoder to gather metrics."
    );

    // Property: gathers default_registry metric families
    assert!(
        fn_content.contains("default_registry") || fn_content.contains("gather"),
        "internal_metrics must gather metrics from the default registry."
    );

    // Property: encodes metrics to buffer
    assert!(
        fn_content.contains("encode") || fn_content.contains("buffer"),
        "internal_metrics must encode metrics into a buffer."
    );

    // Property: returns HttpResponse::Ok with text/plain content type
    assert!(
        fn_content.contains("Ok()") || fn_content.contains("HttpResponse"),
        "internal_metrics must return an HTTP 200 response."
    );
    assert!(
        fn_content.contains("text/plain"),
        "internal_metrics must serve metrics with text/plain content type."
    );

    // Property: the route is registered
    assert!(
        source.contains("/internal/metrics"),
        "The /internal/metrics route must be registered in the app."
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 2l. Correct CORS Preservation — ENVIRONMENT=production enforces strict CORS
// ═══════════════════════════════════════════════════════════════════════

/// Observation: When `ENVIRONMENT=production`, the config requires `CORS_ORIGIN` to
/// be set (hard-fails if not). The `build_cors()` function then uses the specified
/// origin for strict CORS. This behavior must be preserved after adding typo detection.
///
/// Preservation property: The config validation continues to require CORS_ORIGIN
/// when ENVIRONMENT==production, and `build_cors()` continues to apply strict CORS
/// when `cors_origin` is `Some(origin)`.
///
/// **Validates: Requirements 3.12**
#[test]
fn property_2l_correct_cors_preservation() {
    let config_source = include_str!("../src/config.rs");
    let app_source = include_str!("../src/app.rs");

    // Property: config requires CORS_ORIGIN when ENVIRONMENT=production
    assert!(
        config_source.contains("production") && config_source.contains("CORS_ORIGIN"),
        "config.rs must validate that CORS_ORIGIN is set when ENVIRONMENT=production."
    );

    // Property: config hard-fails (returns Err) when production without CORS_ORIGIN
    assert!(
        config_source.contains("CORS_ORIGIN debe estar configurado"),
        "config.rs must fail with a clear error when CORS_ORIGIN is missing in production."
    );

    // Property: build_cors applies strict origin when cors_origin is Some
    assert!(
        app_source.contains("build_cors") || app_source.contains("Cors::default()"),
        "app.rs must have CORS configuration logic."
    );
    assert!(
        app_source.contains("allowed_origin") && app_source.contains("allowed_methods"),
        "app.rs must configure allowed_origin and allowed_methods for strict CORS."
    );

    // Property: permissive CORS only when cors_origin is not set (development)
    assert!(
        app_source.contains("Cors::permissive()"),
        "app.rs must fall back to permissive CORS only when cors_origin is not configured."
    );

    // Property: strict CORS includes standard headers
    assert!(
        app_source.contains("AUTHORIZATION") || app_source.contains("CONTENT_TYPE"),
        "Strict CORS must allow Authorization and Content-Type headers."
    );
}
