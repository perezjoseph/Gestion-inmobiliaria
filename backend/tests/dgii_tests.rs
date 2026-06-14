#![allow(clippy::needless_return)]
use crate::migrations;

#[cfg(test)]
mod dgii_rbac_tests {
    use actix_web::http::StatusCode;
    use actix_web::{App, HttpResponse, test, web};
    use chrono::Utc;
    use uuid::Uuid;

    use realestate_backend::config::AppConfig;
    use realestate_backend::errors::AppError;
    use realestate_backend::middleware::rbac::{AdminOnly, WriteAccess};
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
            iss: "realestate-api".to_string(),
            aud: "realestate-api".to_string(),
        };
        encode_jwt(&claims, JWT_SECRET).unwrap()
    }

    // Stub for GET /dgii/consulta (WriteAccess)
    async fn write_access_stub(_access: WriteAccess) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    // Stub for DELETE /dgii/cache/{rnc} (AdminOnly + path)
    async fn admin_only_path_stub(
        _admin: AdminOnly,
        _path: web::Path<String>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    // --- GET /api/v1/dgii/consulta (WriteAccess) ---

    #[actix_web::test]
    async fn consulta_rnc_rejects_unauthenticated() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/v1/dgii/consulta", web::get().to(write_access_stub)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/dgii/consulta?rnc=101000001")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn consulta_rnc_rejects_visualizador() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/v1/dgii/consulta", web::get().to(write_access_stub)),
        )
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::get()
            .uri("/api/v1/dgii/consulta?rnc=101000001")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn consulta_rnc_allows_admin() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/v1/dgii/consulta", web::get().to(write_access_stub)),
        )
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::get()
            .uri("/api/v1/dgii/consulta?rnc=101000001")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn consulta_rnc_allows_gerente() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/v1/dgii/consulta", web::get().to(write_access_stub)),
        )
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::get()
            .uri("/api/v1/dgii/consulta?rnc=101000001")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- GET /api/v1/dgii/consulta/nombre (WriteAccess) ---

    #[actix_web::test]
    async fn consulta_nombre_rejects_unauthenticated() {
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/dgii/consulta/nombre",
            web::get().to(write_access_stub),
        ))
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/dgii/consulta/nombre?buscar=empresa")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn consulta_nombre_allows_gerente() {
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/dgii/consulta/nombre",
            web::get().to(write_access_stub),
        ))
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::get()
            .uri("/api/v1/dgii/consulta/nombre?buscar=empresa")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- DELETE /api/v1/dgii/cache/{rnc} (AdminOnly) ---

    #[actix_web::test]
    async fn invalidar_cache_rejects_unauthenticated() {
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/dgii/cache/{rnc}",
            web::delete().to(admin_only_path_stub),
        ))
        .await;

        let req = test::TestRequest::delete()
            .uri("/api/v1/dgii/cache/101000001")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn invalidar_cache_rejects_gerente() {
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/dgii/cache/{rnc}",
            web::delete().to(admin_only_path_stub),
        ))
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::delete()
            .uri("/api/v1/dgii/cache/101000001")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn invalidar_cache_allows_admin() {
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/v1/dgii/cache/{rnc}",
            web::delete().to(admin_only_path_stub),
        ))
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::delete()
            .uri("/api/v1/dgii/cache/101000001")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}

/// Integration tests that require a running database.
mod dgii_db_tests {
    use actix_web::test;
    use chrono::Utc;
    use realestate_backend::app::create_app;
    use realestate_backend::config::AppConfig;
    use realestate_backend::services::auth::{Claims, encode_jwt};
    use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, Set};
    use sea_orm_migration::MigratorTrait;
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
            iss: "realestate-api".to_string(),
            aud: "realestate-api".to_string(),
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
            nombre: Set(format!("Org DGII Test {id}")),
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

    /// Test: Invalid RNC format returns 422
    pub fn invalid_rnc_format_returns_422() {
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

            // Invalid RNC (too short)
            let req = test::TestRequest::get()
                .uri("/api/v1/dgii/consulta?rnc=123")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    /// Test: RNC lookup returns data (cache miss scenario — requires external API)
    pub fn rnc_lookup_endpoint_exists() {
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

            // Valid RNC format — may fail due to external API but should not 404
            let req = test::TestRequest::get()
                .uri("/api/v1/dgii/consulta?rnc=101000001")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            // Should be 200 (cached/API success) or 502/503 (API unreachable), not 404
            assert_ne!(resp.status(), 404);
        });
    }

    /// Test: Name lookup endpoint exists and responds
    pub fn nombre_lookup_endpoint_exists() {
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

            let req = test::TestRequest::get()
                .uri("/api/v1/dgii/consulta/nombre?buscar=test")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            // Should respond (may be empty results or API error, but not 404)
            assert_ne!(resp.status(), 404);
        });
    }

    /// Test: Cache invalidation endpoint (AdminOnly)
    pub fn cache_invalidation_admin_only() {
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

            // Admin can invalidate cache
            let req = test::TestRequest::delete()
                .uri("/api/v1/dgii/cache/101000001")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);

            // Gerente cannot
            let gerente_id = create_test_usuario(&db, "gerente", org_id).await;
            let gerente_token = make_token(gerente_id, "gerente", org_id);
            let req = test::TestRequest::delete()
                .uri("/api/v1/dgii/cache/101000001")
                .insert_header(("Authorization", format!("Bearer {gerente_token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 403);
        });
    }
}

// Task 14.4: DB-backed DGII tests
#[test]
fn dgii_invalid_rnc_format() {
    dgii_db_tests::invalid_rnc_format_returns_422();
}

#[test]
fn dgii_rnc_lookup_endpoint() {
    dgii_db_tests::rnc_lookup_endpoint_exists();
}

#[test]
fn dgii_nombre_lookup_endpoint() {
    dgii_db_tests::nombre_lookup_endpoint_exists();
}

#[test]
fn dgii_cache_invalidation() {
    dgii_db_tests::cache_invalidation_admin_only();
}
