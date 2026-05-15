#![allow(clippy::needless_return)]
use crate::migrations;

mod db_async {
    use actix_web::test;
    use realestate_backend::app::create_app;
    use realestate_backend::config::AppConfig;
    use sea_orm::{ConnectOptions, Database, DatabaseConnection};
    use sea_orm_migration::MigratorTrait;
    use serde_json::Value;
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
                ovms_chat_model: "qwen3.6".to_string(),
                ai_chat_timeout_secs: 30,
            },
            database_url: String::new(),
            jwt_secret: JWT_SECRET.to_string(),
            server_port: 0,
            cors_origin: None,
        }
    }

    fn unique_email() -> String {
        format!("test+{}@example.com", Uuid::new_v4())
    }

    /// Valid cédula that passes Luhn check.
    fn valid_cedula() -> String {
        let uuid_digits: String = Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .filter(|c| c.is_ascii_digit())
            .take(10)
            .collect();
        let prefix = format!("{uuid_digits:0<10}");
        let weights = [1u32, 2, 1, 2, 1, 2, 1, 2, 1, 2];
        let sum: u32 = prefix
            .chars()
            .zip(weights.iter())
            .map(|(ch, &w)| {
                let product = ch.to_digit(10).unwrap() * w;
                if product > 9 {
                    product / 10 + product % 10
                } else {
                    product
                }
            })
            .sum();
        let check = (10 - (sum % 10)) % 10;
        format!("{prefix}{check}")
    }

    fn make_app_data()
    -> actix_web::web::Data<realestate_backend::services::ocr_preview::PreviewStore> {
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new())
    }

    fn test_peer() -> std::net::SocketAddr {
        "127.0.0.1:8080".parse().unwrap()
    }

    // ── Register response matches User DTO shape (no token, no password) ──

    pub fn register_response_matches_user_dto_shape() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

            let email = unique_email();
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .peer_addr(test_peer())
                .set_json(serde_json::json!({
                    "nombre": "Test Gerente",
                    "email": email,
                    "password": "SecurePass123!",
                    "tipo": "persona_fisica",
                    "cedula": valid_cedula(),
                    "telefono": "809-555-1234",
                    "nombreOrganizacion": "Test Org"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                201,
                "new-org registration should return 201 Created"
            );

            let body: Value = test::read_body_json(resp).await;

            // Response MUST contain all User DTO fields
            assert!(body["id"].is_string(), "response must have 'id'");
            assert_eq!(body["nombre"], "Test Gerente");
            assert_eq!(body["email"], email);
            assert_eq!(body["rol"], "gerente");
            assert_eq!(body["activo"], true);
            assert!(
                body["organizacionId"].is_string(),
                "response must have 'organizacionId'"
            );
            assert!(
                body["createdAt"].is_string(),
                "response must have 'createdAt'"
            );

            // Response MUST NOT contain token, password, or session fields
            assert!(
                body.get("token").is_none(),
                "response must NOT contain 'token'"
            );
            assert!(
                body.get("password").is_none(),
                "response must NOT contain 'password'"
            );
            assert!(
                body.get("passwordHash").is_none(),
                "response must NOT contain 'passwordHash'"
            );
            assert!(
                body.get("password_hash").is_none(),
                "response must NOT contain 'password_hash'"
            );
            assert!(
                body.get("session").is_none(),
                "response must NOT contain 'session'"
            );

            // Verify the response has exactly the expected keys (no extras)
            let obj = body.as_object().unwrap();
            let expected_keys: std::collections::HashSet<&str> = [
                "id",
                "nombre",
                "email",
                "rol",
                "activo",
                "organizacionId",
                "createdAt",
            ]
            .into_iter()
            .collect();
            let actual_keys: std::collections::HashSet<&str> =
                obj.keys().map(|k| k.as_str()).collect();
            assert_eq!(
                actual_keys, expected_keys,
                "response keys must match User DTO exactly"
            );
        });
    }

    // ── Persisted user has rol == "gerente" and non-null organizacion_id ──

    pub fn register_persists_gerente_role_and_org() {
        with_db(|db| async move {
            use realestate_backend::entities::usuario;
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

            let email = unique_email();
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .peer_addr(test_peer())
                .set_json(serde_json::json!({
                    "nombre": "Gerente Persistido",
                    "email": email,
                    "password": "SecurePass123!",
                    "tipo": "persona_fisica",
                    "cedula": valid_cedula(),
                    "telefono": "809-555-5678",
                    "nombreOrganizacion": "Org Persistida"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);

            // Query the database directly to verify persistence
            let user = usuario::Entity::find()
                .filter(usuario::Column::Email.eq(email.as_str()))
                .one(&db)
                .await
                .unwrap()
                .expect("user should be persisted in the database");

            assert_eq!(
                user.rol, "gerente",
                "persisted user must have rol == 'gerente'"
            );
            assert_ne!(
                user.organizacion_id,
                Uuid::nil(),
                "persisted user must have a non-null organizacion_id"
            );
        });
    }

    // ── Duplicate email returns 409 with Spanish message ──

    pub fn register_duplicate_email_returns_409() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

            let email = unique_email();

            // First registration succeeds
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .peer_addr(test_peer())
                .set_json(serde_json::json!({
                    "nombre": "Primer Usuario",
                    "email": email,
                    "password": "SecurePass123!",
                    "tipo": "persona_fisica",
                    "cedula": valid_cedula(),
                    "telefono": "809-555-0010",
                    "nombreOrganizacion": "Org Primera"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201, "first registration should succeed");

            // Second registration with same email returns 409
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .peer_addr(test_peer())
                .set_json(serde_json::json!({
                    "nombre": "Segundo Usuario",
                    "email": email,
                    "password": "OtherPass456!",
                    "tipo": "persona_fisica",
                    "cedula": valid_cedula(),
                    "telefono": "809-555-0011",
                    "nombreOrganizacion": "Org Segunda"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                409,
                "duplicate email registration should return 409 Conflict"
            );

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["error"], "conflict");
            assert_eq!(
                body["message"], "El correo ya está registrado",
                "error message must be the exact Spanish string"
            );
        });
    }
}

#[test]
fn register_response_matches_user_dto_shape() {
    db_async::register_response_matches_user_dto_shape();
}

#[test]
fn register_persists_gerente_role_and_org() {
    db_async::register_persists_gerente_role_and_org();
}

#[test]
fn register_duplicate_email_returns_409() {
    db_async::register_duplicate_email_returns_409();
}
