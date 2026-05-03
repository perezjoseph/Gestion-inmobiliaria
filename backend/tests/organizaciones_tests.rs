#![allow(clippy::needless_return)]
use crate::migrations;

mod db_async {
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
        let _guard = crate::GLOBAL_DB_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
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

    fn unique_email() -> String {
        format!("test+{}@example.com", Uuid::new_v4())
    }

    /// Valid cédula that passes Luhn check.
    fn valid_cedula() -> String {
        "00114532503".to_string()
    }

    /// Valid RNC that passes DGII weighted modulus check.
    fn valid_rnc() -> String {
        "131246753".to_string()
    }

    fn make_app_data()
    -> actix_web::web::Data<realestate_backend::services::ocr_preview::PreviewStore> {
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new())
    }

    // ── persona_fisica registration creates org + admin user ──

    pub fn persona_fisica_registration_creates_org_and_admin() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

            let email = unique_email();
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Juan Pérez",
                    "email": email,
                    "password": "SecurePass123!",
                    "tipo": "persona_fisica",
                    "cedula": valid_cedula(),
                    "telefono": "809-555-0001",
                    "nombreOrganizacion": "Inmobiliaria JP"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                201,
                "persona_fisica registration should return 201"
            );

            let body: Value = test::read_body_json(resp).await;
            assert!(
                body["token"].is_string(),
                "response should contain a JWT token"
            );
            assert!(!body["token"].as_str().unwrap().is_empty());
            assert_eq!(body["user"]["rol"], "admin");
            assert_eq!(body["user"]["email"], email);

            let org_id_str = body["user"]["organizacionId"].as_str().unwrap();
            let org_id: Uuid = org_id_str
                .parse()
                .expect("organizacionId should be a valid UUID");
            assert_ne!(org_id, Uuid::nil());
        });
    }

    // ── persona_juridica registration creates org + admin user ──

    pub fn persona_juridica_registration_creates_org_and_admin() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

            let email = unique_email();
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "María García",
                    "email": email,
                    "password": "SecurePass456!",
                    "tipo": "persona_juridica",
                    "rnc": valid_rnc(),
                    "razonSocial": "Inversiones MG SRL",
                    "nombreComercial": "MG Propiedades",
                    "direccionFiscal": "Av. Winston Churchill 123, Santo Domingo",
                    "representanteLegal": "María García"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                201,
                "persona_juridica registration should return 201"
            );

            let body: Value = test::read_body_json(resp).await;
            assert!(body["token"].is_string());
            assert_eq!(body["user"]["rol"], "admin");
            assert_eq!(body["user"]["email"], email);

            let org_id_str = body["user"]["organizacionId"].as_str().unwrap();
            let org_id: Uuid = org_id_str
                .parse()
                .expect("organizacionId should be a valid UUID");
            assert_ne!(org_id, Uuid::nil());
        });
    }

    // ── duplicate email returns 409 ──

    pub fn duplicate_email_returns_409() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

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

    pub fn duplicate_cedula_returns_409() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

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

    pub fn duplicate_rnc_returns_409() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

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

    pub fn invalid_rnc_returns_422() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

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

    pub fn invalid_cedula_returns_422() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

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
        });
    }

    // Register a persona_fisica admin and bind (token, org_id).
    macro_rules! register_admin {
        ($app:expr) => {{
            let email = unique_email();
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Admin User",
                    "email": email,
                    "password": "SecurePass123!",
                    "tipo": "persona_fisica",
                    "cedula": valid_cedula(),
                    "telefono": "809-555-9999",
                    "nombreOrganizacion": format!("Org {}", Uuid::new_v4())
                }))
                .to_request();
            let resp = test::call_service(&$app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let token = body["token"].as_str().unwrap().to_string();
            let org_id: Uuid = body["user"]["organizacionId"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            (token, org_id)
        }};
    }

    // ── admin can create invitation with gerente role ──

    pub fn admin_can_create_invitation_gerente() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;
            let (admin_token, _org_id) = register_admin!(app);

            let req = test::TestRequest::post()
                .uri("/api/v1/invitaciones")
                .insert_header(("Authorization", format!("Bearer {admin_token}")))
                .set_json(json!({
                    "email": unique_email(),
                    "rol": "gerente"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                201,
                "admin should be able to create gerente invitation"
            );

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["rol"], "gerente");
            assert!(body["token"].is_string());
            assert_eq!(body["usado"], false);
        });
    }

    // ── admin can create invitation with visualizador role ──

    pub fn admin_can_create_invitation_visualizador() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;
            let (admin_token, _org_id) = register_admin!(app);

            let req = test::TestRequest::post()
                .uri("/api/v1/invitaciones")
                .insert_header(("Authorization", format!("Bearer {admin_token}")))
                .set_json(json!({
                    "email": unique_email(),
                    "rol": "visualizador"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                201,
                "admin should be able to create visualizador invitation"
            );

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["rol"], "visualizador");
        });
    }

    // ── non-admin cannot create invitation (403) ──

    pub fn non_admin_cannot_create_invitation() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;
            let (admin_token, _org_id) = register_admin!(app);

            // Register a gerente via invitation: create invite, then register with token
            let invite_email = unique_email();
            let req = test::TestRequest::post()
                .uri("/api/v1/invitaciones")
                .insert_header(("Authorization", format!("Bearer {admin_token}")))
                .set_json(json!({
                    "email": invite_email,
                    "rol": "gerente"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let inv_body: Value = test::read_body_json(resp).await;
            let inv_token = inv_body["token"].as_str().unwrap().to_string();

            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Gerente User",
                    "email": invite_email,
                    "password": "SecurePass123!",
                    "tokenInvitacion": inv_token
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let gerente_token = body["token"].as_str().unwrap().to_string();

            // Gerente tries to create an invitation → 403
            let req = test::TestRequest::post()
                .uri("/api/v1/invitaciones")
                .insert_header(("Authorization", format!("Bearer {gerente_token}")))
                .set_json(json!({
                    "email": unique_email(),
                    "rol": "visualizador"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                403,
                "non-admin should get 403 when creating invitation"
            );
        });
    }

    // ── registration with valid invitation token joins existing org with invited role ──

    pub fn invitation_registration_joins_org_with_invited_role() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;
            let (admin_token, org_id) = register_admin!(app);

            // Create invitation
            let invite_email = unique_email();
            let req = test::TestRequest::post()
                .uri("/api/v1/invitaciones")
                .insert_header(("Authorization", format!("Bearer {admin_token}")))
                .set_json(json!({
                    "email": invite_email,
                    "rol": "gerente"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let inv_body: Value = test::read_body_json(resp).await;
            let inv_token = inv_body["token"].as_str().unwrap();

            // Register with invitation token
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Invited User",
                    "email": invite_email,
                    "password": "SecurePass789!",
                    "tokenInvitacion": inv_token
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                201,
                "registration with invitation should succeed"
            );

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(
                body["user"]["rol"], "gerente",
                "user should have the invited role"
            );
            let user_org_id: Uuid = body["user"]["organizacionId"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(user_org_id, org_id, "user should belong to the admin's org");

            // Verify JWT also has the correct org
            let jwt_token = body["token"].as_str().unwrap();
            let decoded =
                realestate_backend::services::auth::decode_jwt(jwt_token, JWT_SECRET).unwrap();
            assert_eq!(decoded.organizacion_id, org_id);
            assert_eq!(decoded.rol, "gerente");
        });
    }

    // ── expired invitation returns 410 ──

    pub fn expired_invitation_returns_410() {
        with_db(|db| async move {
            use realestate_backend::entities::invitacion;
            use sea_orm::{ActiveModelTrait, Set};

            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;
            let (admin_token, _org_id) = register_admin!(app);

            // Create invitation via API first
            let invite_email = unique_email();
            let req = test::TestRequest::post()
                .uri("/api/v1/invitaciones")
                .insert_header(("Authorization", format!("Bearer {admin_token}")))
                .set_json(json!({
                    "email": invite_email,
                    "rol": "visualizador"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let inv_body: Value = test::read_body_json(resp).await;
            let inv_id: Uuid = inv_body["id"].as_str().unwrap().parse().unwrap();
            let inv_token = inv_body["token"].as_str().unwrap().to_string();

            // Directly update the invitation to be expired (set expires_at to the past)
            use sea_orm::EntityTrait;
            let record = invitacion::Entity::find_by_id(inv_id)
                .one(&db)
                .await
                .unwrap()
                .unwrap();
            let mut active: invitacion::ActiveModel = record.into();
            let past = chrono::Utc::now() - chrono::Duration::days(1);
            active.expires_at = Set(past.into());
            active.update(&db).await.unwrap();

            // Try to register with expired token
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Expired Invite User",
                    "email": invite_email,
                    "password": "SecurePass123!",
                    "tokenInvitacion": inv_token
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 410, "expired invitation should return 410");

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["error"], "gone");
        });
    }

    // ── used invitation returns 409 ──

    pub fn used_invitation_returns_409() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;
            let (admin_token, _org_id) = register_admin!(app);

            // Create invitation
            let invite_email = unique_email();
            let req = test::TestRequest::post()
                .uri("/api/v1/invitaciones")
                .insert_header(("Authorization", format!("Bearer {admin_token}")))
                .set_json(json!({
                    "email": invite_email,
                    "rol": "gerente"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let inv_body: Value = test::read_body_json(resp).await;
            let inv_token = inv_body["token"].as_str().unwrap().to_string();

            // First registration with token succeeds
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "First User",
                    "email": invite_email,
                    "password": "SecurePass123!",
                    "tokenInvitacion": inv_token
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);

            // Second registration with same token returns 409
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Second User",
                    "email": unique_email(),
                    "password": "SecurePass456!",
                    "tokenInvitacion": inv_token
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 409, "used invitation should return 409");

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["error"], "conflict");
        });
    }

    // ── admin can list and revoke invitations ──

    pub fn admin_can_list_and_revoke_invitations() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;
            let (admin_token, _org_id) = register_admin!(app);

            // Create two invitations
            for role in &["gerente", "visualizador"] {
                let req = test::TestRequest::post()
                    .uri("/api/v1/invitaciones")
                    .insert_header(("Authorization", format!("Bearer {admin_token}")))
                    .set_json(json!({
                        "email": unique_email(),
                        "rol": role
                    }))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), 201);
            }

            // List invitations
            let req = test::TestRequest::get()
                .uri("/api/v1/invitaciones")
                .insert_header(("Authorization", format!("Bearer {admin_token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            let invitations = body.as_array().expect("response should be an array");
            assert!(
                invitations.len() >= 2,
                "should have at least 2 pending invitations"
            );

            // Revoke the first invitation
            let first_id = invitations[0]["id"].as_str().unwrap();
            let req = test::TestRequest::delete()
                .uri(&format!("/api/v1/invitaciones/{first_id}"))
                .insert_header(("Authorization", format!("Bearer {admin_token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 204, "revoke should return 204");

            // List again — should have one fewer
            let req = test::TestRequest::get()
                .uri("/api/v1/invitaciones")
                .insert_header(("Authorization", format!("Bearer {admin_token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            let invitations_after = body.as_array().unwrap();
            assert_eq!(
                invitations_after.len(),
                invitations.len() - 1,
                "should have one fewer invitation after revoke"
            );
        });
    }

    // ── JWT contains organizacion_id after registration ──

    pub fn jwt_contains_organizacion_id() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

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
            let decoded =
                realestate_backend::services::auth::decode_jwt(token, JWT_SECRET).unwrap();
            assert_ne!(
                decoded.organizacion_id,
                Uuid::nil(),
                "JWT should contain a non-nil organizacion_id"
            );

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

    // ── user in org A cannot see propiedades from org B ──

    pub fn org_data_isolation_propiedades() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

            // Register admin A (org A)
            let (token_a, _org_a) = register_admin!(app);

            // Register admin B (org B)
            let (token_b, _org_b) = register_admin!(app);

            // Create a propiedad with org A's token
            let req = test::TestRequest::post()
                .uri("/api/v1/propiedades")
                .insert_header(("Authorization", format!("Bearer {token_a}")))
                .set_json(json!({
                    "titulo": "Casa Org A",
                    "direccion": "Calle A 123",
                    "ciudad": "Santo Domingo",
                    "provincia": "Distrito Nacional",
                    "tipoPropiedad": "casa",
                    "precio": 50000,
                    "moneda": "DOP"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201, "org A should create propiedad");

            // List propiedades with org B's token — should get empty list
            let req = test::TestRequest::get()
                .uri("/api/v1/propiedades")
                .insert_header(("Authorization", format!("Bearer {token_b}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            let items = body["data"]
                .as_array()
                .expect("response should have data array");
            assert!(
                items.is_empty(),
                "org B should see zero propiedades from org A"
            );

            // Verify org A can still see its own propiedad
            let req = test::TestRequest::get()
                .uri("/api/v1/propiedades")
                .insert_header(("Authorization", format!("Bearer {token_a}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            let items = body["data"]
                .as_array()
                .expect("response should have data array");
            assert!(
                items.iter().any(|p| p["titulo"] == "Casa Org A"),
                "org A should see its own propiedad"
            );
        });
    }

    // ── create propiedad sets organizacion_id from claims ──

    pub fn create_propiedad_sets_org_id_from_claims() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

            let (admin_token, org_id) = register_admin!(app);

            // Create a propiedad
            let req = test::TestRequest::post()
                .uri("/api/v1/propiedades")
                .insert_header(("Authorization", format!("Bearer {admin_token}")))
                .set_json(json!({
                    "titulo": "Propiedad OrgId Test",
                    "direccion": "Calle Test 456",
                    "ciudad": "Santiago",
                    "provincia": "Santiago",
                    "tipoPropiedad": "apartamento",
                    "precio": 30000,
                    "moneda": "DOP"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201, "create propiedad should succeed");

            let body: Value = test::read_body_json(resp).await;
            let propiedad_id = body["id"].as_str().unwrap();

            // GET the propiedad and verify it's accessible (proves org_id was set correctly)
            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/propiedades/{propiedad_id}"))
                .insert_header(("Authorization", format!("Bearer {admin_token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                200,
                "should be able to GET the created propiedad"
            );

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["titulo"], "Propiedad OrgId Test");

            // Verify a different org cannot access it
            let (other_token, other_org_id) = register_admin!(app);
            assert_ne!(org_id, other_org_id, "orgs should be different");

            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/propiedades/{propiedad_id}"))
                .insert_header(("Authorization", format!("Bearer {other_token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                404,
                "other org should get 404 for propiedad it doesn't own"
            );
        });
    }

    // ── login returns JWT with correct organizacion_id ──

    pub fn login_returns_jwt_with_correct_org_id() {
        with_db(|db| async move {
            let app =
                test::init_service(create_app(db.clone(), make_config(), make_app_data())).await;

            let email = unique_email();
            let password = "SecurePass123!";

            // Register a new user (creates org)
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(json!({
                    "nombre": "Login Test User",
                    "email": email,
                    "password": password,
                    "tipo": "persona_fisica",
                    "cedula": valid_cedula(),
                    "telefono": "809-555-1234",
                    "nombreOrganizacion": format!("Org Login {}", Uuid::new_v4())
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let reg_body: Value = test::read_body_json(resp).await;
            let expected_org_id: Uuid = reg_body["user"]["organizacionId"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();

            // Login with the same credentials
            let req = test::TestRequest::post()
                .uri("/api/v1/auth/login")
                .set_json(json!({
                    "email": email,
                    "password": password
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200, "login should succeed");

            let login_body: Value = test::read_body_json(resp).await;
            let login_token = login_body["token"].as_str().unwrap();

            // Decode the login JWT and verify organizacion_id matches
            let decoded =
                realestate_backend::services::auth::decode_jwt(login_token, JWT_SECRET).unwrap();
            assert_eq!(
                decoded.organizacion_id, expected_org_id,
                "login JWT should contain the same organizacion_id as registration"
            );

            // Also verify the user response contains the correct org_id
            let login_org_id: Uuid = login_body["user"]["organizacionId"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(
                login_org_id, expected_org_id,
                "login response organizacionId should match registration"
            );
        });
    }
}

// ── Test entry points ──

#[test]
fn persona_fisica_registration_creates_org_and_admin() {
    db_async::persona_fisica_registration_creates_org_and_admin();
}

#[test]
fn persona_juridica_registration_creates_org_and_admin() {
    db_async::persona_juridica_registration_creates_org_and_admin();
}

#[test]
fn duplicate_email_returns_409() {
    db_async::duplicate_email_returns_409();
}

#[test]
fn duplicate_cedula_returns_409() {
    db_async::duplicate_cedula_returns_409();
}

#[test]
fn duplicate_rnc_returns_409() {
    db_async::duplicate_rnc_returns_409();
}

#[test]
fn invalid_rnc_returns_422() {
    db_async::invalid_rnc_returns_422();
}

#[test]
fn invalid_cedula_returns_422() {
    db_async::invalid_cedula_returns_422();
}

#[test]
fn jwt_contains_organizacion_id_after_registration() {
    db_async::jwt_contains_organizacion_id();
}

#[test]
fn admin_can_create_invitation_gerente() {
    db_async::admin_can_create_invitation_gerente();
}

#[test]
fn admin_can_create_invitation_visualizador() {
    db_async::admin_can_create_invitation_visualizador();
}

#[test]
fn non_admin_cannot_create_invitation() {
    db_async::non_admin_cannot_create_invitation();
}

#[test]
fn invitation_registration_joins_org_with_invited_role() {
    db_async::invitation_registration_joins_org_with_invited_role();
}

#[test]
fn expired_invitation_returns_410() {
    db_async::expired_invitation_returns_410();
}

#[test]
fn used_invitation_returns_409() {
    db_async::used_invitation_returns_409();
}

#[test]
fn admin_can_list_and_revoke_invitations() {
    db_async::admin_can_list_and_revoke_invitations();
}

#[test]
fn org_data_isolation_propiedades() {
    db_async::org_data_isolation_propiedades();
}

#[test]
fn create_propiedad_sets_org_id_from_claims() {
    db_async::create_propiedad_sets_org_id_from_claims();
}

#[test]
fn login_returns_jwt_with_correct_org_id() {
    db_async::login_returns_jwt_with_correct_org_id();
}
