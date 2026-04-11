#[cfg(test)]
mod reportes_handler_tests {
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
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };
        encode_jwt(&claims, JWT_SECRET).unwrap()
    }

    async fn claims_stub(_claims: Claims) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    #[actix_web::test]
    async fn ingresos_rejects_unauthenticated_request() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/reportes/ingresos", web::get().to(claims_stub)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/reportes/ingresos")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn ingresos_pdf_rejects_unauthenticated_request() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/reportes/ingresos/pdf", web::get().to(claims_stub)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/reportes/ingresos/pdf")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn ingresos_xlsx_rejects_unauthenticated_request() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/reportes/ingresos/xlsx", web::get().to(claims_stub)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/reportes/ingresos/xlsx")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn historial_pagos_rejects_unauthenticated_request() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/reportes/historial-pagos", web::get().to(claims_stub)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/reportes/historial-pagos")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn ocupacion_tendencia_rejects_unauthenticated_request() {
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/reportes/ocupacion/tendencia",
            web::get().to(claims_stub),
        ))
        .await;

        let req = test::TestRequest::get()
            .uri("/api/reportes/ocupacion/tendencia")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn ingresos_allows_authenticated_request() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/reportes/ingresos", web::get().to(claims_stub)),
        )
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::get()
            .uri("/api/reportes/ingresos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn ingresos_allows_any_authenticated_role() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/reportes/ingresos", web::get().to(claims_stub)),
        )
        .await;

        for role in &["admin", "gerente", "visualizador"] {
            let token = make_token(role);
            let req = test::TestRequest::get()
                .uri("/api/reportes/ingresos")
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
