#![allow(clippy::needless_return)]
use crate::migrations;

use chrono::Utc;
use realestate_backend::app::create_app;
use realestate_backend::config::AppConfig;
use realestate_backend::entities::registro_auditoria;
use realestate_backend::services::auth::{Claims, encode_jwt};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, Set,
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
        titulo: Set("Propiedad Test Deposito".to_string()),
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
        apellido: Set("Test".to_string()),
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

/// Helper: set up org + admin + propiedad + inquilino
async fn setup_deposit_data(db: &DatabaseConnection) -> (Uuid, Uuid, Uuid, Uuid) {
    let org_id = create_test_organizacion(db).await;
    let admin_id = create_test_usuario(db, "admin", org_id).await;
    let propiedad_id = create_test_propiedad(db, org_id).await;
    let inquilino_id = create_test_inquilino(db, org_id).await;
    (org_id, admin_id, propiedad_id, inquilino_id)
}

fn contrato_body(propiedad_id: Uuid, inquilino_id: Uuid, deposito: Option<&str>) -> Value {
    let today = Utc::now().date_naive();
    let fecha_inicio = today.format("%Y-%m-%d").to_string();
    let fecha_fin = (today + chrono::Duration::days(365))
        .format("%Y-%m-%d")
        .to_string();
    let mut body = json!({
        "propiedadId": propiedad_id,
        "inquilinoId": inquilino_id,
        "fechaInicio": fecha_inicio,
        "fechaFin": fecha_fin,
        "montoMensual": "15000",
        "moneda": "DOP"
    });
    if let Some(dep) = deposito {
        body["deposito"] = json!(dep);
    }
    body
}

// ── Tests ──────────────────────────────────────────────────────────────

#[test]
fn test_create_contrato_with_deposit_sets_pendiente() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["estadoDeposito"], "pendiente");
    });
}

#[test]
fn test_create_contrato_without_deposit_estado_null() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, None))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["estadoDeposito"].is_null());
    });
}

#[test]
fn test_full_flow_pendiente_cobrado_devuelto() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create contrato with deposit
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        // pendiente → cobrado
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "cobrado"}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["estadoDeposito"], "cobrado");
        assert!(
            !body["fechaCobroDeposito"].is_null(),
            "fecha_cobro_deposito should be set"
        );

        // cobrado → devuelto
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "devuelto"}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["estadoDeposito"], "devuelto");
        assert!(
            !body["fechaDevolucionDeposito"].is_null(),
            "fecha_devolucion_deposito should be set"
        );
    });
}

#[test]
fn test_full_flow_pendiente_cobrado_retenido() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create contrato with deposit
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        // pendiente → cobrado
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "cobrado"}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // cobrado → retenido
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "estado": "retenido",
                "montoRetenido": "10000",
                "motivoRetencion": "Daños en la propiedad"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["estadoDeposito"], "retenido");
        assert!(!body["fechaDevolucionDeposito"].is_null());
        assert_eq!(body["montoRetenido"], "10000.00");
        assert_eq!(body["motivoRetencion"], "Daños en la propiedad");
    });
}

#[test]
fn test_invalid_transitions_return_422() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        // pendiente → devuelto (invalid)
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "devuelto"}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["message"].as_str().unwrap().contains("cobrado"));

        // pendiente → retenido (invalid)
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "estado": "retenido",
                "montoRetenido": "5000",
                "motivoRetencion": "Daños"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);
    });
}

#[test]
fn test_terminal_states_cannot_transition() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        // pendiente → cobrado → devuelto
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "cobrado"}))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "devuelto"}))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // devuelto → cobrado (invalid: terminal state)
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "cobrado"}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(
            body["message"]
                .as_str()
                .unwrap()
                .contains("devueltos o retenidos")
        );
    });
}

#[test]
fn test_retention_without_monto_retenido_returns_422() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        // pendiente → cobrado
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "cobrado"}))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // cobrado → retenido without montoRetenido
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "retenido", "motivoRetencion": "Daños"}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["message"].as_str().unwrap().contains("monto retenido"));
    });
}

#[test]
fn test_retention_with_monto_zero_returns_422() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        // pendiente → cobrado
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "cobrado"}))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // cobrado → retenido with montoRetenido = 0
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(
                json!({"estado": "retenido", "montoRetenido": "0", "motivoRetencion": "Daños"}),
            )
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["message"].as_str().unwrap().contains("mayor a cero"));
    });
}

#[test]
fn test_retention_with_monto_exceeding_deposit_returns_422() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        // pendiente → cobrado
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "cobrado"}))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // cobrado → retenido with montoRetenido > deposito
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(
                json!({"estado": "retenido", "montoRetenido": "50000", "motivoRetencion": "Daños"}),
            )
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["message"].as_str().unwrap().contains("exceder"));
    });
}

#[test]
fn test_retention_without_motivo_returns_422() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        // pendiente → cobrado
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "cobrado"}))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // cobrado → retenido without motivoRetencion
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "retenido", "montoRetenido": "5000"}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["message"].as_str().unwrap().contains("motivo"));
    });
}

#[test]
fn test_change_estado_on_contrato_without_deposit_returns_422() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create contrato without deposit
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, None))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "cobrado"}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);
    });
}

#[test]
fn test_visualizador_cannot_change_deposit_estado() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let admin_token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {admin_token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        let viewer_id = create_test_usuario(&db, "visualizador", org_id).await;
        let viewer_token = make_token(viewer_id, "visualizador", org_id);

        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {viewer_token}")))
            .set_json(json!({"estado": "cobrado"}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    });
}

#[test]
fn test_nonexistent_contrato_returns_404() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let fake_id = Uuid::new_v4();
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{fake_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "cobrado"}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    });
}

#[test]
fn test_invalid_estado_enum_returns_422() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "invalido_estado"}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 422);
    });
}

#[test]
fn test_deposit_fields_in_get_response() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        // GET /contratos/{id} should include deposit fields
        let req = actix_web::test::TestRequest::get()
            .uri(&format!("/api/v1/contratos/{contrato_id}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["estadoDeposito"], "pendiente");
        assert!(body.get("fechaCobroDeposito").is_some());
        assert!(body.get("fechaDevolucionDeposito").is_some());
        assert!(body.get("montoRetenido").is_some());
        assert!(body.get("motivoRetencion").is_some());
    });
}

#[test]
fn test_auditoria_entries_for_estado_changes() {
    with_db(|db| async move {
        let (org_id, admin_id, propiedad_id, inquilino_id) = setup_deposit_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();
        let contrato_uuid: Uuid = contrato_id.parse().unwrap();

        // pendiente → cobrado
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "cobrado"}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Verify audit entry for cobrado
        let entries = registro_auditoria::Entity::find()
            .filter(registro_auditoria::Column::EntityType.eq("contrato"))
            .filter(registro_auditoria::Column::EntityId.eq(contrato_uuid))
            .filter(registro_auditoria::Column::Accion.eq("cambio_deposito"))
            .order_by_desc(registro_auditoria::Column::CreatedAt)
            .all(&db)
            .await
            .unwrap();
        assert!(
            !entries.is_empty(),
            "Expected audit entry for cambio_deposito"
        );
        assert_eq!(entries[0].cambios["estado_nuevo"], "cobrado");
        assert_eq!(entries[0].cambios["estado_anterior"], "pendiente");

        // cobrado → devuelto
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({"estado": "devuelto"}))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Verify second audit entry
        let entries = registro_auditoria::Entity::find()
            .filter(registro_auditoria::Column::EntityType.eq("contrato"))
            .filter(registro_auditoria::Column::EntityId.eq(contrato_uuid))
            .filter(registro_auditoria::Column::Accion.eq("cambio_deposito"))
            .order_by_desc(registro_auditoria::Column::CreatedAt)
            .all(&db)
            .await
            .unwrap();
        assert!(entries.len() >= 2, "Expected at least 2 audit entries");
        assert_eq!(entries[0].cambios["estado_nuevo"], "devuelto");
        assert_eq!(entries[0].cambios["estado_anterior"], "cobrado");
    });
}
