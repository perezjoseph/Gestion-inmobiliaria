#![allow(clippy::needless_return)]

#[cfg(test)]
mod inquilinos_rbac_tests {
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
        };
        encode_jwt(&claims, JWT_SECRET).unwrap()
    }

    async fn write_access_stub(
        _access: WriteAccess,
        _body: web::Json<serde_json::Value>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Created().finish())
    }

    async fn admin_only_path_stub(
        _admin: AdminOnly,
        _path: web::Path<Uuid>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::NoContent().finish())
    }

    // --- POST /api/inquilinos (WriteAccess) ---

    #[actix_web::test]
    async fn create_rejects_unauthenticated() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/inquilinos", web::post().to(write_access_stub)),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/api/inquilinos")
            .set_json(serde_json::json!({"nombre":"A","apellido":"B","cedula":"001"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn create_rejects_visualizador() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/inquilinos", web::post().to(write_access_stub)),
        )
        .await;
        let token = make_token("visualizador");
        let req = test::TestRequest::post()
            .uri("/api/inquilinos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"nombre":"A","apellido":"B","cedula":"001"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn create_allows_gerente() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/inquilinos", web::post().to(write_access_stub)),
        )
        .await;
        let token = make_token("gerente");
        let req = test::TestRequest::post()
            .uri("/api/inquilinos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"nombre":"A","apellido":"B","cedula":"001"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[actix_web::test]
    async fn create_allows_admin() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/inquilinos", web::post().to(write_access_stub)),
        )
        .await;
        let token = make_token("admin");
        let req = test::TestRequest::post()
            .uri("/api/inquilinos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"nombre":"A","apellido":"B","cedula":"001"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // --- DELETE /api/inquilinos/{id} (AdminOnly) ---

    #[actix_web::test]
    async fn delete_rejects_gerente() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/inquilinos/{id}",
            web::delete().to(admin_only_path_stub),
        ))
        .await;
        let token = make_token("gerente");
        let req = test::TestRequest::delete()
            .uri(&format!("/api/inquilinos/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn delete_allows_admin() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/inquilinos/{id}",
            web::delete().to(admin_only_path_stub),
        ))
        .await;
        let token = make_token("admin");
        let req = test::TestRequest::delete()
            .uri(&format!("/api/inquilinos/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
}

#[cfg(test)]
mod db_async {
    use actix_web::test;
    use chrono::Utc;
    use realestate_backend::app::create_app;
    use realestate_backend::config::AppConfig;
    use realestate_backend::services::auth::{Claims, encode_jwt};
    use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
    use serde_json::{Value, json};
    use uuid::Uuid;

    use crate::common::with_db;

    const JWT_SECRET: &str = "test_secret_key_that_is_long_enough_for_jwt";

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
            nombre: Set(format!("Org Test {id}")),
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

    async fn cleanup_inquilino(db: &DatabaseConnection, id: Uuid) {
        use realestate_backend::entities::inquilino;
        let _ = inquilino::Entity::delete_by_id(id).exec(db).await;
    }

    fn base_uri() -> &'static str {
        "/api/inquilinos"
    }

    pub fn crud_cycle() {
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

            // Create
            let req = test::TestRequest::post()
                .uri(base_uri())
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "nombre": "Juan",
                    "apellido": "Pérez",
                    "cedula": "001-0000001-1",
                    "email": "juan@test.com",
                    "telefono": "809-555-0001"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let id = body["id"].as_str().unwrap().to_string();
            assert_eq!(body["nombre"], "Juan");
            assert_eq!(body["apellido"], "Pérez");
            assert_eq!(body["cedula"], "001-0000001-1");

            // Get by ID
            let req = test::TestRequest::get()
                .uri(&format!("{}/{id}", base_uri()))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let detail: Value = test::read_body_json(resp).await;
            assert_eq!(detail["cedula"], "001-0000001-1");

            // Update
            let req = test::TestRequest::put()
                .uri(&format!("{}/{id}", base_uri()))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"nombre": "Juan Carlos"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let updated: Value = test::read_body_json(resp).await;
            assert_eq!(updated["nombre"], "Juan Carlos");
            assert_eq!(updated["cedula"], "001-0000001-1");

            // List
            let req = test::TestRequest::get()
                .uri(base_uri())
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let list: Value = test::read_body_json(resp).await;
            assert!(list["total"].as_u64().unwrap() >= 1);

            // Delete
            let req = test::TestRequest::delete()
                .uri(&format!("{}/{id}", base_uri()))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 204);

            // Verify deleted
            let req = test::TestRequest::get()
                .uri(&format!("{}/{id}", base_uri()))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404);
        });
    }

    pub fn duplicate_cedula_create() {
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

            let cedula = format!("DUP-{}", Uuid::new_v4().as_simple());

            // First create succeeds
            let req = test::TestRequest::post()
                .uri(base_uri())
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"nombre":"A","apellido":"B","cedula": cedula}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let b: Value = test::read_body_json(resp).await;
            let id: Uuid = b["id"].as_str().unwrap().parse().unwrap();

            // Second create with same cedula -> 409
            let req = test::TestRequest::post()
                .uri(base_uri())
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"nombre":"C","apellido":"D","cedula": cedula}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 409);

            cleanup_inquilino(&db, id).await;
        });
    }

    pub fn duplicate_cedula_update() {
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

            let cedula_a = format!("UPD-A-{}", Uuid::new_v4().as_simple());
            let cedula_b = format!("UPD-B-{}", Uuid::new_v4().as_simple());

            // Create two inquilinos with different cedulas
            let req = test::TestRequest::post()
                .uri(base_uri())
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"nombre":"A","apellido":"A","cedula": cedula_a}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let b1: Value = test::read_body_json(resp).await;
            let id1: Uuid = b1["id"].as_str().unwrap().parse().unwrap();

            let req = test::TestRequest::post()
                .uri(base_uri())
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"nombre":"B","apellido":"B","cedula": cedula_b}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let b2: Value = test::read_body_json(resp).await;
            let id2: Uuid = b2["id"].as_str().unwrap().parse().unwrap();

            // Update second to first's cedula -> 409
            let req = test::TestRequest::put()
                .uri(&format!("{}/{id2}", base_uri()))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"cedula": cedula_a}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 409);

            cleanup_inquilino(&db, id1).await;
            cleanup_inquilino(&db, id2).await;
        });
    }

    pub fn update_same_cedula_succeeds() {
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

            let cedula = format!("SELF-{}", Uuid::new_v4().as_simple());

            let req = test::TestRequest::post()
                .uri(base_uri())
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"nombre":"X","apellido":"Y","cedula": cedula}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let b: Value = test::read_body_json(resp).await;
            let id: Uuid = b["id"].as_str().unwrap().parse().unwrap();

            // Update to same cedula -> should not conflict with itself
            let req = test::TestRequest::put()
                .uri(&format!("{}/{id}", base_uri()))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"cedula": cedula}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);

            cleanup_inquilino(&db, id).await;
        });
    }
}

// DB-backed integration tests
#[test]
fn inquilino_crud_cycle() {
    db_async::crud_cycle();
}

#[test]
fn inquilino_duplicate_cedula_on_create_returns_409() {
    db_async::duplicate_cedula_create();
}

#[test]
fn inquilino_duplicate_cedula_on_update_returns_409() {
    db_async::duplicate_cedula_update();
}

#[test]
fn inquilino_update_same_cedula_does_not_conflict() {
    db_async::update_same_cedula_succeeds();
}
