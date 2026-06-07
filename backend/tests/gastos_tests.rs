#![allow(clippy::needless_return)]
use crate::migrations;

#[cfg(test)]
mod gastos_rbac_tests {
    use actix_web::http::StatusCode;
    use actix_web::{App, HttpResponse, test, web};
    use chrono::Utc;
    use uuid::Uuid;

    use realestate_backend::config::AppConfig;
    use realestate_backend::errors::AppError;
    use realestate_backend::middleware::rbac::WriteAccess;
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

    async fn write_access_path_body_stub(
        _access: WriteAccess,
        _path: web::Path<Uuid>,
        _body: web::Json<serde_json::Value>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    async fn write_access_path_stub(
        _access: WriteAccess,
        _path: web::Path<Uuid>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::NoContent().finish())
    }

    async fn claims_stub(_claims: Claims) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    async fn claims_path_stub(
        _claims: Claims,
        _path: web::Path<Uuid>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    #[actix_web::test]
    async fn visualizador_can_list_gastos() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/gastos", web::get().to(claims_stub)),
        )
        .await;
        let token = make_token("visualizador");
        let req = test::TestRequest::get()
            .uri("/api/gastos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn visualizador_can_get_gasto_by_id() {
        let id = Uuid::new_v4();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/gastos/{id}", web::get().to(claims_path_stub)),
        )
        .await;
        let token = make_token("visualizador");
        let req = test::TestRequest::get()
            .uri(&format!("/api/gastos/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn visualizador_cannot_create_gasto() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/gastos", web::post().to(write_access_stub)),
        )
        .await;
        let token = make_token("visualizador");
        let req = test::TestRequest::post()
            .uri("/api/gastos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn visualizador_cannot_update_gasto() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/gastos/{id}",
            web::put().to(write_access_path_body_stub),
        ))
        .await;
        let token = make_token("visualizador");
        let req = test::TestRequest::put()
            .uri(&format!("/api/gastos/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"estado": "pagado"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn visualizador_cannot_delete_gasto() {
        let id = Uuid::new_v4();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/gastos/{id}", web::delete().to(write_access_path_stub)),
        )
        .await;
        let token = make_token("visualizador");
        let req = test::TestRequest::delete()
            .uri(&format!("/api/gastos/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn admin_can_create_gasto() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/gastos", web::post().to(write_access_stub)),
        )
        .await;
        let token = make_token("admin");
        let req = test::TestRequest::post()
            .uri("/api/gastos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[actix_web::test]
    async fn gerente_can_create_gasto() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/gastos", web::post().to(write_access_stub)),
        )
        .await;
        let token = make_token("gerente");
        let req = test::TestRequest::post()
            .uri("/api/gastos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[actix_web::test]
    async fn admin_can_update_gasto() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/gastos/{id}",
            web::put().to(write_access_path_body_stub),
        ))
        .await;
        let token = make_token("admin");
        let req = test::TestRequest::put()
            .uri(&format!("/api/gastos/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"estado": "pagado"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn admin_can_delete_gasto() {
        let id = Uuid::new_v4();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/gastos/{id}", web::delete().to(write_access_path_stub)),
        )
        .await;
        let token = make_token("admin");
        let req = test::TestRequest::delete()
            .uri(&format!("/api/gastos/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn unauthenticated_cannot_list_gastos() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/gastos", web::get().to(claims_stub)),
        )
        .await;
        let req = test::TestRequest::get().uri("/api/gastos").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn unauthenticated_cannot_create_gasto() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(test_config()))
                .route("/api/gastos", web::post().to(write_access_stub)),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/api/gastos")
            .set_json(serde_json::json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}

mod db_async {
    use actix_web::test;
    use chrono::Utc;
    use realestate_backend::app::create_app;
    use realestate_backend::config::AppConfig;
    use realestate_backend::services::auth::{Claims, encode_jwt};
    use rust_decimal::Decimal;
    use sea_orm::{
        ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait, Set,
    };
    use sea_orm_migration::MigratorTrait;
    use serde_json::{Value, json};
    use std::net::SocketAddr;
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
        let _guard = crate::GLOBAL_DB_SERIAL
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some((rt, db)) = shared_rt_and_db() else {
            eprintln!("Database not reachable -- skipping DB integration test");
            return;
        };
        rt.block_on(f(db.clone()));
    }

    fn make_config() -> AppConfig {
        AppConfig {
            pool: realestate_backend::config::PoolConfig::default(),
            chatbot: realestate_backend::config::ChatbotEnvConfig::for_testing(),
            database_url: String::new(),
            jwt_secret: JWT_SECRET.to_string(),
            server_port: 0,
            cors_origin: None,
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

    async fn create_test_propiedad(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::propiedad;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        propiedad::ActiveModel {
            id: Set(id),
            titulo: Set("Propiedad Test Gastos".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle Test 123".to_string()),
            ciudad: Set("Santo Domingo".to_string()),
            provincia: Set("Distrito Nacional".to_string()),
            tipo_propiedad: Set("apartamento".to_string()),
            habitaciones: Set(Some(2)),
            banos: Set(Some(1)),
            area_m2: Set(Some(Decimal::new(8000, 2))),
            precio: Set(Decimal::new(2500000, 2)),
            moneda: Set("DOP".to_string()),
            estado: Set("disponible".to_string()),
            imagenes: Set(None),
            organizacion_id: Set(org_id),
            valor_catastral: Set(None),
            exento_ipi: Set(false),
            motivo_exencion: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("Failed to create test propiedad");
        id
    }

    async fn cleanup_gasto(db: &DatabaseConnection, id: Uuid) {
        use realestate_backend::entities::gasto;
        let _ = gasto::Entity::delete_by_id(id).exec(db).await;
    }

    pub fn crud_cycle() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "categoria": "mantenimiento",
                    "descripcion": "ReparaciÃ³n de techo",
                    "monto": "15000.50",
                    "moneda": "DOP",
                    "fechaGasto": "2025-04-01",
                    "proveedor": "Constructora ABC",
                    "numeroFactura": "FAC-001"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let gasto_id = body["id"].as_str().unwrap().to_string();
            assert_eq!(body["estado"], "pendiente");
            assert_eq!(body["categoria"], "mantenimiento");
            assert_eq!(body["descripcion"], "ReparaciÃ³n de techo");
            assert_eq!(body["moneda"], "DOP");

            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/gastos/{gasto_id}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let detail: Value = test::read_body_json(resp).await;
            assert_eq!(detail["id"], gasto_id);
            assert_eq!(detail["categoria"], "mantenimiento");

            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/gastos/{gasto_id}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"estado": "pagado", "monto": "20000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let updated: Value = test::read_body_json(resp).await;
            assert_eq!(updated["estado"], "pagado");
            assert_eq!(updated["descripcion"], "ReparaciÃ³n de techo");

            let req = test::TestRequest::delete()
                .uri(&format!("/api/v1/gastos/{gasto_id}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 204);

            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/gastos/{gasto_id}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404);
        });
    }

    pub fn pagination() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let mut created_ids = Vec::new();
            for i in 0..3u32 {
                let req = test::TestRequest::post()
                    .uri("/api/v1/gastos")
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(json!({
                        "propiedadId": propiedad_id,
                        "categoria": "mantenimiento",
                        "descripcion": format!("Gasto paginaciÃ³n {i}"),
                        "monto": "1000",
                        "moneda": "DOP",
                        "fechaGasto": "2025-04-01"
                    }))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                let body: Value = test::read_body_json(resp).await;
                created_ids.push(body["id"].as_str().unwrap().parse::<Uuid>().unwrap());
            }

            let req = test::TestRequest::get()
                .uri(&format!(
                    "/api/v1/gastos?propiedadId={propiedad_id}&page=1&perPage=2"
                ))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            assert!(body["total"].as_u64().unwrap() >= 3);
            assert_eq!(body["page"], 1);
            assert_eq!(body["perPage"], 2);
            assert_eq!(body["data"].as_array().unwrap().len(), 2);

            for id in &created_ids {
                cleanup_gasto(&db, *id).await;
            }
        });
    }

    pub fn filter_propiedad_id() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let prop_a = create_test_propiedad(&db, org_id).await;
            let prop_b = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": prop_a,
                    "categoria": "impuestos",
                    "descripcion": "Gasto prop A",
                    "monto": "5000",
                    "moneda": "DOP",
                    "fechaGasto": "2025-04-01"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let b1: Value = test::read_body_json(resp).await;
            let id1: Uuid = b1["id"].as_str().unwrap().parse().unwrap();

            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": prop_b,
                    "categoria": "seguro",
                    "descripcion": "Gasto prop B",
                    "monto": "3000",
                    "moneda": "DOP",
                    "fechaGasto": "2025-04-01"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let b2: Value = test::read_body_json(resp).await;
            let id2: Uuid = b2["id"].as_str().unwrap().parse().unwrap();

            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/gastos?propiedadId={prop_a}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let body: Value = test::read_body_json(resp).await;
            for item in body["data"].as_array().unwrap() {
                assert_eq!(item["propiedadId"], prop_a.to_string());
            }

            cleanup_gasto(&db, id1).await;
            cleanup_gasto(&db, id2).await;
        });
    }

    pub fn filter_categoria() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "categoria": "administracion",
                    "descripcion": "Gasto legal",
                    "monto": "8000",
                    "moneda": "DOP",
                    "fechaGasto": "2025-04-01"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let b: Value = test::read_body_json(resp).await;
            let gasto_id: Uuid = b["id"].as_str().unwrap().parse().unwrap();

            let req = test::TestRequest::get()
                .uri(&format!(
                    "/api/v1/gastos?propiedadId={propiedad_id}&categoria=administracion"
                ))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let body: Value = test::read_body_json(resp).await;
            for item in body["data"].as_array().unwrap() {
                assert_eq!(item["categoria"], "administracion");
            }

            cleanup_gasto(&db, gasto_id).await;
        });
    }

    pub fn filter_date_range() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let mut ids = Vec::new();
            for date in &["2025-01-15", "2025-03-15", "2025-06-15"] {
                let req = test::TestRequest::post()
                    .uri("/api/v1/gastos")
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(json!({
                        "propiedadId": propiedad_id,
                        "categoria": "mantenimiento",
                        "descripcion": format!("Gasto {date}"),
                        "monto": "1000",
                        "moneda": "DOP",
                        "fechaGasto": date
                    }))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                let b: Value = test::read_body_json(resp).await;
                ids.push(b["id"].as_str().unwrap().parse::<Uuid>().unwrap());
            }

            let req = test::TestRequest::get()
                .uri(&format!(
                    "/api/v1/gastos?propiedadId={propiedad_id}&fechaDesde=2025-02-01&fechaHasta=2025-05-01"
                ))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let body: Value = test::read_body_json(resp).await;
            for item in body["data"].as_array().unwrap() {
                let fecha = item["fechaGasto"].as_str().unwrap();
                assert!(("2025-02-01"..="2025-05-01").contains(&fecha));
            }

            for id in &ids {
                cleanup_gasto(&db, *id).await;
            }
        });
    }

    pub fn csv_import_valid() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let csv = format!(
                "propiedad_id,categoria,descripcion,monto,moneda,fecha_gasto\n\
                 {propiedad_id},mantenimiento,ReparaciÃ³n tuberÃ­a,5000.00,DOP,2025-04-01\n\
                 {propiedad_id},impuestos,Impuesto predial,12000.00,DOP,2025-04-15"
            );
            let boundary = "----TestBoundary";
            let payload = format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"gastos.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n--{boundary}--\r\n"
            );

            let req = test::TestRequest::post()
                .uri("/api/v1/importar/gastos")
                .peer_addr("127.0.0.1:12345".parse::<SocketAddr>().unwrap())
                .insert_header(("Authorization", format!("Bearer {token}")))
                .insert_header((
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                ))
                .set_payload(payload)
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let result: Value = test::read_body_json(resp).await;
            assert_eq!(result["totalFilas"], 2);
            assert_eq!(result["exitosos"], 2);
            assert_eq!(result["fallidos"].as_array().unwrap().len(), 0);
        });
    }

    pub fn csv_import_mixed() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let fake_prop = Uuid::new_v4();
            let csv = format!(
                "propiedad_id,categoria,descripcion,monto,moneda,fecha_gasto\n\
                 {propiedad_id},mantenimiento,Valid row,1000.00,DOP,2025-04-01\n\
                 {fake_prop},mantenimiento,Invalid prop,2000.00,DOP,2025-04-02\n\
                 {propiedad_id},seguros,Another valid,3000.00,DOP,2025-04-03"
            );
            let boundary = "----TestBoundary2";
            let payload = format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"mixed.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n--{boundary}--\r\n"
            );

            let req = test::TestRequest::post()
                .uri("/api/v1/importar/gastos")
                .peer_addr("127.0.0.1:12345".parse::<SocketAddr>().unwrap())
                .insert_header(("Authorization", format!("Bearer {token}")))
                .insert_header((
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                ))
                .set_payload(payload)
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let result: Value = test::read_body_json(resp).await;
            assert_eq!(result["totalFilas"], 3);
            let exitosos = result["exitosos"].as_u64().unwrap();
            let fallidos = result["fallidos"].as_array().unwrap().len() as u64;
            assert_eq!(exitosos + fallidos, 3);
            assert!(fallidos >= 1);
        });
    }

    pub fn csv_import_empty() {
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

            let csv = "propiedad_id,categoria,descripcion,monto,moneda,fecha_gasto\n";
            let boundary = "----TestBoundary3";
            let payload = format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"empty.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n--{boundary}--\r\n"
            );

            let req = test::TestRequest::post()
                .uri("/api/v1/importar/gastos")
                .peer_addr("127.0.0.1:12345".parse::<SocketAddr>().unwrap())
                .insert_header(("Authorization", format!("Bearer {token}")))
                .insert_header((
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                ))
                .set_payload(payload)
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    pub fn profitability_json() {
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

            let req = test::TestRequest::get()
                .uri("/api/v1/reportes/rentabilidad?mes=4&anio=2025")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            assert!(body.get("rows").is_some());
            assert!(body.get("totalIngresos").is_some());
            assert!(body.get("totalGastos").is_some());
            assert!(body.get("totalNeto").is_some());
            assert!(body.get("mes").is_some());
            assert!(body.get("anio").is_some());
        });
    }

    pub fn profitability_pdf() {
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

            let req = test::TestRequest::get()
                .uri("/api/v1/reportes/rentabilidad/pdf?mes=4&anio=2025")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body = test::read_body(resp).await;
            assert!(!body.is_empty());
        });
    }

    pub fn profitability_xlsx() {
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

            let req = test::TestRequest::get()
                .uri("/api/v1/reportes/rentabilidad/xlsx?mes=4&anio=2025")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body = test::read_body(resp).await;
            assert!(!body.is_empty());
        });
    }

    pub fn dashboard_stats_gastos() {
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

            let req = test::TestRequest::get()
                .uri("/api/v1/dashboard/stats")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            assert!(body.get("totalGastosMes").is_some());
        });
    }

    pub fn gastos_comparacion() {
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

            let req = test::TestRequest::get()
                .uri("/api/v1/dashboard/gastos-comparacion")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            assert!(body.get("mesActual").is_some());
            assert!(body.get("mesAnterior").is_some());
            assert!(body.get("porcentajeCambio").is_some());
        });
    }
}

// Task 9.1: DB-backed tests
#[test]
fn crud_cycle_create_get_update_delete() {
    db_async::crud_cycle();
}

#[test]
fn pagination_returns_correct_structure() {
    db_async::pagination();
}

#[test]
fn filter_by_propiedad_id() {
    db_async::filter_propiedad_id();
}

#[test]
fn filter_by_categoria() {
    db_async::filter_categoria();
}

#[test]
fn filter_by_date_range() {
    db_async::filter_date_range();
}

// Task 9.2: CSV import tests
#[test]
fn csv_import_valid_file() {
    db_async::csv_import_valid();
}

#[test]
fn csv_import_mixed_valid_invalid_rows() {
    db_async::csv_import_mixed();
}

#[test]
fn csv_import_empty_returns_422() {
    db_async::csv_import_empty();
}

// Task 9.3: Profitability report tests
#[test]
fn profitability_report_json_returns_correct_structure() {
    db_async::profitability_json();
}

#[test]
fn profitability_report_pdf_returns_bytes() {
    db_async::profitability_pdf();
}

#[test]
fn profitability_report_xlsx_returns_bytes() {
    db_async::profitability_xlsx();
}

// Task 9.4: Dashboard tests
#[test]
fn dashboard_stats_includes_total_gastos_mes() {
    db_async::dashboard_stats_gastos();
}

#[test]
fn gastos_comparacion_returns_correct_structure() {
    db_async::gastos_comparacion();
}

/// Integration tests for utility gasto validation (DR Legal Compliance).
mod gastos_utility_db_tests {
    use actix_web::test;
    use chrono::Utc;
    use realestate_backend::app::create_app;
    use realestate_backend::config::AppConfig;
    use realestate_backend::services::auth::{Claims, encode_jwt};
    use rust_decimal::Decimal;
    use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, Set};
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
        let _guard = crate::GLOBAL_DB_SERIAL
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some((rt, db)) = shared_rt_and_db() else {
            eprintln!("Database not reachable -- skipping DB integration test");
            return;
        };
        rt.block_on(f(db.clone()));
    }

    fn make_config() -> AppConfig {
        AppConfig {
            pool: realestate_backend::config::PoolConfig::default(),
            chatbot: realestate_backend::config::ChatbotEnvConfig::for_testing(),
            database_url: String::new(),
            jwt_secret: JWT_SECRET.to_string(),
            server_port: 0,
            cors_origin: None,
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
            nombre: Set(format!("Org Utility Test {id}")),
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

    async fn create_test_propiedad(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::propiedad;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        propiedad::ActiveModel {
            id: Set(id),
            titulo: Set("Propiedad Utility Test".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle Utility 123".to_string()),
            ciudad: Set("Santo Domingo".to_string()),
            provincia: Set("Distrito Nacional".to_string()),
            tipo_propiedad: Set("apartamento".to_string()),
            habitaciones: Set(Some(2)),
            banos: Set(Some(1)),
            area_m2: Set(Some(Decimal::new(8000, 2))),
            precio: Set(Decimal::new(2500000, 2)),
            moneda: Set("DOP".to_string()),
            estado: Set("disponible".to_string()),
            imagenes: Set(None),
            organizacion_id: Set(org_id),
            valor_catastral: Set(None),
            exento_ipi: Set(false),
            motivo_exencion: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("Failed to create test propiedad");
        id
    }

    /// Test: proveedor_servicio required when categoria=servicio_publico
    pub fn proveedor_servicio_required_for_servicio_publico() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Missing proveedorServicio when categoria=servicio_publico
            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "categoria": "servicios",
                    "descripcion": "Electricidad sin proveedor",
                    "monto": "2500",
                    "moneda": "DOP",
                    "fechaGasto": "2025-04-01"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    /// Test: consumo must be > 0 when provided
    pub fn consumo_must_be_positive() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // consumo = 0 should be rejected
            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "categoria": "servicios",
                    "descripcion": "Electricidad consumo cero",
                    "monto": "2500",
                    "moneda": "DOP",
                    "fechaGasto": "2025-04-01",
                    "proveedorServicio": "EDENORTE",
                    "consumo": "0",
                    "unidadConsumo": "kWh"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    /// Test: periodo_desde must be before periodo_hasta
    pub fn periodo_ordering_validation() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // periodo_desde > periodo_hasta should be rejected
            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "categoria": "servicios",
                    "descripcion": "Electricidad periodo invertido",
                    "monto": "2500",
                    "moneda": "DOP",
                    "fechaGasto": "2025-04-01",
                    "proveedorServicio": "EDENORTE",
                    "consumo": "100",
                    "unidadConsumo": "kWh",
                    "periodoDesde": "2025-04-30",
                    "periodoHasta": "2025-04-01"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    /// Test: Filter gastos by proveedor_servicio
    pub fn filter_by_proveedor_servicio() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create EDENORTE gasto
            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "categoria": "servicios",
                    "descripcion": "Electricidad EDENORTE",
                    "monto": "2500",
                    "moneda": "DOP",
                    "fechaGasto": "2025-04-01",
                    "proveedorServicio": "EDENORTE",
                    "consumo": "100",
                    "unidadConsumo": "kWh",
                    "periodoDesde": "2025-03-01",
                    "periodoHasta": "2025-03-31"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);

            // Create CAASD gasto
            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "categoria": "servicios",
                    "descripcion": "Agua CAASD",
                    "monto": "800",
                    "moneda": "DOP",
                    "fechaGasto": "2025-04-01",
                    "proveedorServicio": "CAASD",
                    "consumo": "15",
                    "unidadConsumo": "m3",
                    "periodoDesde": "2025-03-01",
                    "periodoHasta": "2025-03-31"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);

            // Filter by proveedorServicio=EDENORTE
            let req = test::TestRequest::get()
                .uri(&format!(
                    "/api/v1/gastos?propiedadId={propiedad_id}&proveedorServicio=EDENORTE"
                ))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            for item in body["data"].as_array().unwrap() {
                assert_eq!(item["proveedorServicio"], "EDENORTE");
            }
        });
    }

    /// Test: Filter gastos by periodo date range
    pub fn filter_by_periodo_date_range() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create gastos with different periodos
            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "categoria": "servicios",
                    "descripcion": "Electricidad enero",
                    "monto": "2500",
                    "moneda": "DOP",
                    "fechaGasto": "2025-01-15",
                    "proveedorServicio": "EDENORTE",
                    "consumo": "100",
                    "unidadConsumo": "kWh",
                    "periodoDesde": "2025-01-01",
                    "periodoHasta": "2025-01-31"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);

            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "categoria": "servicios",
                    "descripcion": "Electricidad abril",
                    "monto": "2800",
                    "moneda": "DOP",
                    "fechaGasto": "2025-04-15",
                    "proveedorServicio": "EDENORTE",
                    "consumo": "110",
                    "unidadConsumo": "kWh",
                    "periodoDesde": "2025-04-01",
                    "periodoHasta": "2025-04-30"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);

            // Filter by periodo range that includes only April
            let req = test::TestRequest::get()
                .uri(&format!(
                    "/api/v1/gastos?propiedadId={propiedad_id}&periodoDesde=2025-03-01&periodoHasta=2025-05-01"
                ))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            // Should contain at least the April gasto
            assert!(!body["data"].as_array().unwrap().is_empty());
        });
    }
}

// Task 14.6: Utility gasto validation tests
#[test]
fn utility_gasto_proveedor_servicio_required() {
    gastos_utility_db_tests::proveedor_servicio_required_for_servicio_publico();
}

#[test]
fn utility_gasto_consumo_must_be_positive() {
    gastos_utility_db_tests::consumo_must_be_positive();
}

#[test]
fn utility_gasto_periodo_ordering_validation() {
    gastos_utility_db_tests::periodo_ordering_validation();
}

#[test]
fn utility_gasto_filter_by_proveedor_servicio() {
    gastos_utility_db_tests::filter_by_proveedor_servicio();
}

#[test]
fn utility_gasto_filter_by_periodo_date_range() {
    gastos_utility_db_tests::filter_by_periodo_date_range();
}

// Feature: spec-gap-remediation, Task 11.11
// Tests for `categoria` enum validation and utility-fields round-trip
// **Validates: Requirements 9.4, 9.5**

mod gastos_categoria_and_utility_tests {
    use actix_web::test;
    use chrono::Utc;
    use proptest::prelude::*;
    use proptest::strategy::ValueTree;
    use proptest::test_runner::{Config as ProptestConfig, TestRunner};
    use realestate_backend::app::create_app;
    use realestate_backend::config::AppConfig;
    use realestate_backend::services::auth::{Claims, encode_jwt};
    use realestate_backend::services::gastos::CATEGORIAS_GASTO;
    use rust_decimal::Decimal;
    use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, Set};
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
        let _guard = crate::GLOBAL_DB_SERIAL
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some((rt, db)) = shared_rt_and_db() else {
            eprintln!("Database not reachable -- skipping DB integration test");
            return;
        };
        rt.block_on(f(db.clone()));
    }

    fn make_config() -> AppConfig {
        AppConfig {
            pool: realestate_backend::config::PoolConfig::default(),
            chatbot: realestate_backend::config::ChatbotEnvConfig::for_testing(),
            database_url: String::new(),
            jwt_secret: JWT_SECRET.to_string(),
            server_port: 0,
            cors_origin: None,
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

    async fn create_test_propiedad(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::propiedad;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        propiedad::ActiveModel {
            id: Set(id),
            titulo: Set("Propiedad Test Categoria".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle Test 456".to_string()),
            ciudad: Set("Santo Domingo".to_string()),
            provincia: Set("Distrito Nacional".to_string()),
            tipo_propiedad: Set("apartamento".to_string()),
            habitaciones: Set(Some(2)),
            banos: Set(Some(1)),
            area_m2: Set(Some(Decimal::new(8000, 2))),
            precio: Set(Decimal::new(2500000, 2)),
            moneda: Set("DOP".to_string()),
            estado: Set("disponible".to_string()),
            imagenes: Set(None),
            organizacion_id: Set(org_id),
            valor_catastral: Set(None),
            exento_ipi: Set(false),
            motivo_exencion: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("Failed to create test propiedad");
        id
    }

    async fn cleanup_gasto(db: &DatabaseConnection, id: Uuid) {
        use realestate_backend::entities::gasto;
        use sea_orm::EntityTrait;
        let _ = gasto::Entity::delete_by_id(id).exec(db).await;
    }

    /// Property-style negative test: random non-enum strings for `categoria`
    /// must be rejected with HTTP 422 and a Spanish error message.
    /// **Validates: Requirements 9.4**
    pub fn invalid_categoria_rejected_with_422() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let valid_set: Vec<&str> = CATEGORIAS_GASTO.to_vec();
            let cases = crate::pbt_cases();

            // Generate random invalid categoria strings using proptest's TestRunner
            let invalid_categoria_strategy = "[a-zA-Z0-9_]{1,30}"
                .prop_filter("must not be a valid categoria", move |s| {
                    !valid_set.contains(&s.as_str())
                });

            let mut runner = TestRunner::new(ProptestConfig {
                cases,
                ..Default::default()
            });

            // Collect generated values using new_tree
            let mut invalid_categorias = Vec::with_capacity(cases as usize);
            for _ in 0..cases {
                let tree = invalid_categoria_strategy
                    .new_tree(&mut runner)
                    .expect("Failed to generate test value");
                invalid_categorias.push(tree.current());
            }

            for invalid_cat in &invalid_categorias {
                let req = test::TestRequest::post()
                    .uri("/api/v1/gastos")
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(json!({
                        "propiedadId": propiedad_id,
                        "categoria": invalid_cat,
                        "descripcion": "Test invalid categoria",
                        "monto": "1000",
                        "moneda": "DOP",
                        "fechaGasto": "2025-04-01"
                    }))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(
                    resp.status().as_u16(),
                    422,
                    "Expected 422 for invalid categoria '{invalid_cat}', got {}",
                    resp.status()
                );
                let body: Value = test::read_body_json(resp).await;
                let message = body["message"].as_str().unwrap_or("");
                assert!(
                    message.contains("gasto no v"),
                    "Expected Spanish error message for '{invalid_cat}', got: {message}"
                );
            }
        });
    }

    /// Round-trip test: create a gasto with utility fields (`proveedor`,
    /// `numero_cuenta`, `periodo_inicio`, `periodo_fin`) and verify they
    /// are persisted and returned correctly on read.
    /// **Validates: Requirements 9.5**
    pub fn utility_fields_round_trip() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create a gasto with all utility fields populated
            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "categoria": "administracion",
                    "descripcion": "Gasto con campos de utilidad",
                    "monto": "5000.00",
                    "moneda": "DOP",
                    "fechaGasto": "2025-06-15",
                    "proveedor": "Edenorte Dominicana",
                    "numeroCuenta": "ACCT-2025-001",
                    "periodoInicio": "2025-05-01",
                    "periodoFin": "2025-05-31"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status().as_u16(),
                201,
                "Expected 201 Created for gasto with utility fields"
            );
            let created: Value = test::read_body_json(resp).await;
            let gasto_id = created["id"].as_str().unwrap().to_string();

            // Verify utility fields in create response
            assert_eq!(created["proveedor"], "Edenorte Dominicana");
            assert_eq!(created["numeroCuenta"], "ACCT-2025-001");
            assert_eq!(created["periodoInicio"], "2025-05-01");
            assert_eq!(created["periodoFin"], "2025-05-31");

            // Read back and verify round-trip
            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/gastos/{gasto_id}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status().as_u16(), 200);
            let detail: Value = test::read_body_json(resp).await;

            assert_eq!(detail["id"], gasto_id);
            assert_eq!(detail["proveedor"], "Edenorte Dominicana");
            assert_eq!(detail["numeroCuenta"], "ACCT-2025-001");
            assert_eq!(detail["periodoInicio"], "2025-05-01");
            assert_eq!(detail["periodoFin"], "2025-05-31");
            assert_eq!(detail["categoria"], "administracion");

            // Also test with null utility fields (they should be optional)
            let req = test::TestRequest::post()
                .uri("/api/v1/gastos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "categoria": "impuestos",
                    "descripcion": "Gasto sin campos de utilidad",
                    "monto": "3000.00",
                    "moneda": "DOP",
                    "fechaGasto": "2025-06-20"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status().as_u16(), 201);
            let created_no_utility: Value = test::read_body_json(resp).await;
            let gasto_id_2 = created_no_utility["id"].as_str().unwrap().to_string();

            // Verify null utility fields
            assert!(
                created_no_utility["numeroCuenta"].is_null(),
                "numeroCuenta should be null when not provided"
            );
            assert!(
                created_no_utility["periodoInicio"].is_null(),
                "periodoInicio should be null when not provided"
            );
            assert!(
                created_no_utility["periodoFin"].is_null(),
                "periodoFin should be null when not provided"
            );

            // Cleanup
            cleanup_gasto(&db, gasto_id.parse().unwrap()).await;
            cleanup_gasto(&db, gasto_id_2.parse().unwrap()).await;
        });
    }
}

// Task 11.11: categoria enum and utility-fields tests
#[test]
fn invalid_categoria_rejected_with_422_pbt() {
    gastos_categoria_and_utility_tests::invalid_categoria_rejected_with_422();
}

#[test]
fn utility_fields_round_trip_create_read() {
    gastos_categoria_and_utility_tests::utility_fields_round_trip();
}
