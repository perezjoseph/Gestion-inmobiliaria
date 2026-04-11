#[cfg(test)]
mod perfil_handler_tests {
    use actix_web::http::StatusCode;
    use actix_web::{App, HttpResponse, test, web};
    use chrono::Utc;
    use uuid::Uuid;

    use realestate_backend::config::AppConfig;
    use realestate_backend::errors::AppError;
    use realestate_backend::services::auth::{Claims, encode_jwt};

    const JWT_SECRET: &str = "test_secret_key_that_is_long_enough_32chars!";

    fn test_config() -> AppConfig {
        AppConfig {
            database_url: String::new(),
            jwt_secret: JWT_SECRET.to_string(),
            server_port: 8080,
            cors_origin: None,
        }
    }

    fn make_token(rol: &str) -> String {
        let claims = Claims {
            sub: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            rol: rol.to_string(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };
        encode_jwt(&claims, JWT_SECRET).unwrap()
    }

    async fn claims_stub(claims: Claims) -> Result<HttpResponse, AppError> {
        let _ = claims;
        Ok(HttpResponse::Ok().finish())
    }

    async fn claims_stub_with_body(
        claims: Claims,
        _body: web::Json<serde_json::Value>,
    ) -> Result<HttpResponse, AppError> {
        let _ = claims;
        Ok(HttpResponse::Ok().finish())
    }

    // --- GET /api/perfil ---

    #[actix_web::test]
    async fn perfil_obtener_rejects_unauthenticated() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/perfil", web::get().to(claims_stub)),
        )
        .await;

        let req = test::TestRequest::get().uri("/api/perfil").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn perfil_obtener_allows_visualizador() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/perfil", web::get().to(claims_stub)),
        )
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::get()
            .uri("/api/perfil")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn perfil_obtener_allows_gerente() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/perfil", web::get().to(claims_stub)),
        )
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::get()
            .uri("/api/perfil")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn perfil_obtener_allows_admin() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/perfil", web::get().to(claims_stub)),
        )
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::get()
            .uri("/api/perfil")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- PUT /api/perfil ---

    #[actix_web::test]
    async fn perfil_actualizar_rejects_unauthenticated() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/perfil", web::put().to(claims_stub_with_body)),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/perfil")
            .set_json(serde_json::json!({"nombre": "Nuevo"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn perfil_actualizar_allows_any_authenticated_role() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/perfil", web::put().to(claims_stub_with_body)),
        )
        .await;

        for rol in &["admin", "gerente", "visualizador"] {
            let token = make_token(rol);
            let req = test::TestRequest::put()
                .uri("/api/perfil")
                .insert_header(("Authorization", format!("Bearer {}", token)))
                .set_json(serde_json::json!({"nombre": "Nuevo"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                StatusCode::OK,
                "Role '{}' should be allowed to update profile",
                rol
            );
        }
    }

    // --- PUT /api/perfil/password ---

    #[actix_web::test]
    async fn perfil_password_rejects_unauthenticated() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/perfil/password", web::put().to(claims_stub_with_body)),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/perfil/password")
            .set_json(serde_json::json!({
                "passwordActual": "old",
                "passwordNuevo": "new"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn perfil_password_allows_any_authenticated_role() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/perfil/password", web::put().to(claims_stub_with_body)),
        )
        .await;

        for rol in &["admin", "gerente", "visualizador"] {
            let token = make_token(rol);
            let req = test::TestRequest::put()
                .uri("/api/perfil/password")
                .insert_header(("Authorization", format!("Bearer {}", token)))
                .set_json(serde_json::json!({
                    "passwordActual": "old",
                    "passwordNuevo": "new"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                StatusCode::OK,
                "Role '{}' should be allowed to change password",
                rol
            );
        }
    }
}
