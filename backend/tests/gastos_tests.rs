#![allow(clippy::needless_return)]

#[path = "../migrations/mod.rs"]
mod migrations;

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
    use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait, Set};
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
        opts.max_connections(2)
            .min_connections(1)
            .connect_timeout(std::time::Duration::from_secs(30))
            .idle_timeout(std::time::Duration::from_secs(60));
        let db = Database::connect(opts)
            .await
            .map_err(|e| format!("Failed to connect to database: {e}"))?;
        super::migrations::Migrator::up(&db, None)
            .await
            .map_err(|e| format!("Failed to run migrations: {e}"))?;
        Ok(db)
    }

    fn shared_rt_and_db() -> &'static (tokio::runtime::Runtime, DatabaseConnection) {
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
            .unwrap_or_else(|e| panic!("{e}"))
    }

    fn with_db<F, Fut>(f: F)
    where
        F: FnOnce(DatabaseConnection) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let (rt, db) = shared_rt_and_db();
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

    fn make_token(user_id: Uuid, rol: &str) -> String {
        let claims = Claims {
            sub: user_id,
            email: format!("{rol}@test.com"),
            rol: rol.to_string(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };
        encode_jwt(&claims, JWT_SECRET).unwrap()
    }

    async fn create_test_usuario(db: &DatabaseConnection, rol: &str) -> Uuid {
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
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("Failed to create test usuario");
        id
    }

    async fn create_test_propiedad(db: &DatabaseConnection) -> Uuid {
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
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
            let propiedad_id = create_test_propiedad(&db).await;
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
                    "descripcion": "Reparación de techo",
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
            assert_eq!(body["descripcion"], "Reparación de techo");
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
            assert_eq!(updated["descripcion"], "Reparación de techo");

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
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
            let propiedad_id = create_test_propiedad(&db).await;
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
                        "descripcion": format!("Gasto paginación {i}"),
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
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
            let prop_a = create_test_propiedad(&db).await;
            let prop_b = create_test_propiedad(&db).await;
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
                    "categoria": "seguros",
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
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
            let propiedad_id = create_test_propiedad(&db).await;
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
                    "categoria": "legal",
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
                    "/api/v1/gastos?propiedadId={propiedad_id}&categoria=legal"
                ))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let body: Value = test::read_body_json(resp).await;
            for item in body["data"].as_array().unwrap() {
                assert_eq!(item["categoria"], "legal");
            }

            cleanup_gasto(&db, gasto_id).await;
        });
    }

    pub fn filter_date_range() {
        with_db(|db| async move {
            let config = make_config();
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
            let propiedad_id = create_test_propiedad(&db).await;
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
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
            let propiedad_id = create_test_propiedad(&db).await;
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
                 {propiedad_id},mantenimiento,Reparación tubería,5000.00,DOP,2025-04-01\n\
                 {propiedad_id},impuestos,Impuesto predial,12000.00,DOP,2025-04-15"
            );
            let boundary = "----TestBoundary";
            let payload = format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"gastos.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n--{boundary}--\r\n"
            );

            let req = test::TestRequest::post()
                .uri("/api/v1/importar/gastos")
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
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
            let propiedad_id = create_test_propiedad(&db).await;
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
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
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
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
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
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
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
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
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
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
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
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
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
