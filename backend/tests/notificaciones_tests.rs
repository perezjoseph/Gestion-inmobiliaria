use chrono::Utc;
use realestate_backend::app::create_app;
use realestate_backend::config::AppConfig;
use realestate_backend::services::auth::{Claims, encode_jwt};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait, Set};
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
        titulo: Set("Propiedad Test Notif".to_string()),
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

async fn create_test_contrato(
    db: &DatabaseConnection,
    propiedad_id: Uuid,
    inquilino_id: Uuid,
    org_id: Uuid,
    estado: &str,
    fecha_fin: chrono::NaiveDate,
) -> Uuid {
    use realestate_backend::entities::contrato;
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    contrato::ActiveModel {
        id: Set(id),
        propiedad_id: Set(propiedad_id),
        inquilino_id: Set(inquilino_id),
        fecha_inicio: Set(Utc::now().date_naive() - chrono::Duration::days(365)),
        fecha_fin: Set(fecha_fin),
        monto_mensual: Set(Decimal::new(25000, 0)),
        deposito: Set(None),
        moneda: Set("DOP".to_string()),
        estado: Set(estado.to_string()),
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
    .insert(db)
    .await
    .expect("Failed to create test contrato");
    id
}

async fn create_test_pago_vencido(
    db: &DatabaseConnection,
    contrato_id: Uuid,
    org_id: Uuid,
) -> Uuid {
    use realestate_backend::entities::pago;
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    let overdue_date = Utc::now().date_naive() - chrono::Duration::days(5);
    pago::ActiveModel {
        id: Set(id),
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
    .insert(db)
    .await
    .expect("Failed to create test pago");
    id
}

async fn cleanup_notificaciones(db: &DatabaseConnection, usuario_id: Uuid) {
    use realestate_backend::entities::notificacion;
    use sea_orm::ColumnTrait;
    use sea_orm::QueryFilter;
    let _ = notificacion::Entity::delete_many()
        .filter(notificacion::Column::UsuarioId.eq(usuario_id))
        .exec(db)
        .await;
}

/// Shared test data setup: org + admin user + propiedad + inquilino + overdue contrato + overdue pago
async fn setup_notif_data(db: &DatabaseConnection) -> (Uuid, Uuid, Uuid) {
    let org_id = create_test_organizacion(db).await;
    let admin_id = create_test_usuario(db, "admin", org_id).await;
    let propiedad_id = create_test_propiedad(db, org_id).await;
    let inquilino_id = create_test_inquilino(db, org_id).await;
    let contrato_id = create_test_contrato(
        db,
        propiedad_id,
        inquilino_id,
        org_id,
        "activo",
        Utc::now().date_naive() - chrono::Duration::days(1),
    )
    .await;
    let _pago_id = create_test_pago_vencido(db, contrato_id, org_id).await;
    (org_id, admin_id, propiedad_id)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[test]
fn test_list_empty_notifications() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let user_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(user_id, "admin", org_id);

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["data"].as_array().unwrap().len(), 0);
        assert_eq!(body["total"].as_u64().unwrap(), 0);

        cleanup_notificaciones(&db, user_id).await;
    });
}

#[test]
fn test_generate_notifications_counts() {
    with_db(|db| async move {
        let (org_id, admin_id, _) = setup_notif_data(&db).await;
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
            .uri("/api/v1/notificaciones/generar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["pagoVencido"].as_u64().unwrap() >= 1);
        assert!(body["total"].as_u64().unwrap() >= 1);

        cleanup_notificaciones(&db, admin_id).await;
    });
}

#[test]
fn test_list_after_generation() {
    with_db(|db| async move {
        let (org_id, admin_id, _) = setup_notif_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Generate
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/notificaciones/generar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // List
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(!body["data"].as_array().unwrap().is_empty());
        assert!(body["total"].as_u64().unwrap() >= 1);

        cleanup_notificaciones(&db, admin_id).await;
    });
}

#[test]
fn test_filter_by_tipo() {
    with_db(|db| async move {
        let (org_id, admin_id, _) = setup_notif_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Generate pago_vencido notifications
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/notificaciones/generar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // Filter by tipo=pago_vencido → only matching
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones?tipo=pago_vencido")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        for item in body["data"].as_array().unwrap() {
            assert_eq!(item["tipo"], "pago_vencido");
        }

        // Filter by tipo that shouldn't exist → empty or only matching
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones?tipo=documento_vencido")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        for item in body["data"].as_array().unwrap() {
            assert_eq!(item["tipo"], "documento_vencido");
        }

        cleanup_notificaciones(&db, admin_id).await;
    });
}

#[test]
fn test_filter_by_leida() {
    with_db(|db| async move {
        let (org_id, admin_id, _) = setup_notif_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Generate
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/notificaciones/generar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // Filter leida=false → all generated are unread
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones?leida=false")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(!body["data"].as_array().unwrap().is_empty());
        for item in body["data"].as_array().unwrap() {
            assert_eq!(item["leida"], false);
        }

        // Filter leida=true → none yet
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones?leida=true")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        for item in body["data"].as_array().unwrap() {
            assert_eq!(item["leida"], true);
        }

        cleanup_notificaciones(&db, admin_id).await;
    });
}

#[test]
fn test_unread_count() {
    with_db(|db| async move {
        let (org_id, admin_id, _) = setup_notif_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Generate
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/notificaciones/generar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let gen_resp = actix_web::test::call_service(&app, req).await;
        let gen_body: Value = actix_web::test::read_body_json(gen_resp).await;
        let generated_total = gen_body["total"].as_u64().unwrap();

        // Unread count should match generated total
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones/no-leidas/conteo")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["count"].as_u64().unwrap() >= generated_total);

        cleanup_notificaciones(&db, admin_id).await;
    });
}

#[test]
fn test_mark_one_as_read() {
    with_db(|db| async move {
        let (org_id, admin_id, _) = setup_notif_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Generate
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/notificaciones/generar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // Get unread count before
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones/no-leidas/conteo")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let count_before = body["count"].as_u64().unwrap();

        // Get first notification ID
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let notif_id = body["data"][0]["id"].as_str().unwrap();

        // Mark as read
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/notificaciones/{notif_id}/leer"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["leida"], true);

        // Unread count decremented
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones/no-leidas/conteo")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["count"].as_u64().unwrap(), count_before - 1);

        cleanup_notificaciones(&db, admin_id).await;
    });
}

#[test]
fn test_mark_nonexistent_returns_404() {
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
            .uri(&format!("/api/v1/notificaciones/{fake_id}/leer"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    });
}

#[test]
fn test_mark_another_users_notification_returns_404() {
    with_db(|db| async move {
        let (org_id, admin_id, _) = setup_notif_data(&db).await;
        let other_id = create_test_usuario(&db, "gerente", org_id).await;
        let admin_token = make_token(admin_id, "admin", org_id);
        let other_token = make_token(other_id, "gerente", org_id);

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Generate (creates notifications for both users in the org)
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/notificaciones/generar")
            .insert_header(("Authorization", format!("Bearer {admin_token}")))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // Get admin's notification
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones")
            .insert_header(("Authorization", format!("Bearer {admin_token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let admin_notif_id = body["data"][0]["id"].as_str().unwrap().to_string();

        // Other user tries to mark admin's notification → 404
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/notificaciones/{admin_notif_id}/leer"))
            .insert_header(("Authorization", format!("Bearer {other_token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);

        cleanup_notificaciones(&db, admin_id).await;
        cleanup_notificaciones(&db, other_id).await;
    });
}

#[test]
fn test_mark_all_as_read() {
    with_db(|db| async move {
        let (org_id, admin_id, _) = setup_notif_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Generate
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/notificaciones/generar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // Mark all as read
        let req = actix_web::test::TestRequest::put()
            .uri("/api/v1/notificaciones/leer-todas")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["actualizadas"].as_u64().unwrap() >= 1);

        // Unread count should be 0
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones/no-leidas/conteo")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["count"].as_u64().unwrap(), 0);

        cleanup_notificaciones(&db, admin_id).await;
    });
}

#[test]
fn test_generate_as_visualizador_returns_403() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let viewer_id = create_test_usuario(&db, "visualizador", org_id).await;
        let token = make_token(viewer_id, "visualizador", org_id);

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/notificaciones/generar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    });
}

#[test]
fn test_generate_twice_deduplication() {
    with_db(|db| async move {
        let (org_id, admin_id, _) = setup_notif_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // First generation
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/notificaciones/generar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        let first_total = body["total"].as_u64().unwrap();
        assert!(first_total >= 1);

        // Second generation → zero new (deduplication)
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/notificaciones/generar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["total"].as_u64().unwrap(), 0);

        cleanup_notificaciones(&db, admin_id).await;
    });
}

#[test]
fn test_mantenimiento_state_change_generates_notification() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;

        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Clean pre-existing notifications
        cleanup_notificaciones(&db, admin_id).await;

        // Create solicitud de mantenimiento
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/mantenimiento")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "propiedadId": propiedad_id,
                "titulo": "Reparar tubería notif test"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let solicitud_id = body["id"].as_str().unwrap().to_string();

        // Change state → en_progreso (generates mantenimiento_actualizado notification)
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/estado"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "estado": "en_progreso" }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Check for mantenimiento_actualizado notification
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones?tipo=mantenimiento_actualizado")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let notifs = body["data"].as_array().unwrap();
        assert!(
            notifs.iter().any(|n| {
                n["tipo"] == "mantenimiento_actualizado"
                    && n["entityType"] == "solicitud_mantenimiento"
            }),
            "Expected a mantenimiento_actualizado notification"
        );

        // Cleanup
        use realestate_backend::entities::solicitud_mantenimiento;
        let _ =
            solicitud_mantenimiento::Entity::delete_by_id(solicitud_id.parse::<Uuid>().unwrap())
                .exec(&db)
                .await;
        cleanup_notificaciones(&db, admin_id).await;
    });
}

#[test]
fn test_legacy_pagos_vencidos_endpoint() {
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

        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/notificaciones/pagos-vencidos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        // Legacy endpoint returns Vec<PagoVencido> (an array)
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body.is_array());
    });
}
