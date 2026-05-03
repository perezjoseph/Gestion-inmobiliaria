#![allow(clippy::needless_return)]
#![allow(clippy::too_many_lines)]
use crate::migrations;

#[cfg(test)]
mod unidades_rbac_tests {
    use actix_web::http::StatusCode;
    use actix_web::{App, HttpResponse, test, web};
    use chrono::Utc;
    use uuid::Uuid;

    use realestate_backend::config::AppConfig;
    use realestate_backend::errors::AppError;
    use realestate_backend::middleware::rbac::{AdminOnly, WriteAccess};
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

    async fn write_access_stub(
        _access: WriteAccess,
        _path: web::Path<Uuid>,
        _body: web::Json<serde_json::Value>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Created().finish())
    }

    async fn write_access_update_stub(
        _access: WriteAccess,
        _path: web::Path<(Uuid, Uuid)>,
        _body: web::Json<serde_json::Value>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    async fn admin_only_stub(
        _admin: AdminOnly,
        _path: web::Path<(Uuid, Uuid)>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::NoContent().finish())
    }

    async fn claims_list_stub(
        _claims: Claims,
        _path: web::Path<Uuid>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    async fn claims_get_stub(
        _claims: Claims,
        _path: web::Path<(Uuid, Uuid)>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    #[actix_web::test]
    async fn visualizador_can_list_unidades() {
        let prop_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/propiedades/{propiedad_id}/unidades",
            web::get().to(claims_list_stub),
        ))
        .await;
        let token = make_token("visualizador");
        let req = test::TestRequest::get()
            .uri(&format!("/api/propiedades/{prop_id}/unidades"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn visualizador_can_get_unidad_by_id() {
        let prop_id = Uuid::new_v4();
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/propiedades/{propiedad_id}/unidades/{id}",
            web::get().to(claims_get_stub),
        ))
        .await;
        let token = make_token("visualizador");
        let req = test::TestRequest::get()
            .uri(&format!("/api/propiedades/{prop_id}/unidades/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn visualizador_cannot_create_unidad() {
        let prop_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/propiedades/{propiedad_id}/unidades",
            web::post().to(write_access_stub),
        ))
        .await;
        let token = make_token("visualizador");
        let req = test::TestRequest::post()
            .uri(&format!("/api/propiedades/{prop_id}/unidades"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn admin_can_create_unidad() {
        let prop_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/propiedades/{propiedad_id}/unidades",
            web::post().to(write_access_stub),
        ))
        .await;
        let token = make_token("admin");
        let req = test::TestRequest::post()
            .uri(&format!("/api/propiedades/{prop_id}/unidades"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[actix_web::test]
    async fn gerente_can_create_unidad() {
        let prop_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/propiedades/{propiedad_id}/unidades",
            web::post().to(write_access_stub),
        ))
        .await;
        let token = make_token("gerente");
        let req = test::TestRequest::post()
            .uri(&format!("/api/propiedades/{prop_id}/unidades"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[actix_web::test]
    async fn visualizador_cannot_update_unidad() {
        let prop_id = Uuid::new_v4();
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/propiedades/{propiedad_id}/unidades/{id}",
            web::put().to(write_access_update_stub),
        ))
        .await;
        let token = make_token("visualizador");
        let req = test::TestRequest::put()
            .uri(&format!("/api/propiedades/{prop_id}/unidades/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"estado": "ocupada"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn admin_can_update_unidad() {
        let prop_id = Uuid::new_v4();
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/propiedades/{propiedad_id}/unidades/{id}",
            web::put().to(write_access_update_stub),
        ))
        .await;
        let token = make_token("admin");
        let req = test::TestRequest::put()
            .uri(&format!("/api/propiedades/{prop_id}/unidades/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({"estado": "ocupada"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn visualizador_cannot_delete_unidad() {
        let prop_id = Uuid::new_v4();
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/propiedades/{propiedad_id}/unidades/{id}",
            web::delete().to(admin_only_stub),
        ))
        .await;
        let token = make_token("visualizador");
        let req = test::TestRequest::delete()
            .uri(&format!("/api/propiedades/{prop_id}/unidades/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn gerente_cannot_delete_unidad() {
        let prop_id = Uuid::new_v4();
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/propiedades/{propiedad_id}/unidades/{id}",
            web::delete().to(admin_only_stub),
        ))
        .await;
        let token = make_token("gerente");
        let req = test::TestRequest::delete()
            .uri(&format!("/api/propiedades/{prop_id}/unidades/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn admin_can_delete_unidad() {
        let prop_id = Uuid::new_v4();
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/propiedades/{propiedad_id}/unidades/{id}",
            web::delete().to(admin_only_stub),
        ))
        .await;
        let token = make_token("admin");
        let req = test::TestRequest::delete()
            .uri(&format!("/api/propiedades/{prop_id}/unidades/{id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn unauthenticated_cannot_list_unidades() {
        let prop_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/propiedades/{propiedad_id}/unidades",
            web::get().to(claims_list_stub),
        ))
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/propiedades/{prop_id}/unidades"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn unauthenticated_cannot_create_unidad() {
        let prop_id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/propiedades/{propiedad_id}/unidades",
            web::post().to(write_access_stub),
        ))
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/propiedades/{prop_id}/unidades"))
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
        ActiveModelTrait, ColumnTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait,
        QueryFilter, Set,
    };
    use sea_orm_migration::MigratorTrait;
    use serde_json::{Value, json};
    use uuid::Uuid;

    const JWT_SECRET: &str = "test_secret_key_that_is_long_enough_for_jwt";

    fn db_url() -> String {
        dotenvy::dotenv().ok();
        std::env::var("DATABASE_URL").unwrap_or_default()
    }

    async fn setup_db() -> Result<DatabaseConnection, String> {
        let url = db_url();
        if url.is_empty() {
            return Err("DATABASE_URL not set".to_string());
        }
        let mut opts = ConnectOptions::new(&url);
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
            titulo: Set("Propiedad Test Unidades".to_string()),
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
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("Failed to create test propiedad");
        id
    }

    fn base_uri(propiedad_id: Uuid) -> String {
        format!("/api/v1/propiedades/{propiedad_id}/unidades")
    }

    async fn cleanup_unidad(db: &DatabaseConnection, id: Uuid) {
        use realestate_backend::entities::unidad;
        let _ = unidad::Entity::delete_by_id(id).exec(db).await;
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

            // Create
            let req = test::TestRequest::post()
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "numeroUnidad": "A-101",
                    "piso": 1,
                    "habitaciones": 3,
                    "banos": 2,
                    "areaM2": "85.50",
                    "precio": "25000.00",
                    "moneda": "DOP",
                    "estado": "disponible",
                    "descripcion": "Apartamento con vista"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let unidad_id = body["id"].as_str().unwrap().to_string();
            assert_eq!(body["numeroUnidad"], "A-101");
            assert_eq!(body["estado"], "disponible");
            assert_eq!(body["moneda"], "DOP");
            assert_eq!(body["piso"], 1);

            // Get by ID
            let req = test::TestRequest::get()
                .uri(&format!("{}/{unidad_id}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let detail: Value = test::read_body_json(resp).await;
            assert_eq!(detail["id"], unidad_id);
            assert_eq!(detail["numeroUnidad"], "A-101");
            assert_eq!(detail["descripcion"], "Apartamento con vista");

            // Update
            let req = test::TestRequest::put()
                .uri(&format!("{}/{unidad_id}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"estado": "ocupada", "precio": "30000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let updated: Value = test::read_body_json(resp).await;
            assert_eq!(updated["estado"], "ocupada");
            assert_eq!(updated["descripcion"], "Apartamento con vista");

            // List
            let req = test::TestRequest::get()
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let list: Value = test::read_body_json(resp).await;
            assert!(list["total"].as_u64().unwrap() >= 1);

            // Delete
            let req = test::TestRequest::delete()
                .uri(&format!("{}/{unidad_id}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 204);

            // Verify deleted
            let req = test::TestRequest::get()
                .uri(&format!("{}/{unidad_id}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404);
        });
    }

    pub fn uniqueness_create() {
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

            // Create first unidad
            let req = test::TestRequest::post()
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "DUP-1", "precio": "10000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let b: Value = test::read_body_json(resp).await;
            let id1: Uuid = b["id"].as_str().unwrap().parse().unwrap();

            // Create second with same numero_unidad in same propiedad -> 409
            let req = test::TestRequest::post()
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "DUP-1", "precio": "20000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 409);

            cleanup_unidad(&db, id1).await;
        });
    }

    pub fn uniqueness_update() {
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
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "UPD-A", "precio": "10000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let b1: Value = test::read_body_json(resp).await;
            let id1: Uuid = b1["id"].as_str().unwrap().parse().unwrap();

            let req = test::TestRequest::post()
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "UPD-B", "precio": "10000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let b2: Value = test::read_body_json(resp).await;
            let id2: Uuid = b2["id"].as_str().unwrap().parse().unwrap();

            // Update second to first's numero_unidad -> 409
            let req = test::TestRequest::put()
                .uri(&format!("{}/{id2}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "UPD-A"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 409);

            cleanup_unidad(&db, id1).await;
            cleanup_unidad(&db, id2).await;
        });
    }

    pub fn uniqueness_across_propiedades() {
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

            // Same numero_unidad in different propiedades is allowed
            let req = test::TestRequest::post()
                .uri(&base_uri(prop_a))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "CROSS-1", "precio": "10000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let b1: Value = test::read_body_json(resp).await;
            let id1: Uuid = b1["id"].as_str().unwrap().parse().unwrap();

            let req = test::TestRequest::post()
                .uri(&base_uri(prop_b))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "CROSS-1", "precio": "20000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let b2: Value = test::read_body_json(resp).await;
            let id2: Uuid = b2["id"].as_str().unwrap().parse().unwrap();

            cleanup_unidad(&db, id1).await;
            cleanup_unidad(&db, id2).await;
        });
    }

    pub fn filter_by_estado() {
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
            for (num, estado) in &[
                ("F-1", "disponible"),
                ("F-2", "ocupada"),
                ("F-3", "mantenimiento"),
            ] {
                let req = test::TestRequest::post()
                    .uri(&base_uri(propiedad_id))
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(json!({"numeroUnidad": num, "precio": "10000", "estado": estado}))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                let b: Value = test::read_body_json(resp).await;
                ids.push(b["id"].as_str().unwrap().parse::<Uuid>().unwrap());
            }

            let req = test::TestRequest::get()
                .uri(&format!("{}?estado=ocupada", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            for item in body["data"].as_array().unwrap() {
                assert_eq!(item["estado"], "ocupada");
            }

            for id in &ids {
                cleanup_unidad(&db, *id).await;
            }
        });
    }

    pub fn ordering_by_numero_unidad() {
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
            // Create in reverse order
            for num in &["C-300", "A-100", "B-200"] {
                let req = test::TestRequest::post()
                    .uri(&base_uri(propiedad_id))
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(json!({"numeroUnidad": num, "precio": "10000"}))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                let b: Value = test::read_body_json(resp).await;
                ids.push(b["id"].as_str().unwrap().parse::<Uuid>().unwrap());
            }

            let req = test::TestRequest::get()
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let body: Value = test::read_body_json(resp).await;
            let data = body["data"].as_array().unwrap();
            // Verify ascending order
            for i in 0..data.len().saturating_sub(1) {
                let a = data[i]["numeroUnidad"].as_str().unwrap();
                let b = data[i + 1]["numeroUnidad"].as_str().unwrap();
                assert!(a <= b, "Expected {a} <= {b}");
            }

            for id in &ids {
                cleanup_unidad(&db, *id).await;
            }
        });
    }

    pub fn validation_empty_numero() {
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
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "", "precio": "10000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    pub fn validation_invalid_estado() {
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
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "V-1", "precio": "10000", "estado": "invalido"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    pub fn validation_invalid_moneda() {
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
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "V-2", "precio": "10000", "moneda": "EUR"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    pub fn validation_negative_precio() {
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
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "V-3", "precio": "-500"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    pub fn nonexistent_propiedad_create() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let fake_prop = Uuid::new_v4();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::post()
                .uri(&base_uri(fake_prop))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "NE-1", "precio": "10000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404);
        });
    }

    pub fn nonexistent_propiedad_list() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let fake_prop = Uuid::new_v4();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::get()
                .uri(&base_uri(fake_prop))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404);
        });
    }

    pub fn nonexistent_unidad_get() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let fake_id = Uuid::new_v4();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::get()
                .uri(&format!("{}/{fake_id}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404);
        });
    }

    pub fn nonexistent_unidad_update() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let fake_id = Uuid::new_v4();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::put()
                .uri(&format!("{}/{fake_id}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"estado": "ocupada"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404);
        });
    }

    pub fn nonexistent_unidad_delete() {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let fake_id = Uuid::new_v4();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::delete()
                .uri(&format!("{}/{fake_id}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404);
        });
    }

    pub fn get_by_id_includes_counts() {
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
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "CNT-1", "precio": "10000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let b: Value = test::read_body_json(resp).await;
            let unidad_id = b["id"].as_str().unwrap().to_string();

            let req = test::TestRequest::get()
                .uri(&format!("{}/{unidad_id}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let detail: Value = test::read_body_json(resp).await;
            assert!(detail.get("gastosCount").is_some());
            assert!(detail.get("mantenimientoCount").is_some());
            assert_eq!(detail["gastosCount"], 0);
            assert_eq!(detail["mantenimientoCount"], 0);

            let uid: Uuid = unidad_id.parse().unwrap();
            cleanup_unidad(&db, uid).await;
        });
    }

    pub fn occupancy_metrics_in_propiedad() {
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

            // Create 2 unidades: 1 ocupada, 1 disponible
            let mut ids = Vec::new();
            for (num, estado) in &[("OCC-1", "ocupada"), ("OCC-2", "disponible")] {
                let req = test::TestRequest::post()
                    .uri(&base_uri(propiedad_id))
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(json!({"numeroUnidad": num, "precio": "10000", "estado": estado}))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                let b: Value = test::read_body_json(resp).await;
                ids.push(b["id"].as_str().unwrap().parse::<Uuid>().unwrap());
            }

            // Check propiedad detail includes occupancy
            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/propiedades/{propiedad_id}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let detail: Value = test::read_body_json(resp).await;
            assert!(detail.get("totalUnidades").is_some());
            assert!(detail.get("unidadesOcupadas").is_some());
            assert!(detail.get("tasaOcupacion").is_some());
            assert!(detail["totalUnidades"].as_u64().unwrap() >= 2);
            assert!(detail["unidadesOcupadas"].as_u64().unwrap() >= 1);

            // Check propiedad list includes occupancy
            let req = test::TestRequest::get()
                .uri("/api/v1/propiedades")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let list: Value = test::read_body_json(resp).await;
            let data = list["data"].as_array().unwrap();
            let our_prop = data.iter().find(|p| p["id"] == propiedad_id.to_string());
            assert!(our_prop.is_some());
            let prop = our_prop.unwrap();
            assert!(prop.get("totalUnidades").is_some());

            for id in &ids {
                cleanup_unidad(&db, *id).await;
            }
        });
    }

    pub fn auditoria_entries_created() {
        with_db(|db| async move {
            use realestate_backend::entities::registro_auditoria;

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

            // Create
            let req = test::TestRequest::post()
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "AUD-1", "precio": "10000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let b: Value = test::read_body_json(resp).await;
            let unidad_id: Uuid = b["id"].as_str().unwrap().parse().unwrap();

            // Check crear audit entry
            let crear_entries = registro_auditoria::Entity::find()
                .filter(registro_auditoria::Column::EntityType.eq("unidad"))
                .filter(registro_auditoria::Column::EntityId.eq(unidad_id))
                .filter(registro_auditoria::Column::Accion.eq("crear"))
                .all(&db)
                .await
                .unwrap();
            assert!(!crear_entries.is_empty(), "Expected crear audit entry");

            // Update
            let req = test::TestRequest::put()
                .uri(&format!("{}/{unidad_id}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"estado": "ocupada"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);

            let actualizar_entries = registro_auditoria::Entity::find()
                .filter(registro_auditoria::Column::EntityType.eq("unidad"))
                .filter(registro_auditoria::Column::EntityId.eq(unidad_id))
                .filter(registro_auditoria::Column::Accion.eq("actualizar"))
                .all(&db)
                .await
                .unwrap();
            assert!(
                !actualizar_entries.is_empty(),
                "Expected actualizar audit entry"
            );

            // Delete
            let req = test::TestRequest::delete()
                .uri(&format!("{}/{unidad_id}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 204);

            let eliminar_entries = registro_auditoria::Entity::find()
                .filter(registro_auditoria::Column::EntityType.eq("unidad"))
                .filter(registro_auditoria::Column::EntityId.eq(unidad_id))
                .filter(registro_auditoria::Column::Accion.eq("eliminar"))
                .all(&db)
                .await
                .unwrap();
            assert!(
                !eliminar_entries.is_empty(),
                "Expected eliminar audit entry"
            );
        });
    }

    pub fn defaults_applied() {
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

            // Create with only required fields - defaults should apply
            let req = test::TestRequest::post()
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": "DEF-1", "precio": "5000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let b: Value = test::read_body_json(resp).await;
            assert_eq!(b["estado"], "disponible");
            assert_eq!(b["moneda"], "DOP");

            let uid: Uuid = b["id"].as_str().unwrap().parse().unwrap();
            cleanup_unidad(&db, uid).await;
        });
    }
}

// DB-backed integration tests
#[test]
fn crud_cycle_create_get_update_list_delete() {
    db_async::crud_cycle();
}

#[test]
fn numero_unidad_uniqueness_on_create() {
    db_async::uniqueness_create();
}

#[test]
fn numero_unidad_uniqueness_on_update() {
    db_async::uniqueness_update();
}

#[test]
fn numero_unidad_uniqueness_across_propiedades_allowed() {
    db_async::uniqueness_across_propiedades();
}

#[test]
fn filter_unidades_by_estado() {
    db_async::filter_by_estado();
}

#[test]
fn list_unidades_ordered_by_numero_unidad_asc() {
    db_async::ordering_by_numero_unidad();
}

#[test]
fn validation_empty_numero_unidad_returns_422() {
    db_async::validation_empty_numero();
}

#[test]
fn validation_invalid_estado_returns_422() {
    db_async::validation_invalid_estado();
}

#[test]
fn validation_invalid_moneda_returns_422() {
    db_async::validation_invalid_moneda();
}

#[test]
fn validation_negative_precio_returns_422() {
    db_async::validation_negative_precio();
}

#[test]
fn nonexistent_propiedad_returns_404_on_create() {
    db_async::nonexistent_propiedad_create();
}

#[test]
fn nonexistent_propiedad_returns_404_on_list() {
    db_async::nonexistent_propiedad_list();
}

#[test]
fn nonexistent_unidad_returns_404_on_get() {
    db_async::nonexistent_unidad_get();
}

#[test]
fn nonexistent_unidad_returns_404_on_update() {
    db_async::nonexistent_unidad_update();
}

#[test]
fn nonexistent_unidad_returns_404_on_delete() {
    db_async::nonexistent_unidad_delete();
}

#[test]
fn get_by_id_includes_gastos_and_mantenimiento_counts() {
    db_async::get_by_id_includes_counts();
}

#[test]
fn occupancy_metrics_in_propiedad_detail_and_list() {
    db_async::occupancy_metrics_in_propiedad();
}

#[test]
fn auditoria_entries_created_for_crud_operations() {
    db_async::auditoria_entries_created();
}

#[test]
fn defaults_applied_when_optional_fields_omitted() {
    db_async::defaults_applied();
}
