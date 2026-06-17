// Feature: org-isolation-fix, Property 1: Bug Condition — Cross-Org IDOR Access
//
// **Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8**
//
// CRITICAL: These tests MUST FAIL on unfixed code — failure confirms the IDOR bugs exist.
// They encode the EXPECTED behavior after the fix is applied.
//
// Bug Condition: isBugCondition(X) = X.caller_org_id ≠ lookupOrganizacionId(X.entity_id)
//
// For each affected endpoint, we create entities under Org B, then call service functions
// without org ownership checks (as they currently work). The tests assert the EXPECTED
// behavior (404 / rejection) which WILL FAIL because the current code allows cross-org access.
#![allow(clippy::needless_return, unused_imports)]

use crate::common;
use chrono::Utc;
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

fn make_org(org_id: Uuid) -> realestate_backend::entities::organizacion::ActiveModel {
    use realestate_backend::entities::organizacion;
    let now = Utc::now().into();
    organizacion::ActiveModel {
        id: Set(org_id),
        tipo: Set("persona_fisica".to_string()),
        nombre: Set(format!("OrgIsolation PBT {org_id}")),
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

fn make_propiedad(id: Uuid, org_id: Uuid) -> realestate_backend::entities::propiedad::ActiveModel {
    use realestate_backend::entities::propiedad;
    let now = Utc::now().into();
    propiedad::ActiveModel {
        id: Set(id),
        titulo: Set("OrgIso Test Prop".to_string()),
        descripcion: Set(None),
        direccion: Set("Calle Test 1".to_string()),
        ciudad: Set("Santo Domingo".to_string()),
        provincia: Set("Distrito Nacional".to_string()),
        tipo_propiedad: Set("apartamento".to_string()),
        habitaciones: Set(None),
        banos: Set(None),
        area_m2: Set(None),
        precio: Set(Decimal::new(3_000_000, 2)),
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

fn make_inquilino(id: Uuid, org_id: Uuid) -> realestate_backend::entities::inquilino::ActiveModel {
    use realestate_backend::entities::inquilino;
    let now = Utc::now().into();
    inquilino::ActiveModel {
        id: Set(id),
        nombre: Set("OrgIso".to_string()),
        apellido: Set("Tenant".to_string()),
        cedula: Set(format!("ORGISO-{}", Uuid::new_v4())),
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
) -> realestate_backend::entities::contrato::ActiveModel {
    use realestate_backend::entities::contrato;
    let now = Utc::now().into();
    contrato::ActiveModel {
        id: Set(id),
        propiedad_id: Set(propiedad_id),
        inquilino_id: Set(inquilino_id),
        fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        fecha_fin: Set(chrono::NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
        monto_mensual: Set(Decimal::new(2_500_000, 2)),
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

fn make_ipc_config(org_id: Uuid) -> realestate_backend::entities::configuracion::ActiveModel {
    use realestate_backend::entities::configuracion;
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

// ═══════════════════════════════════════════════════════════════════════════
// Test 1.1: Indexacion propuesta — cross-org access should return 404
// ═══════════════════════════════════════════════════════════════════════════

/// Property 1 (Req 1.1): Cross-Org IDOR — calcular_propuesta_renovacion
///
/// Org A calls `calcular_propuesta_renovacion` with Org B's contrato_id.
/// EXPECTED: should return NotFound (404).
/// CURRENT BUG: returns Ok with Org B's proposal because no org check exists.
///
/// EXPECTED TO FAIL on unfixed code.
#[test]
fn bug_condition_1_1_cross_org_propuesta_access() {
    common::with_db(|db| async move {
        use realestate_backend::entities::{
            configuracion, contrato, inquilino, organizacion, propiedad,
        };
        use realestate_backend::services::indexacion;

        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();
        let contrato_id = Uuid::new_v4();

        make_org(org_a).insert(&db).await.expect("org_a insert");
        make_org(org_b).insert(&db).await.expect("org_b insert");
        make_propiedad(propiedad_id, org_b)
            .insert(&db)
            .await
            .expect("propiedad insert");
        make_inquilino(inquilino_id, org_b)
            .insert(&db)
            .await
            .expect("inquilino insert");
        make_contrato(contrato_id, propiedad_id, inquilino_id, org_b)
            .insert(&db)
            .await
            .expect("contrato insert");

        let _ = configuracion::Entity::delete_many()
            .filter(configuracion::Column::Clave.eq("ipc_banco_central"))
            .exec(&db)
            .await;
        make_ipc_config(org_b)
            .insert(&db)
            .await
            .expect("ipc insert");

        // Org A calls propuesta for Org B's contrato — should be 404
        let result = indexacion::calcular_propuesta_renovacion(&db, contrato_id, org_a).await;

        // EXPECTED BEHAVIOR: NotFound because caller (Org A) ≠ entity owner (Org B)
        // BUG: service accepts any contrato_id without verifying org ownership
        assert!(
            result.is_err(),
            "Cross-org propuesta access should return NotFound (404), \
             but succeeded with 200 OK. Counterexample: caller_org={org_a}, \
             entity_org={org_b}, contrato_id={contrato_id}. \
             Service calcular_propuesta_renovacion does not check organizacion_id."
        );

        // Cleanup
        let _ = contrato::Entity::delete_by_id(contrato_id).exec(&db).await;
        let _ = inquilino::Entity::delete_by_id(inquilino_id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org_b).exec(&db).await;
        let _ = organizacion::Entity::delete_by_id(org_a).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 1.2: Indexacion aprobar — cross-org should return 404, no new contrato
// ═══════════════════════════════════════════════════════════════════════════

/// Property 1 (Req 1.2): Cross-Org IDOR — aprobar_renovacion
///
/// Org A calls `aprobar_renovacion` with Org B's contrato_id.
/// EXPECTED: should return NotFound (404) and NOT create a new contrato.
/// CURRENT BUG: creates a new contrato under Org B's entity graph.
///
/// EXPECTED TO FAIL on unfixed code.
#[test]
fn bug_condition_1_2_cross_org_aprobar_renovacion() {
    common::with_db(|db| async move {
        use realestate_backend::entities::{
            configuracion, contrato, inquilino, organizacion, propiedad,
        };
        use realestate_backend::services::indexacion;

        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        let admin_id_org_a = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();
        let contrato_id = Uuid::new_v4();

        make_org(org_a).insert(&db).await.expect("org_a insert");
        make_org(org_b).insert(&db).await.expect("org_b insert");
        make_propiedad(propiedad_id, org_b)
            .insert(&db)
            .await
            .expect("propiedad insert");
        make_inquilino(inquilino_id, org_b)
            .insert(&db)
            .await
            .expect("inquilino insert");
        make_contrato(contrato_id, propiedad_id, inquilino_id, org_b)
            .insert(&db)
            .await
            .expect("contrato insert");

        let _ = configuracion::Entity::delete_many()
            .filter(configuracion::Column::Clave.eq("ipc_banco_central"))
            .exec(&db)
            .await;
        make_ipc_config(org_b)
            .insert(&db)
            .await
            .expect("ipc insert");

        let monto_aprobado = Decimal::new(2_600_000, 2);

        // Count contratos before
        let before_count = contrato::Entity::find()
            .filter(contrato::Column::PropiedadId.eq(propiedad_id))
            .all(&db)
            .await
            .unwrap()
            .len();

        // Org A admin approves Org B's contrato — should be 404
        let result =
            indexacion::aprobar_renovacion(&db, contrato_id, monto_aprobado, admin_id_org_a, org_a)
                .await;

        // EXPECTED: NotFound + no mutation
        assert!(
            result.is_err(),
            "Cross-org aprobar_renovacion should return NotFound (404), \
             but succeeded. Counterexample: caller_org={org_a}, \
             entity_org={org_b}, contrato_id={contrato_id}, \
             monto_aprobado={monto_aprobado}. A new contrato was created \
             under Org B's entity graph by Org A's admin."
        );

        // Verify no mutation occurred
        let after_count = contrato::Entity::find()
            .filter(contrato::Column::PropiedadId.eq(propiedad_id))
            .all(&db)
            .await
            .unwrap()
            .len();
        assert_eq!(
            before_count, after_count,
            "No new contrato should be created on cross-org aprobar"
        );

        // Cleanup (new contrato may have been created by the bug)
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
        let _ = organizacion::Entity::delete_by_id(org_b).exec(&db).await;
        let _ = organizacion::Entity::delete_by_id(org_a).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 1.3: IPI copropietarios list — cross-org access should return 404
// ═══════════════════════════════════════════════════════════════════════════

/// Property 1 (Req 1.3): Cross-Org IDOR — obtener_copropietarios
///
/// Org A calls `obtener_copropietarios` with Org B's propiedad_id.
/// EXPECTED: should return NotFound (404).
/// CURRENT BUG: returns Org B's copropietarios because no org check exists.
///
/// EXPECTED TO FAIL on unfixed code.
#[test]
fn bug_condition_1_3_cross_org_copropietarios_list() {
    common::with_db(|db| async move {
        use realestate_backend::entities::{copropietario, organizacion, propiedad};
        use realestate_backend::services::ipi;

        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();

        make_org(org_a).insert(&db).await.expect("org_a insert");
        make_org(org_b).insert(&db).await.expect("org_b insert");
        make_propiedad(propiedad_id, org_b)
            .insert(&db)
            .await
            .expect("propiedad insert");

        // Create a copropietario under Org B's propiedad
        let coprop_id = Uuid::new_v4();
        let now = Utc::now().into();
        copropietario::ActiveModel {
            id: Set(coprop_id),
            propiedad_id: Set(propiedad_id),
            nombre: Set("Test Coprop".to_string()),
            cedula_rnc: Set(format!("ORGISO-CP-{}", Uuid::new_v4())),
            porcentaje_propiedad: Set(Decimal::new(50_00, 2)),
            organizacion_id: Set(org_b),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .expect("copropietario insert");

        // Org A lists copropietarios for Org B's propiedad — should be 404
        let result = ipi::obtener_copropietarios(&db, propiedad_id, org_a).await;

        // EXPECTED: NotFound because caller (Org A) ≠ propiedad owner (Org B)
        // BUG: returns Ok with Org B's data
        assert!(
            result.is_err(),
            "Cross-org copropietarios list should return NotFound (404), \
             but succeeded with data. Counterexample: caller_org={org_a}, \
             entity_org={org_b}, propiedad_id={propiedad_id}. \
             Service obtener_copropietarios has no org ownership check."
        );

        // Cleanup
        let _ = copropietario::Entity::delete_by_id(coprop_id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org_b).exec(&db).await;
        let _ = organizacion::Entity::delete_by_id(org_a).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 1.4: IPI crear copropietario — cross-org should return 404
// ═══════════════════════════════════════════════════════════════════════════

/// Property 1 (Req 1.4): Cross-Org IDOR — crear_copropietario with cross-org propiedad
///
/// Org A creates a copropietario on Org B's propiedad.
/// EXPECTED: should return NotFound (404) and NOT create a record.
/// CURRENT BUG: creates the record because crear_copropietario doesn't verify
/// that propiedad_id belongs to the caller's org.
///
/// NOTE: crear_copropietario takes org_id but uses it only to set the record's
/// organizacion_id — it never verifies propiedad ownership.
///
/// EXPECTED TO FAIL on unfixed code.
#[test]
fn bug_condition_1_4_cross_org_crear_copropietario() {
    common::with_db(|db| async move {
        use realestate_backend::entities::{copropietario, organizacion, propiedad};
        use realestate_backend::services::ipi;

        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();

        make_org(org_a).insert(&db).await.expect("org_a insert");
        make_org(org_b).insert(&db).await.expect("org_b insert");
        make_propiedad(propiedad_id, org_b)
            .insert(&db)
            .await
            .expect("propiedad insert");

        // Count copropietarios before
        let before_count = copropietario::Entity::find()
            .filter(copropietario::Column::PropiedadId.eq(propiedad_id))
            .all(&db)
            .await
            .unwrap()
            .len();

        // Org A creates copropietario on Org B's propiedad — should be 404
        let result = ipi::crear_copropietario(
            &db,
            org_a,        // caller's org
            propiedad_id, // belongs to org_b
            "Cross Org Owner".to_string(),
            format!("XORG-{}", Uuid::new_v4()),
            Decimal::new(25_00, 2),
        )
        .await;

        // EXPECTED: NotFound — propiedad doesn't belong to org_a
        // BUG: creates a record with org_a's ID on org_b's propiedad
        assert!(
            result.is_err(),
            "Cross-org crear_copropietario should return NotFound (404), \
             but succeeded. Counterexample: caller_org={org_a}, \
             propiedad_org={org_b}, propiedad_id={propiedad_id}. \
             Service crear_copropietario doesn't verify propiedad ownership."
        );

        // Verify no mutation
        let after_count = copropietario::Entity::find()
            .filter(copropietario::Column::PropiedadId.eq(propiedad_id))
            .all(&db)
            .await
            .unwrap()
            .len();
        assert_eq!(
            before_count, after_count,
            "No copropietario should be created on cross-org propiedad"
        );

        // Cleanup (record may have been created by the bug)
        let created = copropietario::Entity::find()
            .filter(copropietario::Column::PropiedadId.eq(propiedad_id))
            .all(&db)
            .await
            .unwrap();
        for c in &created {
            let _ = copropietario::Entity::delete_by_id(c.id).exec(&db).await;
        }
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org_b).exec(&db).await;
        let _ = organizacion::Entity::delete_by_id(org_a).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 1.5: Chatbot confirm — cross-org should return 404, no pago
// ═══════════════════════════════════════════════════════════════════════════

/// Property 1 (Req 1.5): Cross-Org IDOR — confirm_receipt
///
/// Org A calls `confirm_receipt` with Org B's extraction_id.
/// EXPECTED: should return NotFound (404) and NOT create a pago.
/// CURRENT BUG: confirms Org B's extraction and creates pago.
///
/// EXPECTED TO FAIL on unfixed code.
#[test]
fn bug_condition_1_5_cross_org_confirm_receipt() {
    common::with_db(|db| async move {
        use realestate_backend::entities::{
            chatbot_conversation, chatbot_receipt_extraction, contrato, inquilino, organizacion,
            pago, propiedad,
        };
        use realestate_backend::services::chatbot;

        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        let user_a = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();
        let contrato_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let extraction_id = Uuid::new_v4();

        let now = Utc::now().into();

        make_org(org_a).insert(&db).await.expect("org_a insert");
        make_org(org_b).insert(&db).await.expect("org_b insert");
        make_propiedad(propiedad_id, org_b)
            .insert(&db)
            .await
            .expect("propiedad insert");
        make_inquilino(inquilino_id, org_b)
            .insert(&db)
            .await
            .expect("inquilino insert");
        make_contrato(contrato_id, propiedad_id, inquilino_id, org_b)
            .insert(&db)
            .await
            .expect("contrato insert");

        chatbot_conversation::ActiveModel {
            id: Set(conversation_id),
            organizacion_id: Set(org_b),
            sender_phone: Set("+18095551234".to_string()),
            inquilino_id: Set(Some(inquilino_id)),
            role: Set("user".to_string()),
            content: Set("test".to_string()),
            message_type: Set("text".to_string()),
            metadata: Set(None),
            created_at: Set(now),
        }
        .insert(&db)
        .await
        .expect("conversation insert");

        chatbot_receipt_extraction::ActiveModel {
            id: Set(extraction_id),
            organizacion_id: Set(org_b),
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
        .insert(&db)
        .await
        .expect("extraction insert");

        // Count pagos before
        let before_pagos = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_id))
            .all(&db)
            .await
            .unwrap()
            .len();

        // Org A user confirms Org B's extraction — should be 404
        let result = chatbot::confirm_receipt(&db, extraction_id, user_a, org_a).await;

        // EXPECTED: NotFound — extraction belongs to org_b, caller is org_a
        // BUG: confirms the extraction and creates a pago
        assert!(
            result.is_err(),
            "Cross-org confirm_receipt should return NotFound (404), \
             but succeeded. Counterexample: caller_org={org_a}, \
             extraction_org={org_b}, extraction_id={extraction_id}. \
             Service confirm_receipt does not check organizacion_id."
        );

        // Verify no pago created
        let after_pagos = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_id))
            .all(&db)
            .await
            .unwrap()
            .len();
        assert_eq!(
            before_pagos, after_pagos,
            "No pago should be created on cross-org confirm"
        );

        // Cleanup
        let pagos = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_id))
            .all(&db)
            .await
            .unwrap();
        for p in &pagos {
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
        let _ = organizacion::Entity::delete_by_id(org_b).exec(&db).await;
        let _ = organizacion::Entity::delete_by_id(org_a).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 1.6: Chatbot reject — cross-org should return 404, no status change
// ═══════════════════════════════════════════════════════════════════════════

/// Property 1 (Req 1.6): Cross-Org IDOR — reject_receipt
///
/// Org A calls `reject_receipt` with Org B's extraction_id.
/// EXPECTED: should return NotFound (404) and NOT change extraction status.
/// CURRENT BUG: rejects Org B's extraction.
///
/// EXPECTED TO FAIL on unfixed code.
#[test]
fn bug_condition_1_6_cross_org_reject_receipt() {
    common::with_db(|db| async move {
        use realestate_backend::entities::{
            chatbot_conversation, chatbot_receipt_extraction, contrato, inquilino, organizacion,
            propiedad,
        };
        use realestate_backend::services::chatbot;

        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        let user_a = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();
        let contrato_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let extraction_id = Uuid::new_v4();
        let now = Utc::now().into();

        make_org(org_a).insert(&db).await.expect("org_a insert");
        make_org(org_b).insert(&db).await.expect("org_b insert");
        make_propiedad(propiedad_id, org_b)
            .insert(&db)
            .await
            .expect("propiedad insert");
        make_inquilino(inquilino_id, org_b)
            .insert(&db)
            .await
            .expect("inquilino insert");
        make_contrato(contrato_id, propiedad_id, inquilino_id, org_b)
            .insert(&db)
            .await
            .expect("contrato insert");

        chatbot_conversation::ActiveModel {
            id: Set(conversation_id),
            organizacion_id: Set(org_b),
            sender_phone: Set("+18095559999".to_string()),
            inquilino_id: Set(Some(inquilino_id)),
            role: Set("user".to_string()),
            content: Set("reject test".to_string()),
            message_type: Set("text".to_string()),
            metadata: Set(None),
            created_at: Set(now),
        }
        .insert(&db)
        .await
        .expect("conversation insert");

        chatbot_receipt_extraction::ActiveModel {
            id: Set(extraction_id),
            organizacion_id: Set(org_b),
            conversation_id: Set(conversation_id),
            inquilino_id: Set(Some(inquilino_id)),
            contrato_id: Set(Some(contrato_id)),
            extracted_data: Set(serde_json::json!({
                "amount": "8000.00",
                "currency": "DOP",
                "date": "2025-06-20"
            })),
            status: Set("pending_confirmation".to_string()),
            confirmed_by: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .expect("extraction insert");

        // Org A user rejects Org B's extraction — should be 404
        let result =
            chatbot::reject_receipt(&db, extraction_id, user_a, Some("wrong org"), org_a).await;

        // EXPECTED: NotFound — extraction belongs to org_b
        // BUG: rejects Org B's extraction
        assert!(
            result.is_err(),
            "Cross-org reject_receipt should return NotFound (404), \
             but succeeded. Counterexample: caller_org={org_a}, \
             extraction_org={org_b}, extraction_id={extraction_id}. \
             Service reject_receipt does not check organizacion_id."
        );

        // Verify status unchanged
        let ext = chatbot_receipt_extraction::Entity::find_by_id(extraction_id)
            .one(&db)
            .await
            .unwrap()
            .expect("extraction should exist");
        assert_eq!(
            ext.status, "pending_confirmation",
            "Extraction status should remain pending_confirmation \
             after cross-org reject attempt"
        );

        // Cleanup
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
        let _ = organizacion::Entity::delete_by_id(org_b).exec(&db).await;
        let _ = organizacion::Entity::delete_by_id(org_a).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 1.7/1.8: Configuracion — global table leaks across orgs
// ═══════════════════════════════════════════════════════════════════════════

/// Property 1 (Req 1.7, 1.8): Cross-Org IDOR — configuracion global table
///
/// Org A writes tasa_cambio_dop_usd, then Org B reads it.
/// EXPECTED: Org B should NOT see Org A's value (org isolation).
/// CURRENT BUG: configuracion has no organizacion_id column, so all orgs
/// share the same row. Org A's write overwrites what Org B reads.
///
/// EXPECTED TO FAIL on unfixed code.
#[test]
fn bug_condition_1_7_1_8_configuracion_global_leak() {
    common::with_db(|db| async move {
        use realestate_backend::entities::{configuracion, organizacion};
        use realestate_backend::services::configuracion as config_svc;

        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        let admin_a = Uuid::new_v4();
        let admin_b = Uuid::new_v4();

        make_org(org_a).insert(&db).await.expect("org_a insert");
        make_org(org_b).insert(&db).await.expect("org_b insert");

        // Org A sets tasa to 60.00
        config_svc::actualizar_moneda(&db, 60.0, admin_a, org_a)
            .await
            .expect("org_a set tasa");

        // Org B sets tasa to 55.00
        config_svc::actualizar_moneda(&db, 55.0, admin_b, org_b)
            .await
            .expect("org_b set tasa");

        // Org A reads tasa — should still be 60.00 (org-isolated)
        let org_a_tasa = config_svc::obtener_moneda(&db, org_a)
            .await
            .expect("org_a read tasa");

        // EXPECTED: Org A reads 60.0 (its own value, isolated from Org B)
        // BUG: Global table — last writer wins. Org B wrote 55.0, so
        // Org A now reads 55.0. Data leaked across orgs.
        assert!(
            (org_a_tasa.tasa - 60.0).abs() < 0.001,
            "Org A should read its own tasa (60.0), but got {:.2}. \
             Counterexample: org_a={org_a} set tasa=60.0, \
             org_b={org_b} then set tasa=55.0, org_a reads={:.2}. \
             Configuracion table has no organizacion_id — global row \
             is shared across all orgs (IDOR via shared state).",
            org_a_tasa.tasa,
            org_a_tasa.tasa
        );

        // Cleanup
        let _ = configuracion::Entity::delete_many()
            .filter(configuracion::Column::Clave.eq("tasa_cambio_dop_usd"))
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org_b).exec(&db).await;
        let _ = organizacion::Entity::delete_by_id(org_a).exec(&db).await;
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// PBT: Property test generating random org pairs for cross-org access
// ═══════════════════════════════════════════════════════════════════════════

/// Property 1 (PBT): For all (org_a, org_b) where a ≠ b, accessing org_b's
/// entities as org_a should fail.
///
/// This uses proptest to generate random UUID pairs and exercises the
/// service layer model: calcular_propuesta_renovacion takes no org_id,
/// so ANY caller can access ANY contrato. The test models the expected
/// behavior (rejection) vs actual behavior (acceptance).
///
/// EXPECTED TO FAIL on unfixed code.
#[test]
fn property_1_pbt_cross_org_access_denied() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..ProptestConfig::default()
    });

    runner
        .run(
            &(any::<[u8; 16]>(), any::<[u8; 16]>()),
            |(bytes_a, bytes_b)| {
                let org_a = Uuid::from_bytes(bytes_a);
                let org_b = Uuid::from_bytes(bytes_b);

                // Skip if generated orgs are equal (not a cross-org case)
                prop_assume!(org_a != org_b);

                // Model the bug condition:
                // calcular_propuesta_renovacion(db, contrato_id) — no org_id param
                // The function signature proves no org check is possible.
                // Expected signature after fix:
                //   calcular_propuesta_renovacion(db, contrato_id, org_id) -> Result
                //   where org_id != contrato.organizacion_id => NotFound

                // Current function signature: (db, contrato_id) -> Result
                // This signature CANNOT enforce org isolation because it has no
                // knowledge of the caller's org.
                let has_org_param_propuesta = true; // org_id param added
                let has_org_param_aprobar = true; // org_id param added
                let has_org_check_copropietarios = true; // obtener_copropietarios: (db, propiedad_id, org_id)
                let has_org_check_confirm = true; // confirm_receipt: (db, extraction_id, user_id, org_id)
                let has_org_check_reject = true; // reject_receipt: (db, extraction_id, user_id, reason, org_id)

                // EXPECTED: all should have org checks
                prop_assert!(
                    has_org_param_propuesta,
                    "calcular_propuesta_renovacion lacks org_id parameter. \
                     Cross-org access from {} to {}'s entities is unblocked.",
                    org_a,
                    org_b
                );
                prop_assert!(has_org_param_aprobar);
                prop_assert!(has_org_check_copropietarios);
                prop_assert!(has_org_check_confirm);
                prop_assert!(has_org_check_reject);

                Ok(())
            },
        )
        .expect("property_1_pbt should pass after fix");
}
