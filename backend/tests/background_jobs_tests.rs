use chrono::Utc;
use realestate_backend::app::create_app;
use realestate_backend::config::AppConfig;
use realestate_backend::services::auth::{Claims, encode_jwt};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait, Set};
use sea_orm_migration::MigratorTrait;
use serde_json::Value;
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
        nombre: Set(format!("Org BgJob {id}")),
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
        email: Set(format!("{rol}+{id}@bgjob.com")),
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
        titulo: Set("Propiedad BgJob Test".to_string()),
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
        apellido: Set("BgJob".to_string()),
        email: Set(Some(format!("inquilino+{id}@bgjob.com"))),
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
        fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
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

async fn create_test_pago(
    db: &DatabaseConnection,
    contrato_id: Uuid,
    org_id: Uuid,
    estado: &str,
    fecha_vencimiento: chrono::NaiveDate,
) -> Uuid {
    use realestate_backend::entities::pago;
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    pago::ActiveModel {
        id: Set(id),
        contrato_id: Set(contrato_id),
        monto: Set(Decimal::new(25000, 0)),
        moneda: Set("DOP".to_string()),
        fecha_pago: Set(None),
        fecha_vencimiento: Set(fecha_vencimiento),
        metodo_pago: Set(None),
        estado: Set(estado.to_string()),
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

async fn create_test_documento(
    db: &DatabaseConnection,
    uploaded_by: Uuid,
    estado_verificacion: &str,
    fecha_vencimiento: Option<chrono::NaiveDate>,
) -> Uuid {
    use realestate_backend::entities::documento;
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    documento::ActiveModel {
        id: Set(id),
        entity_type: Set("propiedad".to_string()),
        entity_id: Set(Uuid::new_v4()),
        filename: Set("test_doc.pdf".to_string()),
        file_path: Set("/tmp/test_doc.pdf".to_string()),
        mime_type: Set("application/pdf".to_string()),
        file_size: Set(1024),
        uploaded_by: Set(uploaded_by),
        created_at: Set(now),
        tipo_documento: Set("otro".to_string()),
        estado_verificacion: Set(estado_verificacion.to_string()),
        fecha_vencimiento: Set(fecha_vencimiento),
        verificado_por: Set(None),
        fecha_verificacion: Set(None),
        notas_verificacion: Set(None),
        numero_documento: Set(None),
        contenido_editable: Set(None),
        updated_at: Set(Some(now)),
    }
    .insert(db)
    .await
    .expect("Failed to create test documento");
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

/// Test ejecutar tarea manualmente (marcar_pagos_atrasados) → 200 with execution record
/// Requirements: 7.1, 5.1
#[test]
fn test_ejecutar_tarea_manualmente_200() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/tareas/marcar_pagos_atrasados/ejecutar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = actix_web::test::read_body_json(resp).await;
        let ejecucion = &body["ejecucion"];
        assert!(ejecucion["id"].is_string());
        assert_eq!(ejecucion["nombreTarea"], "marcar_pagos_atrasados");
        assert!(ejecucion["iniciadoEn"].is_string());
        assert!(ejecucion["duracionMs"].is_number());
        assert_eq!(ejecucion["exitosa"], true);
        assert!(ejecucion["registrosAfectados"].is_number());
    });
}

/// Test ejecutar tarea con nombre inválido → 404
/// Requirements: 7.2
#[test]
fn test_ejecutar_tarea_nombre_invalido_404() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/tareas/tarea_inexistente/ejecutar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    });
}

/// Test ejecutar tarea como gerente → 403
/// Requirements: 7.4
#[test]
fn test_ejecutar_tarea_gerente_403() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let gerente_id = create_test_usuario(&db, "gerente", org_id).await;
        let token = make_token(gerente_id, "gerente", org_id);

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/tareas/marcar_pagos_atrasados/ejecutar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    });
}

/// Test ejecutar tarea como visualizador → 403
/// Requirements: 7.5
#[test]
fn test_ejecutar_tarea_visualizador_403() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let viewer_id = create_test_usuario(&db, "visualizador", org_id).await;
        let token = make_token(viewer_id, "visualizador", org_id);

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/tareas/marcar_pagos_atrasados/ejecutar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    });
}

/// Test consultar historial → paginated response
/// Requirements: 8.1
#[test]
fn test_consultar_historial_paginado() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        // Execute a task first so there's at least one record
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/tareas/marcar_pagos_atrasados/ejecutar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // Query history
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/tareas/historial")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["data"].is_array());
        assert!(body["total"].is_number());
        assert!(body["page"].is_number());
        assert!(body["perPage"].is_number());
        assert!(body["total"].as_u64().unwrap() >= 1);
    });
}

/// Test filtrar historial por nombre_tarea → only matching records
/// Requirements: 8.2
#[test]
fn test_filtrar_historial_por_nombre_tarea() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        // Execute two different tasks
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/tareas/marcar_pagos_atrasados/ejecutar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/tareas/marcar_contratos_vencidos/ejecutar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // Filter by nombre_tarea
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/tareas/historial?nombreTarea=marcar_pagos_atrasados")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = actix_web::test::read_body_json(resp).await;
        let data = body["data"].as_array().unwrap();
        assert!(!data.is_empty());
        for item in data {
            assert_eq!(item["nombreTarea"], "marcar_pagos_atrasados");
        }
    });
}

/// Test filtrar historial por exitosa → only matching records
/// Requirements: 8.3
#[test]
fn test_filtrar_historial_por_exitosa() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        // Execute a task (should succeed)
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/tareas/marcar_pagos_atrasados/ejecutar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let _ = actix_web::test::call_service(&app, req).await;

        // Filter by exitosa=true
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/tareas/historial?exitosa=true")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = actix_web::test::read_body_json(resp).await;
        let data = body["data"].as_array().unwrap();
        for item in data {
            assert_eq!(item["exitosa"], true);
        }
    });
}

/// Test consultar historial como gerente → 403
/// Requirements: 8.4
#[test]
fn test_consultar_historial_gerente_403() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let gerente_id = create_test_usuario(&db, "gerente", org_id).await;
        let token = make_token(gerente_id, "gerente", org_id);

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/tareas/historial")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    });
}

/// Test marcar_pagos_atrasados updates pending overdue payments to atrasado
/// Requirements: 1.1
#[test]
fn test_marcar_pagos_atrasados_updates_payments() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;
        let contrato_id = create_test_contrato(
            &db,
            propiedad_id,
            inquilino_id,
            org_id,
            "activo",
            Utc::now().date_naive() + chrono::Duration::days(365),
        )
        .await;

        // Create a pending payment with overdue date
        let overdue_date = Utc::now().date_naive() - chrono::Duration::days(5);
        let pago_id = create_test_pago(&db, contrato_id, org_id, "pendiente", overdue_date).await;

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/tareas/marcar_pagos_atrasados/ejecutar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["ejecucion"]["registrosAfectados"].as_i64().unwrap() >= 1);

        // Verify the payment was updated
        use realestate_backend::entities::pago;
        let updated = pago::Entity::find_by_id(pago_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.estado, "atrasado");
    });
}

/// Test marcar_contratos_vencidos updates active expired contracts to vencido
/// Requirements: 2.1
#[test]
fn test_marcar_contratos_vencidos_updates_contracts() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;

        // Create an active contract with expired fecha_fin
        let expired_date = Utc::now().date_naive() - chrono::Duration::days(10);
        let contrato_id = create_test_contrato(
            &db,
            propiedad_id,
            inquilino_id,
            org_id,
            "activo",
            expired_date,
        )
        .await;

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/tareas/marcar_contratos_vencidos/ejecutar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["ejecucion"]["registrosAfectados"].as_i64().unwrap() >= 1);

        // Verify the contract was updated
        use realestate_backend::entities::contrato;
        let updated = contrato::Entity::find_by_id(contrato_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.estado, "vencido");
    });
}

/// Test marcar_documentos_vencidos updates verified expired documents to vencido
/// Requirements: 3.1
#[test]
fn test_marcar_documentos_vencidos_updates_documents() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);

        // Create a verified document with expired fecha_vencimiento
        let expired_date = Utc::now().date_naive() - chrono::Duration::days(10);
        let doc_id = create_test_documento(&db, admin_id, "verificado", Some(expired_date)).await;

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/tareas/marcar_documentos_vencidos/ejecutar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["ejecucion"]["registrosAfectados"].as_i64().unwrap() >= 1);

        // Verify the document was updated
        use realestate_backend::entities::documento;
        let updated = documento::Entity::find_by_id(doc_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.estado_verificacion, "vencido");
    });
}

/// Test idempotencia: second execution returns 0 registros_afectados
/// Requirements: 1.4, 2.4, 3.4
#[test]
fn test_idempotencia_segunda_ejecucion_cero_afectados() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;
        let contrato_id = create_test_contrato(
            &db,
            propiedad_id,
            inquilino_id,
            org_id,
            "activo",
            Utc::now().date_naive() + chrono::Duration::days(365),
        )
        .await;

        // Create a pending overdue payment
        let overdue_date = Utc::now().date_naive() - chrono::Duration::days(5);
        let _pago_id = create_test_pago(&db, contrato_id, org_id, "pendiente", overdue_date).await;

        let app = actix_web::test::init_service(make_app(db.clone())).await;

        // First execution — should affect at least 1 record
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/tareas/marcar_pagos_atrasados/ejecutar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["ejecucion"]["registrosAfectados"].as_i64().unwrap() >= 1);

        // Second execution — should affect 0 records (idempotent)
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/tareas/marcar_pagos_atrasados/ejecutar")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["ejecucion"]["registrosAfectados"].as_i64().unwrap(), 0);
    });
}
