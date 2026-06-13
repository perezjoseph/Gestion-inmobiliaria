// Feature: e2e-exploratory-bugfixes, Property 10: Preservation
// Non-admin NCF restrictions unchanged — the fix (removing the fiscal gate from
// the READ path) does NOT weaken:
//   1. RBAC: non-admin roles (gerente, visualizador) are blocked by AdminOnly at the handler layer
//   2. Fiscal gate on WRITE paths: `verificar_acceso_fiscal` still blocks informal orgs on
//      configuration/assignment endpoints
//
// **Validates: Requirements 3.6**
//
// Observation-first methodology:
// On UNFIXED code, gerente/visualizador receive 403 from AdminOnly on NCF endpoints (handler-layer,
// not testable at unit level without spinning up the full app). However, we CAN test that:
//   a) The fiscal gate (`verificar_acceso_fiscal`) blocks informal orgs — this preserves write-path
//      protection regardless of the read-path fix.
//   b) Non-informal orgs (persona_fisica, persona_juridica) pass the fiscal gate — this is correct
//      behavior for orgs with DGII registration on write paths.
//
// The RBAC layer (AdminOnly extractor) enforces role restrictions at the handler level and is NOT
// affected by the service-layer fiscal gate removal on the read path. We document this as a known
// constraint of unit-level testing.
//
// EXPECTED OUTCOME: Tests PASS on unfixed code (baseline fiscal gating captured).
#![allow(clippy::needless_return)]

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

// ── Strategies ──────────────────────────────────────────────────────────

/// Roles in the system. Non-admin roles are blocked by AdminOnly at handler layer.
fn role_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("admin".to_string()),
        Just("gerente".to_string()),
        Just("visualizador".to_string()),
    ]
}

/// Fiscal types for organizations.
fn tipo_fiscal_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("informal".to_string()),
        Just("persona_fisica".to_string()),
        Just("persona_juridica".to_string()),
    ]
}

/// Organization names to add variety across test cases.
fn org_name_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z]{5,15}".prop_map(|s| format!("Preservation Org {s}"))
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Build an organizacion::Model with the given tipo_fiscal.
fn make_org_model(
    tipo_fiscal: &str,
    nombre: &str,
) -> realestate_backend::entities::organizacion::Model {
    use chrono::Utc;
    use uuid::Uuid;

    realestate_backend::entities::organizacion::Model {
        id: Uuid::new_v4(),
        tipo: "propietario".to_string(),
        nombre: nombre.to_string(),
        estado: "activo".to_string(),
        cedula: None,
        telefono: None,
        email_organizacion: None,
        rnc: None,
        razon_social: None,
        nombre_comercial: None,
        direccion_fiscal: None,
        representante_legal: None,
        dgii_data: None,
        tipo_fiscal: tipo_fiscal.to_string(),
        regimen_pagos: None,
        fecha_inicio_operaciones: None,
        is_ecf_certificado: false,
        created_at: Utc::now().into(),
        updated_at: Utc::now().into(),
    }
}

// ── Property Tests ──────────────────────────────────────────────────────

/// Property 10a: Preservation — Write-path fiscal gating blocks informal orgs.
///
/// For all combinations of role × tipo_fiscal where tipo_fiscal == "informal",
/// the fiscal gate (`verificar_acceso_fiscal`) returns Forbidden. This is correct
/// behavior for write/configuration paths (configurar_rango_con_acceso, asignar_ncf).
///
/// The fix only removes this gate from the READ path. Write paths must stay gated.
#[test]
fn property_10_fiscal_gate_blocks_informal_on_write_paths() {
    use realestate_backend::services::fiscal::verificar_acceso_fiscal;

    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..ProptestConfig::default()
    });

    runner
        .run(
            &(role_strategy(), org_name_strategy()),
            |(role, org_name)| {
                // For write paths, the fiscal gate blocks informal orgs regardless of role.
                // (RBAC blocks non-admin at handler layer, but if they got through, fiscal still blocks.)
                let org = make_org_model("informal", &org_name);
                let result = verificar_acceso_fiscal(&org);

                prop_assert!(
                    result.is_err(),
                    "Fiscal gate should block informal org on write paths. \
                     Role='{}', org='{}', tipo_fiscal='informal' — expected Forbidden, got Ok",
                    role,
                    org_name,
                );

                // Verify it's specifically a Forbidden error
                if let Err(ref e) = result {
                    let err_str = format!("{e:?}");
                    prop_assert!(
                        err_str.contains("Forbidden") || err_str.contains("403"),
                        "Expected Forbidden error for informal org, got: {err_str}"
                    );
                }

                Ok(())
            },
        )
        .expect("Property 10a failed: fiscal gate does not block informal orgs on write paths");
}

/// Property 10b: Preservation — Non-informal orgs pass the fiscal gate.
///
/// For all combinations of role × tipo_fiscal where tipo_fiscal ∈ {persona_fisica, persona_juridica},
/// the fiscal gate returns Ok. These orgs have DGII registration and should be allowed on
/// write/configuration paths (subject to RBAC at handler level).
#[test]
fn property_10_fiscal_gate_allows_registered_orgs() {
    use realestate_backend::services::fiscal::verificar_acceso_fiscal;

    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..ProptestConfig::default()
    });

    // Only non-informal fiscal types
    let non_informal_tipo = prop_oneof![
        Just("persona_fisica".to_string()),
        Just("persona_juridica".to_string()),
    ];

    runner
        .run(
            &(role_strategy(), non_informal_tipo, org_name_strategy()),
            |(role, tipo_fiscal, org_name)| {
                let org = make_org_model(&tipo_fiscal, &org_name);
                let result = verificar_acceso_fiscal(&org);

                prop_assert!(
                    result.is_ok(),
                    "Fiscal gate should allow registered org (tipo_fiscal='{}') on write paths. \
                     Role='{}', org='{}' — expected Ok, got: {:?}",
                    tipo_fiscal,
                    role,
                    org_name,
                    result.err(),
                );

                Ok(())
            },
        )
        .expect("Property 10b failed: fiscal gate blocks registered orgs that should pass");
}

/// Property 10c: Preservation — Full matrix confirms RBAC + fiscal gating expectations.
///
/// Over the complete matrix {role ∈ {admin, gerente, visualizador}} × {tipo_fiscal ∈ {informal,
/// persona_fisica, persona_juridica}}, we verify:
/// - Non-admin roles are documented as blocked by AdminOnly (handler layer — noted but not
///   exercised at this unit level)
/// - The fiscal gate result depends ONLY on tipo_fiscal, not on role
///
/// This confirms the fiscal gate is role-independent (correct: RBAC and fiscal access are
/// orthogonal concerns).
#[test]
fn property_10_fiscal_gate_is_role_independent() {
    use realestate_backend::services::fiscal::verificar_acceso_fiscal;

    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..ProptestConfig::default()
    });

    runner
        .run(
            &(role_strategy(), tipo_fiscal_strategy(), org_name_strategy()),
            |(role, tipo_fiscal, org_name)| {
                let org = make_org_model(&tipo_fiscal, &org_name);
                let result = verificar_acceso_fiscal(&org);

                // The fiscal gate decision depends only on tipo_fiscal, not on role.
                // This is preservation: the gate is orthogonal to RBAC.
                let expected_blocked = tipo_fiscal == "informal";

                if expected_blocked {
                    prop_assert!(
                        result.is_err(),
                        "Informal org should be blocked by fiscal gate regardless of role. \
                         Role='{}', tipo_fiscal='{}', org='{}'",
                        role,
                        tipo_fiscal,
                        org_name,
                    );
                } else {
                    prop_assert!(
                        result.is_ok(),
                        "Registered org (tipo_fiscal='{}') should pass fiscal gate regardless of role. \
                         Role='{}', org='{}', error: {:?}",
                        tipo_fiscal,
                        role,
                        org_name,
                        result.err(),
                    );
                }

                // Document: non-admin roles (gerente, visualizador) are additionally blocked
                // by the AdminOnly extractor at the handler layer. This is NOT tested here
                // because AdminOnly is a FromRequest implementation that requires a full
                // Actix-web request context. The preservation guarantee is:
                // - RBAC (handler layer): non-admin → 403 regardless of tipo_fiscal
                // - Fiscal gate (service layer): informal → 403 regardless of role
                // The fix removes fiscal gate from READ only; both layers remain for WRITE.
                let _ = role; // used in assertion messages above

                Ok(())
            },
        )
        .expect(
            "Property 10c failed: fiscal gate behavior is not role-independent as expected",
        );
}
