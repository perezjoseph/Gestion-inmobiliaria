// Feature: ley-85-25-compliance, Property 1: Bug Condition — Ley 85-25 Compliance Violations
//
// **Validates: Requirements 1.1, 1.2, 1.3, 1.4**
//
// CRITICAL: These tests MUST FAIL on unfixed code — failure confirms the bugs exist.
// They encode the EXPECTED behavior after the fix is applied.
//
// Bug Conditions:
//   1a: deposito > monto_mensual AND deposito <= 2 * monto_mensual → currently rejected (should accept)
//   1b: ipc_data IS None AND monto_nuevo > original * 1.10 → currently allowed (should reject)
//   1c: days_since_last_transition < 30 for iniciado→en_progreso → currently allowed (should reject)
//   1d: estado_deposito == "cobrado" AND elapsed > 15 days → no custodia_vencida field exists
//
// Approach: Tests 1a-1c exercise service layer with real DB. Test 1d checks the DTO struct.
// Additionally, each test has a unit-level companion that validates the logic without DB.
#![allow(clippy::needless_return, unused_imports)]

use crate::common;
use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

fn make_org(org_id: Uuid) -> realestate_backend::entities::organizacion::ActiveModel {
    use realestate_backend::entities::organizacion;
    let now = Utc::now().into();
    organizacion::ActiveModel {
        id: Set(org_id),
        tipo: Set("persona_fisica".to_string()),
        nombre: Set(format!("Ley85 PBT Org {org_id}")),
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
}

fn make_propiedad(
    id: Uuid,
    org_id: Uuid,
    estado: &str,
) -> realestate_backend::entities::propiedad::ActiveModel {
    use realestate_backend::entities::propiedad;
    let now = Utc::now().into();
    propiedad::ActiveModel {
        id: Set(id),
        titulo: Set("Test Prop".to_string()),
        descripcion: Set(None),
        direccion: Set("Calle Test 123".to_string()),
        ciudad: Set("Santo Domingo".to_string()),
        provincia: Set("Distrito Nacional".to_string()),
        tipo_propiedad: Set("apartamento".to_string()),
        habitaciones: Set(None),
        banos: Set(None),
        area_m2: Set(None),
        precio: Set(Decimal::new(20000_00, 2)),
        moneda: Set("DOP".to_string()),
        estado: Set(estado.to_string()),
        imagenes: Set(None),
        organizacion_id: Set(org_id),
        valor_catastral: Set(None),
        exento_ipi: Set(false),
        motivo_exencion: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
}

fn make_inquilino(id: Uuid, org_id: Uuid) -> realestate_backend::entities::inquilino::ActiveModel {
    use realestate_backend::entities::inquilino;
    let now = Utc::now().into();
    inquilino::ActiveModel {
        id: Set(id),
        nombre: Set("Test".to_string()),
        apellido: Set("Inquilino".to_string()),
        cedula: Set(format!("PBT-LEY85-{}", Uuid::new_v4())),
        telefono: Set(None),
        email: Set(None),
        contacto_emergencia: Set(None),
        notas: Set(None),
        documentos: Set(None),
        organizacion_id: Set(org_id),
        created_at: Set(now),
        updated_at: Set(now),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 1a: Deposit cap — 1.5× should be accepted under Ley 85-25
// ═══════════════════════════════════════════════════════════════════════════

/// Property 1a: Bug Condition — Deposit between 1× and 2× rejected
///
/// Under Ley 85-25, deposits up to 2× monthly rent are legal.
/// The current code rejects anything > 1× with "Ley 4314" error.
///
/// This unit test models the validation logic directly:
/// The `create` function checks `if deposito > input.monto_mensual` (1× cap).
/// After fix it should check `if deposito > input.monto_mensual * 2` (2× cap).
///
/// We assert the EXPECTED model: 1.5× deposit passes the cap check.
/// EXPECTED TO FAIL on unfixed code.
#[test]
fn bug_condition_1a_deposit_1_5x_accepted_under_ley_85_25() {
    let monto_mensual = Decimal::new(10000_00, 2); // 10,000.00
    let deposito = Decimal::new(15000_00, 2); // 15,000.00 = 1.5×

    // Model the validation logic as it EXISTS in contratos::create line ~293
    // Current (buggy): deposito > monto_mensual → reject
    // Expected (fixed): deposito > monto_mensual * 2 → reject
    let should_be_accepted = deposito <= monto_mensual * Decimal::from(2);
    let currently_accepted = deposito <= monto_mensual;

    // Prove the bug condition holds: 1.5× IS within 2× cap but FAILS the 1× check
    assert!(
        should_be_accepted,
        "1.5× deposit should be within the Ley 85-25 2× cap"
    );
    assert!(
        !currently_accepted,
        "1.5× deposit fails the current 1× cap (confirming the code path is hit)"
    );

    // EXPECTED BEHAVIOR ASSERTION: the actual service should ACCEPT this deposit.
    // We call the same logic the service uses. The service does:
    //   if deposito > input.monto_mensual { return Err(...) }
    // After fix:
    //   if deposito > input.monto_mensual * Decimal::from(2) { return Err(...) }
    //
    // Model the CURRENT behavior and assert it matches EXPECTED (will fail):
    let would_reject = deposito > monto_mensual; // current logic
    assert!(
        !would_reject,
        "Deposit of 1.5× monto_mensual should NOT be rejected under Ley 85-25 (2× cap), \
         but the current validation (deposito > monto_mensual) rejects it. \
         Counterexample: monto_mensual={monto_mensual}, deposito={deposito}, \
         error='El depósito no puede exceder un mes de renta (Ley 4314)'"
    );
}

/// Integration test 1a: exercises the full create path against DB.
#[test]
fn bug_condition_1a_integration_deposit_rejected_with_ley_4314() {
    common::with_db(|db| async move {
        use realestate_backend::entities::{inquilino, organizacion, propiedad};
        use realestate_backend::models::contrato::CreateContratoRequest;
        use realestate_backend::services::contratos;

        let org_id = Uuid::new_v4();
        let usuario_id = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();

        make_org(org_id).insert(&db).await.expect("org insert");
        make_propiedad(propiedad_id, org_id, "disponible")
            .insert(&db)
            .await
            .expect("propiedad insert");
        make_inquilino(inquilino_id, org_id)
            .insert(&db)
            .await
            .expect("inquilino insert");

        let monto_mensual = Decimal::new(10000_00, 2);
        let deposito = Decimal::new(15000_00, 2);

        let input = CreateContratoRequest {
            propiedad_id,
            inquilino_id,
            fecha_inicio: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            fecha_fin: chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            monto_mensual,
            deposito: Some(deposito),
            moneda: Some("DOP".to_string()),
            recargo_porcentaje: None,
            dias_gracia: None,
            dia_vencimiento: None,
        };

        let result = contratos::create(&db, input, usuario_id, org_id).await;

        assert!(
            result.is_ok(),
            "Deposit of 1.5× monto_mensual should be accepted under Ley 85-25 (2× cap), \
             but got error: {:?}",
            result.err()
        );

        let _ = inquilino::Entity::delete_by_id(inquilino_id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org_id).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 1b: IPC fallback — 15% increase should be REJECTED when IPC=None
// ═══════════════════════════════════════════════════════════════════════════

/// Property 1b: Bug Condition — IPC None skips validation entirely
///
/// Under Ley 85-25, when IPC data is unavailable, a hard 10% cap applies.
/// The current code: `None => { tracing::warn!(...); }` — skips validation.
///
/// We model the expected behavior: when IPC is None and increase > 10%,
/// the function should return an error. Currently it doesn't.
///
/// EXPECTED TO FAIL on unfixed code.
#[test]
fn bug_condition_1b_ipc_none_15_percent_increase_rejected() {
    let monto_original = Decimal::new(20000_00, 2); // 20,000.00
    let monto_nuevo = Decimal::new(23000_00, 2); // 23,000.00 = 15% increase
    let ipc_configured = false; // IPC is None

    let max_allowed_without_ipc = monto_original * Decimal::new(110, 2); // original * 1.10
    let exceeds_cap = monto_nuevo > max_allowed_without_ipc;

    assert!(exceeds_cap, "15% increase exceeds 10% fallback cap");

    // Model the CURRENT behavior of the `renovar` function:
    // When IPC is None, the code does: `None => { tracing::warn!(...); }`
    // This means NO validation happens — ANY increase is allowed.
    let current_behavior_rejects = if ipc_configured {
        monto_nuevo > max_allowed_without_ipc
    } else {
        false // Current code: skip validation entirely when IPC=None
    };

    // EXPECTED: should reject. CURRENT: does not reject.
    assert!(
        current_behavior_rejects,
        "15% rent increase with IPC=None should be rejected under Ley 85-25 (10% cap), \
         but the current code skips validation entirely when IPC is None. \
         Counterexample: monto_original={monto_original}, monto_nuevo={monto_nuevo}, \
         max_allowed={max_allowed_without_ipc}. Current behavior: logs warning and allows."
    );
}

/// Integration test 1b: exercises the full renovar path against DB.
#[test]
fn bug_condition_1b_integration_ipc_none_allows_unlimited_increase() {
    common::with_db(|db| async move {
        use realestate_backend::entities::{
            configuracion, contrato, inquilino, organizacion, propiedad,
        };
        use realestate_backend::models::contrato::RenovarContratoRequest;
        use realestate_backend::services::contratos;

        let org_id = Uuid::new_v4();
        let usuario_id = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();
        let contrato_id = Uuid::new_v4();
        let now = Utc::now().into();

        make_org(org_id).insert(&db).await.expect("org insert");
        make_propiedad(propiedad_id, org_id, "disponible")
            .insert(&db)
            .await
            .expect("propiedad insert");
        make_inquilino(inquilino_id, org_id)
            .insert(&db)
            .await
            .expect("inquilino insert");

        let monto_mensual = Decimal::new(20000_00, 2);

        contrato::ActiveModel {
            id: Set(contrato_id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            fecha_fin: Set(chrono::NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
            monto_mensual: Set(monto_mensual),
            deposito: Set(None),
            moneda: Set("DOP".to_string()),
            estado: Set("activo".to_string()),
            documentos: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
            estado_deposito: Set(None),
            fecha_cobro_deposito: Set(None),
            fecha_devolucion_deposito: Set(None),
            monto_retenido: Set(None),
            motivo_retencion: Set(None),
            recargo_porcentaje: Set(None),
            dias_gracia: Set(None),
        }
        .insert(&db)
        .await
        .expect("contrato insert");

        let _ = configuracion::Entity::delete_many()
            .filter(configuracion::Column::Clave.eq("ipc_banco_central"))
            .exec(&db)
            .await;

        let new_monto = Decimal::new(23000_00, 2); // 15% increase
        let input = RenovarContratoRequest {
            fecha_fin: chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            monto_mensual: new_monto,
            dia_vencimiento: None,
        };

        let result = contratos::renovar(&db, org_id, contrato_id, input, usuario_id).await;

        assert!(
            result.is_err(),
            "15% rent increase with IPC=None should be rejected under Ley 85-25 (10% cap), \
             but the renewal was silently allowed"
        );

        let _ = contrato::Entity::delete_by_id(contrato_id).exec(&db).await;
        let _ = inquilino::Entity::delete_by_id(inquilino_id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org_id).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 1c: Eviction time gap — instant transition should be REJECTED
// ═══════════════════════════════════════════════════════════════════════════

/// Property 1c: Bug Condition — Instant state transition allowed
///
/// Under Ley 85-25, at least 30 days must elapse before transitioning
/// from `iniciado` to `en_progreso`.
/// The current `validate_estado_transition` only checks graph edges.
///
/// We model the expected behavior: elapsed < 30 days should reject.
/// Currently the function accepts any valid edge transition instantly.
///
/// EXPECTED TO FAIL on unfixed code.
#[test]
fn bug_condition_1c_instant_eviction_transition_rejected() {
    use realestate_backend::services::desahucios::validate_estado_transition;

    let from = "iniciado";
    let to = "en_progreso";
    let days_elapsed = 0i64;
    let minimum_required = 30i64;

    // validate_estado_transition currently only checks the graph edge is valid
    let graph_edge_valid = validate_estado_transition(from, to).is_ok();
    assert!(graph_edge_valid, "iniciado→en_progreso is a valid edge");

    // EXPECTED behavior: should ALSO check elapsed time >= 30 days
    let time_check_passes = days_elapsed >= minimum_required;

    // The FULL validation (after fix) should be: graph_edge_valid AND time_check_passes
    let expected_result_is_allowed = graph_edge_valid && time_check_passes;

    assert!(
        !expected_result_is_allowed,
        "Sanity: 0 days elapsed should NOT pass the time check"
    );

    // ASSERTION: the current system should reject this (but it doesn't)
    // Current behavior: validate_estado_transition returns Ok(()) for valid edges
    // regardless of time. There is NO time check at all.
    let current_allows_transition = validate_estado_transition(from, to).is_ok();
    assert!(
        !current_allows_transition,
        "Instant transition iniciado→en_progreso (0 days elapsed) should be rejected \
         under Ley 85-25 (30-day minimum), but validate_estado_transition only checks \
         graph edges and allows it immediately. \
         Counterexample: from={from}, to={to}, days_elapsed={days_elapsed}, \
         minimum_required={minimum_required}"
    );
}

/// Integration test 1c: exercises the full update path against DB.
#[test]
fn bug_condition_1c_integration_instant_transition_allowed() {
    common::with_db(|db| async move {
        use realestate_backend::entities::{
            contrato, desahucio, inquilino, organizacion, propiedad,
        };
        use realestate_backend::models::desahucio::UpdateDesahucioRequest;
        use realestate_backend::services::desahucios;

        let org_id = Uuid::new_v4();
        let usuario_id = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();
        let contrato_id = Uuid::new_v4();
        let desahucio_id = Uuid::new_v4();
        let now = Utc::now();

        make_org(org_id).insert(&db).await.expect("org insert");
        make_propiedad(propiedad_id, org_id, "ocupada")
            .insert(&db)
            .await
            .expect("propiedad insert");
        make_inquilino(inquilino_id, org_id)
            .insert(&db)
            .await
            .expect("inquilino insert");

        contrato::ActiveModel {
            id: Set(contrato_id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            fecha_fin: Set(chrono::NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
            monto_mensual: Set(Decimal::new(15000_00, 2)),
            deposito: Set(None),
            moneda: Set("DOP".to_string()),
            estado: Set("activo".to_string()),
            documentos: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            estado_deposito: Set(None),
            fecha_cobro_deposito: Set(None),
            fecha_devolucion_deposito: Set(None),
            monto_retenido: Set(None),
            motivo_retencion: Set(None),
            recargo_porcentaje: Set(None),
            dias_gracia: Set(None),
        }
        .insert(&db)
        .await
        .expect("contrato insert");

        desahucio::ActiveModel {
            id: Set(desahucio_id),
            contrato_id: Set(contrato_id),
            estado: Set("iniciado".to_string()),
            fecha_inicio: Set(now.date_naive()),
            fecha_resolucion: Set(None),
            motivo: Set("No pago de renta".to_string()),
            organizacion_id: Set(org_id),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        }
        .insert(&db)
        .await
        .expect("desahucio insert");

        let input = UpdateDesahucioRequest {
            estado: Some("en_progreso".to_string()),
            fecha_resolucion: None,
            motivo: None,
        };

        let result = desahucios::update(&db, org_id, desahucio_id, input, usuario_id).await;

        assert!(
            result.is_err(),
            "Instant transition iniciado→en_progreso (0 days elapsed) should be rejected \
             under Ley 85-25 (30-day minimum), but it was allowed"
        );

        let _ = desahucio::Entity::delete_by_id(desahucio_id)
            .exec(&db)
            .await;
        let _ = contrato::Entity::delete_by_id(contrato_id).exec(&db).await;
        let _ = inquilino::Entity::delete_by_id(inquilino_id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org_id).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 1d: Custody tracking — custodia_vencida should be exposed
// ═══════════════════════════════════════════════════════════════════════════

/// Property 1d: Bug Condition — No custody warning field exists
///
/// Under Ley 85-25, when a deposit has been in `cobrado` state for > 15 days
/// without custody confirmation, the response should include `custodia_vencida = true`.
/// The current ContratoResponse has no such field.
///
/// We verify this by serializing a ContratoResponse and checking the JSON output.
/// EXPECTED TO FAIL on unfixed code (field doesn't exist).
#[test]
fn bug_condition_1d_custodia_vencida_exposed_after_15_days() {
    use realestate_backend::models::contrato::ContratoResponse;

    let id = Uuid::nil();
    let fecha_cobro = Utc::now() - Duration::days(16);

    let resp = ContratoResponse {
        id,
        propiedad_id: id,
        inquilino_id: id,
        fecha_inicio: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        fecha_fin: chrono::NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        monto_mensual: Decimal::new(18000_00, 2),
        deposito: Some(Decimal::new(18000_00, 2)),
        moneda: "DOP".to_string(),
        estado: "activo".to_string(),
        created_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
        updated_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
        pagos_generados: None,
        estado_deposito: Some("cobrado".to_string()),
        fecha_cobro_deposito: Some(fecha_cobro),
        fecha_devolucion_deposito: None,
        monto_retenido: None,
        motivo_retencion: None,
        recargo_porcentaje: None,
        dias_gracia: None,
        custodia_vencida: Some(true),
    };

    let json = serde_json::to_value(&resp).expect("serialize ContratoResponse");

    // EXPECTED BEHAVIOR (Ley 85-25): custodia_vencida field should exist and be true
    // BUG: ContratoResponse struct does not have this field at all
    assert!(
        json.get("custodiaVencida").is_some(),
        "ContratoResponse should include 'custodiaVencida' field for deposits \
         in cobrado state > 15 days without custody confirmation. \
         Field is missing from serialized response. \
         Counterexample: estado_deposito='cobrado', \
         fecha_cobro_deposito={fecha_cobro} (16 days ago), \
         expected custodiaVencida=true in JSON output"
    );

    let custodia_vencida = json["custodiaVencida"].as_bool();
    assert_eq!(
        custodia_vencida,
        Some(true),
        "custodia_vencida should be true when deposit is cobrado for 16 days \
         without custody confirmation"
    );
}
