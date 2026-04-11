#[cfg(test)]
mod contratos_lifecycle_tests {
    use actix_web::http::StatusCode;
    use actix_web::{App, HttpResponse, test, web};
    use chrono::Utc;
    use uuid::Uuid;

    use realestate_backend::config::AppConfig;
    use realestate_backend::errors::AppError;
    use realestate_backend::middleware::rbac::WriteAccess;
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

    async fn write_access_stub_with_path_and_body(
        _access: WriteAccess,
        _path: web::Path<Uuid>,
        _body: web::Json<serde_json::Value>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    async fn claims_stub(_claims: Claims) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    // --- POST /api/contratos/{id}/renovar (WriteAccess) ---

    #[actix_web::test]
    async fn renovar_rejects_unauthenticated() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/renovar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{}/renovar", id))
            .set_json(serde_json::json!({"fechaFin":"2026-12-31","montoMensual":"25000"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn renovar_rejects_visualizador() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/renovar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{}/renovar", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({"fechaFin":"2026-12-31","montoMensual":"25000"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn renovar_allows_admin() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/renovar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{}/renovar", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({"fechaFin":"2026-12-31","montoMensual":"25000"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn renovar_allows_gerente() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/renovar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{}/renovar", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({"fechaFin":"2026-12-31","montoMensual":"25000"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- POST /api/contratos/{id}/terminar (WriteAccess) ---

    #[actix_web::test]
    async fn terminar_rejects_unauthenticated() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/terminar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{}/terminar", id))
            .set_json(serde_json::json!({"fechaTerminacion":"2025-06-15"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn terminar_rejects_visualizador() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/terminar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{}/terminar", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({"fechaTerminacion":"2025-06-15"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn terminar_allows_admin() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/terminar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{}/terminar", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({"fechaTerminacion":"2025-06-15"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn terminar_allows_gerente() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/terminar",
            web::post().to(write_access_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{}/terminar", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({"fechaTerminacion":"2025-06-15"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- GET /api/contratos/por-vencer (any authenticated user via Claims) ---

    #[actix_web::test]
    async fn por_vencer_rejects_unauthenticated() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/contratos/por-vencer", web::get().to(claims_stub)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/contratos/por-vencer")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn por_vencer_allows_visualizador() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/contratos/por-vencer", web::get().to(claims_stub)),
        )
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::get()
            .uri("/api/contratos/por-vencer")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn por_vencer_allows_admin() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/contratos/por-vencer", web::get().to(claims_stub)),
        )
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::get()
            .uri("/api/contratos/por-vencer")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn por_vencer_allows_gerente() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/contratos/por-vencer", web::get().to(claims_stub)),
        )
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::get()
            .uri("/api/contratos/por-vencer")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
