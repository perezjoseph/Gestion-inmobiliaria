#[cfg(test)]
mod contratos_lifecycle_tests {
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
                ovms_chat_model: "Qwen3.6-35B-A3B".to_string(),
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

    async fn write_access_stub_with_path_and_body(
        _access: WriteAccess,
        _path: web::Path<Uuid>,
        _body: web::Json<serde_json::Value>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    async fn claims_stub(_claims: Claims) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    // --- POST /api/contratos/{id}/renovar (WriteAccess) ---

    #[actix_web::test]
    async fn renovar_rejects_unauthenticated() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/renovar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{id}/renovar"))
            .set_json(serde_json::json!({"fechaFin":"2026-12-31","montoMensual":"25000"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn renovar_rejects_visualizador() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/renovar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{id}/renovar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"fechaFin":"2026-12-31","montoMensual":"25000"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn renovar_allows_admin() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/renovar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{id}/renovar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"fechaFin":"2026-12-31","montoMensual":"25000"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn renovar_allows_gerente() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/renovar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{id}/renovar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"fechaFin":"2026-12-31","montoMensual":"25000"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- POST /api/contratos/{id}/terminar (WriteAccess) ---

    #[actix_web::test]
    async fn terminar_rejects_unauthenticated() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/terminar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{id}/terminar"))
            .set_json(serde_json::json!({"fechaTerminacion":"2025-06-15"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn terminar_rejects_visualizador() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/terminar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{id}/terminar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"fechaTerminacion":"2025-06-15"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn terminar_allows_admin() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/terminar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{id}/terminar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"fechaTerminacion":"2025-06-15"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn terminar_allows_gerente() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/terminar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{id}/terminar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"fechaTerminacion":"2025-06-15"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- GET /api/contratos/por-vencer (any authenticated user via Claims) ---

    #[actix_web::test]
    async fn por_vencer_rejects_unauthenticated() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/contratos/por-vencer", web::get().to(claims_stub)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/contratos/por-vencer")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn por_vencer_allows_visualizador() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/contratos/por-vencer", web::get().to(claims_stub)),
        )
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::get()
            .uri("/api/contratos/por-vencer")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn por_vencer_allows_admin() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/contratos/por-vencer", web::get().to(claims_stub)),
        )
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::get()
            .uri("/api/contratos/por-vencer")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn por_vencer_allows_gerente() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/contratos/por-vencer", web::get().to(claims_stub)),
        )
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::get()
            .uri("/api/contratos/por-vencer")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}

/// Integration tests for deposit cap and IPC renewal validation (requires database).
mod contratos_dr_legal_db_tests {
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
        crate::migrations::Migrator::up(&db, None)
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
                ovms_chat_model: "Qwen3.6-35B-A3B".to_string(),
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
            nombre: Set(format!("Org Contrato DR Test {id}")),
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
            titulo: Set("Propiedad Contrato DR Test".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle Test 789".to_string()),
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
        .expect("Failed to create test propiedad");
        id
    }

    async fn create_test_inquilino(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::inquilino;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        inquilino::ActiveModel {
            id: Set(id),
            nombre: Set("Carlos".to_string()),
            apellido: Set("Deposit Test".to_string()),
            cedula: Set(format!("003-{}-0001", &Uuid::new_v4().to_string()[..7])),
            telefono: Set(Some("809-555-0002".to_string())),
            email: Set(Some(format!("deposit+{id}@test.com"))),
            contacto_emergencia: Set(None),
            notas: Set(None),
            documentos: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("Failed to create test inquilino");
        id
    }

    /// Test: Deposit exceeding monto_mensual is rejected (422)
    pub fn deposit_exceeding_monto_mensual_rejected() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let inquilino_id = create_test_inquilino(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create contract with deposito > monto_mensual — should be rejected
            let req = test::TestRequest::post()
                .uri("/api/v1/contratos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "inquilinoId": inquilino_id,
                    "fechaInicio": "2025-07-01",
                    "fechaFin": "2026-07-01",
                    "montoMensual": "25000",
                    "moneda": "DOP",
                    "deposito": "30000",
                    "diaCobro": 1
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    /// Test: Deposit equal to monto_mensual is accepted
    pub fn deposit_equal_to_monto_mensual_accepted() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let inquilino_id = create_test_inquilino(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create contract with deposito == monto_mensual — should be accepted
            let req = test::TestRequest::post()
                .uri("/api/v1/contratos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "inquilinoId": inquilino_id,
                    "fechaInicio": "2025-07-01",
                    "fechaFin": "2026-07-01",
                    "montoMensual": "25000",
                    "moneda": "DOP",
                    "deposito": "25000",
                    "diaCobro": 1
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
        });
    }

    /// Test: Renewal with IPC cap — amount within cap accepted
    pub fn renewal_within_ipc_cap_accepted() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let inquilino_id = create_test_inquilino(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Set IPC to 5%
            let req = test::TestRequest::put()
                .uri("/api/v1/configuracion/ipc")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "valorIpc": "5.00",
                    "fechaEfectiva": "2025-01-01"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);

            // Create active contract with monto_mensual = 25000
            let req = test::TestRequest::post()
                .uri("/api/v1/contratos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "inquilinoId": inquilino_id,
                    "fechaInicio": "2025-01-01",
                    "fechaFin": "2025-12-31",
                    "montoMensual": "25000",
                    "moneda": "DOP",
                    "deposito": "25000",
                    "diaCobro": 1
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let contrato_id = body["id"].as_str().unwrap().to_string();

            // Renew with amount within IPC cap: 25000 * 1.05 = 26250 max
            let req = test::TestRequest::post()
                .uri(&format!("/api/v1/contratos/{contrato_id}/renovar"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "fechaFin": "2026-12-31",
                    "montoMensual": "26000"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            // Should be accepted (26000 <= 26250)
            assert_eq!(resp.status(), 200);
        });
    }

    /// Test: Renewal exceeding IPC cap is rejected with max_allowed in response
    pub fn renewal_exceeding_ipc_cap_rejected() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let inquilino_id = create_test_inquilino(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Set IPC to 5%
            let req = test::TestRequest::put()
                .uri("/api/v1/configuracion/ipc")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "valorIpc": "5.00",
                    "fechaEfectiva": "2025-01-01"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);

            // Create active contract with monto_mensual = 25000
            let req = test::TestRequest::post()
                .uri("/api/v1/contratos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "inquilinoId": inquilino_id,
                    "fechaInicio": "2025-01-01",
                    "fechaFin": "2025-12-31",
                    "montoMensual": "25000",
                    "moneda": "DOP",
                    "deposito": "25000",
                    "diaCobro": 1
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let contrato_id = body["id"].as_str().unwrap().to_string();

            // Renew with amount exceeding IPC cap: 25000 * 1.05 = 26250 max, try 28000
            let req = test::TestRequest::post()
                .uri(&format!("/api/v1/contratos/{contrato_id}/renovar"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "fechaFin": "2026-12-31",
                    "montoMensual": "28000"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            // Should be rejected (28000 > 26250)
            assert_eq!(resp.status(), 422);
            let body: Value = test::read_body_json(resp).await;
            // Response should include max_allowed
            assert!(
                body["maxAllowed"].is_string() || body["max_allowed"].is_string(),
                "Response should include max_allowed field"
            );
        });
    }
}

// Task 14.5: DR Legal deposit cap and IPC renewal tests
#[test]
fn contrato_deposit_exceeding_monto_mensual_rejected() {
    contratos_dr_legal_db_tests::deposit_exceeding_monto_mensual_rejected();
}

#[test]
fn contrato_deposit_equal_to_monto_mensual_accepted() {
    contratos_dr_legal_db_tests::deposit_equal_to_monto_mensual_accepted();
}

#[test]
fn contrato_renewal_within_ipc_cap_accepted() {
    contratos_dr_legal_db_tests::renewal_within_ipc_cap_accepted();
}

#[test]
fn contrato_renewal_exceeding_ipc_cap_rejected() {
    contratos_dr_legal_db_tests::renewal_exceeding_ipc_cap_rejected();
}
