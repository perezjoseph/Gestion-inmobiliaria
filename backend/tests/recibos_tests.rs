#[cfg(test)]
mod recibos_handler_tests {
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
            pool: realestate_backend::config::PoolConfig::default(),
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

    async fn claims_stub(
        _claims: Claims,
        _path: web::Path<Uuid>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok()
            .content_type("application/pdf")
            .body(vec![]))
    }

    #[actix_web::test]
    async fn recibo_rejects_unauthenticated_request() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/pagos/{id}/recibo", web::get().to(claims_stub)),
        )
        .await;

        let pago_id = Uuid::new_v4();
        let req = test::TestRequest::get()
            .uri(&format!("/api/pagos/{pago_id}/recibo"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn recibo_allows_authenticated_request() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/pagos/{id}/recibo", web::get().to(claims_stub)),
        )
        .await;

        let pago_id = Uuid::new_v4();
        let token = make_token("admin");
        let req = test::TestRequest::get()
            .uri(&format!("/api/pagos/{pago_id}/recibo"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn recibo_allows_any_authenticated_role() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/pagos/{id}/recibo", web::get().to(claims_stub)),
        )
        .await;

        for role in &["admin", "gerente", "visualizador"] {
            let pago_id = Uuid::new_v4();
            let token = make_token(role);
            let req = test::TestRequest::get()
                .uri(&format!("/api/pagos/{pago_id}/recibo"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                StatusCode::OK,
                "Role '{role}' should be allowed"
            );
        }
    }
}
