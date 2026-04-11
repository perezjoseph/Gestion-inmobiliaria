#[cfg(test)]
mod auditoria_handler_tests {
    use actix_web::http::StatusCode;
    use actix_web::{App, HttpResponse, test, web};
    use chrono::Utc;
    use uuid::Uuid;

    use realestate_backend::config::AppConfig;
    use realestate_backend::errors::AppError;
    use realestate_backend::middleware::rbac::AdminOnly;
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

    async fn admin_only_stub(_admin: AdminOnly) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    #[actix_web::test]
    async fn auditoria_list_rejects_unauthenticated_request() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/auditoria", web::get().to(admin_only_stub)),
        )
        .await;

        let req = test::TestRequest::get().uri("/api/auditoria").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn auditoria_list_rejects_visualizador_role() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/auditoria", web::get().to(admin_only_stub)),
        )
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::get()
            .uri("/api/auditoria")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn auditoria_list_rejects_gerente_role() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/auditoria", web::get().to(admin_only_stub)),
        )
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::get()
            .uri("/api/auditoria")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn auditoria_list_allows_admin_role() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/auditoria", web::get().to(admin_only_stub)),
        )
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::get()
            .uri("/api/auditoria")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
