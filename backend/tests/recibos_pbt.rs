// Feature: spec-gap-remediation, Property 1: Cross-tenant receipt access never leaks
// **Validates: Requirements 1.2, 1.3, 1.5**
#![allow(clippy::needless_return)]

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

use crate::migrations;

mod pbt_async {
    use actix_web::test;
    use chrono::{NaiveDate, Utc};
    use realestate_backend::app::create_app;
    use realestate_backend::config::AppConfig;
    use realestate_backend::services::auth::{Claims, encode_jwt};
    use rust_decimal::Decimal;
    use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, Set};
    use sea_orm_migration::MigratorTrait;
    use uuid::Uuid;

    const JWT_SECRET: &str = "test_secret_key_that_is_long_enough_for_jwt";

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
                        .map_err(|e| format!("Failed to connect to database: {e}"))?;
                    super::migrations::Migrator::up(&db, None)
                        .await
                        .map_err(|e| format!("Failed to run migrations: {e}"))?;
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
            eprintln!("DATABASE_URL not set -- skipping DB integration test");
            return;
        }
        let _guard = crate::GLOBAL_DB_SERIAL
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some((rt, db)) = shared_rt_and_db() else {
            eprintln!("Database not reachable -- skipping DB integration test");
            return;
        };
        rt.block_on(f(db.clone()));
    }

    fn make_config() -> AppConfig {
        AppConfig {
            pool: realestate_backend::config::PoolConfig::default(),
            chatbot: realestate_backend::config::ChatbotEnvConfig::for_testing(),
            database_url: String::new(),
            jwt_secret: JWT_SECRET.to_string(),
            server_port: 0,
            cors_origin: None,
            ocr_service_token: None,
            metrics_token: None,
        }
    }

    async fn create_user(db: &DatabaseConnection, org_id: Uuid, rol: &str) -> Uuid {
        use realestate_backend::entities::usuario;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        usuario::ActiveModel {
            id: Set(id),
            nombre: Set(format!("Test {rol}")),
            email: Set(format!("{rol}+{id}@test.com")),
            password_hash: Set("not_used".to_string()),
            rol: Set(rol.to_string()),
            activo: Set(true),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
            password_changed_at: Set(now),
        }
        .insert(db)
        .await
        .expect("Failed to create test usuario");
        id
    }

    async fn make_token_for_org(db: &DatabaseConnection, org_id: Uuid, rol: &str) -> String {
        let user_id = create_user(db, org_id, rol).await;
        let claims = Claims {
            sub: user_id,
            email: format!("user-{user_id}@test.com"),
            rol: rol.to_string(),
            organizacion_id: org_id,
            jti: Uuid::new_v4(),
            iat: Utc::now().timestamp(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };
        encode_jwt(&claims, JWT_SECRET).unwrap()
    }

    async fn create_org(db: &DatabaseConnection) -> Uuid {
        use realestate_backend::entities::organizacion;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        organizacion::ActiveModel {
            id: Set(id),
            tipo: Set("persona_fisica".to_string()),
            nombre: Set(format!("Org PBT {id}")),
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
        .insert(db)
        .await
        .expect("Failed to create org");
        id
    }

    async fn create_propiedad(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::propiedad;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        propiedad::ActiveModel {
            id: Set(id),
            titulo: Set("Propiedad PBT Recibos".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle PBT 456".to_string()),
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
            valor_catastral: Set(None),
            exento_ipi: Set(false),
            motivo_exencion: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("Failed to create propiedad");
        id
    }

    async fn create_inquilino(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::inquilino;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        inquilino::ActiveModel {
            id: Set(id),
            nombre: Set("Inquilino".to_string()),
            apellido: Set("PBT".to_string()),
            email: Set(Some(format!("inquilino-{id}@test.com"))),
            telefono: Set(None),
            cedula: Set(format!("P{}", &id.simple().to_string()[..19])),
            contacto_emergencia: Set(None),
            notas: Set(None),
            documentos: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("Failed to create inquilino");
        id
    }

    async fn create_contrato(
        db: &DatabaseConnection,
        org_id: Uuid,
        propiedad_id: Uuid,
        inquilino_id: Uuid,
    ) -> Uuid {
        use realestate_backend::entities::contrato;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        contrato::ActiveModel {
            id: Set(id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            fecha_fin: Set(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            monto_mensual: Set(Decimal::new(2500000, 2)),
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
        .insert(db)
        .await
        .expect("Failed to create contrato");
        id
    }

    async fn create_pago(db: &DatabaseConnection, org_id: Uuid, contrato_id: Uuid) -> Uuid {
        use realestate_backend::entities::pago;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        pago::ActiveModel {
            id: Set(id),
            contrato_id: Set(contrato_id),
            monto: Set(Decimal::new(2500000, 2)),
            moneda: Set("DOP".to_string()),
            fecha_pago: Set(Some(NaiveDate::from_ymd_opt(2025, 3, 1).unwrap())),
            fecha_vencimiento: Set(NaiveDate::from_ymd_opt(2025, 3, 1).unwrap()),
            metodo_pago: Set(Some("transferencia".to_string())),
            estado: Set("pagado".to_string()),
            notas: Set(None),
            recargo: Set(None),
            organizacion_id: Set(org_id),
            monto_base: Set(None),
            monto_itbis: Set(None),
            monto_itbis_retenido: Set(None),
            ncf: Set(None),
            fecha_comprobante: Set(None),
            tipo_ncf: Set(None),
            es_parcial: Set(false),
            saldo_pendiente: Set(None),
            tipo_linea: Set("renta".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("Failed to create pago");
        id
    }

    async fn cleanup_pago(db: &DatabaseConnection, id: Uuid) {
        use realestate_backend::entities::pago;
        use sea_orm::EntityTrait;
        let _ = pago::Entity::delete_by_id(id).exec(db).await;
    }

    async fn cleanup_contrato(db: &DatabaseConnection, id: Uuid) {
        use realestate_backend::entities::contrato;
        use sea_orm::EntityTrait;
        let _ = contrato::Entity::delete_by_id(id).exec(db).await;
    }

    async fn cleanup_inquilino(db: &DatabaseConnection, id: Uuid) {
        use realestate_backend::entities::inquilino;
        use sea_orm::EntityTrait;
        let _ = inquilino::Entity::delete_by_id(id).exec(db).await;
    }

    async fn cleanup_propiedad(db: &DatabaseConnection, id: Uuid) {
        use realestate_backend::entities::propiedad;
        use sea_orm::EntityTrait;
        let _ = propiedad::Entity::delete_by_id(id).exec(db).await;
    }

    async fn cleanup_org(db: &DatabaseConnection, id: Uuid) {
        use realestate_backend::entities::organizacion;
        use sea_orm::EntityTrait;
        let _ = organizacion::Entity::delete_by_id(id).exec(db).await;
    }

    /// Property 1: Cross-tenant receipt access never leaks.
    /// For any pair of organizations (orgA, orgB) with orgA != orgB and any Pago
    /// belonging to orgB, a receipt request issued by a user whose JWT carries
    /// organizacion_id = orgA must produce HTTP 404 and the response body must
    /// not contain any PDF bytes.
    pub fn cross_tenant_receipt_never_leaks(rol: String) {
        with_db(|db| async move {
            let config = make_config();

            // Create two distinct organizations
            let org_a = create_org(&db).await;
            let org_b = create_org(&db).await;

            // Seed a pago under org_b
            let propiedad_id = create_propiedad(&db, org_b).await;
            let inquilino_id = create_inquilino(&db, org_b).await;
            let contrato_id = create_contrato(&db, org_b, propiedad_id, inquilino_id).await;
            let pago_id = create_pago(&db, org_b, contrato_id).await;

            // Create a token for a user in org_a (the attacker)
            let token = make_token_for_org(&db, org_a, &rol).await;

            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Request the receipt belonging to org_b as a user from org_a
            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/pagos/{pago_id}/recibo"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;

            // Assert: must be 404
            assert_eq!(
                resp.status().as_u16(),
                404,
                "Cross-tenant receipt request must return 404, got {}",
                resp.status()
            );

            // Assert: response body contains zero PDF bytes
            let body = test::read_body(resp).await;
            // PDF files start with %PDF magic bytes
            assert!(
                !body.starts_with(b"%PDF"),
                "Response body must not contain PDF bytes for cross-tenant access"
            );
            // Additionally verify body is empty or is a JSON error (not PDF content)
            assert!(
                body.is_empty() || serde_json::from_slice::<serde_json::Value>(&body).is_ok(),
                "Response body should be empty or a JSON error, not PDF content"
            );

            // Cleanup in reverse order of creation
            cleanup_pago(&db, pago_id).await;
            cleanup_contrato(&db, contrato_id).await;
            cleanup_inquilino(&db, inquilino_id).await;
            cleanup_propiedad(&db, propiedad_id).await;
            cleanup_org(&db, org_b).await;
            cleanup_org(&db, org_a).await;
        });
    }
}

// Feature: spec-gap-remediation, Property 1: Cross-tenant receipt access never leaks
// **Validates: Requirements 1.2, 1.3, 1.5**
#[test]
fn test_cross_tenant_receipt_access_never_leaks() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Generate random roles Ã¢â‚¬â€ any authenticated role should still get 404
    // when accessing another org's receipt
    let rol_strategy = prop_oneof![
        Just("admin".to_string()),
        Just("gerente".to_string()),
        Just("visualizador".to_string()),
    ];

    runner
        .run(&rol_strategy, |rol| {
            pbt_async::cross_tenant_receipt_never_leaks(rol);
            Ok(())
        })
        .unwrap();
}
