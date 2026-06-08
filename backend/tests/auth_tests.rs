#![allow(clippy::needless_return)]

mod db_async {
    use actix_web::test;
    use realestate_backend::app::create_app;
    use serde_json::Value;
    use uuid::Uuid;

    use crate::common::{self, JWT_SECRET};

    fn make_config() -> realestate_backend::config::AppConfig {
        common::test_app_config(common::db_url())
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
        common::with_db(|db| async move {
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

            // Response MUST contain token and user object (LoginResponse)
            assert!(body["token"].is_string(), "response must have 'token'");
            assert!(!body["token"].as_str().unwrap().is_empty());
            assert!(body["user"].is_object(), "response must have 'user' object");

            // User object MUST contain all expected fields
            assert!(body["user"]["id"].is_string(), "user must have 'id'");
            assert_eq!(body["user"]["nombre"], "Test Gerente");
            assert_eq!(body["user"]["email"], email);
            assert_eq!(body["user"]["rol"], "admin");
            assert_eq!(body["user"]["activo"], true);
            assert!(
                body["user"]["organizacionId"].is_string(),
                "user must have 'organizacionId'"
            );
            assert!(
                body["user"]["createdAt"].is_string(),
                "user must have 'createdAt'"
            );

            // Response MUST NOT contain password or session fields
            assert!(
                body.get("password").is_none(),
                "response must NOT contain 'password'"
            );
            assert!(
                body.get("passwordHash").is_none(),
                "response must NOT contain 'passwordHash'"
            );
            assert!(
                body.get("session").is_none(),
                "response must NOT contain 'session'"
            );

            // Verify the top-level response has exactly the expected keys
            let obj = body.as_object().unwrap();
            let expected_keys: std::collections::HashSet<&str> =
                ["token", "user"].into_iter().collect();
            let actual_keys: std::collections::HashSet<&str> =
                obj.keys().map(|k| k.as_str()).collect();
            assert_eq!(
                actual_keys, expected_keys,
                "response keys must match LoginResponse exactly"
            );
        });
    }

    // ── Persisted user has rol == "gerente" and non-null organizacion_id ──

    pub fn register_persists_gerente_role_and_org() {
        common::with_db(|db| async move {
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

            assert_eq!(user.rol, "admin", "persisted user must have rol == 'admin'");
            assert_ne!(
                user.organizacion_id,
                Uuid::nil(),
                "persisted user must have a non-null organizacion_id"
            );
        });
    }

    // ── Duplicate email returns 409 with Spanish message ──

    pub fn register_duplicate_email_returns_409() {
        common::with_db(|db| async move {
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
                body["message"], "El email ya está registrado",
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
