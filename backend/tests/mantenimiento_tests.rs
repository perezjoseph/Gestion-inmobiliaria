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

#[path = "../migrations/mod.rs"]
mod migrations;

const JWT_SECRET: &str = "test_secret_key_that_is_long_enough_for_jwt";

async fn shared_db() -> Option<&'static DatabaseConnection> {
    static SHARED: tokio::sync::OnceCell<Result<DatabaseConnection, String>> =
        tokio::sync::OnceCell::const_new();
    SHARED
        .get_or_init(|| async {
            dotenvy::dotenv().ok();
            let url = match std::env::var("DATABASE_URL") {
                Ok(u) => u,
                Err(_) => return Err("DATABASE_URL not set".to_string()),
            };
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
        })
        .await
        .as_ref()
        .ok()
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
        titulo: Set("Propiedad Test".to_string()),
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

#[allow(dead_code)]
async fn create_test_inquilino(db: &DatabaseConnection) -> Uuid {
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
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .expect("Failed to create test inquilino");
    id
}

async fn create_test_unidad(db: &DatabaseConnection, propiedad_id: Uuid) -> Uuid {
    use realestate_backend::entities::unidad;
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    unidad::ActiveModel {
        id: Set(id),
        propiedad_id: Set(propiedad_id),
        numero_unidad: Set(format!("U-{id}")),
        piso: Set(Some(1)),
        habitaciones: Set(Some(2)),
        banos: Set(Some(1)),
        area_m2: Set(Some(Decimal::new(5000, 2))),
        precio: Set(Decimal::new(1500000, 2)),
        moneda: Set("DOP".to_string()),
        estado: Set("disponible".to_string()),
        descripcion: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .expect("Failed to create test unidad");
    id
}

async fn cleanup_solicitud(db: &DatabaseConnection, id: Uuid) {
    use realestate_backend::entities::solicitud_mantenimiento;
    use sea_orm::EntityTrait;
    let _ = solicitud_mantenimiento::Entity::delete_by_id(id)
        .exec(db)
        .await;
}

#[actix_web::test]
async fn test_crud_cycle() {
    let Some(db) = shared_db().await else {
        eprintln!("⚠ DB not reachable – skipping integration test");
        return;
    };
    let config = make_config();
    let admin_id = create_test_usuario(db, "admin").await;
    let token = make_token(admin_id, "admin");
    let propiedad_id = create_test_propiedad(db).await;

    let app = test::init_service(create_app(
        db.clone(),
        config,
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new()),
    ))
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Reparar tubería",
            "descripcion": "Fuga en el baño",
            "prioridad": "alta",
            "costoMonto": "150.00",
            "costoMoneda": "DOP"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: Value = test::read_body_json(resp).await;
    let solicitud_id = body["id"].as_str().unwrap();
    let solicitud_uuid: Uuid = solicitud_id.parse().unwrap();
    assert_eq!(body["estado"], "pendiente");
    assert_eq!(body["prioridad"], "alta");
    assert_eq!(body["titulo"], "Reparar tubería");
    assert_eq!(body["propiedadId"], propiedad_id.to_string());

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["titulo"], "Reparar tubería");
    assert!(body["notas"].is_array());

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "titulo": "Reparar tubería urgente",
            "prioridad": "urgente"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["titulo"], "Reparar tubería urgente");
    assert_eq!(body["prioridad"], "urgente");

    let req = test::TestRequest::get()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(body["data"].is_array());
    assert!(body["total"].as_u64().unwrap() >= 1);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    let _ = solicitud_uuid;
}

#[actix_web::test]
async fn test_state_machine_flow() {
    let Some(db) = shared_db().await else {
        eprintln!("⚠ DB not reachable – skipping integration test");
        return;
    };
    let config = make_config();
    let admin_id = create_test_usuario(db, "admin").await;
    let token = make_token(admin_id, "admin");
    let propiedad_id = create_test_propiedad(db).await;

    let app = test::init_service(create_app(
        db.clone(),
        config,
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new()),
    ))
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Pintar paredes"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: Value = test::read_body_json(resp).await;
    let solicitud_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["estado"], "pendiente");
    assert!(body["fechaInicio"].is_null());

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/estado"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "estado": "en_progreso" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["estado"], "en_progreso");
    assert!(!body["fechaInicio"].is_null());

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/estado"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "estado": "completado" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["estado"], "completado");
    assert!(!body["fechaFin"].is_null());

    cleanup_solicitud(db, solicitud_id.parse().unwrap()).await;
}

#[actix_web::test]
async fn test_invalid_state_transitions() {
    let Some(db) = shared_db().await else {
        eprintln!("⚠ DB not reachable – skipping integration test");
        return;
    };
    let config = make_config();
    let admin_id = create_test_usuario(db, "admin").await;
    let token = make_token(admin_id, "admin");
    let propiedad_id = create_test_propiedad(db).await;

    let app = test::init_service(create_app(
        db.clone(),
        config,
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new()),
    ))
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Test transiciones"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let solicitud_id = body["id"].as_str().unwrap().to_string();

    // pendiente → completado (invalid, must go through en_progreso)
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/estado"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "estado": "completado" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422);

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/estado"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "estado": "en_progreso" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // en_progreso → pendiente (invalid)
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/estado"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "estado": "pendiente" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422);

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/estado"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "estado": "completado" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // completado → en_progreso (invalid)
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/estado"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "estado": "en_progreso" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422);

    // completado → pendiente (invalid)
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/estado"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "estado": "pendiente" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422);

    cleanup_solicitud(db, solicitud_id.parse().unwrap()).await;
}

#[actix_web::test]
async fn test_notes_add_and_list() {
    let Some(db) = shared_db().await else {
        eprintln!("⚠ DB not reachable – skipping integration test");
        return;
    };
    let config = make_config();
    let admin_id = create_test_usuario(db, "admin").await;
    let token = make_token(admin_id, "admin");
    let propiedad_id = create_test_propiedad(db).await;

    let app = test::init_service(create_app(
        db.clone(),
        config,
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new()),
    ))
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Test notas"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let solicitud_id = body["id"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/notas"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "contenido": "Primera nota" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let nota1: Value = test::read_body_json(resp).await;
    assert_eq!(nota1["contenido"], "Primera nota");
    assert_eq!(nota1["autorId"], admin_id.to_string());

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/notas"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "contenido": "Segunda nota" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    // notes should be ordered by created_at DESC
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let notas = body["notas"].as_array().unwrap();
    assert_eq!(notas.len(), 2);
    assert_eq!(notas[0]["contenido"], "Segunda nota");
    assert_eq!(notas[1]["contenido"], "Primera nota");

    cleanup_solicitud(db, solicitud_id.parse().unwrap()).await;
}

#[actix_web::test]
async fn test_filters() {
    let Some(db) = shared_db().await else {
        eprintln!("⚠ DB not reachable – skipping integration test");
        return;
    };
    let config = make_config();
    let admin_id = create_test_usuario(db, "admin").await;
    let token = make_token(admin_id, "admin");
    let propiedad_id = create_test_propiedad(db).await;
    let propiedad_id2 = create_test_propiedad(db).await;

    let app = test::init_service(create_app(
        db.clone(),
        config,
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new()),
    ))
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Filtro test 1",
            "prioridad": "alta"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let b1: Value = test::read_body_json(resp).await;
    let id1: Uuid = b1["id"].as_str().unwrap().parse().unwrap();

    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id2,
            "titulo": "Filtro test 2",
            "prioridad": "baja"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let b2: Value = test::read_body_json(resp).await;
    let id2: Uuid = b2["id"].as_str().unwrap().parse().unwrap();

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{id1}/estado"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "estado": "en_progreso" }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/mantenimiento?estado=en_progreso")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    for item in body["data"].as_array().unwrap() {
        assert_eq!(item["estado"], "en_progreso");
    }

    let req = test::TestRequest::get()
        .uri("/api/v1/mantenimiento?prioridad=baja")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    for item in body["data"].as_array().unwrap() {
        assert_eq!(item["prioridad"], "baja");
    }

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/mantenimiento?propiedadId={propiedad_id}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    for item in body["data"].as_array().unwrap() {
        assert_eq!(item["propiedadId"], propiedad_id.to_string());
    }

    cleanup_solicitud(db, id1).await;
    cleanup_solicitud(db, id2).await;
}

#[actix_web::test]
async fn test_access_control() {
    let Some(db) = shared_db().await else {
        eprintln!("⚠ DB not reachable – skipping integration test");
        return;
    };
    let config = make_config();
    let admin_id = create_test_usuario(db, "admin").await;
    let gerente_id = create_test_usuario(db, "gerente").await;
    let visualizador_id = create_test_usuario(db, "visualizador").await;
    let admin_token = make_token(admin_id, "admin");
    let gerente_token = make_token(gerente_id, "gerente");
    let viewer_token = make_token(visualizador_id, "visualizador");
    let propiedad_id = create_test_propiedad(db).await;

    let app = test::init_service(create_app(
        db.clone(),
        config,
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new()),
    ))
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {viewer_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {viewer_token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "No debería crearse"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);

    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {gerente_token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Creada por gerente"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: Value = test::read_body_json(resp).await;
    let solicitud_id = body["id"].as_str().unwrap().to_string();

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}"))
        .insert_header(("Authorization", format!("Bearer {viewer_token}")))
        .set_json(json!({ "titulo": "Modificada" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/estado"))
        .insert_header(("Authorization", format!("Bearer {viewer_token}")))
        .set_json(json!({ "estado": "en_progreso" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/notas"))
        .insert_header(("Authorization", format!("Bearer {viewer_token}")))
        .set_json(json!({ "contenido": "No debería crearse" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}"))
        .insert_header(("Authorization", format!("Bearer {gerente_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}"))
        .insert_header(("Authorization", format!("Bearer {viewer_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}"))
        .insert_header(("Authorization", format!("Bearer {admin_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);
}

#[actix_web::test]
async fn test_fk_validations() {
    let Some(db) = shared_db().await else {
        eprintln!("⚠ DB not reachable – skipping integration test");
        return;
    };
    let config = make_config();
    let admin_id = create_test_usuario(db, "admin").await;
    let token = make_token(admin_id, "admin");
    let propiedad_id = create_test_propiedad(db).await;
    let propiedad_id2 = create_test_propiedad(db).await;
    let unidad_id = create_test_unidad(db, propiedad_id2).await;

    let app = test::init_service(create_app(
        db.clone(),
        config,
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new()),
    ))
    .await;

    // Non-existent propiedad_id → 404
    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": Uuid::new_v4(),
            "titulo": "Propiedad inexistente"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    // Non-existent inquilino_id → 404
    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Inquilino inexistente",
            "inquilinoId": Uuid::new_v4()
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    // Unidad not belonging to propiedad → 422
    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Unidad no pertenece",
            "unidadId": unidad_id
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422);
    let body: Value = test::read_body_json(resp).await;
    assert!(body["message"].as_str().unwrap().contains("no pertenece"));
}

#[actix_web::test]
async fn test_validations() {
    let Some(db) = shared_db().await else {
        eprintln!("⚠ DB not reachable – skipping integration test");
        return;
    };
    let config = make_config();
    let admin_id = create_test_usuario(db, "admin").await;
    let token = make_token(admin_id, "admin");
    let propiedad_id = create_test_propiedad(db).await;

    let app = test::init_service(create_app(
        db.clone(),
        config,
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new()),
    ))
    .await;

    // Empty titulo → 422
    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": ""
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422);

    // Invalid prioridad → 422
    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Test prioridad",
            "prioridad": "critica"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422);

    // Invalid moneda → 422
    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Test moneda",
            "costoMonto": "100.00",
            "costoMoneda": "EUR"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422);

    // Negative costo_monto → 422
    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Test monto negativo",
            "costoMonto": "-50.00",
            "costoMoneda": "DOP"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422);

    // Empty note contenido → 422
    // First create a solicitud to add a note to
    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Para nota vacía"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let solicitud_id = body["id"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/notas"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "contenido": "" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422);

    // Whitespace-only note contenido → 422
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/notas"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "contenido": "   " }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 422);

    cleanup_solicitud(db, solicitud_id.parse().unwrap()).await;
}

#[actix_web::test]
async fn test_auditoria_entries() {
    let Some(db) = shared_db().await else {
        eprintln!("⚠ DB not reachable – skipping integration test");
        return;
    };
    let config = make_config();
    let admin_id = create_test_usuario(db, "admin").await;
    let token = make_token(admin_id, "admin");
    let propiedad_id = create_test_propiedad(db).await;

    let app = test::init_service(create_app(
        db.clone(),
        config,
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new()),
    ))
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Auditoría test"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: Value = test::read_body_json(resp).await;
    let solicitud_id = body["id"].as_str().unwrap().to_string();
    let solicitud_uuid: Uuid = solicitud_id.parse().unwrap();

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/auditoria?entity_type=solicitud_mantenimiento&entity_id={solicitud_id}"
        ))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let entries = body["data"].as_array().unwrap();
    assert!(entries.iter().any(|e| e["accion"] == "crear"));

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "titulo": "Auditoría test actualizado" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/estado"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "estado": "en_progreso" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}/notas"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "contenido": "Nota de auditoría" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/auditoria?entity_type=solicitud_mantenimiento&entity_id={solicitud_id}"
        ))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let entries = body["data"].as_array().unwrap();
    assert!(entries.iter().any(|e| e["accion"] == "crear"));
    assert!(entries.iter().any(|e| e["accion"] == "actualizar"));
    assert!(entries.iter().any(|e| e["accion"] == "cambiar_estado"));

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/mantenimiento/{solicitud_id}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/auditoria?entity_type=solicitud_mantenimiento&entity_id={solicitud_id}"
        ))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let entries = body["data"].as_array().unwrap();
    assert!(entries.iter().any(|e| e["accion"] == "eliminar"));

    let _ = solicitud_uuid;
}

#[actix_web::test]
async fn test_default_prioridad() {
    let Some(db) = shared_db().await else {
        eprintln!("⚠ DB not reachable – skipping integration test");
        return;
    };
    let config = make_config();
    let admin_id = create_test_usuario(db, "admin").await;
    let token = make_token(admin_id, "admin");
    let propiedad_id = create_test_propiedad(db).await;

    let app = test::init_service(create_app(
        db.clone(),
        config,
        actix_web::web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new()),
    ))
    .await;

    // Create without specifying prioridad → defaults to "media"
    let req = test::TestRequest::post()
        .uri("/api/v1/mantenimiento")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "propiedadId": propiedad_id,
            "titulo": "Sin prioridad"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["prioridad"], "media");

    cleanup_solicitud(db, body["id"].as_str().unwrap().parse().unwrap()).await;
}
