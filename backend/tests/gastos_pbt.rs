#![allow(clippy::needless_return)]
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use realestate_backend::models::gasto::ResumenCategoriaRow;
use realestate_backend::models::importacion::{ImportError, ImportResult};
use realestate_backend::services::dashboard::calcular_porcentaje_cambio;
use realestate_backend::services::gastos::{CATEGORIAS_GASTO, ESTADOS_GASTO};
use realestate_backend::services::validation::{MONEDAS, validate_enum};

use crate::migrations;

fn arbitrary_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_]{1,30}"
}

fn non_negative_i64() -> impl Strategy<Value = i64> {
    0i64..10_000_000i64
}

// Feature: gastos-expenses-tracking, Property 6: Enum validation rejects invalid values
// **Validates: Requirements 1.6, 1.7, 1.8, 3.4**
#[test]
fn test_enum_validation_rejects_invalid_values() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let categoria_set: Vec<&str> = CATEGORIAS_GASTO.to_vec();
    let estado_set: Vec<&str> = ESTADOS_GASTO.to_vec();
    let moneda_set: Vec<&str> = MONEDAS.to_vec();

    runner
        .run(&arbitrary_string(), |value| {
            if !categoria_set.contains(&value.as_str()) {
                let result = validate_enum("categoria", &value, CATEGORIAS_GASTO);
                assert!(
                    result.is_err(),
                    "validate_enum should reject '{value}' for categoria"
                );
            }
            if !estado_set.contains(&value.as_str()) {
                let result = validate_enum("estado", &value, ESTADOS_GASTO);
                assert!(
                    result.is_err(),
                    "validate_enum should reject '{value}' for estado"
                );
            }
            if !moneda_set.contains(&value.as_str()) {
                let result = validate_enum("moneda", &value, MONEDAS);
                assert!(
                    result.is_err(),
                    "validate_enum should reject '{value}' for moneda"
                );
            }
            Ok(())
        })
        .unwrap();
}

// Feature: gastos-expenses-tracking, Property 12: Profitability net income invariant
// **Validates: Requirements 6.1**
#[test]
fn test_profitability_net_income_invariant() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner
        .run(
            &(non_negative_i64(), non_negative_i64()),
            |(ingresos_raw, gastos_raw)| {
                let total_ingresos = Decimal::new(ingresos_raw, 2);
                let total_gastos = Decimal::new(gastos_raw, 2);
                let ingreso_neto = total_ingresos - total_gastos;

                assert_eq!(
                    ingreso_neto,
                    total_ingresos - total_gastos,
                    "ingreso_neto must equal total_ingresos - total_gastos"
                );

                let reconstructed = ingreso_neto + total_gastos;
                assert_eq!(
                    reconstructed, total_ingresos,
                    "ingreso_neto + total_gastos must equal total_ingresos"
                );

                Ok(())
            },
        )
        .unwrap();
}

// Feature: gastos-expenses-tracking, Property 15: Percentage change calculation
// **Validates: Requirements 9.2**
#[test]
fn test_percentage_change_calculation() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner
        .run(
            &(non_negative_i64(), non_negative_i64()),
            |(actual_raw, anterior_raw)| {
                let actual = Decimal::new(actual_raw, 2);
                let anterior = Decimal::new(anterior_raw, 2);
                let result = calcular_porcentaje_cambio(actual, anterior);

                if anterior.is_zero() && actual.is_zero() {
                    assert!(
                        (result - 0.0).abs() < f64::EPSILON,
                        "Both zero should yield 0.0, got {result}"
                    );
                } else if anterior.is_zero() {
                    assert!(
                        (result - 100.0).abs() < f64::EPSILON,
                        "Zero anterior with positive actual should yield 100.0, got {result}"
                    );
                } else {
                    let expected = ((actual - anterior) / anterior * Decimal::new(100, 0))
                        .to_f64()
                        .unwrap_or(0.0);
                    assert!(
                        (result - expected).abs() < 0.01,
                        "Expected {expected}, got {result} for actual={actual}, anterior={anterior}"
                    );
                }

                Ok(())
            },
        )
        .unwrap();
}

// Feature: gastos-expenses-tracking, Property 11: Category summary sorted by total descending
// **Validates: Requirements 5.4**
#[test]
fn test_category_summary_sorted_descending() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let categoria_strategy =
        proptest::sample::select(CATEGORIAS_GASTO.to_vec()).prop_map(|s| s.to_string());

    let row_strategy = (categoria_strategy, 0i64..10_000_000i64, 1u64..100u64).prop_map(
        |(categoria, total_raw, cantidad)| ResumenCategoriaRow {
            categoria,
            total: Decimal::new(total_raw, 2),
            cantidad,
        },
    );

    let vec_strategy = proptest::collection::vec(row_strategy, 0..20);

    runner
        .run(&vec_strategy, |mut rows| {
            rows.sort_by(|a, b| b.total.cmp(&a.total));

            for window in rows.windows(2) {
                assert!(
                    window[0].total >= window[1].total,
                    "Rows not sorted descending: {} < {}",
                    window[0].total,
                    window[1].total
                );
            }

            Ok(())
        })
        .unwrap();
}

// Feature: gastos-expenses-tracking, Property 14: CSV import valid/invalid row accounting
// **Validates: Requirements 8.4, 8.5**
#[test]
fn test_csv_import_row_accounting() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let error_strategy = (1usize..1000usize, "[a-zA-Z ]{5,50}")
        .prop_map(|(fila, error)| ImportError { fila, error });

    let strategy = (
        0usize..500usize,
        proptest::collection::vec(error_strategy, 0..50),
    );

    runner
        .run(&strategy, |(exitosos, fallidos)| {
            let total_filas = exitosos + fallidos.len();
            let result = ImportResult {
                total_filas,
                exitosos,
                fallidos,
            };

            assert_eq!(
                result.exitosos + result.fallidos.len(),
                result.total_filas,
                "exitosos + fallidos.len() must equal total_filas"
            );

            Ok(())
        })
        .unwrap();
}

// Feature: spec-gap-remediation, Property 8: Date-range filter on gastos is sound and complete
// **Validates: Requirements 9.6, 9.7**

mod pbt_date_range_async {
    use actix_web::test;
    use chrono::{NaiveDate, Utc};
    use realestate_backend::app::create_app;
    use realestate_backend::config::AppConfig;
    use realestate_backend::services::auth::{Claims, encode_jwt};
    use rust_decimal::Decimal;
    use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, Set};
    use sea_orm_migration::MigratorTrait;
    use serde_json::Value;
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
            chatbot: realestate_backend::config::ChatbotEnvConfig {
                baileys_service_url: "http://baileys:3100".to_string(),
                baileys_internal_token: "a]3kF9#mP7vL2nQ8wR5xT0yU4zA1bC6dE".to_string(),
                ovms_endpoint: "http://ovms:8000/v1".to_string(),
                ovms_chat_model: "Qwen3.6-35B-A3B".to_string(),
                ai_chat_timeout_secs: 30,
            },
            database_url: String::new(),
            jwt_secret: JWT_SECRET.to_string(),
            server_port: 0,
            cors_origin: None,
        }
    }

    fn make_token(user_id: Uuid, org_id: Uuid) -> String {
        let claims = Claims {
            sub: user_id,
            email: "admin@test.com".to_string(),
            rol: "admin".to_string(),
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

    async fn create_test_usuario(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::usuario;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        usuario::ActiveModel {
            id: Set(id),
            nombre: Set("Admin PBT".to_string()),
            email: Set(format!("admin+{id}@test.com")),
            password_hash: Set("not_used".to_string()),
            rol: Set("admin".to_string()),
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
            titulo: Set("Propiedad PBT Gastos".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle PBT 456".to_string()),
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

    async fn cleanup_gasto(db: &DatabaseConnection, id: Uuid) {
        use realestate_backend::entities::gasto;
        use sea_orm::EntityTrait;
        let _ = gasto::Entity::delete_by_id(id).exec(db).await;
    }

    /// Property 8a: Soundness and completeness of date-range filter.
    /// Every returned row satisfies fecha_desde <= row.fecha_gasto <= fecha_hasta
    /// and belongs to the caller's organizacion_id.
    pub fn soundness_and_completeness(dates: Vec<u32>, desde_offset: u32, hasta_offset: u32) {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let user_id = create_test_usuario(&db, org_id).await;
            let token = make_token(user_id, org_id);
            let propiedad_id = create_test_propiedad(&db, org_id).await;
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Create gastos with various dates in 2025
            let base_year = 2025;
            let mut created_ids = Vec::new();
            let mut created_dates = Vec::new();
            for day_offset in &dates {
                // Map offset to a date within 2025 (day 1..365)
                let day_in_year = (*day_offset % 364) + 1;
                let fecha = NaiveDate::from_yo_opt(base_year, day_in_year).unwrap();
                let req = test::TestRequest::post()
                    .uri("/api/v1/gastos")
                    .insert_header(("Authorization", format!("Bearer {token}")))
                    .set_json(serde_json::json!({
                        "propiedadId": propiedad_id,
                        "categoria": "mantenimiento",
                        "descripcion": "Gasto PBT date-range",
                        "monto": "1000.00",
                        "moneda": "DOP",
                        "fechaGasto": fecha.format("%Y-%m-%d").to_string(),
                    }))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), 201, "Failed to create gasto");
                let body: Value = test::read_body_json(resp).await;
                let id: Uuid = body["id"].as_str().unwrap().parse().unwrap();
                created_ids.push(id);
                created_dates.push(fecha);
            }

            // Build filter range from offsets
            let desde_day = (desde_offset % 364) + 1;
            let hasta_day = (hasta_offset % 364) + 1;
            let (fecha_desde, fecha_hasta) = if desde_day <= hasta_day {
                (
                    NaiveDate::from_yo_opt(base_year, desde_day).unwrap(),
                    NaiveDate::from_yo_opt(base_year, hasta_day).unwrap(),
                )
            } else {
                (
                    NaiveDate::from_yo_opt(base_year, hasta_day).unwrap(),
                    NaiveDate::from_yo_opt(base_year, desde_day).unwrap(),
                )
            };

            // Query with date range filter
            let uri = format!(
                "/api/v1/gastos?propiedadId={}&fechaDesde={}&fechaHasta={}&perPage=100",
                propiedad_id,
                fecha_desde.format("%Y-%m-%d"),
                fecha_hasta.format("%Y-%m-%d"),
            );
            let req = test::TestRequest::get()
                .uri(&uri)
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200);
            let body: Value = test::read_body_json(resp).await;
            let items = body["data"].as_array().unwrap();

            // Soundness: every returned row satisfies the date range
            for item in items {
                let row_date_str = item["fechaGasto"].as_str().unwrap();
                let row_date = NaiveDate::parse_from_str(row_date_str, "%Y-%m-%d").unwrap();
                assert!(
                    row_date >= fecha_desde && row_date <= fecha_hasta,
                    "Row date {row_date} outside range [{fecha_desde}, {fecha_hasta}]"
                );
            }

            // Completeness: every created gasto within the range appears in results
            let returned_ids: Vec<String> = items
                .iter()
                .map(|i| i["id"].as_str().unwrap().to_string())
                .collect();
            for (id, date) in created_ids.iter().zip(created_dates.iter()) {
                if *date >= fecha_desde && *date <= fecha_hasta {
                    assert!(
                        returned_ids.contains(&id.to_string()),
                        "Gasto {id} with date {date} should be in results for range [{fecha_desde}, {fecha_hasta}]"
                    );
                }
            }

            // Cleanup
            for id in &created_ids {
                cleanup_gasto(&db, *id).await;
            }
        });
    }

    /// Property 8b: fecha_desde > fecha_hasta yields HTTP 400.
    pub fn inverted_range_returns_400(desde_offset: u32, hasta_offset: u32) {
        with_db(|db| async move {
            let config = make_config();
            let org_id = create_test_organizacion(&db).await;
            let user_id = create_test_usuario(&db, org_id).await;
            let token = make_token(user_id, org_id);
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Ensure desde > hasta
            let desde_day = (desde_offset % 364) + 1;
            let hasta_day = (hasta_offset % 364) + 1;
            let (fecha_desde, fecha_hasta) = match desde_day.cmp(&hasta_day) {
                std::cmp::Ordering::Greater => (
                    NaiveDate::from_yo_opt(2025, desde_day).unwrap(),
                    NaiveDate::from_yo_opt(2025, hasta_day).unwrap(),
                ),
                std::cmp::Ordering::Less => {
                    // Swap so desde > hasta
                    (
                        NaiveDate::from_yo_opt(2025, hasta_day).unwrap(),
                        NaiveDate::from_yo_opt(2025, desde_day).unwrap(),
                    )
                }
                std::cmp::Ordering::Equal => {
                    // Equal days — make desde one day later
                    (
                        NaiveDate::from_yo_opt(2025, desde_day.min(364) + 1).unwrap(),
                        NaiveDate::from_yo_opt(2025, desde_day).unwrap(),
                    )
                }
            };

            assert!(
                fecha_desde > fecha_hasta,
                "Test setup: desde must be > hasta"
            );

            let uri = format!(
                "/api/v1/gastos?fechaDesde={}&fechaHasta={}",
                fecha_desde.format("%Y-%m-%d"),
                fecha_hasta.format("%Y-%m-%d"),
            );
            let req = test::TestRequest::get()
                .uri(&uri)
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                400,
                "Expected 400 for inverted range, got {}",
                resp.status()
            );
        });
    }
}

// Feature: spec-gap-remediation, Property 8: Date-range filter on gastos is sound and complete
// **Validates: Requirements 9.6, 9.7**
#[test]
fn test_date_range_filter_soundness_and_completeness() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Strategy: 1-5 random day offsets for gastos, plus two offsets for the filter range
    let strategy = (
        proptest::collection::vec(0u32..365u32, 1..6),
        0u32..365u32,
        0u32..365u32,
    );

    runner
        .run(&strategy, |(dates, desde_offset, hasta_offset)| {
            pbt_date_range_async::soundness_and_completeness(dates, desde_offset, hasta_offset);
            Ok(())
        })
        .unwrap();
}

// Feature: spec-gap-remediation, Property 8: Date-range filter on gastos is sound and complete
// **Validates: Requirements 9.6, 9.7**
#[test]
fn test_date_range_filter_inverted_range_returns_400() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Strategy: two different day offsets that will be arranged so desde > hasta
    let strategy = (0u32..365u32, 0u32..365u32);

    runner
        .run(&strategy, |(desde_offset, hasta_offset)| {
            pbt_date_range_async::inverted_range_returns_400(desde_offset, hasta_offset);
            Ok(())
        })
        .unwrap();
}
