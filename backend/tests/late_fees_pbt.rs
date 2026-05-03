#![allow(clippy::needless_return)]
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

use crate::migrations;

// ── Custom Strategies ──────────────────────────────────────────────────

/// Random monto as `i64` cents (`1..99_999_999`) to build `Decimal` with scale 2.
fn monto_cents() -> impl Strategy<Value = i64> {
    1i64..99_999_999i64
}

/// Random porcentaje as `i64` hundredths (`0..10_000`) to build `Decimal` with scale 2.
fn porcentaje_hundredths() -> impl Strategy<Value = i64> {
    0i64..=10_000i64
}

/// Random valid `recargo_porcentaje` (0..100) as `i64` hundredths for contrato fields.
fn valid_recargo_porcentaje_hundredths() -> impl Strategy<Value = i64> {
    0i64..=10_000i64
}

/// Random valid `dias_gracia` (0..365).
fn valid_dias_gracia() -> impl Strategy<Value = i32> {
    0i32..=365i32
}

/// Random invalid porcentaje: either < 0 or > 100 (as `i64` hundredths).
fn invalid_porcentaje_hundredths() -> impl Strategy<Value = i64> {
    prop_oneof![
        -99_999i64..=-1i64,    // negative
        10_001i64..=99_999i64, // > 100
    ]
}

/// Random negative `dias_gracia`.
fn negative_dias_gracia() -> impl Strategy<Value = i32> {
    -365i32..=-1i32
}

/// Days offset for grace period tests: 1..30.
fn grace_days() -> impl Strategy<Value = i32> {
    1i32..=30i32
}

// ── Async helpers module ───────────────────────────────────────────────

mod pbt_async {
    use chrono::Utc;
    use realestate_backend::entities::{configuracion, contrato, pago};
    use realestate_backend::services::{configuracion as config_service, pagos, recargos};
    use rust_decimal::Decimal;
    use sea_orm::{
        ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait, Set,
    };
    use sea_orm_migration::MigratorTrait;
    use uuid::Uuid;

    fn shared_rt_and_db() -> Option<&'static (tokio::runtime::Runtime, DatabaseConnection)> {
        static SHARED: std::sync::OnceLock<
            Result<(tokio::runtime::Runtime, DatabaseConnection), String>,
        > = std::sync::OnceLock::new();
        SHARED
            .get_or_init(|| {
                dotenvy::dotenv().ok();
                let url = std::env::var("DATABASE_URL")
                    .map_err(|_| "DATABASE_URL not set".to_string())?;
                let rt =
                    tokio::runtime::Runtime::new().map_err(|e| format!("Runtime error: {e}"))?;
                let db = rt.block_on(async {
                    let mut opts = ConnectOptions::new(&url);
                    opts.max_connections(5)
                        .min_connections(1)
                        .connect_timeout(std::time::Duration::from_secs(30))
                        .idle_timeout(std::time::Duration::from_secs(60))
                        .acquire_timeout(std::time::Duration::from_secs(30));
                    let db = Database::connect(opts)
                        .await
                        .map_err(|e| format!("Failed to connect: {e}"))?;
                    super::migrations::Migrator::up(&db, None)
                        .await
                        .map_err(|e| format!("Failed migrations: {e}"))?;
                    Ok::<_, String>(db)
                })?;
                Ok((rt, db))
            })
            .as_ref()
            .ok()
    }

    fn with_db<F, Fut>(f: F)
    where
        F: FnOnce(DatabaseConnection) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        dotenvy::dotenv().ok();
        if std::env::var("DATABASE_URL").is_err() {
            eprintln!("⚠ DATABASE_URL not set – skipping PBT");
            return;
        }
        static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let Some((rt, db)) = shared_rt_and_db() else {
            eprintln!("⚠ DB not reachable – skipping PBT");
            return;
        };
        rt.block_on(f(db.clone()));
    }

    async fn create_org(db: &DatabaseConnection) -> Uuid {
        use realestate_backend::entities::organizacion;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        organizacion::ActiveModel {
            id: Set(id),
            tipo: Set("persona_fisica".to_string()),
            nombre: Set(format!("PBT LF Org {id}")),
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
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("create org");
        id
    }

    async fn create_propiedad(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::propiedad;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        propiedad::ActiveModel {
            id: Set(id),
            titulo: Set("Propiedad PBT LateFees".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle PBT LF 123".to_string()),
            ciudad: Set("Santo Domingo".to_string()),
            provincia: Set("Distrito Nacional".to_string()),
            tipo_propiedad: Set("apartamento".to_string()),
            habitaciones: Set(Some(2)),
            banos: Set(Some(1)),
            area_m2: Set(Some(Decimal::new(8000, 2))),
            precio: Set(Decimal::new(2500000, 2)),
            moneda: Set("DOP".to_string()),
            estado: Set("disponible".to_string()),
            imagenes: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("create propiedad");
        id
    }

    async fn create_inquilino(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::inquilino;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        inquilino::ActiveModel {
            id: Set(id),
            nombre: Set("PBT Inquilino".to_string()),
            apellido: Set("LateFees".to_string()),
            email: Set(Some(format!("inq+lf+{id}@pbt.com"))),
            telefono: Set(None),
            cedula: Set(format!("CED-LF-{id}")),
            contacto_emergencia: Set(None),
            notas: Set(None),
            documentos: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("create inquilino");
        id
    }

    async fn create_usuario(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::usuario;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        usuario::ActiveModel {
            id: Set(id),
            nombre: Set("PBT Usuario LF".to_string()),
            email: Set(format!("usr+lf+{id}@pbt.com")),
            password_hash: Set("hash_placeholder".to_string()),
            rol: Set("admin".to_string()),
            activo: Set(true),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("create usuario");
        id
    }

    async fn create_contrato_raw(
        db: &DatabaseConnection,
        propiedad_id: Uuid,
        inquilino_id: Uuid,
        org_id: Uuid,
        recargo_porcentaje: Option<Decimal>,
        dias_gracia: Option<i32>,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        contrato::ActiveModel {
            id: Set(id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            fecha_fin: Set(Utc::now().date_naive() + chrono::Duration::days(365)),
            monto_mensual: Set(Decimal::new(25000, 0)),
            deposito: Set(None),
            moneda: Set("DOP".to_string()),
            estado: Set("activo".to_string()),
            documentos: Set(None),
            organizacion_id: Set(org_id),
            estado_deposito: Set(None),
            fecha_cobro_deposito: Set(None),
            fecha_devolucion_deposito: Set(None),
            monto_retenido: Set(None),
            motivo_retencion: Set(None),
            recargo_porcentaje: Set(recargo_porcentaje),
            dias_gracia: Set(dias_gracia),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("create contrato raw");
        id
    }

    async fn insert_pago_raw(
        db: &DatabaseConnection,
        contrato_id: Uuid,
        org_id: Uuid,
        monto: Decimal,
        fecha_vencimiento: chrono::NaiveDate,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        pago::ActiveModel {
            id: Set(id),
            contrato_id: Set(contrato_id),
            monto: Set(monto),
            moneda: Set("DOP".to_string()),
            fecha_pago: Set(None),
            fecha_vencimiento: Set(fecha_vencimiento),
            metodo_pago: Set(None),
            estado: Set("pendiente".to_string()),
            notas: Set(None),
            recargo: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("insert pago raw");
        id
    }

    async fn cleanup_pago(db: &DatabaseConnection, id: Uuid) {
        let _ = pago::Entity::delete_by_id(id).exec(db).await;
    }

    async fn cleanup_contrato(db: &DatabaseConnection, id: Uuid) {
        // Delete pagos first (FK constraint)
        use sea_orm::{ColumnTrait, QueryFilter};
        let _ = pago::Entity::delete_many()
            .filter(pago::Column::ContratoId.eq(id))
            .exec(db)
            .await;
        let _ = contrato::Entity::delete_by_id(id).exec(db).await;
    }

    async fn clear_recargo_config(db: &DatabaseConnection) {
        let _ = configuracion::Entity::delete_by_id("recargo_porcentaje_defecto")
            .exec(db)
            .await;
    }

    // ── P1: Cálculo de recargo es correcto (pure function) ─────────────
    // No DB needed — tests calcular_recargo directly.
    // Handled inline in the test function below.

    // ── P2: Round-trip de campos de contrato ───────────────────────────
    pub fn p2(recargo_hundredths: i64, dias_gracia: i32) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;

            let recargo_porcentaje = Decimal::new(recargo_hundredths, 2);
            let contrato_id = create_contrato_raw(
                &db,
                propiedad_id,
                inquilino_id,
                org_id,
                Some(recargo_porcentaje),
                Some(dias_gracia),
            )
            .await;

            // Retrieve by ID and verify round-trip
            let resp = realestate_backend::services::contratos::get_by_id(&db, org_id, contrato_id)
                .await
                .expect("get contrato by id");

            assert_eq!(
                resp.recargo_porcentaje,
                Some(recargo_porcentaje),
                "recargo_porcentaje mismatch: expected {recargo_porcentaje}, got {:?}",
                resp.recargo_porcentaje
            );
            assert_eq!(
                resp.dias_gracia,
                Some(dias_gracia),
                "dias_gracia mismatch: expected {dias_gracia}, got {:?}",
                resp.dias_gracia
            );

            cleanup_contrato(&db, contrato_id).await;
        });
    }

    // ── P3: Resolución contrato tiene prioridad ────────────────────────
    pub fn p3(contrato_pct_hundredths: i64, org_pct_hundredths: i64) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_id = create_usuario(&db, org_id).await;
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;

            let contrato_pct = Decimal::new(contrato_pct_hundredths, 2);
            let org_pct = Decimal::new(org_pct_hundredths, 2);

            // Set org default
            config_service::actualizar_recargo_defecto(&db, org_pct, user_id)
                .await
                .expect("set org default");

            // Create contrato with its own recargo_porcentaje
            let contrato_id = create_contrato_raw(
                &db,
                propiedad_id,
                inquilino_id,
                org_id,
                Some(contrato_pct),
                None,
            )
            .await;

            let contrato_model = contrato::Entity::find_by_id(contrato_id)
                .one(&db)
                .await
                .expect("find contrato")
                .expect("contrato exists");

            let resolved = recargos::resolver_porcentaje_recargo(&db, &contrato_model)
                .await
                .expect("resolver");

            assert_eq!(
                resolved,
                Some(contrato_pct),
                "Expected contrato value {contrato_pct}, got {resolved:?}"
            );

            cleanup_contrato(&db, contrato_id).await;
            clear_recargo_config(&db).await;
        });
    }

    // ── P4: Resolución fallback a organización ─────────────────────────
    pub fn p4(org_pct_hundredths: i64) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_id = create_usuario(&db, org_id).await;
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;

            let org_pct = Decimal::new(org_pct_hundredths, 2);

            // Set org default
            config_service::actualizar_recargo_defecto(&db, org_pct, user_id)
                .await
                .expect("set org default");

            // Create contrato WITHOUT recargo_porcentaje
            let contrato_id = create_contrato_raw(
                &db,
                propiedad_id,
                inquilino_id,
                org_id,
                None, // NULL → fallback to org
                None,
            )
            .await;

            let contrato_model = contrato::Entity::find_by_id(contrato_id)
                .one(&db)
                .await
                .expect("find contrato")
                .expect("contrato exists");

            let resolved = recargos::resolver_porcentaje_recargo(&db, &contrato_model)
                .await
                .expect("resolver");

            assert_eq!(
                resolved,
                Some(org_pct),
                "Expected org fallback {org_pct}, got {resolved:?}"
            );

            cleanup_contrato(&db, contrato_id).await;
            clear_recargo_config(&db).await;
        });
    }

    // ── P5: Resolución ambos NULL produce None ─────────────────────────
    pub fn p5() {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;

            // Ensure no org default
            clear_recargo_config(&db).await;

            // Create contrato WITHOUT recargo_porcentaje
            let contrato_id =
                create_contrato_raw(&db, propiedad_id, inquilino_id, org_id, None, None).await;

            let contrato_model = contrato::Entity::find_by_id(contrato_id)
                .one(&db)
                .await
                .expect("find contrato")
                .expect("contrato exists");

            let resolved = recargos::resolver_porcentaje_recargo(&db, &contrato_model)
                .await
                .expect("resolver");

            assert_eq!(resolved, None, "Expected None, got {resolved:?}");

            cleanup_contrato(&db, contrato_id).await;
        });
    }

    // ── P6: Validación de rango de porcentaje ──────────────────────────
    pub fn p6(invalid_pct_hundredths: i64) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_id = create_usuario(&db, org_id).await;
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;

            let invalid_pct = Decimal::new(invalid_pct_hundredths, 2);

            // Test contrato create with invalid recargo_porcentaje
            let create_req = realestate_backend::models::contrato::CreateContratoRequest {
                propiedad_id,
                inquilino_id,
                fecha_inicio: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                fecha_fin: chrono::NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
                monto_mensual: Decimal::new(25000, 0),
                deposito: None,
                moneda: None,
                recargo_porcentaje: Some(invalid_pct),
                dias_gracia: None,
            };
            let result =
                realestate_backend::services::contratos::create(&db, create_req, user_id, org_id)
                    .await;
            assert!(
                result.is_err(),
                "Expected error for invalid recargo_porcentaje {invalid_pct} on create, got Ok"
            );

            // Test config update with invalid porcentaje
            let config_result =
                config_service::actualizar_recargo_defecto(&db, invalid_pct, user_id).await;
            assert!(
                config_result.is_err(),
                "Expected error for invalid config porcentaje {invalid_pct}, got Ok"
            );
        });
    }

    // ── P7: Validación de dias_gracia no negativo ──────────────────────
    pub fn p7(negative_dias: i32) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_id = create_usuario(&db, org_id).await;
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;

            // Test contrato create with negative dias_gracia
            let create_req = realestate_backend::models::contrato::CreateContratoRequest {
                propiedad_id,
                inquilino_id,
                fecha_inicio: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                fecha_fin: chrono::NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
                monto_mensual: Decimal::new(25000, 0),
                deposito: None,
                moneda: None,
                recargo_porcentaje: None,
                dias_gracia: Some(negative_dias),
            };
            let result =
                realestate_backend::services::contratos::create(&db, create_req, user_id, org_id)
                    .await;
            assert!(
                result.is_err(),
                "Expected error for negative dias_gracia {negative_dias} on create, got Ok"
            );
        });
    }

    // ── P8: Período de gracia retrasa atraso ───────────────────────────
    pub fn p8(dias_gracia: i32, monto_cents: i64) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;

            // Clear org config to isolate test
            clear_recargo_config(&db).await;

            let monto = Decimal::new(monto_cents, 2);

            // Create contrato with dias_gracia
            let contrato_id = create_contrato_raw(
                &db,
                propiedad_id,
                inquilino_id,
                org_id,
                Some(Decimal::new(500, 2)), // 5.00%
                Some(dias_gracia),
            )
            .await;

            // Pago 1: overdue by 1 day (within grace if dias_gracia >= 1)
            // fecha_vencimiento = today - 1 → effective_due = today - 1 + dias_gracia
            // If dias_gracia >= 1, effective_due >= today, so NOT overdue
            let yesterday = Utc::now().date_naive() - chrono::Duration::days(1);
            let pago_within_id = insert_pago_raw(&db, contrato_id, org_id, monto, yesterday).await;

            // Pago 2: overdue well beyond grace period
            // fecha_vencimiento = today - dias_gracia - 2
            let far_past =
                Utc::now().date_naive() - chrono::Duration::days(i64::from(dias_gracia) + 2);
            let pago_beyond_id = insert_pago_raw(&db, contrato_id, org_id, monto, far_past).await;

            // Run mark_overdue
            pagos::mark_overdue(&db).await.expect("mark_overdue");

            // Pago within grace should still be pendiente
            let within = pago::Entity::find_by_id(pago_within_id)
                .one(&db)
                .await
                .expect("find pago within")
                .expect("pago within exists");
            assert_eq!(
                within.estado, "pendiente",
                "Pago within grace period (dias_gracia={dias_gracia}) should be pendiente, got {}",
                within.estado
            );

            // Pago beyond grace should be atrasado
            let beyond = pago::Entity::find_by_id(pago_beyond_id)
                .one(&db)
                .await
                .expect("find pago beyond")
                .expect("pago beyond exists");
            assert_eq!(
                beyond.estado, "atrasado",
                "Pago beyond grace period (dias_gracia={dias_gracia}) should be atrasado, got {}",
                beyond.estado
            );

            // Cleanup
            cleanup_pago(&db, pago_within_id).await;
            cleanup_pago(&db, pago_beyond_id).await;
            cleanup_contrato(&db, contrato_id).await;
        });
    }

    // ── P9: Recargo se calcula al marcar atrasado ──────────────────────
    pub fn p9(monto_cents: i64, pct_hundredths: i64) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;

            // Clear org config to isolate test
            clear_recargo_config(&db).await;

            let monto = Decimal::new(monto_cents, 2);
            let porcentaje = Decimal::new(pct_hundredths, 2);

            // Create contrato with known porcentaje, no grace period
            let contrato_id = create_contrato_raw(
                &db,
                propiedad_id,
                inquilino_id,
                org_id,
                Some(porcentaje),
                None, // no grace → overdue immediately after fecha_vencimiento
            )
            .await;

            // Create overdue pago (5 days past due)
            let overdue_date = Utc::now().date_naive() - chrono::Duration::days(5);
            let pago_id = insert_pago_raw(&db, contrato_id, org_id, monto, overdue_date).await;

            // Run mark_overdue
            pagos::mark_overdue(&db).await.expect("mark_overdue");

            // Verify recargo
            let updated = pago::Entity::find_by_id(pago_id)
                .one(&db)
                .await
                .expect("find pago")
                .expect("pago exists");

            assert_eq!(updated.estado, "atrasado");

            let expected_recargo = recargos::calcular_recargo(monto, porcentaje);
            assert_eq!(
                updated.recargo,
                Some(expected_recargo),
                "Recargo mismatch: monto={monto}, porcentaje={porcentaje}, expected={expected_recargo}, got={:?}",
                updated.recargo
            );

            // Cleanup
            cleanup_pago(&db, pago_id).await;
            cleanup_contrato(&db, contrato_id).await;
        });
    }

    // ── P10: Recargo con porcentaje 0 produce 0.00 ─────────────────────
    pub fn p10(monto_cents: i64) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;

            // Clear org config to isolate test
            clear_recargo_config(&db).await;

            let monto = Decimal::new(monto_cents, 2);

            // Create contrato with porcentaje 0
            let contrato_id = create_contrato_raw(
                &db,
                propiedad_id,
                inquilino_id,
                org_id,
                Some(Decimal::ZERO),
                None,
            )
            .await;

            // Create overdue pago
            let overdue_date = Utc::now().date_naive() - chrono::Duration::days(5);
            let pago_id = insert_pago_raw(&db, contrato_id, org_id, monto, overdue_date).await;

            // Run mark_overdue
            pagos::mark_overdue(&db).await.expect("mark_overdue");

            // Verify recargo is exactly 0.00, NOT NULL
            let updated = pago::Entity::find_by_id(pago_id)
                .one(&db)
                .await
                .expect("find pago")
                .expect("pago exists");

            assert_eq!(updated.estado, "atrasado");
            assert_eq!(
                updated.recargo,
                Some(Decimal::new(0, 2)),
                "Recargo with 0% should be Some(0.00), got {:?}",
                updated.recargo
            );

            // Cleanup
            cleanup_pago(&db, pago_id).await;
            cleanup_contrato(&db, contrato_id).await;
        });
    }
} // end pbt_async

// ── Test functions ─────────────────────────────────────────────────────

// Feature: late-fees, Property 1: Cálculo de recargo es correcto
// **Validates: Requirements 5.1, 5.3**
#[test]
fn test_calculo_recargo_correcto() {
    use realestate_backend::services::recargos::calcular_recargo;
    use rust_decimal::Decimal;

    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(
            &(monto_cents(), porcentaje_hundredths()),
            |(m_cents, p_hundredths)| {
                let monto = Decimal::new(m_cents, 2);
                let porcentaje = Decimal::new(p_hundredths, 2);

                let result = calcular_recargo(monto, porcentaje);
                let expected = (monto * porcentaje / Decimal::from(100)).round_dp(2);

                prop_assert_eq!(
                    result,
                    expected,
                    "calcular_recargo({}, {}) = {}, expected {}",
                    monto,
                    porcentaje,
                    result,
                    expected
                );
                Ok(())
            },
        )
        .unwrap();
}

// Feature: late-fees, Property 2: Round-trip de campos de contrato
// **Validates: Requirements 1.1, 1.2, 1.7**
#[test]
fn test_contrato_recargo_roundtrip() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(
            &(valid_recargo_porcentaje_hundredths(), valid_dias_gracia()),
            |(pct_hundredths, dias)| {
                pbt_async::p2(pct_hundredths, dias);
                Ok(())
            },
        )
        .unwrap();
}

// Feature: late-fees, Property 3: Resolución contrato tiene prioridad
// **Validates: Requirements 4.1**
#[test]
fn test_resolucion_contrato_prioridad() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(
            &(
                valid_recargo_porcentaje_hundredths(),
                valid_recargo_porcentaje_hundredths(),
            ),
            |(contrato_pct, org_pct)| {
                pbt_async::p3(contrato_pct, org_pct);
                Ok(())
            },
        )
        .unwrap();
}

// Feature: late-fees, Property 4: Resolución fallback a organización
// **Validates: Requirements 4.2**
#[test]
fn test_resolucion_fallback_org() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&valid_recargo_porcentaje_hundredths(), |org_pct| {
            pbt_async::p4(org_pct);
            Ok(())
        })
        .unwrap();
}

// Feature: late-fees, Property 5: Resolución ambos NULL produce None
// **Validates: Requirements 4.3, 5.4**
#[test]
fn test_resolucion_ambos_null() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&Just(()), |()| {
            pbt_async::p5();
            Ok(())
        })
        .unwrap();
}

// Feature: late-fees, Property 6: Validación de rango de porcentaje
// **Validates: Requirements 1.5, 3.2**
#[test]
fn test_validacion_rango_porcentaje() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&invalid_porcentaje_hundredths(), |invalid_pct| {
            pbt_async::p6(invalid_pct);
            Ok(())
        })
        .unwrap();
}

// Feature: late-fees, Property 7: Validación de dias_gracia no negativo
// **Validates: Requirements 1.6**
#[test]
fn test_validacion_dias_gracia_negativo() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&negative_dias_gracia(), |neg_dias| {
            pbt_async::p7(neg_dias);
            Ok(())
        })
        .unwrap();
}

// Feature: late-fees, Property 8: Período de gracia retrasa el marcado de atraso
// **Validates: Requirements 6.1, 6.2, 6.3**
#[test]
fn test_periodo_gracia_retrasa_atraso() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&(grace_days(), monto_cents()), |(dias, monto)| {
            pbt_async::p8(dias, monto);
            Ok(())
        })
        .unwrap();
}

// Feature: late-fees, Property 9: Recargo se calcula al marcar atrasado
// **Validates: Requirements 5.1, 5.2**
#[test]
fn test_recargo_al_marcar_atrasado() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&(monto_cents(), porcentaje_hundredths()), |(monto, pct)| {
            pbt_async::p9(monto, pct);
            Ok(())
        })
        .unwrap();
}

// Feature: late-fees, Property 10: Recargo con porcentaje 0 produce 0.00
// **Validates: Requirements 5.5**
#[test]
fn test_recargo_porcentaje_cero() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&monto_cents(), |monto| {
            pbt_async::p10(monto);
            Ok(())
        })
        .unwrap();
}
