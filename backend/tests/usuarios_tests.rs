#[cfg(test)]
mod usuarios_handler_tests {
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

    async fn admin_only_stub_with_path(
        _admin: AdminOnly,
        _path: web::Path<Uuid>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    async fn admin_only_stub_with_path_and_body(
        _admin: AdminOnly,
        _path: web::Path<Uuid>,
        _body: web::Json<serde_json::Value>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    // --- GET /api/usuarios (list) ---

    #[actix_web::test]
    async fn usuarios_list_rejects_unauthenticated() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/usuarios", web::get().to(admin_only_stub)),
        )
        .await;

        let req = test::TestRequest::get().uri("/api/usuarios").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn usuarios_list_rejects_visualizador() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/usuarios", web::get().to(admin_only_stub)),
        )
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::get()
            .uri("/api/usuarios")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn usuarios_list_rejects_gerente() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/usuarios", web::get().to(admin_only_stub)),
        )
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::get()
            .uri("/api/usuarios")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn usuarios_list_allows_admin() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/usuarios", web::get().to(admin_only_stub)),
        )
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::get()
            .uri("/api/usuarios")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- PUT /api/usuarios/{id}/rol ---

    #[actix_web::test]
    async fn usuarios_cambiar_rol_rejects_unauthenticated() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/usuarios/{id}/rol",
            web::put().to(admin_only_stub_with_path_and_body),
        ))
        .await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/usuarios/{}/rol", id))
            .set_json(serde_json::json!({"nuevoRol": "gerente"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn usuarios_cambiar_rol_rejects_visualizador() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/usuarios/{id}/rol",
            web::put().to(admin_only_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::put()
            .uri(&format!("/api/usuarios/{}/rol", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({"nuevoRol": "gerente"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn usuarios_cambiar_rol_rejects_gerente() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/usuarios/{id}/rol",
            web::put().to(admin_only_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::put()
            .uri(&format!("/api/usuarios/{}/rol", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({"nuevoRol": "gerente"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn usuarios_cambiar_rol_allows_admin() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/usuarios/{id}/rol",
            web::put().to(admin_only_stub_with_path_and_body),
        ))
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::put()
            .uri(&format!("/api/usuarios/{}/rol", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({"nuevoRol": "gerente"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- PUT /api/usuarios/{id}/activar ---

    #[actix_web::test]
    async fn usuarios_activar_rejects_unauthenticated() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/usuarios/{id}/activar",
            web::put().to(admin_only_stub_with_path),
        ))
        .await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/usuarios/{}/activar", id))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn usuarios_activar_rejects_visualizador() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/usuarios/{id}/activar",
            web::put().to(admin_only_stub_with_path),
        ))
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::put()
            .uri(&format!("/api/usuarios/{}/activar", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn usuarios_activar_rejects_gerente() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/usuarios/{id}/activar",
            web::put().to(admin_only_stub_with_path),
        ))
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::put()
            .uri(&format!("/api/usuarios/{}/activar", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn usuarios_activar_allows_admin() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/usuarios/{id}/activar",
            web::put().to(admin_only_stub_with_path),
        ))
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::put()
            .uri(&format!("/api/usuarios/{}/activar", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- PUT /api/usuarios/{id}/desactivar ---

    #[actix_web::test]
    async fn usuarios_desactivar_rejects_unauthenticated() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/usuarios/{id}/desactivar",
            web::put().to(admin_only_stub_with_path),
        ))
        .await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/usuarios/{}/desactivar", id))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn usuarios_desactivar_rejects_visualizador() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/usuarios/{id}/desactivar",
            web::put().to(admin_only_stub_with_path),
        ))
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::put()
            .uri(&format!("/api/usuarios/{}/desactivar", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn usuarios_desactivar_rejects_gerente() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/usuarios/{id}/desactivar",
            web::put().to(admin_only_stub_with_path),
        ))
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::put()
            .uri(&format!("/api/usuarios/{}/desactivar", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn usuarios_desactivar_allows_admin() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/usuarios/{id}/desactivar",
            web::put().to(admin_only_stub_with_path),
        ))
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::put()
            .uri(&format!("/api/usuarios/{}/desactivar", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
