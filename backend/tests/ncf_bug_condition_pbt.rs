// Feature: e2e-exploratory-bugfixes, Property 9: Bug Condition
// Admin can read NCF sequences — an admin of an `informal` org should NOT be
// blocked from reading (listing) NCF sequences.
//
// **Validates: Requirements 1.5**
//
// CRITICAL: This test is EXPECTED TO FAIL on unfixed code.
// The fiscal-access gate in `services::ncf::listar_secuencias` calls
// `obtener_org_con_acceso_fiscal`, which invokes `verificar_acceso_fiscal`
// and returns 403 when `tipo_fiscal == "informal"`.
// Failure confirms the bug exists.
//
// Approach: We test the fiscal gate directly (no DB required) by constructing
// an `organizacion::Model` with `tipo_fiscal = "informal"` and asserting that
// `verificar_acceso_fiscal` does NOT block it. Since the service unconditionally
// calls this gate before listing sequences, a Forbidden result here proves an
// admin would be blocked.
#![allow(clippy::needless_return)]

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

use crate::common;

// ── Strategy ────────────────────────────────────────────────────────────

/// The bug condition is scoped to:
///   {role: "admin", endpoint: GET /api/v1/ncf/secuencias, org.tipo_fiscal: "informal"}
///
/// We generate organization names to exercise the property across varied inputs.
/// The key invariant is tipo_fiscal = "informal".
fn informal_org_name() -> impl Strategy<Value = String> {
    "[A-Za-z]{5,20}".prop_map(|s| format!("NCF PBT Org {s}"))
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Build an organizacion::Model with the given tipo_fiscal.
/// This is the same helper pattern used in `services::fiscal::tests`.
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

// ── Property Test ───────────────────────────────────────────────────────

/// Property 9: Bug Condition — Admin can read NCF sequences.
///
/// The service function `listar_secuencias` unconditionally calls
/// `verificar_acceso_fiscal` before querying sequences. For an org with
/// `tipo_fiscal == "informal"`, this gate returns Forbidden(403).
///
/// This test asserts that `verificar_acceso_fiscal` should NOT block an
/// informal org from the read path (i.e., it should return Ok).
/// On unfixed code this assertion FAILS, confirming the bug:
///   admin of informal org → 403 "Funciones fiscales requieren registro en DGII"
#[test]
fn property_9_admin_informal_org_can_read_ncf_sequences() {
    use realestate_backend::services::fiscal::verificar_acceso_fiscal;

    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..ProptestConfig::default()
    });

    runner
        .run(&informal_org_name(), |org_name| {
            // Construct an org model that mirrors the bug condition:
            // role = admin (enforced at handler layer via AdminOnly),
            // tipo_fiscal = "informal" (new orgs default to this).
            let org = make_org_model("informal", &org_name);

            // The service calls verificar_acceso_fiscal before listing sequences.
            // For the read endpoint, this gate should NOT block access.
            // Assert: the fiscal gate allows reading for informal orgs.
            let result = verificar_acceso_fiscal(&org);

            // Expected behavior (after fix): Ok(()) — admin can read sequences.
            // Current (unfixed) behavior: Err(Forbidden("Funciones fiscales..."))
            prop_assert!(
                result.is_ok(),
                "Admin of informal org should be able to read NCF sequences via \
                 the fiscal gate, but got: {:?}. \
                 Counterexample: admin of org '{}' with tipo_fiscal='informal' → 403 \
                 'Funciones fiscales requieren registro en DGII'",
                result.err(),
                org_name
            );
            Ok(())
        })
        .expect("Property 9 failed: admin of informal org blocked from reading NCF sequences");
}

/// Integration test: exercises the full service path when a database is available.
/// This confirms the end-to-end behavior: `listar_secuencias` returns Ok for an
/// informal org's admin. Skipped when DATABASE_URL is not set or DB is unreachable.
#[test]
fn property_9_integration_admin_informal_org_listar_secuencias() {
    common::with_db(|db| async move {
        use chrono::Utc;
        use realestate_backend::entities::organizacion;
        use realestate_backend::services::ncf;
        use sea_orm::{ActiveModelTrait, EntityTrait, Set};
        use uuid::Uuid;

        // Create an organization with tipo_fiscal = "informal"
        let org_id = Uuid::new_v4();
        let now = Utc::now().into();
        organizacion::ActiveModel {
            id: Set(org_id),
            tipo: Set("propietario".to_string()),
            nombre: Set(format!("NCF Integration PBT Org {org_id}")),
            estado: Set("activo".to_string()),
            cedula: Set(None),
            telefono: Set(None),
            email_organizacion: Set(None),
            rnc: Set(None),
            razon_social: Set(None),
            nombre_comercial: Set(None),
            direccion_fiscal: Set(None),
            representante_legal: Set(None),
            dgii_data: Set(None),
            tipo_fiscal: Set("informal".to_string()),
            regimen_pagos: Set(None),
            fecha_inicio_operaciones: Set(None),
            is_ecf_certificado: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .expect("Failed to create test org");

        // Call listar_secuencias — admin role is enforced at handler layer
        let result = ncf::listar_secuencias(&db, org_id).await;

        // Assert: should return Ok with an empty list (no configured sequences)
        assert!(
            result.is_ok(),
            "Admin of informal org should be able to list NCF sequences, \
             but got error: {:?}",
            result.err()
        );

        // Cleanup
        let _ = organizacion::Entity::delete_by_id(org_id).exec(&db).await;
    });
}
