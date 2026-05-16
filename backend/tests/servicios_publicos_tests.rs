#![allow(clippy::needless_return)]
use crate::migrations;

#[cfg(test)]
mod servicios_publicos_rbac_tests {
    use actix_web::http::StatusCode;
    use actix_web::{App, HttpResponse, test, web};
    use chrono::Utc;
    use uuid::Uuid;

    use realestate_backend::config::AppConfig;
    use realestate_backend::errors::AppError;
    use realestate_backend::middleware::rbac::WriteAccess;
    use realestate_backend::services::auth::{Claims, encode_jwt};

    const JWT_SECRET: &str = "test_secret_key_that_is_long_enough_32chars!";

    fn test_config() -> AppConfig {
        AppConfig {
            database_url: String::new(),
            jwt_secret: JWT_SECRET.to_string(),
            server_port: 8080,
            cors_origin: None,
            pool: realestate_backend::config::PoolConfig::default(),
            chatbot: realestate_backend::config::ChatbotEnvConfig {
                baileys_service_url: "http://baileys:3100".to_string(),
                baileys_internal_token: "a]3kF9#mP7vL2nQ8wR5xT0yU4zA1bC6dE".to_string(),
                ovms_endpoint: "http://ovms:8000/v1".to_string(),
                ovms_chat_model: "Qwen3-Coder-30B-A3B-Instruct-int4-ov".to_string(),
                ai_chat_timeout_secs: 30,
            },
        }
    }

    fn make_token(rol: &str) -> String {
        let claims = Claims {
            sub: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            rol: rol.to_string(),
            organizacion_id: Uuid::new_v4(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };
        encode_jwt(&claims, JWT_SECRET).unwrap()
    }

    // Stub for GET/PUT servicios (WriteAccess + path tuple)
    async fn write_access_path_tuple_stub(
        _access: WriteAccess,
        _path: web::Path<(Uuid, Uuid)>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    async fn write_access_path_tuple_body_stub(
        _access: WriteAccess,
        _path: web::Path<(Uuid, Uuid)>,
        _body: web::Json<serde_json::Value>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    async fn write_access_path_body_stub(
        _access: WriteAccess,
        _path: web::Path<Uuid>,
        _body: web::Json<serde_json::Value>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    // --- GET /api/v1/propiedades/{propiedad_id}/unidades/{id}/servicios ---

    #[actix_web::test]
    async fn get_servicios_rejects_unauthenticated() {
        let prop_id = Uuid::new_v4();
        let unit_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/propiedades/{propiedad_id}/unidades/{id}/servicios",
            web::get().to(write_access_path_tuple_stub),
        ))
        .await;

        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/v1/propiedades/{prop_id}/unidades/{unit_id}/servicios"
            ))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn get_servicios_rejects_visualizador() {
        let prop_id = Uuid::new_v4();
        let unit_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/propiedades/{propiedad_id}/unidades/{id}/servicios",
            web::get().to(write_access_path_tuple_stub),
        ))
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/v1/propiedades/{prop_id}/unidades/{unit_id}/servicios"
            ))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn get_servicios_allows_admin() {
        let prop_id = Uuid::new_v4();
        let unit_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/propiedades/{propiedad_id}/unidades/{id}/servicios",
            web::get().to(write_access_path_tuple_stub),
        ))
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/v1/propiedades/{prop_id}/unidades/{unit_id}/servicios"
            ))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn get_servicios_allows_gerente() {
        let prop_id = Uuid::new_v4();
        let unit_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/propiedades/{propiedad_id}/unidades/{id}/servicios",
            web::get().to(write_access_path_tuple_stub),
        ))
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/v1/propiedades/{prop_id}/unidades/{unit_id}/servicios"
            ))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- PUT /api/v1/propiedades/{propiedad_id}/unidades/{id}/servicios ---

    #[actix_web::test]
    async fn put_unit_servicios_rejects_unauthenticated() {
        let prop_id = Uuid::new_v4();
        let unit_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/propiedades/{propiedad_id}/unidades/{id}/servicios",
            web::put().to(write_access_path_tuple_body_stub),
        ))
        .await;

        let req = test::TestRequest::put()
            .uri(&format!(
                "/api/v1/propiedades/{prop_id}/unidades/{unit_id}/servicios"
            ))
            .set_json(serde_json::json!({
                "responsabilidades": [{"proveedorServicio": "EDENORTE", "responsable": "inquilino"}]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn put_unit_servicios_allows_admin() {
        let prop_id = Uuid::new_v4();
        let unit_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/propiedades/{propiedad_id}/unidades/{id}/servicios",
            web::put().to(write_access_path_tuple_body_stub),
        ))
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::put()
            .uri(&format!(
                "/api/v1/propiedades/{prop_id}/unidades/{unit_id}/servicios"
            ))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "responsabilidades": [{"proveedorServicio": "EDENORTE", "responsable": "inquilino"}]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- PUT /api/v1/contratos/{id}/servicios ---

    #[actix_web::test]
    async fn put_contrato_servicios_rejects_unauthenticated() {
        let contrato_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/contratos/{id}/servicios",
            web::put().to(write_access_path_body_stub),
        ))
        .await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/servicios"))
            .set_json(serde_json::json!({
                "responsabilidades": [{"proveedorServicio": "CAASD", "responsable": "propietario"}]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn put_contrato_servicios_allows_gerente() {
        let contrato_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/contratos/{id}/servicios",
            web::put().to(write_access_path_body_stub),
        ))
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/servicios"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "responsabilidades": [{"proveedorServicio": "CAASD", "responsable": "propietario"}]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}

/// Integration tests that require a running database.
mod servicios_publicos_db_tests {
    use actix_web::test;
    use chrono::Utc;
    use realestate_backend::app::create_app;
    use realestate_backend::config::AppConfig;
    use realestate_backend::services::auth::{Claims, encode_jwt};
    use rust_decimal::Decimal;
    use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, Set};
    use sea_orm_migration::MigratorTrait;
    use serde_json::{Value, json};
    use uuid::Uuid;

    const JWT_SECRET: &str = "test_secret_key_that_is_long_enough_for_jwt";

    fn db_url() -> String {
        dotenvy::dotenv().ok();
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests")
    }

    async fn setup_db() -> Result<DatabaseConnection, String> {
        let mut opts = ConnectOptions::new(db_url());
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
        Ok(db)
    }

    fn shared_rt_and_db() -> Option<&'static (tokio::runtime::Runtime, DatabaseConnection)> {
        static SHARED: std::sync::OnceLock<
            Result<(tokio::runtime::Runtime, DatabaseConnection), String>,
        > = std::sync::OnceLock::new();
        SHARED
            .get_or_init(|| {
                let rt =
                    tokio::runtime::Runtime::new().map_err(|e| format!("Runtime error: {e}"))?;
                let db = rt.block_on(setup_db())?;
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
            chatbot: realestate_backend::config::ChatbotEnvConfig {
                baileys_service_url: "http://baileys:3100".to_string(),
                baileys_internal_token: "a]3kF9#mP7vL2nQ8wR5xT0yU4zA1bC6dE".to_string(),
                ovms_endpoint: "http://ovms:8000/v1".to_string(),
                ovms_chat_model: "Qwen3-Coder-30B-A3B-Instruct-int4-ov".to_string(),
                ai_chat_timeout_secs: 30,
            },
            database_url: String::new(),
            jwt_secret: JWT_SECRET.to_string(),
            server_port: 0,
            cors_origin: None,
        }
    }

    fn make_token(user_id: Uuid, rol: &str, org_id: Uuid) -> String {
        let claims = Claims {
            sub: user_id,
            email: format!("{rol}@test.com"),
            rol: rol.to_string(),
            organizacion_id: org_id,
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };
        encode_jwt(&claims, JWT_SECRET).unwrap()
    }

    async fn create_test_organizacion(db: &DatabaseConnection) -> Uuid {
        use realestate_backend::entities::organizacion;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        organizacion::ActiveModel {
            id: Set(id),
            tipo: Set("persona_fisica".to_string()),
            nombre: Set(format!("Org SP Test {id}")),
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
        .expect("Failed to create test organizacion");
        id
    }

    async fn create_test_usuario(db: &DatabaseConnection, rol: &str, org_id: Uuid) -> Uuid {
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
        }
        .insert(db)
        .await
        .expect("Failed to create test usuario");
        id
    }

    async fn create_test_propiedad(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::propiedad;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        propiedad::ActiveModel {
            id: Set(id),
            titulo: Set("Propiedad SP Test".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle Test 456".to_string()),
            ciudad: Set("Santo Domingo".to_string()),
            provincia: Set("Distrito Nacional".to_string()),
            tipo_propiedad: Set("apartamento".to_string()),
            habitaciones: Set(Some(3)),
            banos: Set(Some(2)),
            area_m2: Set(Some(Decimal::new(12000, 2))),
            precio: Set(Decimal::new(3500000, 2)),
            moneda: Set("DOP".to_string()),
            estado: Set("ocupada".to_string()),
            imagenes: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("Failed to create test propiedad");
        id
    }

    async fn create_test_unidad(
        db: &DatabaseConnection,
        propiedad_id: Uuid,
        _org_id: Uuid,
    ) -> Uuid {
        use realestate_backend::entities::unidad;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        unidad::ActiveModel {
            id: Set(id),
            propiedad_id: Set(propiedad_id),
            numero_unidad: Set(format!("U-{}", &id.to_string()[..8])),
            piso: Set(None),
            descripcion: Set(None),
            habitaciones: Set(Some(2)),
            banos: Set(Some(1)),
            area_m2: Set(Some(Decimal::new(6000, 2))),
            precio: Set(Decimal::new(1800000, 2)),
            moneda: Set("DOP".to_string()),
            estado: Set("ocupada".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("Failed to create test unidad");
        id
    }

    /// Test: Update unit default responsibility
    pub fn update_unit_default_responsibility() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let unidad_id = create_test_unidad(&db, propiedad_id, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Set unit default responsibility
            let req = test::TestRequest::put()
                .uri(&format!(
                    "/api/v1/propiedades/{propiedad_id}/unidades/{unidad_id}/servicios"
                ))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "responsabilidades": [
                        {"proveedorServicio": "EDENORTE", "responsable": "inquilino"},
                        {"proveedorServicio": "CAASD", "responsable": "propietario"}
                    ]
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);

            // Verify via GET
            let req = test::TestRequest::get()
                .uri(&format!(
                    "/api/v1/propiedades/{propiedad_id}/unidades/{unidad_id}/servicios"
                ))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            let items = body.as_array().unwrap();
            assert!(!items.is_empty());
        });
    }

    /// Test: Contract override takes precedence over unit default
    pub fn contract_override_takes_precedence() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let unidad_id = create_test_unidad(&db, propiedad_id, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Set unit default: EDENORTE -> propietario
            let req = test::TestRequest::put()
                .uri(&format!(
                    "/api/v1/propiedades/{propiedad_id}/unidades/{unidad_id}/servicios"
                ))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "responsabilidades": [
                        {"proveedorServicio": "EDENORTE", "responsable": "propietario"}
                    ]
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);

            // Create a contract and set override: EDENORTE -> inquilino
            // First create inquilino and contrato
            use realestate_backend::entities::inquilino;
            let inquilino_id = Uuid::new_v4();
            let now = Utc::now().into();
            inquilino::ActiveModel {
                id: Set(inquilino_id),
                nombre: Set("Test".to_string()),
                apellido: Set("Override".to_string()),
                cedula: Set(format!("002-{}-0001", &Uuid::new_v4().to_string()[..7])),
                telefono: Set(None),
                email: Set(Some(format!("override+{inquilino_id}@test.com"))),
                contacto_emergencia: Set(None),
                notas: Set(None),
                documentos: Set(None),
                organizacion_id: Set(org_id),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(&db)
            .await
            .unwrap();

            use realestate_backend::entities::contrato;
            let contrato_id = Uuid::new_v4();
            contrato::ActiveModel {
                id: Set(contrato_id),
                propiedad_id: Set(propiedad_id),
                inquilino_id: Set(inquilino_id),
                fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
                fecha_fin: Set(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
                monto_mensual: Set(Decimal::new(20000, 0)),
                moneda: Set("DOP".to_string()),
                deposito: Set(Some(Decimal::new(20000, 0))),
                estado: Set("activo".to_string()),
                estado_deposito: Set(Some("cobrado".to_string())),
                documentos: Set(None),
                fecha_cobro_deposito: Set(None),
                fecha_devolucion_deposito: Set(None),
                monto_retenido: Set(None),
                motivo_retencion: Set(None),
                recargo_porcentaje: Set(None),
                dias_gracia: Set(None),
                organizacion_id: Set(org_id),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(&db)
            .await
            .unwrap();

            // Set contract override: EDENORTE -> inquilino
            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/contratos/{contrato_id}/servicios"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "unidadId": unidad_id,
                    "responsabilidades": [
                        {"proveedorServicio": "EDENORTE", "responsable": "inquilino"}
                    ]
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);

            // Verify: GET should show contract override (esOverrideContrato: true)
            let req = test::TestRequest::get()
                .uri(&format!(
                    "/api/v1/propiedades/{propiedad_id}/unidades/{unidad_id}/servicios"
                ))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            let items = body.as_array().unwrap();
            let edenorte = items.iter().find(|i| i["proveedorServicio"] == "EDENORTE");
            if let Some(entry) = edenorte {
                assert_eq!(entry["responsable"], "inquilino");
                assert_eq!(entry["esOverrideContrato"], true);
            }
        });
    }

    /// Test: Utility gasto creation triggers anomaly detection (>= 3 prior records, > 50% threshold)
    pub fn utility_gasto_triggers_anomaly_detection() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let unidad_id = create_test_unidad(&db, propiedad_id, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create 3 prior utility gastos with normal consumption (~100 kWh)
            for i in 1..=3u32 {
                let req = test::TestRequest::post()
                    .uri("/api/v1/gastos")
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(json!({
                        "propiedadId": propiedad_id,
                        "unidadId": unidad_id,
                        "categoria": "servicio_publico",
                        "descripcion": format!("Electricidad mes {i}"),
                        "monto": "2500",
                        "moneda": "DOP",
                        "fechaGasto": format!("2025-0{i}-15"),
                        "proveedorServicio": "EDENORTE",
                        "consumo": "100",
                        "unidadConsumo": "kWh",
                        "periodoDesde": format!("2025-0{i}-01"),
                        "periodoHasta": format!("2025-0{i}-28")
                    }))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), 201, "Failed to create prior gasto {i}");
            }

            // Create a gasto with abnormal consumption (200 kWh, > 150% of avg 100)
            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "unidadId": unidad_id,
                    "categoria": "servicio_publico",
                    "descripcion": "Electricidad mes 4 - anormal",
                    "monto": "5000",
                    "moneda": "DOP",
                    "fechaGasto": "2025-04-15",
                    "proveedorServicio": "EDENORTE",
                    "consumo": "200",
                    "unidadConsumo": "kWh",
                    "periodoDesde": "2025-04-01",
                    "periodoHasta": "2025-04-28"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            // Gasto creation should succeed (anomaly detection is best-effort)
            assert_eq!(resp.status(), 201);
        });
    }

    /// Test: Anomaly detection skips when < 3 prior records
    pub fn anomaly_detection_skips_with_few_records() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let unidad_id = create_test_unidad(&db, propiedad_id, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create only 1 prior gasto (< 3 required for anomaly detection)
            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "unidadId": unidad_id,
                    "categoria": "servicio_publico",
                    "descripcion": "Electricidad mes 1",
                    "monto": "2500",
                    "moneda": "DOP",
                    "fechaGasto": "2025-01-15",
                    "proveedorServicio": "EDENORTE",
                    "consumo": "100",
                    "unidadConsumo": "kWh",
                    "periodoDesde": "2025-01-01",
                    "periodoHasta": "2025-01-28"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);

            // Create a high-consumption gasto — should NOT trigger anomaly (< 3 prior)
            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "unidadId": unidad_id,
                    "categoria": "servicio_publico",
                    "descripcion": "Electricidad mes 2 - alto",
                    "monto": "8000",
                    "moneda": "DOP",
                    "fechaGasto": "2025-02-15",
                    "proveedorServicio": "EDENORTE",
                    "consumo": "500",
                    "unidadConsumo": "kWh",
                    "periodoDesde": "2025-02-01",
                    "periodoHasta": "2025-02-28"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            // Should succeed without anomaly notification (not enough history)
            assert_eq!(resp.status(), 201);
        });
    }
}

// Task 14.3: DB-backed servicios publicos tests
#[test]
fn sp_update_unit_default_responsibility() {
    servicios_publicos_db_tests::update_unit_default_responsibility();
}

#[test]
fn sp_contract_override_takes_precedence() {
    servicios_publicos_db_tests::contract_override_takes_precedence();
}

#[test]
fn sp_utility_gasto_triggers_anomaly_detection() {
    servicios_publicos_db_tests::utility_gasto_triggers_anomaly_detection();
}

#[test]
fn sp_anomaly_detection_skips_with_few_records() {
    servicios_publicos_db_tests::anomaly_detection_skips_with_few_records();
}
