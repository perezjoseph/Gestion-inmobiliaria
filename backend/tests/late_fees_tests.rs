use chrono::Utc;
use realestate_backend::app::create_app;
use realestate_backend::config::AppConfig;
use realestate_backend::services::auth::{Claims, encode_jwt};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait, Set};
use sea_orm_migration::MigratorTrait;
use serde_json::Value;
use std::str::FromStr;
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
    let _guard = crate::GLOBAL_DB_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
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
        email: format!("{rol}@latefees.com"),
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
        nombre: Set(format!("Org LateFee {id}")),
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
        email: Set(format!("{rol}+{id}@latefees.com")),
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
        titulo: Set("Propiedad LateFee Test".to_string()),
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
        nombre: Set("Inquilino".to_string()),
        apellido: Set("LateFee".to_string()),
        email: Set(Some(format!("inquilino+{id}@latefees.com"))),
        telefono: Set(None),
        cedula: Set(format!("CED-LF-{id}")),
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

fn make_app(
    db: DatabaseConnection,
) -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    create_app(
        db,
        make_config(),
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new()),
    )
}

// ── Tests ──────────────────────────────────────────────────────────────

/// Test: Create contrato with recargo_porcentaje and dias_gracia → fields stored and returned
/// Requirements: 1.1, 1.2, 1.7
#[test]
fn test_create_contrato_with_recargo_fields() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": "2025-01-01",
                "fechaFin": "2025-12-31",
                "montoMensual": "25000.00",
                "recargoPorcentaje": "5.50",
                "diasGracia": 3
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["recargoPorcentaje"], "5.50");
        assert_eq!(body["diasGracia"], 3);
    });
}

/// Test: Create contrato without recargo fields → NULL values returned
/// Requirements: 1.3, 1.4
#[test]
fn test_create_contrato_without_recargo_fields() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": "2025-01-01",
                "fechaFin": "2025-12-31",
                "montoMensual": "25000.00"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["recargoPorcentaje"].is_null());
        assert!(body["diasGracia"].is_null());
    });
}

/// Test: Update contrato recargo_porcentaje and dias_gracia → fields updated
/// Requirements: 1.5, 1.6
#[test]
fn test_update_contrato_recargo_fields() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        // Create contrato without recargo fields
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": "2025-01-01",
                "fechaFin": "2025-12-31",
                "montoMensual": "25000.00"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        // Update with recargo fields
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "recargoPorcentaje": "10.00",
                "diasGracia": 5
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["recargoPorcentaje"], "10.00");
        assert_eq!(body["diasGracia"], 5);
    });
}

/// Test: Create contrato with recargo_porcentaje < 0 → 422
/// Requirements: 1.5
#[test]
fn test_create_contrato_recargo_porcentaje_negative_422() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": "2025-01-01",
                "fechaFin": "2025-12-31",
                "montoMensual": "25000.00",
                "recargoPorcentaje": "-1.00"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);
    });
}

/// Test: Create contrato with recargo_porcentaje > 100 → 422
/// Requirements: 1.5
#[test]
fn test_create_contrato_recargo_porcentaje_over_100_422() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": "2025-01-01",
                "fechaFin": "2025-12-31",
                "montoMensual": "25000.00",
                "recargoPorcentaje": "100.01"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);
    });
}

/// Test: Create contrato with dias_gracia < 0 → 422
/// Requirements: 1.6
#[test]
fn test_create_contrato_dias_gracia_negative_422() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": "2025-01-01",
                "fechaFin": "2025-12-31",
                "montoMensual": "25000.00",
                "diasGracia": -1
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);
    });
}

/// Test: GET /configuracion/recargo when not set → NULL
/// Requirements: 3.4
#[test]
fn test_get_recargo_config_not_set_returns_null() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);

        // Clean up any existing recargo config
        use realestate_backend::entities::configuracion;
        let _ = configuracion::Entity::delete_by_id("recargo_porcentaje_defecto")
            .exec(&db)
            .await;

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/configuracion/recargo")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["porcentaje"].is_null());
    });
}

/// Test: PUT /configuracion/recargo with valid value → stored and returned
/// Requirements: 3.1, 3.3
#[test]
fn test_put_recargo_config_valid_value() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        // Set recargo config
        let req = actix_web::test::TestRequest::put()
            .uri("/api/v1/configuracion/recargo")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "porcentaje": "7.50"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Verify it was stored by reading it back
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/configuracion/recargo")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["porcentaje"], "7.50");
    });
}

/// Test: PUT /configuracion/recargo with invalid value → 422
/// Requirements: 3.2
#[test]
fn test_put_recargo_config_invalid_value_422() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        // Try negative value
        let req = actix_web::test::TestRequest::put()
            .uri("/api/v1/configuracion/recargo")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "porcentaje": "-5.00"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);

        // Try over 100
        let req = actix_web::test::TestRequest::put()
            .uri("/api/v1/configuracion/recargo")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "porcentaje": "101.00"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);
    });
}

/// Test: PUT /configuracion/recargo as non-admin → 403
/// Requirements: 3.1
#[test]
fn test_put_recargo_config_non_admin_403() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let gerente_id = create_test_usuario(&db, "gerente", org_id).await;
        let token = make_token(gerente_id, "gerente", org_id);

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::put()
            .uri("/api/v1/configuracion/recargo")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "porcentaje": "5.00"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    });
}

/// Test: mark_overdue with dias_gracia → respects grace period
/// Requirements: 6.1, 6.2, 6.3
#[test]
fn test_mark_overdue_respects_grace_period() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let _admin_id = create_test_usuario(&db, "admin", org_id).await;
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        // Create contrato with 5 days grace period
        use realestate_backend::entities::contrato;
        let contrato_id = Uuid::new_v4();
        let now = Utc::now().into();
        contrato::ActiveModel {
            id: Set(contrato_id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            fecha_fin: Set(Utc::now().date_naive() + chrono::Duration::days(365)),
            monto_mensual: Set(Decimal::new(25000, 0)),
            deposito: Set(None),
            moneda: Set("DOP".to_string()),
            estado: Set("activo".to_string()),
            documentos: Set(None),
            organizacion_id: Set(org_id),
            estado_deposito: Set(None),
            fecha_cobro_deposito: Set(None),
            fecha_devolucion_deposito: Set(None),
            monto_retenido: Set(None),
            motivo_retencion: Set(None),
            recargo_porcentaje: Set(Some(Decimal::new(500, 2))), // 5.00%
            dias_gracia: Set(Some(5)),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .unwrap();

        // Create pago overdue by 3 days (within grace period of 5)
        use realestate_backend::entities::pago;
        let pago_within_grace_id = Uuid::new_v4();
        let overdue_3_days = Utc::now().date_naive() - chrono::Duration::days(3);
        pago::ActiveModel {
            id: Set(pago_within_grace_id),
            contrato_id: Set(contrato_id),
            monto: Set(Decimal::new(25000, 0)),
            moneda: Set("DOP".to_string()),
            fecha_pago: Set(None),
            fecha_vencimiento: Set(overdue_3_days),
            metodo_pago: Set(None),
            estado: Set("pendiente".to_string()),
            notas: Set(None),
            recargo: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .unwrap();

        // Create pago overdue by 10 days (beyond grace period of 5)
        let pago_beyond_grace_id = Uuid::new_v4();
        let overdue_10_days = Utc::now().date_naive() - chrono::Duration::days(10);
        pago::ActiveModel {
            id: Set(pago_beyond_grace_id),
            contrato_id: Set(contrato_id),
            monto: Set(Decimal::new(25000, 0)),
            moneda: Set("DOP".to_string()),
            fecha_pago: Set(None),
            fecha_vencimiento: Set(overdue_10_days),
            metodo_pago: Set(None),
            estado: Set("pendiente".to_string()),
            notas: Set(None),
            recargo: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .unwrap();

        // Run mark_overdue
        use realestate_backend::services::pagos;
        pagos::mark_overdue(&db).await.unwrap();

        // Pago within grace period should still be pendiente
        let within = pago::Entity::find_by_id(pago_within_grace_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(within.estado, "pendiente");

        // Pago beyond grace period should be atrasado
        let beyond = pago::Entity::find_by_id(pago_beyond_grace_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(beyond.estado, "atrasado");
    });
}

/// Test: mark_overdue calculates recargo using contrato porcentaje
/// Requirements: 4.1, 5.1
#[test]
fn test_mark_overdue_calculates_recargo_contrato_porcentaje() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let _admin_id = create_test_usuario(&db, "admin", org_id).await;
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        // Create contrato with 10% recargo
        use realestate_backend::entities::contrato;
        let contrato_id = Uuid::new_v4();
        let now = Utc::now().into();
        contrato::ActiveModel {
            id: Set(contrato_id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            fecha_fin: Set(Utc::now().date_naive() + chrono::Duration::days(365)),
            monto_mensual: Set(Decimal::new(25000, 0)),
            deposito: Set(None),
            moneda: Set("DOP".to_string()),
            estado: Set("activo".to_string()),
            documentos: Set(None),
            organizacion_id: Set(org_id),
            estado_deposito: Set(None),
            fecha_cobro_deposito: Set(None),
            fecha_devolucion_deposito: Set(None),
            monto_retenido: Set(None),
            motivo_retencion: Set(None),
            recargo_porcentaje: Set(Some(Decimal::new(1000, 2))), // 10.00%
            dias_gracia: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .unwrap();

        // Create overdue pago
        use realestate_backend::entities::pago;
        let pago_id = Uuid::new_v4();
        let overdue_date = Utc::now().date_naive() - chrono::Duration::days(5);
        pago::ActiveModel {
            id: Set(pago_id),
            contrato_id: Set(contrato_id),
            monto: Set(Decimal::new(25000, 0)),
            moneda: Set("DOP".to_string()),
            fecha_pago: Set(None),
            fecha_vencimiento: Set(overdue_date),
            metodo_pago: Set(None),
            estado: Set("pendiente".to_string()),
            notas: Set(None),
            recargo: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .unwrap();

        // Run mark_overdue
        use realestate_backend::services::pagos;
        pagos::mark_overdue(&db).await.unwrap();

        // Verify recargo = 25000 * 10% = 2500.00
        let updated = pago::Entity::find_by_id(pago_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.estado, "atrasado");
        assert_eq!(updated.recargo, Some(Decimal::from_str("2500.00").unwrap()));
    });
}

/// Test: mark_overdue calculates recargo using org default when contrato is NULL
/// Requirements: 4.2, 5.1
#[test]
fn test_mark_overdue_calculates_recargo_org_default() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        // Set org default recargo to 8%
        let app = actix_web::test::init_service(make_app(db.clone())).await;
        let req = actix_web::test::TestRequest::put()
            .uri("/api/v1/configuracion/recargo")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({ "porcentaje": "8.00" }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Create contrato WITHOUT recargo_porcentaje (NULL → falls back to org)
        use realestate_backend::entities::contrato;
        let contrato_id = Uuid::new_v4();
        let now = Utc::now().into();
        contrato::ActiveModel {
            id: Set(contrato_id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            fecha_fin: Set(Utc::now().date_naive() + chrono::Duration::days(365)),
            monto_mensual: Set(Decimal::new(10000, 0)),
            deposito: Set(None),
            moneda: Set("DOP".to_string()),
            estado: Set("activo".to_string()),
            documentos: Set(None),
            organizacion_id: Set(org_id),
            estado_deposito: Set(None),
            fecha_cobro_deposito: Set(None),
            fecha_devolucion_deposito: Set(None),
            monto_retenido: Set(None),
            motivo_retencion: Set(None),
            recargo_porcentaje: Set(None), // NULL → use org default
            dias_gracia: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .unwrap();

        // Create overdue pago
        use realestate_backend::entities::pago;
        let pago_id = Uuid::new_v4();
        let overdue_date = Utc::now().date_naive() - chrono::Duration::days(5);
        pago::ActiveModel {
            id: Set(pago_id),
            contrato_id: Set(contrato_id),
            monto: Set(Decimal::new(10000, 0)),
            moneda: Set("DOP".to_string()),
            fecha_pago: Set(None),
            fecha_vencimiento: Set(overdue_date),
            metodo_pago: Set(None),
            estado: Set("pendiente".to_string()),
            notas: Set(None),
            recargo: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .unwrap();

        // Run mark_overdue
        use realestate_backend::services::pagos;
        pagos::mark_overdue(&db).await.unwrap();

        // Verify recargo = 10000 * 8% = 800.00
        let updated = pago::Entity::find_by_id(pago_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.estado, "atrasado");
        assert_eq!(updated.recargo, Some(Decimal::from_str("800.00").unwrap()));
    });
}

/// Test: mark_overdue with both NULL → recargo stays NULL
/// Requirements: 4.3, 5.4
#[test]
fn test_mark_overdue_both_null_recargo_stays_null() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let _admin_id = create_test_usuario(&db, "admin", org_id).await;
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        // Ensure no org default recargo exists
        use realestate_backend::entities::configuracion;
        let _ = configuracion::Entity::delete_by_id("recargo_porcentaje_defecto")
            .exec(&db)
            .await;

        // Create contrato without recargo_porcentaje
        use realestate_backend::entities::contrato;
        let contrato_id = Uuid::new_v4();
        let now = Utc::now().into();
        contrato::ActiveModel {
            id: Set(contrato_id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            fecha_fin: Set(Utc::now().date_naive() + chrono::Duration::days(365)),
            monto_mensual: Set(Decimal::new(25000, 0)),
            deposito: Set(None),
            moneda: Set("DOP".to_string()),
            estado: Set("activo".to_string()),
            documentos: Set(None),
            organizacion_id: Set(org_id),
            estado_deposito: Set(None),
            fecha_cobro_deposito: Set(None),
            fecha_devolucion_deposito: Set(None),
            monto_retenido: Set(None),
            motivo_retencion: Set(None),
            recargo_porcentaje: Set(None),
            dias_gracia: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .unwrap();

        // Create overdue pago
        use realestate_backend::entities::pago;
        let pago_id = Uuid::new_v4();
        let overdue_date = Utc::now().date_naive() - chrono::Duration::days(5);
        pago::ActiveModel {
            id: Set(pago_id),
            contrato_id: Set(contrato_id),
            monto: Set(Decimal::new(25000, 0)),
            moneda: Set("DOP".to_string()),
            fecha_pago: Set(None),
            fecha_vencimiento: Set(overdue_date),
            metodo_pago: Set(None),
            estado: Set("pendiente".to_string()),
            notas: Set(None),
            recargo: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .unwrap();

        // Run mark_overdue
        use realestate_backend::services::pagos;
        pagos::mark_overdue(&db).await.unwrap();

        // Verify pago is atrasado but recargo is NULL
        let updated = pago::Entity::find_by_id(pago_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.estado, "atrasado");
        assert!(updated.recargo.is_none());
    });
}

/// Test: Manual update estado to "atrasado" → recargo calculated
/// Requirements: 5.2
#[test]
fn test_manual_update_estado_atrasado_calculates_recargo() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        // Create contrato with 5% recargo via API
        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": "2025-01-01",
                "fechaFin": "2025-12-31",
                "montoMensual": "20000.00",
                "recargoPorcentaje": "5.00"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let contrato_body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = contrato_body["id"].as_str().unwrap();

        // Create a pago via API
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/pagos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "contratoId": contrato_id,
                "monto": "20000.00",
                "fechaVencimiento": "2025-06-01"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let pago_body: Value = actix_web::test::read_body_json(resp).await;
        let pago_id = pago_body["id"].as_str().unwrap();

        // Manually update estado to "atrasado"
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/pagos/{pago_id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "estado": "atrasado"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = actix_web::test::read_body_json(resp).await;
        // recargo = 20000 * 5% = 1000.00
        assert_eq!(body["recargo"], "1000.00");
    });
}

/// Test: Update estado from "atrasado" to "pagado" → recargo cleared to NULL
/// Requirements: 5.2
#[test]
fn test_update_estado_atrasado_to_pagado_clears_recargo() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        // Create contrato with recargo
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": "2025-01-01",
                "fechaFin": "2025-12-31",
                "montoMensual": "15000.00",
                "recargoPorcentaje": "10.00"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let contrato_body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = contrato_body["id"].as_str().unwrap();

        // Create pago
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/pagos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "contratoId": contrato_id,
                "monto": "15000.00",
                "fechaVencimiento": "2025-06-01"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let pago_body: Value = actix_web::test::read_body_json(resp).await;
        let pago_id = pago_body["id"].as_str().unwrap();

        // Mark as atrasado
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/pagos/{pago_id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({ "estado": "atrasado" }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["recargo"], "1500.00"); // 15000 * 10%

        // Now mark as pagado → recargo should be cleared
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/pagos/{pago_id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "estado": "pagado",
                "fechaPago": "2025-06-15",
                "metodoPago": "transferencia"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["recargo"].is_null());
    });
}

/// Test: Pago response includes recargo field
/// Requirements: 2.2, 2.3
#[test]
fn test_pago_response_includes_recargo_field() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        // Create contrato
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "propiedadId": propiedad_id,
                "inquilinoId": inquilino_id,
                "fechaInicio": "2025-01-01",
                "fechaFin": "2025-12-31",
                "montoMensual": "30000.00"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let contrato_body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = contrato_body["id"].as_str().unwrap();

        // Create pago
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/pagos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "contratoId": contrato_id,
                "monto": "30000.00",
                "fechaVencimiento": "2025-06-01"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let pago_body: Value = actix_web::test::read_body_json(resp).await;
        let pago_id = pago_body["id"].as_str().unwrap();

        // Verify recargo field is present (null for new pago)
        assert!(pago_body.get("recargo").is_some());
        assert!(pago_body["recargo"].is_null());

        // GET pago by ID also includes recargo
        let req = actix_web::test::TestRequest::get()
            .uri(&format!("/api/v1/pagos/{pago_id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body.get("recargo").is_some());
    });
}
