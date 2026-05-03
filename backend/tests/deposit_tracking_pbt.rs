#![allow(clippy::needless_return)]
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use realestate_backend::services::contratos::{ESTADOS_DEPOSITO, validar_transicion_deposito};
use rust_decimal::Decimal;

use crate::migrations;

// ── Strategies ─────────────────────────────────────────────────────────

/// Deposit amount: positive value suitable for a deposit (100..500_000 cents → 1.00..5000.00)
fn positive_deposit() -> impl Strategy<Value = Decimal> {
    (100i64..500_000i64).prop_map(|v| Decimal::new(v, 2))
}

/// Deposit amount that is zero
fn zero_deposit() -> impl Strategy<Value = Decimal> {
    Just(Decimal::ZERO)
}

/// Generate a deposit option: Some(positive), Some(0), or None
fn deposit_option() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        positive_deposit().prop_map(|d| Some(d.to_string())),
        zero_deposit().prop_map(|d| Some(d.to_string())),
        Just(None),
    ]
}

/// Valid transition path index: 0 = cobrado, 1 = devuelto, 2 = retenido
fn valid_transition_path() -> impl Strategy<Value = u8> {
    prop_oneof![Just(0u8), Just(1u8), Just(2u8)]
}

/// All invalid (estado_actual, nuevo_estado) pairs for deposit state machine
fn invalid_deposit_transition() -> impl Strategy<Value = (String, String)> {
    let pairs: Vec<(String, String)> = vec![
        ("pendiente".into(), "devuelto".into()),
        ("pendiente".into(), "retenido".into()),
        ("pendiente".into(), "pendiente".into()),
        ("cobrado".into(), "pendiente".into()),
        ("cobrado".into(), "cobrado".into()),
        ("devuelto".into(), "pendiente".into()),
        ("devuelto".into(), "cobrado".into()),
        ("devuelto".into(), "devuelto".into()),
        ("devuelto".into(), "retenido".into()),
        ("retenido".into(), "pendiente".into()),
        ("retenido".into(), "cobrado".into()),
        ("retenido".into(), "devuelto".into()),
        ("retenido".into(), "retenido".into()),
    ];
    proptest::sample::select(pairs)
}

/// Random string NOT in the valid estado_deposito set
fn invalid_estado_deposito() -> impl Strategy<Value = String> {
    "[a-z]{3,15}".prop_filter("must not be a valid estado_deposito", |s| {
        !ESTADOS_DEPOSITO.contains(&s.as_str())
    })
}

/// Retention monto: positive value for retention tests
fn retention_monto() -> impl Strategy<Value = Decimal> {
    (1i64..1_000_000i64).prop_map(|v| Decimal::new(v, 2))
}

/// Motivo retencion: non-empty string
fn valid_motivo() -> impl Strategy<Value = String> {
    "[a-zA-Z ]{5,50}"
        .prop_map(|s| s.trim().to_string())
        .prop_filter("motivo must not be empty", |s| !s.trim().is_empty())
}

/// Whitespace-only or empty strings for invalid motivo tests
fn empty_or_whitespace_motivo() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        Just(Some("".to_string())),
        Just(Some(" ".to_string())),
        Just(Some("  \t  ".to_string())),
    ]
}

// ── Async helpers (Rust 2024: avoid calling async from sync #[test]) ───

mod pbt_async {
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

    fn shared_rt_and_db() -> Option<&'static (tokio::runtime::Runtime, DatabaseConnection)> {
        static SHARED: std::sync::OnceLock<
            Result<(tokio::runtime::Runtime, DatabaseConnection), String>,
        > = std::sync::OnceLock::new();
        SHARED
            .get_or_init(|| {
                dotenvy::dotenv().ok();
                let url = std::env::var("DATABASE_URL")
                    .map_err(|_| "DATABASE_URL not set".to_string())?;
                let rt =
                    tokio::runtime::Runtime::new().map_err(|e| format!("Runtime error: {e}"))?;
                let db = rt.block_on(async {
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
                    Ok::<_, String>(db)
                })?;
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
        static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
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
            nombre: Set(format!("Org PBT Dep {id}")),
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
            titulo: Set("Propiedad PBT Deposito".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle PBT Dep 123".to_string()),
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
            apellido: Set("PBT".to_string()),
            email: Set(Some(format!("inq+{id}@test.com"))),
            telefono: Set(None),
            cedula: Set(format!("CED-{id}")),
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

    /// Setup org + admin + propiedad + inquilino, return (org_id, admin_id, propiedad_id, inquilino_id)
    async fn setup(db: &DatabaseConnection) -> (Uuid, Uuid, Uuid, Uuid) {
        let org_id = create_test_organizacion(db).await;
        let admin_id = create_test_usuario(db, "admin", org_id).await;
        let propiedad_id = create_test_propiedad(db, org_id).await;
        let inquilino_id = create_test_inquilino(db, org_id).await;
        (org_id, admin_id, propiedad_id, inquilino_id)
    }

    fn contrato_body(propiedad_id: Uuid, inquilino_id: Uuid, deposito: Option<&str>) -> Value {
        let mut body = json!({
            "propiedadId": propiedad_id,
            "inquilinoId": inquilino_id,
            "fechaInicio": "2025-01-01",
            "fechaFin": "2025-12-31",
            "montoMensual": "15000",
            "moneda": "DOP"
        });
        if let Some(dep) = deposito {
            body["deposito"] = json!(dep);
        }
        body
    }

    // ── P1: Deposit status defaults correctly on creation ──

    pub fn p1(deposito: Option<String>) {
        with_db(|db| async move {
            let (org_id, admin_id, propiedad_id, inquilino_id) = setup(&db).await;
            let token = make_token(admin_id, "admin", org_id);
            let app = test::init_service(create_app(
                db.clone(),
                make_config(),
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::post()
                .uri("/api/v1/contratos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(contrato_body(
                    propiedad_id,
                    inquilino_id,
                    deposito.as_deref(),
                ))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;

            // Determine if deposit is positive
            let is_positive = deposito
                .as_ref()
                .and_then(|d| d.parse::<f64>().ok())
                .is_some_and(|v| v > 0.0);

            if is_positive {
                assert_eq!(
                    body["estadoDeposito"], "pendiente",
                    "deposito={deposito:?} should default to pendiente"
                );
            } else {
                assert!(
                    body["estadoDeposito"].is_null(),
                    "deposito={deposito:?} should have null estadoDeposito"
                );
            }
        });
    }

    // ── P2: Valid deposit state transitions set timestamps ──

    pub fn p2(path: u8, deposit_amount: Decimal) {
        with_db(|db| async move {
            let (org_id, admin_id, propiedad_id, inquilino_id) = setup(&db).await;
            let token = make_token(admin_id, "admin", org_id);
            let app = test::init_service(create_app(
                db.clone(),
                make_config(),
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create contrato with deposit
            let req = test::TestRequest::post()
                .uri("/api/v1/contratos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(contrato_body(
                    propiedad_id,
                    inquilino_id,
                    Some(&deposit_amount.to_string()),
                ))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let contrato_id = body["id"].as_str().unwrap();

            // pendiente → cobrado (always first step)
            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"estado": "cobrado"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["estadoDeposito"], "cobrado");
            assert!(
                !body["fechaCobroDeposito"].is_null(),
                "fecha_cobro_deposito should be set after cobrado"
            );

            match path {
                0 => {
                    // Just cobrado — already verified above
                }
                1 => {
                    // cobrado → devuelto
                    let req = test::TestRequest::put()
                        .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
                        .insert_header(("Authorization", format!("Bearer {token}")))
                        .set_json(json!({"estado": "devuelto"}))
                        .to_request();
                    let resp = test::call_service(&app, req).await;
                    assert_eq!(resp.status(), 200);
                    let body: Value = test::read_body_json(resp).await;
                    assert_eq!(body["estadoDeposito"], "devuelto");
                    assert!(
                        !body["fechaDevolucionDeposito"].is_null(),
                        "fecha_devolucion_deposito should be set after devuelto"
                    );
                }
                2 => {
                    // cobrado → retenido
                    // Use half the deposit as retention amount
                    let half = (deposit_amount / Decimal::new(2, 0)).max(Decimal::new(1, 2));
                    let req = test::TestRequest::put()
                        .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
                        .insert_header(("Authorization", format!("Bearer {token}")))
                        .set_json(json!({
                            "estado": "retenido",
                            "montoRetenido": half.to_string(),
                            "motivoRetencion": "Daños en la propiedad"
                        }))
                        .to_request();
                    let resp = test::call_service(&app, req).await;
                    assert_eq!(resp.status(), 200);
                    let body: Value = test::read_body_json(resp).await;
                    assert_eq!(body["estadoDeposito"], "retenido");
                    assert!(
                        !body["fechaDevolucionDeposito"].is_null(),
                        "fecha_devolucion_deposito should be set after retenido"
                    );
                }
                _ => unreachable!(),
            }
        });
    }

    // ── P4: Retention requires valid monto and motivo ──

    pub fn p4_valid(deposit_amount: Decimal, retention_amount: Decimal, motivo: String) {
        with_db(|db| async move {
            let (org_id, admin_id, propiedad_id, inquilino_id) = setup(&db).await;
            let token = make_token(admin_id, "admin", org_id);
            let app = test::init_service(create_app(
                db.clone(),
                make_config(),
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create contrato with deposit
            let req = test::TestRequest::post()
                .uri("/api/v1/contratos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(contrato_body(
                    propiedad_id,
                    inquilino_id,
                    Some(&deposit_amount.to_string()),
                ))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let contrato_id = body["id"].as_str().unwrap();

            // pendiente → cobrado
            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"estado": "cobrado"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);

            // cobrado → retenido with valid monto and motivo
            let valid = retention_amount > Decimal::ZERO && retention_amount <= deposit_amount;
            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "estado": "retenido",
                    "montoRetenido": retention_amount.to_string(),
                    "motivoRetencion": motivo
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;

            if valid {
                assert_eq!(resp.status(), 200, "Valid retention should succeed");
            } else {
                assert_eq!(
                    resp.status(),
                    422,
                    "Invalid retention amount should be rejected"
                );
            }
        });
    }

    pub fn p4_invalid_motivo(deposit_amount: Decimal, motivo: Option<String>) {
        with_db(|db| async move {
            let (org_id, admin_id, propiedad_id, inquilino_id) = setup(&db).await;
            let token = make_token(admin_id, "admin", org_id);
            let app = test::init_service(create_app(
                db.clone(),
                make_config(),
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create contrato with deposit
            let req = test::TestRequest::post()
                .uri("/api/v1/contratos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(contrato_body(
                    propiedad_id,
                    inquilino_id,
                    Some(&deposit_amount.to_string()),
                ))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let contrato_id = body["id"].as_str().unwrap();

            // pendiente → cobrado
            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"estado": "cobrado"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);

            // cobrado → retenido with valid monto but invalid motivo
            let half = (deposit_amount / Decimal::new(2, 0)).max(Decimal::new(1, 2));
            let mut body = json!({
                "estado": "retenido",
                "montoRetenido": half.to_string()
            });
            if let Some(ref m) = motivo {
                body["motivoRetencion"] = json!(m);
            }
            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(&body)
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                422,
                "Missing/empty motivo should be rejected"
            );
        });
    }

    // ── P5: Deposit status change round-trip preserves data ──

    pub fn p5(path: u8, deposit_amount: Decimal) {
        with_db(|db| async move {
            let (org_id, admin_id, propiedad_id, inquilino_id) = setup(&db).await;
            let token = make_token(admin_id, "admin", org_id);
            let app = test::init_service(create_app(
                db.clone(),
                make_config(),
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create contrato with deposit
            let req = test::TestRequest::post()
                .uri("/api/v1/contratos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(contrato_body(
                    propiedad_id,
                    inquilino_id,
                    Some(&deposit_amount.to_string()),
                ))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let contrato_id = body["id"].as_str().unwrap().to_string();

            // pendiente → cobrado
            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"estado": "cobrado"}))
                .to_request();
            let _ = test::call_service(&app, req).await;

            let expected_estado;
            let expected_monto_retenido: Option<String>;
            let expected_motivo: Option<String>;

            match path {
                0 => {
                    // Stay at cobrado
                    expected_estado = "cobrado";
                    expected_monto_retenido = None;
                    expected_motivo = None;
                }
                1 => {
                    // cobrado → devuelto
                    let req = test::TestRequest::put()
                        .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
                        .insert_header(("Authorization", format!("Bearer {token}")))
                        .set_json(json!({"estado": "devuelto"}))
                        .to_request();
                    let _ = test::call_service(&app, req).await;
                    expected_estado = "devuelto";
                    expected_monto_retenido = None;
                    expected_motivo = None;
                }
                _ => {
                    // cobrado → retenido
                    let half = (deposit_amount / Decimal::new(2, 0)).max(Decimal::new(1, 2));
                    let motivo = "Daños verificados en inspección";
                    let req = test::TestRequest::put()
                        .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
                        .insert_header(("Authorization", format!("Bearer {token}")))
                        .set_json(json!({
                            "estado": "retenido",
                            "montoRetenido": half.to_string(),
                            "motivoRetencion": motivo
                        }))
                        .to_request();
                    let _ = test::call_service(&app, req).await;
                    expected_estado = "retenido";
                    expected_monto_retenido = Some(half.to_string());
                    expected_motivo = Some(motivo.to_string());
                }
            }

            // GET the contrato and verify round-trip
            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/contratos/{contrato_id}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let detail: Value = test::read_body_json(resp).await;

            assert_eq!(detail["estadoDeposito"], expected_estado);
            assert!(!detail["fechaCobroDeposito"].is_null());

            if expected_estado == "devuelto" || expected_estado == "retenido" {
                assert!(!detail["fechaDevolucionDeposito"].is_null());
            }
            if let Some(ref monto) = expected_monto_retenido {
                assert_eq!(detail["montoRetenido"].as_str().unwrap(), monto.as_str());
            }
            if let Some(ref motivo) = expected_motivo {
                assert_eq!(detail["motivoRetencion"], *motivo);
            }
        });
    }

    // ── P6: Invalid estado enum — tested via API ──

    pub fn p6(bad_estado: String) {
        with_db(|db| async move {
            let (org_id, admin_id, propiedad_id, inquilino_id) = setup(&db).await;
            let token = make_token(admin_id, "admin", org_id);
            let app = test::init_service(create_app(
                db.clone(),
                make_config(),
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create contrato with deposit
            let req = test::TestRequest::post()
                .uri("/api/v1/contratos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(contrato_body(propiedad_id, inquilino_id, Some("30000")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let contrato_id = body["id"].as_str().unwrap();

            // Try invalid estado
            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"estado": bad_estado}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                422,
                "Invalid estado '{bad_estado}' should be rejected"
            );
        });
    }

    // ── P7: Deposit operations on contracts without deposit are rejected ──

    pub fn p7(deposito: Option<String>) {
        with_db(|db| async move {
            let (org_id, admin_id, propiedad_id, inquilino_id) = setup(&db).await;
            let token = make_token(admin_id, "admin", org_id);
            let app = test::init_service(create_app(
                db.clone(),
                make_config(),
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create contrato without deposit (None or "0")
            let req = test::TestRequest::post()
                .uri("/api/v1/contratos")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(contrato_body(
                    propiedad_id,
                    inquilino_id,
                    deposito.as_deref(),
                ))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let contrato_id = body["id"].as_str().unwrap();

            // Attempt to change estado — should be rejected
            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/contratos/{contrato_id}/deposito"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"estado": "cobrado"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                422,
                "Deposit operation on contrato without deposit should be rejected"
            );
        });
    }
} // end mod pbt_async

// ── Property test functions ────────────────────────────────────────────

// Feature: deposit-tracking, Property 1: Deposit status defaults correctly on creation
// **Validates: Requirements 1.6, 1.7**
#[test]
fn test_deposit_status_defaults_on_creation() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner
        .run(&deposit_option(), |deposito| {
            pbt_async::p1(deposito);
            Ok(())
        })
        .unwrap();
}

// Feature: deposit-tracking, Property 2: Valid deposit state transitions set timestamps
// **Validates: Requirements 2.1, 2.2, 2.3**
#[test]
fn test_valid_transitions_set_timestamps() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let strategy = (valid_transition_path(), positive_deposit());

    runner
        .run(&strategy, |(path, deposit_amount)| {
            pbt_async::p2(path, deposit_amount);
            Ok(())
        })
        .unwrap();
}

// Feature: deposit-tracking, Property 3: Invalid deposit state transitions are rejected
// **Validates: Requirements 2.4, 2.5**
#[test]
fn test_invalid_transitions_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner
        .run(
            &invalid_deposit_transition(),
            |(estado_actual, nuevo_estado)| {
                let result = validar_transicion_deposito(&estado_actual, &nuevo_estado);
                assert!(
                    result.is_err(),
                    "Transition {estado_actual}→{nuevo_estado} should be rejected"
                );
                Ok(())
            },
        )
        .unwrap();
}

// Feature: deposit-tracking, Property 4: Retention requires valid monto and motivo
// **Validates: Requirements 3.1, 3.2, 3.3, 3.4**
#[test]
fn test_retention_validation() {
    // Part A: valid and invalid monto amounts
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let strategy = (positive_deposit(), retention_monto(), valid_motivo());

    runner
        .run(&strategy, |(deposit, retention, motivo)| {
            pbt_async::p4_valid(deposit, retention, motivo);
            Ok(())
        })
        .unwrap();

    // Part B: invalid motivo (empty/whitespace/missing)
    let mut runner2 = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let strategy2 = (positive_deposit(), empty_or_whitespace_motivo());

    runner2
        .run(&strategy2, |(deposit, motivo)| {
            pbt_async::p4_invalid_motivo(deposit, motivo);
            Ok(())
        })
        .unwrap();
}

// Feature: deposit-tracking, Property 5: Deposit status change round-trip preserves data
// **Validates: Requirements 4.1, 4.4**
#[test]
fn test_deposit_round_trip() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let strategy = (valid_transition_path(), positive_deposit());

    runner
        .run(&strategy, |(path, deposit_amount)| {
            pbt_async::p5(path, deposit_amount);
            Ok(())
        })
        .unwrap();
}

// Feature: deposit-tracking, Property 6: Invalid deposit estado enum values are rejected
// **Validates: Requirements 4.3**
#[test]
fn test_invalid_estado_enum_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner
        .run(&invalid_estado_deposito(), |bad_estado| {
            pbt_async::p6(bad_estado);
            Ok(())
        })
        .unwrap();
}

// Feature: deposit-tracking, Property 7: Deposit operations on contracts without deposit are rejected
// **Validates: Requirements 2.6**
#[test]
fn test_no_deposit_operations_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    // Generate None or "0" deposit values
    let strategy = prop_oneof![
        Just(None),
        Just(Some("0".to_string())),
        Just(Some("0.00".to_string())),
    ];

    runner
        .run(&strategy, |deposito| {
            pbt_async::p7(deposito);
            Ok(())
        })
        .unwrap();
}
