#![allow(clippy::needless_return)]
use crate::migrations;

#[cfg(test)]
mod organizaciones_registration_tests {
    use actix_web::test;
    use realestate_backend::app::create_app;
    use realestate_backend::config::AppConfig;
    use sea_orm::{ConnectOptions, Database, DatabaseConnection};
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
        static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let Some((rt, db)) = shared_rt_and_db() else {
            eprintln!("Database not reachable -- skipping DB integration test");
            return;
        };
        rt.block_on(f(db.clone()));
    }

    fn make_config() -> AppConfig {
        AppConfig {
            pool: realestate_backend::config::PoolConfig::default(),
            database_url: String::new(),
            jwt_secret: JWT_SECRET.to_string(),
            server_port: 0,
            cors_origin: None,
        }
    }

    /// Generate a unique email using a UUID suffix.
    fn unique_email() -> String {
        format!("test+{}@example.com", Uuid::new_v4())
    }

    /// Valid cédula that passes Luhn check: "00114532503"
    fn valid_cedula() -> String {
        "00114532503".to_string()
    }

    /// Valid RNC that passes DGII weighted modulus check: "131246753"
    fn valid_rnc() -> String {
        "131246753".to_string()
    }

    // ── persona_fisica registration creates org + admin user ──

    #[test]
    fn persona_fisica_registration_creates_org_and_admin() {
        with_db(|db| async move {
            let config = make_config();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let email = unique_email();
            let cedula = valid_cedula();
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Juan Pérez",
                    "email": email,
                    "password": "SecurePass123!",
                    "tipo": "persona_fisica",
                    "cedula": cedula,
                    "telefono": "809-555-0001",
                    "nombreOrganizacion": "Inmobiliaria JP"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201, "persona_fisica registration should return 201");

            let body: Value = test::read_body_json(resp).await;

            // JWT token is returned
            assert!(body["token"].is_string(), "response should contain a JWT token");
            let token = body["token"].as_str().unwrap();
            assert!(!token.is_empty());

            // User has admin role
            assert_eq!(body["user"]["rol"], "admin");
            assert_eq!(body["user"]["email"], email);

            // organizacion_id is present and valid UUID
            let org_id_str = body["user"]["organizacionId"].as_str().unwrap();
            let org_id: Uuid = org_id_str.parse().expect("organizacionId should be a valid UUID");
            assert_ne!(org_id, Uuid::nil());
        });
    }

    // ── persona_juridica registration creates org + admin user ──

    #[test]
    fn persona_juridica_registration_creates_org_and_admin() {
        with_db(|db| async move {
            let config = make_config();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let email = unique_email();
            let rnc = valid_rnc();
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "María García",
                    "email": email,
                    "password": "SecurePass456!",
                    "tipo": "persona_juridica",
                    "rnc": rnc,
                    "razonSocial": "Inversiones MG SRL",
                    "nombreComercial": "MG Propiedades",
                    "direccionFiscal": "Av. Winston Churchill 123, Santo Domingo",
                    "representanteLegal": "María García"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201, "persona_juridica registration should return 201");

            let body: Value = test::read_body_json(resp).await;

            assert!(body["token"].is_string());
            assert_eq!(body["user"]["rol"], "admin");
            assert_eq!(body["user"]["email"], email);

            let org_id_str = body["user"]["organizacionId"].as_str().unwrap();
            let org_id: Uuid = org_id_str.parse().expect("organizacionId should be a valid UUID");
            assert_ne!(org_id, Uuid::nil());
        });
    }

    // ── duplicate email returns 409 ──

    #[test]
    fn duplicate_email_returns_409() {
        with_db(|db| async move {
            let config = make_config();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let email = unique_email();

            // First registration succeeds
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Primer Usuario",
                    "email": email,
                    "password": "SecurePass123!",
                    "tipo": "persona_fisica",
                    "cedula": valid_cedula(),
                    "telefono": "809-555-0002",
                    "nombreOrganizacion": "Org Uno"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);

            // Second registration with same email returns 409
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Segundo Usuario",
                    "email": email,
                    "password": "SecurePass456!",
                    "tipo": "persona_fisica",
                    "cedula": "22400022111",
                    "telefono": "809-555-0003",
                    "nombreOrganizacion": "Org Dos"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 409, "duplicate email should return 409");

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["error"], "conflict");
            assert!(
                body["message"].as_str().unwrap().contains("email"),
                "error message should mention email"
            );
        });
    }

    // ── duplicate cedula returns 409 ──

    #[test]
    fn duplicate_cedula_returns_409() {
        with_db(|db| async move {
            let config = make_config();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let cedula = valid_cedula();

            // First registration succeeds
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Usuario A",
                    "email": unique_email(),
                    "password": "SecurePass123!",
                    "tipo": "persona_fisica",
                    "cedula": cedula,
                    "telefono": "809-555-0004",
                    "nombreOrganizacion": "Org A"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);

            // Second registration with same cédula returns 409
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Usuario B",
                    "email": unique_email(),
                    "password": "SecurePass456!",
                    "tipo": "persona_fisica",
                    "cedula": cedula,
                    "telefono": "809-555-0005",
                    "nombreOrganizacion": "Org B"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 409, "duplicate cédula should return 409");

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["error"], "conflict");
            assert!(
                body["message"].as_str().unwrap().contains("cédula"),
                "error message should mention cédula"
            );
        });
    }

    // ── duplicate RNC returns 409 ──

    #[test]
    fn duplicate_rnc_returns_409() {
        with_db(|db| async move {
            let config = make_config();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let rnc = valid_rnc();

            // First registration succeeds
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Empresa A",
                    "email": unique_email(),
                    "password": "SecurePass123!",
                    "tipo": "persona_juridica",
                    "rnc": rnc,
                    "razonSocial": "Empresa A SRL",
                    "nombreComercial": "Empresa A",
                    "direccionFiscal": "Calle A 123",
                    "representanteLegal": "Rep A"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);

            // Second registration with same RNC returns 409
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Empresa B",
                    "email": unique_email(),
                    "password": "SecurePass456!",
                    "tipo": "persona_juridica",
                    "rnc": rnc,
                    "razonSocial": "Empresa B SRL",
                    "nombreComercial": "Empresa B",
                    "direccionFiscal": "Calle B 456",
                    "representanteLegal": "Rep B"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 409, "duplicate RNC should return 409");

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["error"], "conflict");
            assert!(
                body["message"].as_str().unwrap().contains("RNC"),
                "error message should mention RNC"
            );
        });
    }

    // ── invalid RNC returns 422 ──

    #[test]
    fn invalid_rnc_returns_422() {
        with_db(|db| async move {
            let config = make_config();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Empresa Inválida",
                    "email": unique_email(),
                    "password": "SecurePass123!",
                    "tipo": "persona_juridica",
                    "rnc": "999999999",
                    "razonSocial": "Empresa Inválida SRL",
                    "nombreComercial": "Inválida",
                    "direccionFiscal": "Calle X 789",
                    "representanteLegal": "Rep X"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422, "invalid RNC should return 422");

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["error"], "validation");
            assert!(
                body["message"].as_str().unwrap().contains("RNC"),
                "error message should mention RNC"
            );
        });
    }

    // ── invalid cédula returns 422 ──

    #[test]
    fn invalid_cedula_returns_422() {
        with_db(|db| async move {
            let config = make_config();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Persona Inválida",
                    "email": unique_email(),
                    "password": "SecurePass123!",
                    "tipo": "persona_fisica",
                    "cedula": "00000000000",
                    "telefono": "809-555-0006",
                    "nombreOrganizacion": "Org Inválida"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422, "invalid cédula should return 422");

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["error"], "validation");
            assert!(
                body["message"].as_str().unwrap().to_lowercase().contains("cédula")
                    || body["message"].as_str().unwrap().to_lowercase().contains("cedula"),
                "error message should mention cédula"
            );
        });
    }

    // ── JWT contains organizacion_id after registration ──

    #[test]
    fn jwt_contains_organizacion_id_after_registration() {
        with_db(|db| async move {
            let config = make_config();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let email = unique_email();
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "JWT Test User",
                    "email": email,
                    "password": "SecurePass789!",
                    "tipo": "persona_fisica",
                    "cedula": "22400022111",
                    "telefono": "809-555-0007",
                    "nombreOrganizacion": "Org JWT Test"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);

            let body: Value = test::read_body_json(resp).await;
            let token = body["token"].as_str().unwrap();

            // Decode the JWT and verify organizacion_id is present
            let decoded = realestate_backend::services::auth::decode_jwt(token, JWT_SECRET).unwrap();
            assert_ne!(decoded.organizacion_id, Uuid::nil(), "JWT should contain a non-nil organizacion_id");

            // The organizacion_id in JWT should match the one in the user response
            let resp_org_id: Uuid = body["user"]["organizacionId"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(
                decoded.organizacion_id, resp_org_id,
                "JWT organizacion_id should match user response"
            );
        });
    }
}
