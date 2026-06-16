// Feature: ley-85-25-compliance, Property 2: Preservation
//
// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**
//
// These tests confirm that existing valid operations remain unchanged.
// They MUST PASS on unfixed code (baseline behavior to preserve).
#![allow(clippy::needless_return, unused_imports, dead_code)]

use crate::common;
use chrono::{Duration, NaiveDate, Utc};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use realestate_backend::entities::{
    configuracion, contrato, desahucio, inquilino, organizacion, propiedad,
};
use realestate_backend::models::contrato::{
    CambiarEstadoDepositoRequest, CreateContratoRequest, RenovarContratoRequest,
};
use realestate_backend::models::desahucio::{CreateDesahucioRequest, UpdateDesahucioRequest};
use realestate_backend::services::contratos::validar_transicion_deposito;
use realestate_backend::services::desahucios::validate_estado_transition;
use realestate_backend::services::{contratos, desahucios};

// ═══ Strategies ═══════════════════════════════════════════════════════════════

fn monto_mensual_strategy() -> impl Strategy<Value = Decimal> {
    (5000i64..200_000i64).prop_map(|v| Decimal::new(v, 2))
}

fn deposit_within_1x(monto: Decimal) -> impl Strategy<Value = Decimal> {
    let max_cents = (monto * Decimal::from(100))
        .to_string()
        .parse::<i64>()
        .unwrap_or(500_000);
    (100i64..=max_cents).prop_map(|v| Decimal::new(v, 2))
}

fn ipc_percentage_strategy() -> impl Strategy<Value = Decimal> {
    (100i64..1000i64).prop_map(|v| Decimal::new(v, 2))
}

fn renewal_within_cap(monto_actual: Decimal, ipc_cap: Decimal) -> Decimal {
    let max = realestate_backend::services::ipc::calcular_monto_maximo(monto_actual, ipc_cap);
    max
}

fn valid_deposit_transitions() -> impl Strategy<Value = Vec<&'static str>> {
    prop_oneof![
        Just(vec!["cobrado"]),
        Just(vec!["cobrado", "devuelto"]),
        Just(vec!["cobrado", "retenido"]),
    ]
}

fn desahucio_valid_transitions() -> impl Strategy<Value = (&'static str, &'static str)> {
    prop_oneof![
        Just(("iniciado", "en_progreso")),
        Just(("iniciado", "completado")),
        Just(("en_progreso", "completado")),
    ]
}

// ═══ Helpers ══════════════════════════════════════════════════════════════════

fn make_org(org_id: Uuid) -> organizacion::ActiveModel {
    let now = Utc::now().into();
    organizacion::ActiveModel {
        id: Set(org_id),
        tipo: Set("persona_fisica".to_string()),
        nombre: Set(format!("Ley85 Preservation Org {org_id}")),
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

fn make_propiedad(id: Uuid, org_id: Uuid, estado: &str) -> propiedad::ActiveModel {
    let now = Utc::now().into();
    propiedad::ActiveModel {
        id: Set(id),
        titulo: Set("Preservation Test Prop".to_string()),
        descripcion: Set(None),
        direccion: Set("Calle Preservación 456".to_string()),
        ciudad: Set("Santo Domingo".to_string()),
        provincia: Set("Distrito Nacional".to_string()),
        tipo_propiedad: Set("apartamento".to_string()),
        habitaciones: Set(None),
        banos: Set(None),
        area_m2: Set(None),
        precio: Set(Decimal::new(30000_00, 2)),
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

fn make_inquilino(id: Uuid, org_id: Uuid) -> inquilino::ActiveModel {
    let now = Utc::now().into();
    inquilino::ActiveModel {
        id: Set(id),
        nombre: Set("Preservation".to_string()),
        apellido: Set("Test".to_string()),
        cedula: Set(format!("PRES-LEY85-{}", Uuid::new_v4())),
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

async fn insert_ipc_config(db: &sea_orm::DatabaseConnection, valor_ipc: Decimal) {
    use realestate_backend::models::ipc::IpcData;
    let data = IpcData {
        valor_ipc,
        fecha_efectiva: Utc::now().date_naive(),
        ultimo_fetch_exitoso: Utc::now(),
    };
    let valor_json = serde_json::to_value(&data).unwrap();
    let model = configuracion::ActiveModel {
        clave: Set("ipc_banco_central".to_string()),
        valor: Set(valor_json),
        updated_at: Set(Utc::now().into()),
        updated_by: Set(None),
    };
    let existing = configuracion::Entity::find_by_id("ipc_banco_central")
        .one(db)
        .await
        .unwrap();
    if existing.is_some() {
        model.update(db).await.unwrap();
    } else {
        model.insert(db).await.unwrap();
    }
}

// ═══ Test 2a: Deposit ≤ monto_mensual → contract creation succeeds ═══════════
// **Validates: Requirements 3.1**

#[test]
fn preservation_2a_deposit_within_1x_accepted() {
    common::with_db(|_db| async move {
        let mut runner = TestRunner::new(ProptestConfig {
            cases: crate::pbt_cases(),
            ..ProptestConfig::default()
        });

        let strat = monto_mensual_strategy().prop_flat_map(|monto| {
            let max_cents = (monto * Decimal::from(100))
                .to_string()
                .parse::<i64>()
                .unwrap_or(500_000);
            (
                Just(monto),
                (100i64..=max_cents).prop_map(|v| Decimal::new(v, 2)),
            )
        });

        runner
            .run(&strat, |(monto_mensual, deposito)| {
                assert!(
                    deposito <= monto_mensual,
                    "Strategy bug: generated deposit {deposito} > monto {monto_mensual}"
                );
                Ok(())
            })
            .unwrap();
    });

    common::with_db(|db| async move {
        let org_id = Uuid::new_v4();
        let usuario_id = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();

        make_org(org_id).insert(&db).await.unwrap();
        make_propiedad(propiedad_id, org_id, "disponible")
            .insert(&db)
            .await
            .unwrap();
        make_inquilino(inquilino_id, org_id)
            .insert(&db)
            .await
            .unwrap();

        let monto_mensual = Decimal::new(15000_00, 2);
        let deposito = Decimal::new(10000_00, 2);

        let input = CreateContratoRequest {
            propiedad_id,
            inquilino_id,
            fecha_inicio: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            fecha_fin: NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
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
            "Deposit {deposito} ≤ monto_mensual {monto_mensual} should be accepted, got: {:?}",
            result.err()
        );

        let _ = contrato::Entity::delete_many()
            .filter(contrato::Column::OrganizacionId.eq(org_id))
            .exec(&db)
            .await;
        let _ = inquilino::Entity::delete_by_id(inquilino_id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org_id).exec(&db).await;
    });
}

// ═══ Test 2b: IPC configured, renewal within cap → succeeds ══════════════════
// **Validates: Requirements 3.3**

#[test]
fn preservation_2b_ipc_configured_renewal_within_cap_succeeds() {
    common::with_db(|db| async move {
        let org_id = Uuid::new_v4();
        let usuario_id = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();
        let contrato_id = Uuid::new_v4();
        let now = Utc::now();

        make_org(org_id).insert(&db).await.unwrap();
        make_propiedad(propiedad_id, org_id, "ocupada")
            .insert(&db)
            .await
            .unwrap();
        make_inquilino(inquilino_id, org_id)
            .insert(&db)
            .await
            .unwrap();

        let monto_mensual = Decimal::new(20000_00, 2);

        contrato::ActiveModel {
            id: Set(contrato_id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            fecha_fin: Set(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
            monto_mensual: Set(monto_mensual),
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
        .unwrap();

        let ipc_value = Decimal::new(500, 2); // 5%
        insert_ipc_config(&db, ipc_value).await;

        let max_allowed =
            realestate_backend::services::ipc::calcular_monto_maximo(monto_mensual, ipc_value);
        let new_monto = max_allowed; // exactly at cap

        let input = RenovarContratoRequest {
            fecha_fin: NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            monto_mensual: new_monto,
            dia_vencimiento: None,
        };

        let result = contratos::renovar(&db, org_id, contrato_id, input, usuario_id).await;
        assert!(
            result.is_ok(),
            "Renewal within IPC cap should succeed, got: {:?}",
            result.err()
        );

        let _ = contrato::Entity::delete_many()
            .filter(contrato::Column::OrganizacionId.eq(org_id))
            .exec(&db)
            .await;
        let _ = inquilino::Entity::delete_by_id(inquilino_id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org_id).exec(&db).await;
    });
}

// ═══ Test 2c: IPC configured, renewal exceeds cap → ValidationWithFields ═════
// **Validates: Requirements 3.4**

#[test]
fn preservation_2c_ipc_configured_renewal_exceeds_cap_rejected() {
    common::with_db(|db| async move {
        let org_id = Uuid::new_v4();
        let usuario_id = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();
        let contrato_id = Uuid::new_v4();
        let now = Utc::now();

        make_org(org_id).insert(&db).await.unwrap();
        make_propiedad(propiedad_id, org_id, "ocupada")
            .insert(&db)
            .await
            .unwrap();
        make_inquilino(inquilino_id, org_id)
            .insert(&db)
            .await
            .unwrap();

        let monto_mensual = Decimal::new(20000_00, 2);

        contrato::ActiveModel {
            id: Set(contrato_id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            fecha_fin: Set(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
            monto_mensual: Set(monto_mensual),
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
        .unwrap();

        let ipc_value = Decimal::new(500, 2); // 5%
        insert_ipc_config(&db, ipc_value).await;

        let max_allowed =
            realestate_backend::services::ipc::calcular_monto_maximo(monto_mensual, ipc_value);
        let new_monto = max_allowed + Decimal::new(1_00, 2); // exceeds cap by 1.00

        let input = RenovarContratoRequest {
            fecha_fin: NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            monto_mensual: new_monto,
            dia_vencimiento: None,
        };

        let result = contratos::renovar(&db, org_id, contrato_id, input, usuario_id).await;
        assert!(
            result.is_err(),
            "Renewal exceeding IPC cap should be rejected"
        );

        let err = result.unwrap_err();
        let err_str = format!("{err:?}");
        assert!(
            err_str.contains("maxAllowed") || err_str.contains("ValidationWithFields"),
            "Error should contain maxAllowed field, got: {err_str}"
        );

        let _ = contrato::Entity::delete_many()
            .filter(contrato::Column::OrganizacionId.eq(org_id))
            .exec(&db)
            .await;
        let _ = inquilino::Entity::delete_by_id(inquilino_id)
            .exec(&db)
            .await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id)
            .exec(&db)
            .await;
        let _ = organizacion::Entity::delete_by_id(org_id).exec(&db).await;
    });
}

// ═══ Test 2d: Valid deposit state transitions succeed ═════════════════════════
// **Validates: Requirements 3.2**

#[test]
fn preservation_2d_valid_deposit_transitions_succeed() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..ProptestConfig::default()
    });

    runner
        .run(&valid_deposit_transitions(), |transitions| {
            let mut current = "pendiente";
            for target in &transitions {
                let result = validar_transicion_deposito(current, target);
                prop_assert!(
                    result.is_ok(),
                    "Transition {} → {} should succeed, got: {:?}",
                    current,
                    target,
                    result.err()
                );
                current = target;
            }
            Ok(())
        })
        .unwrap();
}

// ═══ Test 2e: Desahucio valid transitions (graph edges) succeed ═══════════════
// **Validates: Requirements 3.5**

#[test]
fn preservation_2e_desahucio_valid_transitions_succeed() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..ProptestConfig::default()
    });

    runner
        .run(&desahucio_valid_transitions(), |(from, to)| {
            let result = validate_estado_transition(from, to);
            prop_assert!(
                result.is_ok(),
                "Desahucio transition {} → {} should be valid, got: {:?}",
                from,
                to,
                result.err()
            );
            Ok(())
        })
        .unwrap();
}

// ═══ Test 2f: Desahucio creation initializes correctly ════════════════════════
// **Validates: Requirements 3.6**

#[test]
fn preservation_2f_desahucio_creation_initializes_correctly() {
    common::with_db(|db| async move {
        let org_id = Uuid::new_v4();
        let usuario_id = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let inquilino_id = Uuid::new_v4();
        let contrato_id = Uuid::new_v4();
        let now = Utc::now();

        make_org(org_id).insert(&db).await.unwrap();
        make_propiedad(propiedad_id, org_id, "ocupada")
            .insert(&db)
            .await
            .unwrap();
        make_inquilino(inquilino_id, org_id)
            .insert(&db)
            .await
            .unwrap();

        contrato::ActiveModel {
            id: Set(contrato_id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            fecha_fin: Set(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
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
        .unwrap();

        let input = CreateDesahucioRequest {
            contrato_id,
            motivo: "No pago de renta".to_string(),
        };

        let result = desahucios::create(&db, input, usuario_id, org_id).await;
        assert!(
            result.is_ok(),
            "Desahucio creation should succeed: {:?}",
            result.err()
        );

        let response = result.unwrap();
        assert_eq!(response.estado, "iniciado");
        assert_eq!(response.fecha_inicio, now.date_naive());

        let _ = desahucio::Entity::delete_many()
            .filter(desahucio::Column::OrganizacionId.eq(org_id))
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
