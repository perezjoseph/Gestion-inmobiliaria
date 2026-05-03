#![allow(clippy::needless_return)]
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use rust_decimal::Decimal;
use uuid::Uuid;

use realestate_backend::services::unidades::ESTADOS_UNIDAD;
use realestate_backend::services::validation::MONEDAS;

use crate::migrations;

fn valid_numero_unidad() -> impl Strategy<Value = String> {
    "[A-Z]{1,3}-[0-9]{1,4}"
}

fn valid_estado() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("disponible".to_string()),
        Just("ocupada".to_string()),
        Just("mantenimiento".to_string()),
    ]
}

fn valid_moneda() -> impl Strategy<Value = String> {
    prop_oneof![Just("DOP".to_string()), Just("USD".to_string())]
}

fn non_negative_precio() -> impl Strategy<Value = Decimal> {
    (0i64..10_000_000i64).prop_map(|v| Decimal::new(v, 2))
}

fn negative_precio() -> impl Strategy<Value = Decimal> {
    (-10_000_000i64..-1i64).prop_map(|v| Decimal::new(v, 2))
}

fn optional_i32() -> impl Strategy<Value = Option<i32>> {
    prop_oneof![Just(None), (1i32..100i32).prop_map(Some),]
}

fn optional_area() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        (100i64..100_000i64).prop_map(|v| Some(Decimal::new(v, 2).to_string())),
    ]
}

fn optional_descripcion() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "[a-zA-Z0-9 ]{5,50}".prop_map(Some),]
}

fn invalid_estado() -> impl Strategy<Value = String> {
    "[a-zA-Z]{1,20}".prop_filter("must not be a valid estado", |s| {
        !ESTADOS_UNIDAD.contains(&s.as_str())
    })
}

fn invalid_moneda() -> impl Strategy<Value = String> {
    "[A-Z]{1,5}".prop_filter("must not be a valid moneda", |s| {
        !MONEDAS.contains(&s.as_str())
    })
}

// All async test logic lives in a separate module so that calling async fns
// from sync #[test] functions (Rust 2024 edition restriction) is avoided.
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
            nombre: Set(format!("Org PBT {id}")),
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
            titulo: Set("Propiedad PBT Unidades".to_string()),
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
        use sea_orm::EntityTrait;
        let _ = unidad::Entity::delete_by_id(id).exec(db).await;
    }

    // P1: Creation round-trip preserves data
    #[allow(clippy::too_many_arguments)]
    pub fn p1_round_trip(
        numero_unidad: String,
        piso: Option<i32>,
        habitaciones: Option<i32>,
        banos: Option<i32>,
        area_m2: Option<String>,
        precio: Decimal,
        moneda: Option<String>,
        estado: Option<String>,
        descripcion: Option<String>,
    ) {
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

            let mut body = json!({
                "numeroUnidad": numero_unidad,
                "precio": precio.to_string(),
            });
            if let Some(p) = piso {
                body["piso"] = json!(p);
            }
            if let Some(h) = habitaciones {
                body["habitaciones"] = json!(h);
            }
            if let Some(b) = banos {
                body["banos"] = json!(b);
            }
            if let Some(ref a) = area_m2 {
                body["areaM2"] = json!(a);
            }
            if let Some(ref m) = moneda {
                body["moneda"] = json!(m);
            }
            if let Some(ref e) = estado {
                body["estado"] = json!(e);
            }
            if let Some(ref d) = descripcion {
                body["descripcion"] = json!(d);
            }

            let req = test::TestRequest::post()
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(&body)
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let created: Value = test::read_body_json(resp).await;
            let unidad_id = created["id"].as_str().unwrap();

            // Retrieve by ID
            let req = test::TestRequest::get()
                .uri(&format!("{}/{unidad_id}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let detail: Value = test::read_body_json(resp).await;

            // Verify fields match
            assert_eq!(detail["numeroUnidad"], numero_unidad);
            let expected_estado = estado.as_deref().unwrap_or("disponible");
            assert_eq!(detail["estado"], expected_estado);
            let expected_moneda = moneda.as_deref().unwrap_or("DOP");
            assert_eq!(detail["moneda"], expected_moneda);

            if let Some(p) = piso {
                assert_eq!(detail["piso"], p);
            } else {
                assert!(detail["piso"].is_null());
            }
            if let Some(h) = habitaciones {
                assert_eq!(detail["habitaciones"], h);
            } else {
                assert!(detail["habitaciones"].is_null());
            }
            if let Some(b) = banos {
                assert_eq!(detail["banos"], b);
            } else {
                assert!(detail["banos"].is_null());
            }
            if let Some(ref d) = descripcion {
                assert_eq!(detail["descripcion"], *d);
            } else {
                assert!(detail["descripcion"].is_null());
            }

            cleanup_unidad(&db, unidad_id.parse().unwrap()).await;
        });
    }

    // P2: Numero_unidad uniqueness within propiedad
    pub fn p2_uniqueness(numero_unidad: String) {
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

            // Create first
            let req = test::TestRequest::post()
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": numero_unidad, "precio": "10000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let b: Value = test::read_body_json(resp).await;
            let id1: Uuid = b["id"].as_str().unwrap().parse().unwrap();

            // Create second with same numero_unidad -> 409
            let req = test::TestRequest::post()
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": numero_unidad, "precio": "20000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 409);

            cleanup_unidad(&db, id1).await;
        });
    }

    // P3: List ordering invariant
    pub fn p3_ordering(nums: Vec<String>) {
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
            for num in &nums {
                let req = test::TestRequest::post()
                    .uri(&base_uri(propiedad_id))
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(json!({"numeroUnidad": num, "precio": "10000"}))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), 201);
                let b: Value = test::read_body_json(resp).await;
                ids.push(b["id"].as_str().unwrap().parse::<Uuid>().unwrap());
            }

            // List and verify ordering
            let req = test::TestRequest::get()
                .uri(&format!("{}?perPage=100", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            let data = body["data"].as_array().unwrap();

            for window in data.windows(2) {
                let a = window[0]["numeroUnidad"].as_str().unwrap();
                let b = window[1]["numeroUnidad"].as_str().unwrap();
                assert!(a <= b, "List not in ascending order: {a} > {b}");
            }

            for id in &ids {
                cleanup_unidad(&db, *id).await;
            }
        });
    }

    // P4: Filtering returns only matching records
    pub fn p4_filtering(estados: Vec<String>) {
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
            for (i, estado) in estados.iter().enumerate() {
                let num = format!("FLT-{i}-{}", Uuid::new_v4().as_simple());
                let req = test::TestRequest::post()
                    .uri(&base_uri(propiedad_id))
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(json!({"numeroUnidad": num, "precio": "10000", "estado": estado}))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), 201);
                let b: Value = test::read_body_json(resp).await;
                ids.push(b["id"].as_str().unwrap().parse::<Uuid>().unwrap());
            }

            // Filter by each estado and verify all returned records match
            for filter_estado in &["disponible", "ocupada", "mantenimiento"] {
                let req = test::TestRequest::get()
                    .uri(&format!(
                        "{}?estado={}&perPage=100",
                        base_uri(propiedad_id),
                        filter_estado
                    ))
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), 200);
                let body: Value = test::read_body_json(resp).await;
                for item in body["data"].as_array().unwrap() {
                    assert_eq!(
                        item["estado"].as_str().unwrap(),
                        *filter_estado,
                        "Filtered result has wrong estado"
                    );
                }
            }

            for id in &ids {
                cleanup_unidad(&db, *id).await;
            }
        });
    }

    // P5: Update replaces provided fields and preserves others
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub fn p5_update_preserves(
        update_numero: bool,
        new_numero: String,
        update_estado: bool,
        new_estado: String,
        update_precio: bool,
        new_precio: Decimal,
        update_moneda: bool,
        new_moneda: String,
    ) {
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

            // Create initial unidad
            let req = test::TestRequest::post()
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({
                    "numeroUnidad": format!("ORIG-{}", Uuid::new_v4().as_simple()),
                    "precio": "15000.00",
                    "moneda": "DOP",
                    "estado": "disponible",
                    "descripcion": "Original desc"
                }))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let created: Value = test::read_body_json(resp).await;
            let unidad_id = created["id"].as_str().unwrap().to_string();
            let orig_numero = created["numeroUnidad"].as_str().unwrap().to_string();
            let orig_estado = created["estado"].as_str().unwrap().to_string();
            let orig_moneda = created["moneda"].as_str().unwrap().to_string();
            let orig_descripcion = created["descripcion"].as_str().unwrap().to_string();

            // Build partial update
            let mut update_body = json!({});
            if update_numero {
                update_body["numeroUnidad"] = json!(new_numero);
            }
            if update_estado {
                update_body["estado"] = json!(new_estado);
            }
            if update_precio {
                update_body["precio"] = json!(new_precio.to_string());
            }
            if update_moneda {
                update_body["moneda"] = json!(new_moneda);
            }

            let req = test::TestRequest::put()
                .uri(&format!("{}/{unidad_id}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(&update_body)
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let updated: Value = test::read_body_json(resp).await;

            // Verify updated fields changed
            if update_numero {
                assert_eq!(updated["numeroUnidad"], new_numero);
            } else {
                assert_eq!(updated["numeroUnidad"], orig_numero);
            }
            if update_estado {
                assert_eq!(updated["estado"], new_estado);
            } else {
                assert_eq!(updated["estado"], orig_estado);
            }
            if update_moneda {
                assert_eq!(updated["moneda"], new_moneda);
            } else {
                assert_eq!(updated["moneda"], orig_moneda);
            }
            // Non-updated fields preserved
            assert_eq!(updated["descripcion"], orig_descripcion);

            cleanup_unidad(&db, unidad_id.parse().unwrap()).await;
        });
    }

    // P6: Update preserves numero_unidad uniqueness
    pub fn p6_update_uniqueness(num_a: String, num_b: String) {
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
                .set_json(json!({"numeroUnidad": num_a, "precio": "10000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let b1: Value = test::read_body_json(resp).await;
            let id1: Uuid = b1["id"].as_str().unwrap().parse().unwrap();

            // Create second unidad
            let req = test::TestRequest::post()
                .uri(&base_uri(propiedad_id))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": num_b, "precio": "10000"}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 201);
            let b2: Value = test::read_body_json(resp).await;
            let id2: Uuid = b2["id"].as_str().unwrap().parse().unwrap();

            // Update second's numero_unidad to first's -> 409
            let req = test::TestRequest::put()
                .uri(&format!("{}/{id2}", base_uri(propiedad_id)))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .set_json(json!({"numeroUnidad": num_a}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 409);

            cleanup_unidad(&db, id1).await;
            cleanup_unidad(&db, id2).await;
        });
    }

    // P7: Invalid enum values are rejected
    pub fn p7_invalid_estado(bad_estado: String) {
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
                .set_json(json!({"numeroUnidad": "INV-E", "precio": "10000", "estado": bad_estado}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    pub fn p7_invalid_moneda(bad_moneda: String) {
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
                .set_json(json!({"numeroUnidad": "INV-M", "precio": "10000", "moneda": bad_moneda}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    // P8: Negative prices are rejected
    pub fn p8_negative_price(neg_precio: Decimal) {
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
                .set_json(json!({"numeroUnidad": "NEG-P", "precio": neg_precio.to_string()}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 422);
        });
    }

    // P9: Non-existent propiedad references are rejected
    pub fn p9_nonexistent_propiedad(fake_prop: Uuid) {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let admin_id = create_test_usuario(&db, "admin", org_id).await;
            let token = make_token(admin_id, "admin", org_id);
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

            let req = test::TestRequest::get()
                .uri(&base_uri(fake_prop))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404);
        });
    }

    // P10: Occupancy counts are consistent
    pub fn p10_occupancy(estados: Vec<String>) {
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
            for (i, estado) in estados.iter().enumerate() {
                let num = format!("OCC-{i}-{}", Uuid::new_v4().as_simple());
                let req = test::TestRequest::post()
                    .uri(&base_uri(propiedad_id))
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(json!({"numeroUnidad": num, "precio": "10000", "estado": estado}))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), 201);
                let b: Value = test::read_body_json(resp).await;
                ids.push(b["id"].as_str().unwrap().parse::<Uuid>().unwrap());
            }

            let expected_total = estados.len() as u64;
            let expected_occupied = estados.iter().filter(|e| *e == "ocupada").count() as u64;
            let expected_rate = if expected_total == 0 {
                0.0
            } else {
                (expected_occupied as f64 / expected_total as f64) * 100.0
            };

            // Check propiedad detail for occupancy
            let req = test::TestRequest::get()
                .uri(&format!("/api/v1/propiedades/{propiedad_id}"))
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let detail: Value = test::read_body_json(resp).await;

            let total = detail["totalUnidades"].as_u64().unwrap();
            let occupied = detail["unidadesOcupadas"].as_u64().unwrap();
            assert_eq!(total, expected_total, "total_unidades mismatch");
            assert_eq!(occupied, expected_occupied, "unidades_ocupadas mismatch");

            let rate = detail["tasaOcupacion"].as_f64().unwrap();
            assert!(
                (rate - expected_rate).abs() < 0.01,
                "tasa_ocupacion mismatch: expected {expected_rate}, got {rate}"
            );

            for id in &ids {
                cleanup_unidad(&db, *id).await;
            }
        });
    }
} // end pbt_async

// Feature: unidades-management, Property 1: Creation round-trip preserves data
// **Validates: Requirements 1.1, 2.3**
#[test]
fn test_creation_round_trip() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let strategy = (
        valid_numero_unidad(),
        optional_i32(),
        optional_i32(),
        optional_i32(),
        optional_area(),
        non_negative_precio(),
        prop_oneof![Just(None), valid_moneda().prop_map(Some)],
        prop_oneof![Just(None), valid_estado().prop_map(Some)],
        optional_descripcion(),
    );

    runner
        .run(
            &strategy,
            |(
                numero_unidad,
                piso,
                habitaciones,
                banos,
                area_m2,
                precio,
                moneda,
                estado,
                descripcion,
            )| {
                pbt_async::p1_round_trip(
                    numero_unidad,
                    piso,
                    habitaciones,
                    banos,
                    area_m2,
                    precio,
                    moneda,
                    estado,
                    descripcion,
                );
                Ok(())
            },
        )
        .unwrap();
}

// Feature: unidades-management, Property 2: Numero_unidad uniqueness within propiedad
// **Validates: Requirements 1.3, 5.1, 5.2**
#[test]
fn test_numero_unidad_uniqueness() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner
        .run(&valid_numero_unidad(), |numero_unidad| {
            pbt_async::p2_uniqueness(numero_unidad);
            Ok(())
        })
        .unwrap();
}

// Feature: unidades-management, Property 3: List ordering invariant
// **Validates: Requirements 2.1**
#[test]
fn test_list_ordering() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    // Generate 2-5 unique numero_unidad values
    let strategy = proptest::collection::hash_set(valid_numero_unidad(), 2..=5)
        .prop_map(|s| s.into_iter().collect::<Vec<_>>());

    runner
        .run(&strategy, |nums| {
            pbt_async::p3_ordering(nums);
            Ok(())
        })
        .unwrap();
}

// Feature: unidades-management, Property 4: Filtering returns only matching records
// **Validates: Requirements 2.2**
#[test]
fn test_filtering_returns_matching() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let strategy = proptest::collection::vec(valid_estado(), 2..=5);

    runner
        .run(&strategy, |estados| {
            pbt_async::p4_filtering(estados);
            Ok(())
        })
        .unwrap();
}

// Feature: unidades-management, Property 5: Update replaces provided fields and preserves others
// **Validates: Requirements 3.1**
#[test]
fn test_update_preserves_and_replaces() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let strategy = (
        proptest::bool::ANY,
        valid_numero_unidad(),
        proptest::bool::ANY,
        valid_estado(),
        proptest::bool::ANY,
        non_negative_precio(),
        proptest::bool::ANY,
        valid_moneda(),
    );

    runner
        .run(
            &strategy,
            |(
                update_numero,
                new_numero,
                update_estado,
                new_estado,
                update_precio,
                new_precio,
                update_moneda,
                new_moneda,
            )| {
                pbt_async::p5_update_preserves(
                    update_numero,
                    new_numero,
                    update_estado,
                    new_estado,
                    update_precio,
                    new_precio,
                    update_moneda,
                    new_moneda,
                );
                Ok(())
            },
        )
        .unwrap();
}

// Feature: unidades-management, Property 6: Update preserves numero_unidad uniqueness
// **Validates: Requirements 3.3, 5.1**
#[test]
fn test_update_uniqueness() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    // Generate two distinct numero_unidad values
    let strategy = (valid_numero_unidad(), valid_numero_unidad())
        .prop_filter("must be distinct", |(a, b)| a != b);

    runner
        .run(&strategy, |(num_a, num_b)| {
            pbt_async::p6_update_uniqueness(num_a, num_b);
            Ok(())
        })
        .unwrap();
}

// Feature: unidades-management, Property 7: Invalid enum values are rejected
// **Validates: Requirements 1.5, 1.6, 3.4, 3.5**
#[test]
fn test_invalid_enums_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner
        .run(&invalid_estado(), |bad_estado| {
            pbt_async::p7_invalid_estado(bad_estado);
            Ok(())
        })
        .unwrap();

    let mut runner2 = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner2
        .run(&invalid_moneda(), |bad_moneda| {
            pbt_async::p7_invalid_moneda(bad_moneda);
            Ok(())
        })
        .unwrap();
}

// Feature: unidades-management, Property 8: Negative prices are rejected
// **Validates: Requirements 1.7, 3.6**
#[test]
fn test_negative_price_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner
        .run(&negative_precio(), |neg| {
            pbt_async::p8_negative_price(neg);
            Ok(())
        })
        .unwrap();
}

// Feature: unidades-management, Property 9: Non-existent propiedad references are rejected
// **Validates: Requirements 1.2, 2.5**
#[test]
fn test_nonexistent_propiedad_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let uuid_strategy = (proptest::bits::u128::ANY,).prop_map(|(bits,)| Uuid::from_u128(bits));

    runner
        .run(&uuid_strategy, |fake_prop| {
            pbt_async::p9_nonexistent_propiedad(fake_prop);
            Ok(())
        })
        .unwrap();
}

// Feature: unidades-management, Property 10: Occupancy counts are consistent
// **Validates: Requirements 8.1, 8.2, 8.3**
#[test]
fn test_occupancy_counts_consistent() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let strategy = proptest::collection::vec(valid_estado(), 1..=6);

    runner
        .run(&strategy, |estados| {
            pbt_async::p10_occupancy(estados);
            Ok(())
        })
        .unwrap();
}
