#![allow(clippy::needless_return)]

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use realestate_backend::services::mantenimiento::validar_transicion;
use rust_decimal::Decimal;

fn valid_prioridad() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("baja".to_string()),
        Just("media".to_string()),
        Just("alta".to_string()),
        Just("urgente".to_string()),
    ]
}

fn valid_moneda() -> impl Strategy<Value = String> {
    prop_oneof![Just("DOP".to_string()), Just("USD".to_string())]
}

fn valid_titulo() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{3,50}"
        .prop_map(|s| s.trim().to_string())
        .prop_filter("titulo must not be empty", |s| !s.trim().is_empty())
}

fn valid_descripcion() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "[a-zA-Z0-9 ]{5,100}".prop_map(Some),]
}

fn non_negative_decimal() -> impl Strategy<Value = Decimal> {
    (0i64..1_000_000i64).prop_map(|v| Decimal::new(v, 2))
}

fn negative_decimal() -> impl Strategy<Value = Decimal> {
    (-1_000_000i64..-1i64).prop_map(|v| Decimal::new(v, 2))
}

fn whitespace_string() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("".to_string()),
        Just(" ".to_string()),
        Just("  ".to_string()),
        Just("\t".to_string()),
        Just("\n".to_string()),
        Just("   \t\n  ".to_string()),
    ]
}

fn invalid_prioridad() -> impl Strategy<Value = String> {
    "[a-zA-Z]{1,20}".prop_filter("must not be a valid prioridad", |s| {
        !["baja", "media", "alta", "urgente"].contains(&s.as_str())
    })
}

fn invalid_moneda() -> impl Strategy<Value = String> {
    "[A-Z]{1,5}".prop_filter("must not be a valid moneda", |s| {
        !["DOP", "USD"].contains(&s.as_str())
    })
}

fn invalid_transition_pair() -> impl Strategy<Value = (String, String)> {
    let invalid_pairs: Vec<(String, String)> = vec![
        ("pendiente".into(), "completado".into()),
        ("completado".into(), "pendiente".into()),
        ("completado".into(), "en_progreso".into()),
        ("completado".into(), "completado".into()),
        ("en_progreso".into(), "pendiente".into()),
        ("en_progreso".into(), "en_progreso".into()),
        ("pendiente".into(), "pendiente".into()),
    ];
    proptest::sample::select(invalid_pairs)
}

#[path = "../migrations/mod.rs"]
mod migrations;

// All async test logic lives in a separate module so that calling async fns
// from sync #[test] functions (Rust 2024 edition restriction) is avoided.
// Each public function in this module creates its own tokio runtime.
mod pbt_async {
    use actix_web::test;
    use chrono::Utc;
    use realestate_backend::app::create_app;
    use realestate_backend::config::AppConfig;
    use realestate_backend::services::auth::{Claims, encode_jwt};
    use rust_decimal::Decimal;
    use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, Set};
    use sea_orm_migration::MigratorTrait;
    use serde_json::{Value, json};
    use uuid::Uuid;

    const JWT_SECRET: &str = "test_secret_key_that_is_long_enough_for_jwt";

    static MIGRATIONS_DONE: std::sync::Once = std::sync::Once::new();

    fn db_url() -> String {
        dotenvy::dotenv().ok();
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests")
    }

    fn with_db<F, Fut>(f: F)
    where
        F: FnOnce(DatabaseConnection) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        MIGRATIONS_DONE.call_once(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = Database::connect(db_url())
                    .await
                    .expect("Failed to connect to database for migrations");
                super::migrations::Migrator::up(&db, None)
                    .await
                    .expect("Failed to run migrations");
                db.close().await.ok();
            });
        });
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = Database::connect(db_url())
                .await
                .expect("Failed to connect to database");
            f(db).await;
        });
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
            titulo: Set("Propiedad PBT".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle PBT 123".to_string()),
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

    pub fn p1(
        titulo: String,
        descripcion: Option<String>,
        prioridad: String,
        monto: Decimal,
        moneda: String,
    ) {
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
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "propiedadId": propiedad_id,
                    "titulo": titulo,
                    "descripcion": descripcion,
                    "prioridad": prioridad,
                    "costoMonto": monto.to_string(),
                    "costoMoneda": moneda,
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let solicitud_id = body["id"].as_str().unwrap();

            assert_eq!(body["estado"], "pendiente");
            assert_eq!(body["prioridad"], prioridad);
            assert_eq!(body["titulo"], titulo);
            assert_eq!(body["propiedadId"], propiedad_id.to_string());
            assert_eq!(body["costoMoneda"], moneda);
            if let Some(ref desc) = descripcion {
                assert_eq!(body["descripcion"], *desc);
            } else {
                assert!(body["descripcion"].is_null());
            }

            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/mantenimiento/{solicitud_id}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let detail: Value = test::read_body_json(resp).await;
            assert_eq!(detail["titulo"], titulo);
            assert_eq!(detail["estado"], "pendiente");
            assert_eq!(detail["prioridad"], prioridad);

            cleanup_solicitud(&db, solicitud_id.parse().unwrap()).await;
        });
    }

    pub fn p2(count: u32) {
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
            for i in 0..count {
                let req = test::TestRequest::post()
                    .uri("/api/v1/mantenimiento")
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(json!({
                        "propiedadId": propiedad_id,
                        "titulo": format!("Ordering test {}", i),
                    }))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                let body: Value = test::read_body_json(resp).await;
                created_ids.push(body["id"].as_str().unwrap().parse::<Uuid>().unwrap());
            }

            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/mantenimiento?propiedadId={propiedad_id}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let body: Value = test::read_body_json(resp).await;
            let items = body["data"].as_array().unwrap();

            for window in items.windows(2) {
                let a = window[0]["createdAt"].as_str().unwrap();
                let b = window[1]["createdAt"].as_str().unwrap();
                assert!(a >= b, "List not in descending order: {a} < {b}");
            }

            for id in &created_ids {
                cleanup_solicitud(&db, *id).await;
            }
        });
    }

    pub fn p3(prioridad_a: String, prioridad_b: String) {
        with_db(|db| async move {
            let config = make_config();
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
            let propiedad_a = create_test_propiedad(&db).await;
            let propiedad_b = create_test_propiedad(&db).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::post()
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "propiedadId": propiedad_a, "titulo": "Filter A", "prioridad": prioridad_a }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let b1: Value = test::read_body_json(resp).await;
            let id1: Uuid = b1["id"].as_str().unwrap().parse().unwrap();

            let req = test::TestRequest::post()
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "propiedadId": propiedad_b, "titulo": "Filter B", "prioridad": prioridad_b }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let b2: Value = test::read_body_json(resp).await;
            let id2: Uuid = b2["id"].as_str().unwrap().parse().unwrap();

            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/mantenimiento?prioridad={prioridad_a}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let body: Value = test::read_body_json(resp).await;
            for item in body["data"].as_array().unwrap() {
                assert_eq!(item["prioridad"], prioridad_a);
            }

            let req = test::TestRequest::get()
                .uri("/api/v1/mantenimiento?estado=pendiente")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let body: Value = test::read_body_json(resp).await;
            for item in body["data"].as_array().unwrap() {
                assert_eq!(item["estado"], "pendiente");
            }

            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/mantenimiento?propiedadId={propiedad_a}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let body: Value = test::read_body_json(resp).await;
            for item in body["data"].as_array().unwrap() {
                assert_eq!(item["propiedadId"], propiedad_a.to_string());
            }

            cleanup_solicitud(&db, id1).await;
            cleanup_solicitud(&db, id2).await;
        });
    }

    pub fn p4(ot: String, op: String, nt: String, np: String, ut: bool, up: bool) {
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
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "propiedadId": propiedad_id, "titulo": ot, "prioridad": op }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let created: Value = test::read_body_json(resp).await;
            let solicitud_id = created["id"].as_str().unwrap().to_string();

            let mut update_body = json!({});
            if ut {
                update_body["titulo"] = json!(nt);
            }
            if up {
                update_body["prioridad"] = json!(np);
            }

            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/mantenimiento/{solicitud_id}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(&update_body)
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let updated: Value = test::read_body_json(resp).await;

            if ut {
                assert_eq!(updated["titulo"], nt);
            } else {
                assert_eq!(updated["titulo"], ot);
            }
            if up {
                assert_eq!(updated["prioridad"], np);
            } else {
                assert_eq!(updated["prioridad"], op);
            }
            assert_eq!(updated["estado"], "pendiente");

            cleanup_solicitud(&db, solicitud_id.parse().unwrap()).await;
        });
    }

    pub fn p5(titulo: String) {
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
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "propiedadId": propiedad_id, "titulo": titulo }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let body: Value = test::read_body_json(resp).await;
            let sid = body["id"].as_str().unwrap().to_string();
            assert!(body["fechaInicio"].is_null());
            assert!(body["fechaFin"].is_null());

            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/mantenimiento/{sid}/estado"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "estado": "en_progreso" }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            assert!(!body["fechaInicio"].is_null());
            assert!(body["fechaFin"].is_null());

            let req = test::TestRequest::put()
                .uri(&format!("/api/v1/mantenimiento/{sid}/estado"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "estado": "completado" }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            assert!(!body["fechaInicio"].is_null());
            assert!(!body["fechaFin"].is_null());

            cleanup_solicitud(&db, sid.parse().unwrap()).await;
        });
    }

    pub fn p7_prioridad(bad: String) {
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
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "propiedadId": propiedad_id, "titulo": "Test invalid prioridad", "prioridad": bad }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    pub fn p7_moneda(bad: String) {
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
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "propiedadId": propiedad_id, "titulo": "Test invalid moneda", "costoMonto": "100.00", "costoMoneda": bad }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    pub fn p8(neg: Decimal) {
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
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "propiedadId": propiedad_id, "titulo": "Test negative cost", "costoMonto": neg.to_string(), "costoMoneda": "DOP" }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    pub fn p9(ws: String) {
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
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "propiedadId": propiedad_id, "titulo": "Test empty notes" }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let body: Value = test::read_body_json(resp).await;
            let sid = body["id"].as_str().unwrap().to_string();

            let req = test::TestRequest::post()
                .uri(&format!("/api/v1/mantenimiento/{sid}/notas"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "contenido": ws }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);

            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/mantenimiento/{sid}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let detail: Value = test::read_body_json(resp).await;
            assert_eq!(detail["notas"].as_array().unwrap().len(), 0);

            cleanup_solicitud(&db, sid.parse().unwrap()).await;
        });
    }

    pub fn p10(note_count: u32) {
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
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "propiedadId": propiedad_id, "titulo": "Notes ordering test" }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let body: Value = test::read_body_json(resp).await;
            let sid = body["id"].as_str().unwrap().to_string();

            for i in 0..note_count {
                let req = test::TestRequest::post()
                    .uri(&format!("/api/v1/mantenimiento/{sid}/notas"))
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(json!({ "contenido": format!("Nota {}", i) }))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), 201);
            }

            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/mantenimiento/{sid}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let detail: Value = test::read_body_json(resp).await;
            let notas = detail["notas"].as_array().unwrap();
            assert_eq!(notas.len(), note_count as usize);
            for window in notas.windows(2) {
                let a = window[0]["createdAt"].as_str().unwrap();
                let b = window[1]["createdAt"].as_str().unwrap();
                assert!(a >= b, "Notes not in descending order: {a} < {b}");
            }

            cleanup_solicitud(&db, sid.parse().unwrap()).await;
        });
    }

    pub fn p11(note_count: u32) {
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
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "propiedadId": propiedad_id, "titulo": "Cascade delete test" }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            let body: Value = test::read_body_json(resp).await;
            let sid = body["id"].as_str().unwrap().to_string();

            for i in 0..note_count {
                let req = test::TestRequest::post()
                    .uri(&format!("/api/v1/mantenimiento/{sid}/notas"))
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(json!({ "contenido": format!("Nota cascade {}", i) }))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), 201);
            }

            let req = test::TestRequest::delete()
                .uri(&format!("/api/v1/mantenimiento/{sid}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 204);

            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/mantenimiento/{sid}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404);

            use realestate_backend::entities::nota_mantenimiento;
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
            let sol_uuid: Uuid = sid.parse().unwrap();
            let remaining = nota_mantenimiento::Entity::find()
                .filter(nota_mantenimiento::Column::SolicitudId.eq(sol_uuid))
                .all(&db)
                .await
                .unwrap();
            assert_eq!(remaining.len(), 0);
        });
    }

    pub fn p12(titulo: String) {
        with_db(|db| async move {
            let config = make_config();
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
            let propiedad_a = create_test_propiedad(&db).await;
            let propiedad_b = create_test_propiedad(&db).await;
            let unidad_on_b = create_test_unidad(&db, propiedad_b).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::post()
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "propiedadId": propiedad_a, "titulo": titulo, "unidadId": unidad_on_b }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
            let body: Value = test::read_body_json(resp).await;
            assert!(body["message"].as_str().unwrap().contains("no pertenece"));
        });
    }

    pub fn p13(bytes_a: [u8; 16], bytes_b: [u8; 16]) {
        let fake_propiedad_id = Uuid::from_bytes(bytes_a);
        let fake_inquilino_id = Uuid::from_bytes(bytes_b);
        with_db(|db| async move {
            let config = make_config();
            let admin_id = create_test_usuario(&db, "admin").await;
            let token = make_token(admin_id, "admin");
            let real_propiedad_id = create_test_propiedad(&db).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            let req = test::TestRequest::post()
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(
                    json!({ "propiedadId": fake_propiedad_id, "titulo": "FK test propiedad" }),
                )
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404);

            let req = test::TestRequest::post()
                .uri("/api/v1/mantenimiento")
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({ "propiedadId": real_propiedad_id, "titulo": "FK test inquilino", "inquilinoId": fake_inquilino_id }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404);
        });
    }
} // end mod pbt_async

// ── Property test functions ──

// Feature: mantenimiento, Property 1: Creation round-trip preserves data
// **Validates: Requirements 1.1, 2.5, 6.4**
#[test]
#[ignore]
fn test_creation_round_trip() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    let strategy = (
        valid_titulo(),
        valid_descripcion(),
        valid_prioridad(),
        non_negative_decimal(),
        valid_moneda(),
    );
    runner
        .run(
            &strategy,
            |(titulo, descripcion, prioridad, monto, moneda)| {
                pbt_async::p1(titulo, descripcion, prioridad, monto, moneda);
                Ok(())
            },
        )
        .unwrap();
}

// Feature: mantenimiento, Property 2: List ordering invariant
// **Validates: Requirements 2.1**
#[test]
#[ignore]
fn test_list_ordering() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    runner
        .run(&(2u32..5u32), |count| {
            pbt_async::p2(count);
            Ok(())
        })
        .unwrap();
}

// Feature: mantenimiento, Property 3: Filtering returns only matching records
// **Validates: Requirements 2.2, 2.3, 2.4**
#[test]
#[ignore]
fn test_filtering_returns_matching() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    runner
        .run(&(valid_prioridad(), valid_prioridad()), |(pa, pb)| {
            pbt_async::p3(pa, pb);
            Ok(())
        })
        .unwrap();
}

// Feature: mantenimiento, Property 4: Update replaces provided fields and preserves others
// **Validates: Requirements 3.1, 5.1, 5.2**
#[test]
#[ignore]
fn test_update_preserves_and_replaces() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    let strategy = (
        valid_titulo(),
        valid_prioridad(),
        valid_titulo(),
        valid_prioridad(),
        proptest::bool::ANY,
        proptest::bool::ANY,
    );
    runner
        .run(&strategy, |(ot, op, nt, np, ut, up)| {
            pbt_async::p4(ot, op, nt, np, ut, up);
            Ok(())
        })
        .unwrap();
}

// Feature: mantenimiento, Property 5: Valid state transitions set timestamps
// **Validates: Requirements 4.1, 4.2**
#[test]
#[ignore]
fn test_valid_transitions_set_timestamps() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    runner
        .run(&valid_titulo(), |titulo| {
            pbt_async::p5(titulo);
            Ok(())
        })
        .unwrap();
}

// Feature: mantenimiento, Property 6: Invalid state transitions are rejected
// **Validates: Requirements 4.3, 4.4**
#[test]
fn test_invalid_transitions_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    runner
        .run(
            &invalid_transition_pair(),
            |(estado_actual, nuevo_estado)| {
                let result = validar_transicion(&estado_actual, &nuevo_estado);
                assert!(
                    result.is_err(),
                    "Transition {estado_actual}→{nuevo_estado} should be rejected"
                );
                Ok(())
            },
        )
        .unwrap();
}

// Feature: mantenimiento, Property 7: Invalid enum values are rejected
// **Validates: Requirements 1.4, 3.3, 6.2**
#[test]
#[ignore]
fn test_invalid_prioridad_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    runner
        .run(&invalid_prioridad(), |bad| {
            pbt_async::p7_prioridad(bad);
            Ok(())
        })
        .unwrap();
}

#[test]
#[ignore]
fn test_invalid_moneda_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    runner
        .run(&invalid_moneda(), |bad| {
            pbt_async::p7_moneda(bad);
            Ok(())
        })
        .unwrap();
}

// Feature: mantenimiento, Property 8: Negative cost amounts are rejected
// **Validates: Requirements 6.3**
#[test]
#[ignore]
fn test_negative_cost_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    runner
        .run(&negative_decimal(), |neg| {
            pbt_async::p8(neg);
            Ok(())
        })
        .unwrap();
}

// Feature: mantenimiento, Property 9: Empty or whitespace-only notes are rejected
// **Validates: Requirements 7.2**
#[test]
#[ignore]
fn test_empty_notes_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    runner
        .run(&whitespace_string(), |ws| {
            pbt_async::p9(ws);
            Ok(())
        })
        .unwrap();
}

// Feature: mantenimiento, Property 10: Notes ordering invariant
// **Validates: Requirements 7.3**
#[test]
#[ignore]
fn test_notes_ordering() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    runner
        .run(&(2u32..5u32), |count| {
            pbt_async::p10(count);
            Ok(())
        })
        .unwrap();
}

// Feature: mantenimiento, Property 11: Cascade delete removes solicitud and all notes
// **Validates: Requirements 8.1**
#[test]
#[ignore]
fn test_cascade_delete() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    runner
        .run(&(1u32..4u32), |count| {
            pbt_async::p11(count);
            Ok(())
        })
        .unwrap();
}

// Feature: mantenimiento, Property 12: Unit-property ownership validation
// **Validates: Requirements 9.4**
#[test]
#[ignore]
fn test_unit_property_ownership() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    runner
        .run(&valid_titulo(), |titulo| {
            pbt_async::p12(titulo);
            Ok(())
        })
        .unwrap();
}

// Feature: mantenimiento, Property 13: Non-existent FK references are rejected
// **Validates: Requirements 1.2, 9.5**
#[test]
#[ignore]
fn test_nonexistent_fk_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 15,
        ..Default::default()
    });
    let strategy = (
        proptest::array::uniform16(0u8..),
        proptest::array::uniform16(0u8..),
    );
    runner
        .run(&strategy, |(a, b)| {
            pbt_async::p13(a, b);
            Ok(())
        })
        .unwrap();
}
