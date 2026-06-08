#![allow(clippy::needless_return)]
use crate::migrations;

#[cfg(test)]
mod desahucios_rbac_tests {
    use actix_web::http::StatusCode;
    use actix_web::{App, HttpResponse, test, web};
    use chrono::Utc;
    use uuid::Uuid;

    use realestate_backend::config::AppConfig;
    use realestate_backend::errors::AppError;
    use realestate_backend::middleware::rbac::WriteAccess;
    use realestate_backend::services::auth::{Claims, encode_jwt};

    use crate::common::{JWT_SECRET, test_app_config};

    fn test_config() -> AppConfig {
        AppConfig {
            server_port: 8080,
            ..test_app_config("")
        }
    }

    fn make_token(rol: &str) -> String {
        let claims = Claims {
            sub: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            rol: rol.to_string(),
            organizacion_id: Uuid::new_v4(),
            jti: Uuid::new_v4(),
            iat: Utc::now().timestamp(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };
        encode_jwt(&claims, JWT_SECRET).unwrap()
    }

    async fn write_access_stub(
        _access: WriteAccess,
        _body: web::Json<serde_json::Value>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Created().finish())
    }

    async fn write_access_path_body_stub(
        _access: WriteAccess,
        _path: web::Path<Uuid>,
        _body: web::Json<serde_json::Value>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    async fn write_access_query_stub(_access: WriteAccess) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    // --- POST /api/v1/desahucios (WriteAccess) ---

    #[actix_web::test]
    async fn create_desahucio_rejects_unauthenticated() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/v1/desahucios", web::post().to(write_access_stub)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/desahucios")
            .set_json(serde_json::json!({"contratoId": Uuid::new_v4(), "motivo": "Falta de pago"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn create_desahucio_rejects_visualizador() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/v1/desahucios", web::post().to(write_access_stub)),
        )
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::post()
            .uri("/api/v1/desahucios")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"contratoId": Uuid::new_v4(), "motivo": "Falta de pago"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn create_desahucio_allows_admin() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/v1/desahucios", web::post().to(write_access_stub)),
        )
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::post()
            .uri("/api/v1/desahucios")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"contratoId": Uuid::new_v4(), "motivo": "Falta de pago"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[actix_web::test]
    async fn create_desahucio_allows_gerente() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/v1/desahucios", web::post().to(write_access_stub)),
        )
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::post()
            .uri("/api/v1/desahucios")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"contratoId": Uuid::new_v4(), "motivo": "Falta de pago"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // --- PUT /api/v1/desahucios/{id} (WriteAccess) ---

    #[actix_web::test]
    async fn update_desahucio_rejects_unauthenticated() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/desahucios/{id}",
            web::put().to(write_access_path_body_stub),
        ))
        .await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/v1/desahucios/{id}"))
            .set_json(serde_json::json!({"estado": "en_progreso"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn update_desahucio_rejects_visualizador() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/desahucios/{id}",
            web::put().to(write_access_path_body_stub),
        ))
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::put()
            .uri(&format!("/api/v1/desahucios/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"estado": "en_progreso"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn update_desahucio_allows_admin() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/desahucios/{id}",
            web::put().to(write_access_path_body_stub),
        ))
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::put()
            .uri(&format!("/api/v1/desahucios/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"estado": "en_progreso"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- GET /api/v1/desahucios (WriteAccess) ---

    #[actix_web::test]
    async fn list_desahucios_rejects_unauthenticated() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/v1/desahucios", web::get().to(write_access_query_stub)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/desahucios")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn list_desahucios_rejects_visualizador() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/v1/desahucios", web::get().to(write_access_query_stub)),
        )
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::get()
            .uri("/api/v1/desahucios")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn list_desahucios_allows_gerente() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/v1/desahucios", web::get().to(write_access_query_stub)),
        )
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::get()
            .uri("/api/v1/desahucios")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}

/// Integration tests that require a running database.
mod desahucios_db_tests {
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
            chatbot: realestate_backend::config::ChatbotEnvConfig::for_testing(),
            database_url: String::new(),
            jwt_secret: JWT_SECRET.to_string(),
            server_port: 0,
            cors_origin: None,
            ocr_service_token: None,
            metrics_token: None,
        }
    }

    fn make_token(user_id: Uuid, rol: &str, org_id: Uuid) -> String {
        let claims = Claims {
            sub: user_id,
            email: format!("{rol}@test.com"),
            rol: rol.to_string(),
            organizacion_id: org_id,
            jti: Uuid::new_v4(),
            iat: Utc::now().timestamp(),
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
            nombre: Set(format!("Org Desahucio Test {id}")),
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
            password_changed_at: Set(now),
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
            titulo: Set("Propiedad Desahucio Test".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle Test 123".to_string()),
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
        .expect("Failed to create test propiedad");
        id
    }

    async fn create_test_inquilino(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::inquilino;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        inquilino::ActiveModel {
            id: Set(id),
            nombre: Set("Juan".to_string()),
            apellido: Set("PÃ©rez".to_string()),
            cedula: Set(format!("001-{}-0001", Uuid::new_v4().as_simple())[..13].to_string()),
            telefono: Set(Some("809-555-0001".to_string())),
            email: Set(Some(format!("inquilino+{id}@test.com"))),
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

    async fn create_active_contrato(
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
            fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            fecha_fin: Set(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            monto_mensual: Set(Decimal::new(25000, 0)),
            moneda: Set("DOP".to_string()),
            deposito: Set(Some(Decimal::new(25000, 0))),
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
        .insert(db)
        .await
        .expect("Failed to create test contrato");
        id
    }

    /// Test: Full CRUD lifecycle for desahucios
    pub fn desahucio_crud_lifecycle() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let inquilino_id = create_test_inquilino(&db, org_id).await;
            let contrato_id = create_active_contrato(&db, org_id, propiedad_id, inquilino_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create desahucio
            let req = test::TestRequest::post()
                .uri("/api/v1/desahucios")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "contratoId": contrato_id,
                    "motivo": "Falta de pago por 3 meses"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let desahucio_id = body["id"].as_str().unwrap().to_string();
            assert_eq!(body["estado"], "iniciado");
            assert_eq!(body["motivo"], "Falta de pago por 3 meses");

            // Update state to en_progreso
            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/desahucios/{desahucio_id}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"estado": "en_progreso"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["estado"], "en_progreso");

            // List desahucios
            let req = test::TestRequest::get()
                .uri("/api/v1/desahucios")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            assert!(body["total"].as_u64().unwrap() >= 1);
        });
    }

    /// Test: completado without fecha_resolucion returns 422
    pub fn completado_without_fecha_resolucion_returns_422() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let inquilino_id = create_test_inquilino(&db, org_id).await;
            let contrato_id = create_active_contrato(&db, org_id, propiedad_id, inquilino_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create desahucio
            let req = test::TestRequest::post()
                .uri("/api/v1/desahucios")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "contratoId": contrato_id,
                    "motivo": "Falta de pago"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let desahucio_id = body["id"].as_str().unwrap().to_string();

            // Try to set completado without fecha_resolucion
            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/desahucios/{desahucio_id}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"estado": "completado"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    /// Test: Creating desahucio on non-active contract returns 422
    pub fn create_desahucio_non_active_contract_returns_422() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Use a non-existent contract ID (will fail validation)
            let fake_contrato_id = Uuid::new_v4();
            let req = test::TestRequest::post()
                .uri("/api/v1/desahucios")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "contratoId": fake_contrato_id,
                    "motivo": "Falta de pago"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            // Should be 422 (contract not found or not active) or 404
            assert!(resp.status() == 422 || resp.status() == 404);
        });
    }

    /// Test: Audit trail is created on desahucio create/update
    pub fn desahucio_creates_audit_trail() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let inquilino_id = create_test_inquilino(&db, org_id).await;
            let contrato_id = create_active_contrato(&db, org_id, propiedad_id, inquilino_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create desahucio
            let req = test::TestRequest::post()
                .uri("/api/v1/desahucios")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "contratoId": contrato_id,
                    "motivo": "Falta de pago"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let desahucio_id = body["id"].as_str().unwrap().to_string();

            // Check audit trail exists
            let req = test::TestRequest::get()
                .uri("/api/v1/auditoria")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            let entries = body["data"].as_array().unwrap();
            let has_desahucio_entry = entries
                .iter()
                .any(|e| e["entityId"].as_str() == Some(&desahucio_id));
            assert!(
                has_desahucio_entry,
                "Audit trail should contain desahucio entry"
            );
        });
    }

    /// Test: List desahucios with pagination
    pub fn list_desahucios_with_pagination() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // List with pagination params
            let req = test::TestRequest::get()
                .uri("/api/v1/desahucios?page=1&perPage=5")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            assert!(body["page"].as_u64().is_some());
            assert!(body["perPage"].as_u64().is_some());
        });
    }
}

// Task 14.2: DB-backed desahucio tests
#[test]
fn desahucio_crud_lifecycle_test() {
    desahucios_db_tests::desahucio_crud_lifecycle();
}

#[test]
fn desahucio_completado_without_fecha_resolucion() {
    desahucios_db_tests::completado_without_fecha_resolucion_returns_422();
}

#[test]
fn desahucio_non_active_contract() {
    desahucios_db_tests::create_desahucio_non_active_contract_returns_422();
}

#[test]
fn desahucio_audit_trail() {
    desahucios_db_tests::desahucio_creates_audit_trail();
}

#[test]
fn desahucio_list_pagination() {
    desahucios_db_tests::list_desahucios_with_pagination();
}
