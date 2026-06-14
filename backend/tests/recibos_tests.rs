#![allow(clippy::needless_return)]

use crate::migrations;

mod db_async {
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

    fn make_token(user_id: Uuid, org_id: Uuid, rol: &str) -> String {
        let claims = Claims {
            sub: user_id,
            email: format!("user-{user_id}@test.com"),
            rol: rol.to_string(),
            organizacion_id: org_id,
            jti: Uuid::new_v4(),
            iat: Utc::now().timestamp(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iss: "realestate-api".to_string(),
            aud: "realestate-api".to_string(),
        };
        encode_jwt(&claims, JWT_SECRET).unwrap()
    }

    async fn make_token_for_org(db: &DatabaseConnection, org_id: Uuid, rol: &str) -> String {
        let user_id = create_user(db, org_id, rol).await;
        make_token(user_id, org_id, rol)
    }

    fn make_app_data()
    -> actix_web::web::Data<realestate_backend::services::ocr_preview::PreviewStore> {
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new())
    }

    async fn create_org(db: &DatabaseConnection) -> Uuid {
        use realestate_backend::entities::organizacion;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        organizacion::ActiveModel {
            id: Set(id),
            tipo: Set("persona_fisica".to_string()),
            nombre: Set(format!("Org Recibos {id}")),
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
            titulo: Set("Propiedad Recibos Test".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle Recibos 123".to_string()),
            ciudad: Set("Santo Domingo".to_string()),
            provincia: Set("Distrito Nacional".to_string()),
            tipo_propiedad: Set("apartamento".to_string()),
            habitaciones: Set(Some(2)),
            banos: Set(Some(1)),
            area_m2: Set(Some(Decimal::new(8000, 2))),
            precio: Set(Decimal::new(2500000, 2)),
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
            apellido: Set("Recibos".to_string()),
            email: Set(Some(format!("inquilino-{id}@test.com"))),
            telefono: Set(None),
            cedula: Set(format!("R{}", &id.simple().to_string()[..19])),
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

    async fn cleanup(
        db: &DatabaseConnection,
        pago_id: Uuid,
        contrato_id: Uuid,
        inquilino_id: Uuid,
        propiedad_id: Uuid,
        org_id: Uuid,
    ) {
        use realestate_backend::entities::{contrato, inquilino, organizacion, pago, propiedad};
        use sea_orm::EntityTrait;
        let _ = pago::Entity::delete_by_id(pago_id).exec(db).await;
        let _ = contrato::Entity::delete_by_id(contrato_id).exec(db).await;
        let _ = inquilino::Entity::delete_by_id(inquilino_id).exec(db).await;
        let _ = propiedad::Entity::delete_by_id(propiedad_id).exec(db).await;
        let _ = organizacion::Entity::delete_by_id(org_id).exec(db).await;
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Task 1.3: Cross-tenant receipt access returns 404 Ã¢â€â‚¬Ã¢â€â‚¬

    pub fn cross_tenant_receipt_returns_404() {
        with_db(|db| async move {
            let config = make_config();

            // Create two distinct organizations
            let org_a = create_org(&db).await;
            let org_b = create_org(&db).await;

            // Seed a paid pago under org_b
            let prop_b = create_propiedad(&db, org_b).await;
            let inq_b = create_inquilino(&db, org_b).await;
            let contrato_b = create_contrato(&db, org_b, prop_b, inq_b).await;
            let pago_b = create_pago(&db, org_b, contrato_b).await;

            let app = test::init_service(create_app(db.clone(), config, make_app_data())).await;

            // Request org_b's pago receipt as a user from org_a
            let token_a = make_token_for_org(&db, org_a, "admin").await;
            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/pagos/{pago_b}/recibo"))
                .insert_header(("Authorization", format!("Bearer {token_a}")))
                .to_request();
            let resp = test::call_service(&app, req).await;

            assert_eq!(
                resp.status().as_u16(),
                404,
                "Cross-tenant receipt request must return 404"
            );

            // Body must not contain PDF bytes
            let body = test::read_body(resp).await;
            assert!(
                !body.starts_with(b"%PDF"),
                "Response must not contain PDF bytes for cross-tenant access"
            );

            // Verify same-org access works (org_b user can get the receipt)
            let token_b = make_token_for_org(&db, org_b, "admin").await;
            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/pagos/{pago_b}/recibo"))
                .insert_header(("Authorization", format!("Bearer {token_b}")))
                .to_request();
            let resp = test::call_service(&app, req).await;

            assert_eq!(
                resp.status().as_u16(),
                200,
                "Same-org receipt request must return 200"
            );
            let body = test::read_body(resp).await;
            assert!(
                !body.is_empty(),
                "Same-org receipt should return PDF content"
            );

            // Cleanup
            cleanup(&db, pago_b, contrato_b, inq_b, prop_b, org_b).await;
            use realestate_backend::entities::organizacion;
            use sea_orm::EntityTrait;
            let _ = organizacion::Entity::delete_by_id(org_a).exec(&db).await;
        });
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Cross-tenant access returns 404 for all roles Ã¢â€â‚¬Ã¢â€â‚¬

    pub fn cross_tenant_receipt_404_all_roles() {
        with_db(|db| async move {
            let config = make_config();

            let org_a = create_org(&db).await;
            let org_b = create_org(&db).await;

            let prop_b = create_propiedad(&db, org_b).await;
            let inq_b = create_inquilino(&db, org_b).await;
            let contrato_b = create_contrato(&db, org_b, prop_b, inq_b).await;
            let pago_b = create_pago(&db, org_b, contrato_b).await;

            let app = test::init_service(create_app(db.clone(), config, make_app_data())).await;

            // All roles from org_a should get 404 when accessing org_b's receipt
            for role in &["admin", "gerente", "visualizador"] {
                let token = make_token_for_org(&db, org_a, role).await;
                let req = test::TestRequest::get()
                    .uri(&format!("/api/v1/pagos/{pago_b}/recibo"))
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .to_request();
                let resp = test::call_service(&app, req).await;

                assert_eq!(
                    resp.status().as_u16(),
                    404,
                    "Role '{role}' from org_a must get 404 for org_b's receipt"
                );
            }

            // Cleanup
            cleanup(&db, pago_b, contrato_b, inq_b, prop_b, org_b).await;
            use realestate_backend::entities::organizacion;
            use sea_orm::EntityTrait;
            let _ = organizacion::Entity::delete_by_id(org_a).exec(&db).await;
        });
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Non-existent pago returns 404 (not a server error) Ã¢â€â‚¬Ã¢â€â‚¬

    pub fn nonexistent_pago_returns_404() {
        with_db(|db| async move {
            let config = make_config();
            let org = create_org(&db).await;

            let app = test::init_service(create_app(db.clone(), config, make_app_data())).await;

            let token = make_token_for_org(&db, org, "admin").await;
            let fake_pago = Uuid::new_v4();
            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/pagos/{fake_pago}/recibo"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;

            assert_eq!(
                resp.status().as_u16(),
                404,
                "Non-existent pago must return 404"
            );

            // Cleanup
            use realestate_backend::entities::organizacion;
            use sea_orm::EntityTrait;
            let _ = organizacion::Entity::delete_by_id(org).exec(&db).await;
        });
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Task 1.3: Cross-tenant access logs structured warning Ã¢â€â‚¬Ã¢â€â‚¬

    pub fn cross_tenant_receipt_logs_structured_warning() {
        with_db(|db| async move {
            let config = make_config();

            // Create two distinct organizations
            let org_a = create_org(&db).await;
            let org_b = create_org(&db).await;

            // Seed a paid pago under org_b
            let prop_b = create_propiedad(&db, org_b).await;
            let inq_b = create_inquilino(&db, org_b).await;
            let contrato_b = create_contrato(&db, org_b, prop_b, inq_b).await;
            let pago_b = create_pago(&db, org_b, contrato_b).await;

            // Set up a tracing subscriber that captures logs to a buffer
            use std::io;
            use std::sync::{Arc, Mutex};

            let log_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
            let buf_clone = log_buf.clone();

            #[derive(Clone)]
            struct TestWriter(Arc<Mutex<Vec<u8>>>);

            impl io::Write for TestWriter {
                fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                    self.0
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .extend_from_slice(buf);
                    Ok(buf.len())
                }
                fn flush(&mut self) -> io::Result<()> {
                    Ok(())
                }
            }

            impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for TestWriter {
                type Writer = Self;
                fn make_writer(&'a self) -> Self::Writer {
                    self.clone()
                }
            }

            let writer = TestWriter(buf_clone);
            let subscriber = tracing_subscriber::fmt()
                .with_writer(writer)
                .with_target(true)
                .with_level(true)
                .without_time()
                .with_env_filter("security.cross_tenant=warn")
                .finish();

            // Use the subscriber as the default for this scope
            let _guard = tracing::subscriber::set_default(subscriber);

            let app = test::init_service(create_app(db.clone(), config, make_app_data())).await;

            // Request org_b's pago receipt as a user from org_a
            let token_a = make_token_for_org(&db, org_a, "admin").await;
            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/pagos/{pago_b}/recibo"))
                .insert_header(("Authorization", format!("Bearer {token_a}")))
                .to_request();
            let resp = test::call_service(&app, req).await;

            // Verify 404 response
            assert_eq!(
                resp.status().as_u16(),
                404,
                "Cross-tenant receipt request must return 404"
            );

            // Verify empty body (no PDF bytes leaked)
            let body = test::read_body(resp).await;
            assert!(
                body.is_empty() || !body.starts_with(b"%PDF"),
                "Response must not contain PDF bytes for cross-tenant access"
            );

            // Verify structured warning was logged
            let logs = {
                let lock = log_buf.lock().unwrap_or_else(|e| e.into_inner());
                String::from_utf8_lossy(&lock).to_string()
            };

            assert!(
                logs.contains("security.cross_tenant"),
                "Expected structured warning with target 'security.cross_tenant' to be logged. Got: {logs}"
            );
            assert!(
                logs.contains("Intento de acceso a recibo fuera de la organiza"),
                "Expected Spanish warning message in log. Got: {logs}"
            );
            assert!(
                logs.contains(&pago_b.to_string()),
                "Expected pago_id in structured warning log. Got: {logs}"
            );
            assert!(
                logs.contains(&org_a.to_string()),
                "Expected organizacion_id in structured warning log. Got: {logs}"
            );

            // Cleanup
            cleanup(&db, pago_b, contrato_b, inq_b, prop_b, org_b).await;
            use realestate_backend::entities::organizacion;
            use sea_orm::EntityTrait;
            let _ = organizacion::Entity::delete_by_id(org_a).exec(&db).await;
        });
    }
}

#[test]
fn cross_tenant_receipt_returns_404() {
    db_async::cross_tenant_receipt_returns_404();
}

#[test]
fn cross_tenant_receipt_404_all_roles() {
    db_async::cross_tenant_receipt_404_all_roles();
}

#[test]
fn nonexistent_pago_returns_404() {
    db_async::nonexistent_pago_returns_404();
}

#[test]
fn cross_tenant_receipt_logs_structured_warning() {
    db_async::cross_tenant_receipt_logs_structured_warning();
}
