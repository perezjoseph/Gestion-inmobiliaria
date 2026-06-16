// Feature: org-isolation-fix, Property 2: Preservation — Same-Org Access Unchanged
//
// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7**
//
// These tests confirm same-org requests succeed on UNFIXED code.
// They MUST PASS — if they fail, the test setup is wrong, not the code.
//
// Property: for all X where NOT isBugCondition(X),
//   endpoint(X).status is success AND correct_response_body AND expected_mutations_applied
#![allow(clippy::needless_return, unused_imports, dead_code)]

use crate::common;
use chrono::Utc;
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use realestate_backend::entities::{
    chatbot_conversation, chatbot_receipt_extraction, configuracion, contrato, copropietario,
    inquilino, organizacion, pago, propiedad,
};
use realestate_backend::services::{chatbot, configuracion as config_svc, indexacion, ipi};

fn make_org(org_id: Uuid) -> organizacion::ActiveModel {
    let now = Utc::now().into();
    organizacion::ActiveModel {
        id: Set(org_id),
        tipo: Set("persona_fisica".to_string()),
        nombre: Set(format!("OrgIso Preservation {org_id}")),
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

fn make_propiedad(id: Uuid, org_id: Uuid) -> propiedad::ActiveModel {
    let now = Utc::now().into();
    propiedad::ActiveModel {
        id: Set(id),
        titulo: Set("Preservation Test Prop".to_string()),
        descripcion: Set(None),
        direccion: Set("Calle Pres 1".to_string()),
        ciudad: Set("Santo Domingo".to_string()),
        provincia: Set("Distrito Nacional".to_string()),
        tipo_propiedad: Set("apartamento".to_string()),
        habitaciones: Set(None),
        banos: Set(None),
        area_m2: Set(None),
        precio: Set(Decimal::new(30000_00, 2)),
        moneda: Set("DOP".to_string()),
        estado: Set("ocupada".to_string()),
        imagenes: Set(None),
        organizacion_id: Set(org_id),
        valor_catastral: Set(None),
        exento_ipi: Set(false),
        motivo_exencion: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
}

fn make_inquilino(id: Uuid, org_id: Uuid) -> inquilino::ActiveModel {
    let now = Utc::now().into();
    inquilino::ActiveModel {
        id: Set(id),
        nombre: Set("Preservation".to_string()),
        apellido: Set("Tenant".to_string()),
        cedula: Set(format!("PRES-{}", Uuid::new_v4())),
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

fn make_contrato(
    id: Uuid,
    propiedad_id: Uuid,
    inquilino_id: Uuid,
    org_id: Uuid,
) -> contrato::ActiveModel {
    let now = Utc::now().into();
    contrato::ActiveModel {
        id: Set(id),
        propiedad_id: Set(propiedad_id),
        inquilino_id: Set(inquilino_id),
        fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        fecha_fin: Set(chrono::NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
        monto_mensual: Set(Decimal::new(25000_00, 2)),
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
}

fn make_ipc_config(org_id: Uuid) -> configuracion::ActiveModel {
    let now = Utc::now();
    let ipc_data = serde_json::json!({
        "valorIpc": "5.00",
        "fechaEfectiva": "2025-06-01",
        "ultimoFetchExitoso": now.to_rfc3339()
    });
    configuracion::ActiveModel {
        clave: Set("ipc_banco_central".to_string()),
        organizacion_id: Set(org_id),
        valor: Set(ipc_data),
        updated_at: Set(now.into()),
        updated_by: Set(None),
    }
}

fn make_extraction(
    id: Uuid,
    org_id: Uuid,
    conversation_id: Uuid,
    inquilino_id: Uuid,
    contrato_id: Uuid,
) -> chatbot_receipt_extraction::ActiveModel {
    let now = Utc::now().into();
    chatbot_receipt_extraction::ActiveModel {
        id: Set(id),
        organizacion_id: Set(org_id),
        conversation_id: Set(conversation_id),
        inquilino_id: Set(Some(inquilino_id)),
        contrato_id: Set(Some(contrato_id)),
        extracted_data: Set(serde_json::json!({
            "amount": "15000.00",
            "currency": "DOP",
            "date": "2025-06-15"
        })),
        status: Set("pending_confirmation".to_string()),
        confirmed_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
}

fn make_conversation(
    id: Uuid,
    org_id: Uuid,
    inquilino_id: Uuid,
) -> chatbot_conversation::ActiveModel {
    let now = Utc::now().into();
    chatbot_conversation::ActiveModel {
        id: Set(id),
        organizacion_id: Set(org_id),
        sender_phone: Set(format!("+1809555{}", &id.to_string()[..4])),
        inquilino_id: Set(Some(inquilino_id)),
        role: Set("user".to_string()),
        content: Set("preservation test".to_string()),
        message_type: Set("text".to_string()),
        metadata: Set(None),
        created_at: Set(now),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 3.1: Indexacion — same-org propuesta succeeds
// ═══════════════════════════════════════════════════════════════════════════

/// Property 2 (Req 3.1): Preservation — calcular_propuesta_renovacion same-org
///
/// Org A calls propuesta for its OWN contrato → 200 with valid proposal.
/// MUST PASS on unfixed code.
#[test]
fn preservation_3_1_same_org_propuesta() {
    common::with_db(|db| async move {
        let org = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();
        let contrato_id = Uuid::new_v4();

        make_org(org).insert(&db).await.expect("org insert");
        make_propiedad(propiedad_id, org)
            .insert(&db)
            .await
            .expect("propiedad insert");
        make_inquilino(inquilino_id, org)
            .insert(&db)
            .await
            .expect("inquilino insert");
        make_contrato(contrato_id, propiedad_id, inquilino_id, org)
            .insert(&db)
            .await
            .expect("contrato insert");

        let _ = configuracion::Entity::delete_many()
            .filter(configuracion::Column::Clave.eq("ipc_banco_central"))
            .exec(&db)
            .await;
        make_ipc_config(org).insert(&db).await.expect("ipc insert");

        let result = indexacion::calcular_propuesta_renovacion(&db, contrato_id).await;

        assert!(
            result.is_ok(),
            "Same-org propuesta should succeed, got: {:?}",
            result.err()
        );
        let proposal = result.unwrap();
        assert_eq!(proposal.contrato_id, contrato_id);
        assert!(proposal.monto_maximo >= proposal.monto_actual);

        let _ = contrato::Entity::delete_by_id(contrato_id).exec(&db).await;
        let _ = inquilino::Entity::delete_by_id(inquilino_id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 3.1b: Indexacion — same-org aprobar succeeds
// ═══════════════════════════════════════════════════════════════════════════

/// Property 2 (Req 3.1): Preservation — aprobar_renovacion same-org
///
/// Org A approves its OWN contrato → 200 with new contrato created.
/// MUST PASS on unfixed code.
#[test]
fn preservation_3_1b_same_org_aprobar() {
    common::with_db(|db| async move {
        let org = Uuid::new_v4();
        let admin_id = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();
        let contrato_id = Uuid::new_v4();

        make_org(org).insert(&db).await.expect("org insert");
        make_propiedad(propiedad_id, org)
            .insert(&db)
            .await
            .expect("propiedad insert");
        make_inquilino(inquilino_id, org)
            .insert(&db)
            .await
            .expect("inquilino insert");
        make_contrato(contrato_id, propiedad_id, inquilino_id, org)
            .insert(&db)
            .await
            .expect("contrato insert");

        let _ = configuracion::Entity::delete_many()
            .filter(configuracion::Column::Clave.eq("ipc_banco_central"))
            .exec(&db)
            .await;
        make_ipc_config(org).insert(&db).await.expect("ipc insert");

        let monto_aprobado = Decimal::new(26000_00, 2);

        let result =
            indexacion::aprobar_renovacion(&db, contrato_id, monto_aprobado, admin_id).await;

        assert!(
            result.is_ok(),
            "Same-org aprobar should succeed, got: {:?}",
            result.err()
        );
        let new_contrato = result.unwrap();
        assert_eq!(new_contrato.monto_mensual, monto_aprobado);
        assert_eq!(new_contrato.organizacion_id, org);
        assert_eq!(new_contrato.estado, "activo");

        let all_contratos = contrato::Entity::find()
            .filter(contrato::Column::PropiedadId.eq(propiedad_id))
            .all(&db)
            .await
            .unwrap();
        for c in &all_contratos {
            let _ = contrato::Entity::delete_by_id(c.id).exec(&db).await;
        }
        let _ = inquilino::Entity::delete_by_id(inquilino_id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 3.2: IPI — same-org copropietarios list succeeds
// ═══════════════════════════════════════════════════════════════════════════

/// Property 2 (Req 3.2): Preservation — obtener_copropietarios same-org
///
/// Org A lists copropietarios for its OWN propiedad → 200 with list.
/// MUST PASS on unfixed code.
#[test]
fn preservation_3_2_same_org_copropietarios_list() {
    common::with_db(|db| async move {
        let org = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let coprop_id = Uuid::new_v4();

        make_org(org).insert(&db).await.expect("org insert");
        make_propiedad(propiedad_id, org)
            .insert(&db)
            .await
            .expect("propiedad insert");

        let now = Utc::now().into();
        copropietario::ActiveModel {
            id: Set(coprop_id),
            propiedad_id: Set(propiedad_id),
            nombre: Set("Same Org Owner".to_string()),
            cedula_rnc: Set(format!("PRES-CP-{}", Uuid::new_v4())),
            porcentaje_propiedad: Set(Decimal::new(100_00, 2)),
            organizacion_id: Set(org),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .expect("copropietario insert");

        let result = ipi::obtener_copropietarios(&db, propiedad_id).await;

        assert!(
            result.is_ok(),
            "Same-org copropietarios list should succeed, got: {:?}",
            result.err()
        );
        let list = result.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, coprop_id);

        let _ = copropietario::Entity::delete_by_id(coprop_id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 3.3: IPI — same-org crear copropietario succeeds
// ═══════════════════════════════════════════════════════════════════════════

/// Property 2 (Req 3.3): Preservation — crear_copropietario same-org
///
/// Org A creates copropietario on its OWN propiedad → 201.
/// MUST PASS on unfixed code.
#[test]
fn preservation_3_3_same_org_crear_copropietario() {
    common::with_db(|db| async move {
        let org = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();

        make_org(org).insert(&db).await.expect("org insert");
        make_propiedad(propiedad_id, org)
            .insert(&db)
            .await
            .expect("propiedad insert");

        let result = ipi::crear_copropietario(
            &db,
            org,
            propiedad_id,
            "Valid Owner".to_string(),
            format!("PRES-NEW-{}", Uuid::new_v4()),
            Decimal::new(50_00, 2),
        )
        .await;

        assert!(
            result.is_ok(),
            "Same-org crear_copropietario should succeed, got: {:?}",
            result.err()
        );
        let created = result.unwrap();
        assert_eq!(created.propiedad_id, propiedad_id);
        assert_eq!(created.organizacion_id, org);
        assert_eq!(created.porcentaje_propiedad, Decimal::new(50_00, 2));

        let _ = copropietario::Entity::delete_by_id(created.id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 3.4a: Chatbot — same-org confirm_receipt succeeds
// ═══════════════════════════════════════════════════════════════════════════

/// Property 2 (Req 3.4): Preservation — confirm_receipt same-org
///
/// Org A confirms its OWN extraction → 200 and pago created.
/// MUST PASS on unfixed code.
#[test]
fn preservation_3_4a_same_org_confirm_receipt() {
    common::with_db(|db| async move {
        let org = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();
        let contrato_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let extraction_id = Uuid::new_v4();

        make_org(org).insert(&db).await.expect("org insert");
        make_propiedad(propiedad_id, org)
            .insert(&db)
            .await
            .expect("propiedad insert");
        make_inquilino(inquilino_id, org)
            .insert(&db)
            .await
            .expect("inquilino insert");
        make_contrato(contrato_id, propiedad_id, inquilino_id, org)
            .insert(&db)
            .await
            .expect("contrato insert");
        make_conversation(conversation_id, org, inquilino_id)
            .insert(&db)
            .await
            .expect("conversation insert");
        make_extraction(
            extraction_id,
            org,
            conversation_id,
            inquilino_id,
            contrato_id,
        )
        .insert(&db)
        .await
        .expect("extraction insert");

        let before_pagos = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_id))
            .all(&db)
            .await
            .unwrap()
            .len();

        let result = chatbot::confirm_receipt(&db, extraction_id, user_id).await;

        assert!(
            result.is_ok(),
            "Same-org confirm_receipt should succeed, got: {:?}",
            result.err()
        );
        let updated_ext = result.unwrap();
        assert_eq!(updated_ext.status, "confirmed");
        assert_eq!(updated_ext.confirmed_by, Some(user_id));

        let after_pagos = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_id))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(
            after_pagos.len(),
            before_pagos + 1,
            "A pago should be created on same-org confirm"
        );

        for p in &after_pagos {
            let _ = pago::Entity::delete_by_id(p.id).exec(&db).await;
        }
        let _ = chatbot_receipt_extraction::Entity::delete_by_id(extraction_id)
            .exec(&db)
            .await;
        let _ = chatbot_conversation::Entity::delete_by_id(conversation_id)
            .exec(&db)
            .await;
        let _ = contrato::Entity::delete_by_id(contrato_id).exec(&db).await;
        let _ = inquilino::Entity::delete_by_id(inquilino_id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 3.4b: Chatbot — same-org reject_receipt succeeds
// ═══════════════════════════════════════════════════════════════════════════

/// Property 2 (Req 3.4): Preservation — reject_receipt same-org
///
/// Org A rejects its OWN extraction → 200 and status updated.
/// MUST PASS on unfixed code.
#[test]
fn preservation_3_4b_same_org_reject_receipt() {
    common::with_db(|db| async move {
        let org = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();
        let contrato_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let extraction_id = Uuid::new_v4();

        make_org(org).insert(&db).await.expect("org insert");
        make_propiedad(propiedad_id, org)
            .insert(&db)
            .await
            .expect("propiedad insert");
        make_inquilino(inquilino_id, org)
            .insert(&db)
            .await
            .expect("inquilino insert");
        make_contrato(contrato_id, propiedad_id, inquilino_id, org)
            .insert(&db)
            .await
            .expect("contrato insert");
        make_conversation(conversation_id, org, inquilino_id)
            .insert(&db)
            .await
            .expect("conversation insert");
        make_extraction(
            extraction_id,
            org,
            conversation_id,
            inquilino_id,
            contrato_id,
        )
        .insert(&db)
        .await
        .expect("extraction insert");

        let result =
            chatbot::reject_receipt(&db, extraction_id, user_id, Some("monto incorrecto")).await;

        assert!(
            result.is_ok(),
            "Same-org reject_receipt should succeed, got: {:?}",
            result.err()
        );
        let updated_ext = result.unwrap();
        assert_eq!(updated_ext.status, "rejected");

        let _ = chatbot_receipt_extraction::Entity::delete_by_id(extraction_id)
            .exec(&db)
            .await;
        let _ = chatbot_conversation::Entity::delete_by_id(conversation_id)
            .exec(&db)
            .await;
        let _ = contrato::Entity::delete_by_id(contrato_id).exec(&db).await;
        let _ = inquilino::Entity::delete_by_id(inquilino_id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 3.7: Configuracion — same-org read/write succeeds
// ═══════════════════════════════════════════════════════════════════════════

/// Property 2 (Req 3.7): Preservation — configuracion same-org read/write
///
/// Org A writes tasa_cambio and reads back → 200 with correct value.
/// MUST PASS on unfixed code (configuracion is global pre-fix, single-org still works).
#[test]
fn preservation_3_7_same_org_configuracion() {
    common::with_db(|db| async move {
        let org = Uuid::new_v4();
        let admin_id = Uuid::new_v4();

        make_org(org).insert(&db).await.expect("org insert");

        let tasa = 58.75;
        let write_result = config_svc::actualizar_moneda(&db, tasa, admin_id, org).await;
        assert!(
            write_result.is_ok(),
            "Same-org configuracion write should succeed, got: {:?}",
            write_result.err()
        );
        let written = write_result.unwrap();
        assert!((written.tasa - tasa).abs() < f64::EPSILON);

        let read_result = config_svc::obtener_moneda(&db, org).await;
        assert!(
            read_result.is_ok(),
            "Same-org configuracion read should succeed, got: {:?}",
            read_result.err()
        );
        let read = read_result.unwrap();
        assert!((read.tasa - tasa).abs() < f64::EPSILON);

        let _ = organizacion::Entity::delete_by_id(org).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// PBT: Property-based test for same-org access across all endpoints
// ═══════════════════════════════════════════════════════════════════════════

/// Property 2 (All Reqs 3.1-3.7): PBT — Same-org access succeeds with random data
///
/// Generates random org_id and entity setups, verifies all same-org
/// operations succeed with expected responses and mutations.
#[test]
fn preservation_pbt_same_org_access_all_endpoints() {
    common::with_db(|db| async move {
        let config = ProptestConfig {
            cases: crate::pbt_cases(),
            ..ProptestConfig::default()
        };
        let mut runner = TestRunner::new(config);

        let db_clone = db.clone();
        runner
            .run(
                &(prop::array::uniform4(any::<u128>()), 50_00i64..100_00i64),
                |(uuids, porcentaje_raw)| {
                    let db = db_clone.clone();
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(async move {
                        let org = Uuid::from_u128(uuids[0] | 1);
                        let propiedad_id = Uuid::from_u128(uuids[1] | 1);
                        let inquilino_id = Uuid::from_u128(uuids[2] | 1);
                        let contrato_id = Uuid::from_u128(uuids[3] | 1);

                        make_org(org).insert(&db).await.map_err(|_| {
                            proptest::test_runner::TestCaseError::Reject("db setup".into())
                        })?;
                        make_propiedad(propiedad_id, org)
                            .insert(&db)
                            .await
                            .map_err(|_| {
                                proptest::test_runner::TestCaseError::Reject("db setup".into())
                            })?;
                        make_inquilino(inquilino_id, org)
                            .insert(&db)
                            .await
                            .map_err(|_| {
                                proptest::test_runner::TestCaseError::Reject("db setup".into())
                            })?;
                        make_contrato(contrato_id, propiedad_id, inquilino_id, org)
                            .insert(&db)
                            .await
                            .map_err(|_| {
                                proptest::test_runner::TestCaseError::Reject("db setup".into())
                            })?;

                        let _ = configuracion::Entity::delete_many()
                            .filter(configuracion::Column::Clave.eq("ipc_banco_central"))
                            .exec(&db)
                            .await;
                        make_ipc_config(org).insert(&db).await.map_err(|_| {
                            proptest::test_runner::TestCaseError::Reject("db setup".into())
                        })?;

                        let propuesta =
                            indexacion::calcular_propuesta_renovacion(&db, contrato_id).await;
                        prop_assert!(
                            propuesta.is_ok(),
                            "Same-org propuesta failed for org={org}, contrato={contrato_id}"
                        );

                        let coprop_result = ipi::crear_copropietario(
                            &db,
                            org,
                            propiedad_id,
                            format!("PBT Owner {}", org),
                            format!("PBT-{}", Uuid::new_v4()),
                            Decimal::new(porcentaje_raw, 2),
                        )
                        .await;
                        prop_assert!(
                            coprop_result.is_ok(),
                            "Same-org crear_copropietario failed for org={org}"
                        );

                        let list_result = ipi::obtener_copropietarios(&db, propiedad_id).await;
                        prop_assert!(
                            list_result.is_ok(),
                            "Same-org copropietarios list failed for org={org}"
                        );
                        let list = list_result.unwrap();
                        prop_assert!(!list.is_empty(), "Copropietarios list should not be empty");

                        if let Ok(c) = &coprop_result {
                            let _ = copropietario::Entity::delete_by_id(c.id).exec(&db).await;
                        }
                        let all_contratos = contrato::Entity::find()
                            .filter(contrato::Column::PropiedadId.eq(propiedad_id))
                            .all(&db)
                            .await
                            .unwrap_or_default();
                        for c in &all_contratos {
                            let _ = contrato::Entity::delete_by_id(c.id).exec(&db).await;
                        }
                        let _ = inquilino::Entity::delete_by_id(inquilino_id)
                            .exec(&db)
                            .await;
                        let _ = propiedad::Entity::delete_by_id(propiedad_id)
                            .exec(&db)
                            .await;
                        let _ = organizacion::Entity::delete_by_id(org).exec(&db).await;

                        Ok(())
                    })
                },
            )
            .expect("PBT: Same-org access should always succeed");
    });
}
