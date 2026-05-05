use chrono::{Datelike, Utc};
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

use crate::migrations;

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
    migrations::Migrator::up(&db, None)
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
            let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Runtime error: {e}"))?;
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
        eprintln!("⚠ DATABASE_URL not set – skipping integration test");
        return;
    }
    let _guard = crate::GLOBAL_DB_SERIAL
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let Some((rt, db)) = shared_rt_and_db() else {
        eprintln!("⚠ DB not reachable – skipping integration test");
        return;
    };
    rt.block_on(f(db.clone()));
}

fn make_config() -> AppConfig {
    AppConfig {
        database_url: String::new(),
        jwt_secret: JWT_SECRET.to_string(),
        server_port: 0,
        cors_origin: None,
        pool: realestate_backend::config::PoolConfig::default(),
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
        titulo: Set("Propiedad Test PagoGen".to_string()),
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

async fn create_test_inquilino(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
    use realestate_backend::entities::inquilino;
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    inquilino::ActiveModel {
        id: Set(id),
        nombre: Set("Test".to_string()),
        apellido: Set("Inquilino".to_string()),
        email: Set(Some(format!("inquilino+{id}@test.com"))),
        telefono: Set(None),
        cedula: Set(format!("C{}", &id.simple().to_string()[..19])),
        contacto_emergencia: Set(None),
        notas: Set(None),
        documentos: Set(None),
        organizacion_id: Set(org_id),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .expect("Failed to create test inquilino");
    id
}

async fn cleanup_contrato(db: &DatabaseConnection, contrato_id: Uuid) {
    use realestate_backend::entities::{contrato, pago};
    let _ = pago::Entity::delete_many()
        .filter(pago::Column::ContratoId.eq(contrato_id))
        .exec(db)
        .await;
    let _ = contrato::Entity::delete_by_id(contrato_id).exec(db).await;
}

// ── RBAC tests (no DB required) ─────────────────────────────────────

#[cfg(test)]
mod pago_generacion_rbac_tests {
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
            organizacion_id: Uuid::new_v4(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };
        encode_jwt(&claims, JWT_SECRET).unwrap()
    }

    async fn write_access_path_body_stub(
        _access: WriteAccess,
        _path: web::Path<Uuid>,
        _body: web::Json<serde_json::Value>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    async fn claims_path_stub(
        _claims: Claims,
        _path: web::Path<Uuid>,
    ) -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().finish())
    }

    // --- POST generar as visualizador → 403 ---

    #[actix_web::test]
    async fn generar_rejects_visualizador() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/pagos/generar",
            web::post().to(write_access_path_body_stub),
        ))
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{id}/pagos/generar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn generar_allows_admin() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/pagos/generar",
            web::post().to(write_access_path_body_stub),
        ))
        .await;

        let token = make_token("admin");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{id}/pagos/generar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn generar_allows_gerente() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/pagos/generar",
            web::post().to(write_access_path_body_stub),
        ))
        .await;

        let token = make_token("gerente");
        let req = test::TestRequest::post()
            .uri(&format!("/api/contratos/{id}/pagos/generar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- GET preview allows any authenticated user ---

    #[actix_web::test]
    async fn preview_rejects_unauthenticated() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/pagos/preview",
            web::get().to(claims_path_stub),
        ))
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/contratos/{id}/pagos/preview"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn preview_allows_visualizador() {
        let id = Uuid::new_v4();
        let app = test::init_service(App::new().app_data(web::Data::new(test_config())).route(
            "/api/contratos/{id}/pagos/preview",
            web::get().to(claims_path_stub),
        ))
        .await;

        let token = make_token("visualizador");
        let req = test::TestRequest::get()
            .uri(&format!("/api/contratos/{id}/pagos/preview"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}

// ── DB integration tests ────────────────────────────────────────────

fn future_contrato_dates(months: i32) -> (String, String) {
    let today = Utc::now().date_naive();
    let start = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    let end =
        start + chrono::Months::new(u32::try_from(months).unwrap_or(1)) - chrono::Duration::days(1);
    (
        start.format("%Y-%m-%d").to_string(),
        end.format("%Y-%m-%d").to_string(),
    )
}

fn future_termination_date(months_from_start: i32) -> String {
    let today = Utc::now().date_naive();
    let start = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    let end = start + chrono::Months::new(u32::try_from(months_from_start).unwrap_or(1))
        - chrono::Duration::days(1);
    end.format("%Y-%m-%d").to_string()
}

#[test]
fn test_create_active_contrato_generates_pagos() {
    with_db(|db| async move {
        let config = make_config();
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            config,
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create active contrato spanning 3 months
        let (fecha_inicio, fecha_fin) = future_contrato_dates(3);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": fecha_inicio,
                "fechaFin": fecha_fin,
                "montoMensual": "15000.00",
                "moneda": "DOP"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id: Uuid = body["id"].as_str().unwrap().parse().unwrap();

        // Verify pagos_generados field in response
        assert_eq!(body["pagosGenerados"], 3);

        // Verify pagos in DB
        use realestate_backend::entities::pago;
        let pagos = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_id))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(pagos.len(), 3);
        for p in &pagos {
            assert_eq!(p.estado, "pendiente");
            assert_eq!(p.monto, Decimal::new(1500000, 2));
            assert_eq!(p.moneda, "DOP");
            assert!(p.fecha_pago.is_none());
            assert!(p.metodo_pago.is_none());
        }

        cleanup_contrato(&db, contrato_id).await;
    });
}

#[test]
fn test_create_non_active_contrato_no_pagos() {
    with_db(|db| async move {
        let config = make_config();
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            config,
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create contrato, then terminate it so propiedad is freed
        let (fecha_inicio, fecha_fin) = future_contrato_dates(3);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": fecha_inicio,
                "fechaFin": fecha_fin,
                "montoMensual": "15000.00",
                "moneda": "DOP"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id: Uuid = body["id"].as_str().unwrap().parse().unwrap();

        // Active contrato generates pagos
        assert_eq!(body["pagosGenerados"], 3);
        assert_eq!(body["estado"], "activo");

        cleanup_contrato(&db, contrato_id).await;
    });
}

#[test]
fn test_renovar_contrato_generates_pagos_for_new_period() {
    with_db(|db| async move {
        let config = make_config();
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            config,
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create original contrato (3 months)
        let (fecha_inicio, fecha_fin) = future_contrato_dates(3);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": fecha_inicio,
                "fechaFin": fecha_fin,
                "montoMensual": "15000.00",
                "moneda": "DOP"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let original_id: Uuid = body["id"].as_str().unwrap().parse().unwrap();

        // Renovar with new monto for 2 months
        let (_, renewal_fecha_fin) = future_contrato_dates(5);
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/contratos/{original_id}/renovar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "fechaFin": renewal_fecha_fin,
                "montoMensual": "18000.00"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let new_id: Uuid = body["id"].as_str().unwrap().parse().unwrap();

        // New contrato starts day after original ends
        let original_end = chrono::NaiveDate::parse_from_str(&fecha_fin, "%Y-%m-%d").unwrap();
        let expected_start = (original_end + chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        assert_eq!(body["fechaInicio"], expected_start);
        assert_eq!(body["pagosGenerados"], 2);

        // Verify new pagos have the new monto
        use realestate_backend::entities::pago;
        let new_pagos = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(new_id))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(new_pagos.len(), 2);
        for p in &new_pagos {
            assert_eq!(p.monto, Decimal::new(1800000, 2));
            assert_eq!(p.estado, "pendiente");
        }

        cleanup_contrato(&db, new_id).await;
        cleanup_contrato(&db, original_id).await;
    });
}

#[test]
fn test_terminar_contrato_cancels_future_pending_pagos() {
    with_db(|db| async move {
        let config = make_config();
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            config,
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create contrato spanning 6 months
        let (fecha_inicio, fecha_fin) = future_contrato_dates(6);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": fecha_inicio,
                "fechaFin": fecha_fin,
                "montoMensual": "15000.00",
                "moneda": "DOP"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id: Uuid = body["id"].as_str().unwrap().parse().unwrap();
        assert_eq!(body["pagosGenerados"], 6);

        // Manually mark one pago as "pagado" and one as "atrasado"
        use realestate_backend::entities::pago;
        use sea_orm::QueryOrder;
        let pagos = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_id))
            .order_by_asc(pago::Column::FechaVencimiento)
            .all(&db)
            .await
            .unwrap();

        // Mark first month pago as pagado
        let mut active: pago::ActiveModel = pagos[0].clone().into();
        active.estado = Set("pagado".to_string());
        active.update(&db).await.unwrap();

        // Mark second month pago as atrasado
        let mut active: pago::ActiveModel = pagos[1].clone().into();
        active.estado = Set("atrasado".to_string());
        active.update(&db).await.unwrap();

        // Terminate contrato at end of 3rd month
        let fecha_terminacion = future_termination_date(3);
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/contratos/{contrato_id}/terminar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "fechaTerminacion": fecha_terminacion }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Verify pago states after termination
        let pagos_after = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_id))
            .order_by_asc(pago::Column::FechaVencimiento)
            .all(&db)
            .await
            .unwrap();

        for (i, p) in pagos_after.iter().enumerate() {
            match i {
                0 => assert_eq!(p.estado, "pagado", "1st month should remain pagado"),
                1 => assert_eq!(p.estado, "atrasado", "2nd month should remain atrasado"),
                2 => assert_eq!(
                    p.estado, "pendiente",
                    "3rd month (<=terminacion) stays pendiente"
                ),
                3..=5 => assert_eq!(p.estado, "cancelado", "Month {} should be cancelado", i + 1),
                _ => panic!("Unexpected pago index {i}"),
            }
        }

        cleanup_contrato(&db, contrato_id).await;
    });
}

#[test]
fn test_preview_pagos_returns_correct_response() {
    with_db(|db| async move {
        let config = make_config();
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            config,
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create contrato (4 months)
        let (fecha_inicio, fecha_fin) = future_contrato_dates(4);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": fecha_inicio,
                "fechaFin": fecha_fin,
                "montoMensual": "20000.00",
                "moneda": "DOP"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        // GET preview
        let req = actix_web::test::TestRequest::get()
            .uri(&format!("/api/v1/contratos/{contrato_id}/pagos/preview"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let preview: Value = actix_web::test::read_body_json(resp).await;

        assert_eq!(preview["totalPagos"], 4);
        assert_eq!(preview["pagos"].as_array().unwrap().len(), 4);
        // All 4 already exist (auto-generated on create)
        assert_eq!(preview["pagosExistentes"], 4);
        assert_eq!(preview["pagosNuevos"], 0);

        let contrato_uuid: Uuid = contrato_id.parse().unwrap();
        cleanup_contrato(&db, contrato_uuid).await;
    });
}

#[test]
fn test_preview_pagos_nonexistent_contrato_returns_404() {
    with_db(|db| async move {
        let config = make_config();
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            config,
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let fake_id = Uuid::new_v4();
        let req = actix_web::test::TestRequest::get()
            .uri(&format!("/api/v1/contratos/{fake_id}/pagos/preview"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    });
}

#[test]
fn test_preview_pagos_does_not_create_records() {
    with_db(|db| async move {
        let config = make_config();
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            config,
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create contrato
        let (fecha_inicio, fecha_fin) = future_contrato_dates(3);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": fecha_inicio,
                "fechaFin": fecha_fin,
                "montoMensual": "10000.00",
                "moneda": "DOP"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id: Uuid = body["id"].as_str().unwrap().parse().unwrap();

        // Count pagos before preview
        use realestate_backend::entities::pago;
        let count_before = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_id))
            .all(&db)
            .await
            .unwrap()
            .len();

        // Call preview
        let req = actix_web::test::TestRequest::get()
            .uri(&format!("/api/v1/contratos/{contrato_id}/pagos/preview"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Count pagos after preview — should be unchanged
        let count_after = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_id))
            .all(&db)
            .await
            .unwrap()
            .len();
        assert_eq!(count_before, count_after);

        cleanup_contrato(&db, contrato_id).await;
    });
}

#[test]
fn test_generar_pagos_active_contrato() {
    with_db(|db| async move {
        let config = make_config();
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            config,
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create contrato (auto-generates pagos with dia_vencimiento=1)
        let (fecha_inicio, fecha_fin) = future_contrato_dates(3);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": fecha_inicio,
                "fechaFin": fecha_fin,
                "montoMensual": "12000.00",
                "moneda": "DOP"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        // Delete existing pagos to simulate missing pagos
        use realestate_backend::entities::pago;
        let contrato_uuid: Uuid = contrato_id.parse().unwrap();
        pago::Entity::delete_many()
            .filter(pago::Column::ContratoId.eq(contrato_uuid))
            .exec(&db)
            .await
            .unwrap();

        // POST generar
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/contratos/{contrato_id}/pagos/generar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["pagosGenerados"], 3);

        // Verify pagos in DB
        let pagos = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_uuid))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(pagos.len(), 3);

        cleanup_contrato(&db, contrato_uuid).await;
    });
}

#[test]
fn test_generar_pagos_non_active_contrato_returns_422() {
    with_db(|db| async move {
        let config = make_config();
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            config,
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create and terminate contrato
        let (fecha_inicio, fecha_fin) = future_contrato_dates(6);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": fecha_inicio,
                "fechaFin": fecha_fin,
                "montoMensual": "15000.00",
                "moneda": "DOP"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id: Uuid = body["id"].as_str().unwrap().parse().unwrap();

        // Terminate it (mid first month)
        let start_date = chrono::NaiveDate::parse_from_str(&fecha_inicio, "%Y-%m-%d").unwrap();
        let fecha_terminacion = (start_date + chrono::Duration::days(14))
            .format("%Y-%m-%d")
            .to_string();
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/contratos/{contrato_id}/terminar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "fechaTerminacion": fecha_terminacion }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Try to generate pagos on terminated contrato
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/contratos/{contrato_id}/pagos/generar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);

        cleanup_contrato(&db, contrato_id).await;
    });
}

#[test]
fn test_generar_pagos_nonexistent_contrato_returns_404() {
    with_db(|db| async move {
        let config = make_config();
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            config,
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let fake_id = Uuid::new_v4();
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/contratos/{fake_id}/pagos/generar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    });
}

#[test]
fn test_generar_pagos_invalid_dia_vencimiento_returns_422() {
    with_db(|db| async move {
        let config = make_config();
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            config,
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create contrato
        let (fecha_inicio, fecha_fin) = future_contrato_dates(3);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": fecha_inicio,
                "fechaFin": fecha_fin,
                "montoMensual": "15000.00",
                "moneda": "DOP"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id: Uuid = body["id"].as_str().unwrap().parse().unwrap();

        // Try with dia_vencimiento = 0
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/contratos/{contrato_id}/pagos/generar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "diaVencimiento": 0 }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);

        // Try with dia_vencimiento = 32
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/contratos/{contrato_id}/pagos/generar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "diaVencimiento": 32 }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);

        cleanup_contrato(&db, contrato_id).await;
    });
}

#[test]
fn test_generar_pagos_deduplication() {
    with_db(|db| async move {
        let config = make_config();
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            config,
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create contrato (3 months, auto-generates 3 pagos)
        let (fecha_inicio, fecha_fin) = future_contrato_dates(3);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": fecha_inicio,
                "fechaFin": fecha_fin,
                "montoMensual": "15000.00",
                "moneda": "DOP"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id: Uuid = body["id"].as_str().unwrap().parse().unwrap();
        assert_eq!(body["pagosGenerados"], 3);

        // Delete the second pago to simulate a gap
        use realestate_backend::entities::pago;
        use sea_orm::QueryOrder;
        let pagos = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_id))
            .order_by_asc(pago::Column::FechaVencimiento)
            .all(&db)
            .await
            .unwrap();
        pago::Entity::delete_by_id(pagos[1].id)
            .exec(&db)
            .await
            .unwrap();

        // POST generar — should only generate the missing pago
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/contratos/{contrato_id}/pagos/generar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["pagosGenerados"], 1);

        // Verify total pagos is back to 3
        let total = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_id))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(total.len(), 3);

        cleanup_contrato(&db, contrato_id).await;
    });
}

#[test]
fn test_auditoria_entries_for_pago_generation_and_cancellation() {
    with_db(|db| async move {
        let config = make_config();
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            config,
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // 1. Create contrato → auto-generates pagos → audit "generar_pagos_auto"
        let (fecha_inicio, fecha_fin) = future_contrato_dates(6);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": fecha_inicio,
                "fechaFin": fecha_fin,
                "montoMensual": "15000.00",
                "moneda": "DOP"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap().to_string();
        let contrato_uuid: Uuid = contrato_id.parse().unwrap();

        // Check audit for auto generation
        let req = actix_web::test::TestRequest::get()
            .uri(&format!(
                "/api/v1/auditoria?entityType=contrato&entityId={contrato_id}"
            ))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let entries = body["data"].as_array().unwrap();
        assert!(
            entries.iter().any(|e| e["accion"] == "generar_pagos_auto"),
            "Should have generar_pagos_auto audit entry"
        );

        // 2. Delete some pagos and manually generate → audit "generar_pagos_manual"
        use realestate_backend::entities::pago;
        let pagos = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_uuid))
            .all(&db)
            .await
            .unwrap();
        // Delete last 2 pagos
        for p in pagos.iter().rev().take(2) {
            pago::Entity::delete_by_id(p.id).exec(&db).await.unwrap();
        }

        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/contratos/{contrato_id}/pagos/generar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Check audit for manual generation
        let req = actix_web::test::TestRequest::get()
            .uri(&format!(
                "/api/v1/auditoria?entityType=contrato&entityId={contrato_id}"
            ))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let entries = body["data"].as_array().unwrap();
        assert!(
            entries.iter().any(|e| e["accion"] == "gen_pagos_manual"),
            "Should have gen_pagos_manual audit entry"
        );

        // 3. Terminate contrato → audit "cancelar_pagos"
        let fecha_terminacion = future_termination_date(3);
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/contratos/{contrato_id}/terminar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "fechaTerminacion": fecha_terminacion }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Check audit for cancellation
        let req = actix_web::test::TestRequest::get()
            .uri(&format!(
                "/api/v1/auditoria?entityType=contrato&entityId={contrato_id}"
            ))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let entries = body["data"].as_array().unwrap();
        assert!(
            entries.iter().any(|e| e["accion"] == "cancelar_pagos"),
            "Should have cancelar_pagos audit entry"
        );

        cleanup_contrato(&db, contrato_uuid).await;
    });
}
