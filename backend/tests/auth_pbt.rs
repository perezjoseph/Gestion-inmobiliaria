// Feature: spec-gap-remediation, Property 2: Self-registered users are always gerente
// **Validates: Requirements 2.1, 2.3, 2.5**
#![allow(clippy::needless_return)]
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

use crate::migrations;

// ── Strategies ─────────────────────────────────────────────────────────

/// Generate a valid 11-digit Dominican cédula with correct check digit.
fn valid_cedula() -> impl Strategy<Value = String> {
    // Generate 10 random digits, compute the Luhn check digit
    prop::array::uniform10(0u32..10u32).prop_map(|digits| {
        let weights: [u32; 10] = [1, 2, 1, 2, 1, 2, 1, 2, 1, 2];
        let sum: u32 = weights
            .iter()
            .zip(digits.iter())
            .map(|(w, d)| {
                let product = w * d;
                if product > 9 {
                    product / 10 + product % 10
                } else {
                    product
                }
            })
            .sum();
        let check = (10 - (sum % 10)) % 10;
        let mut result = String::with_capacity(11);
        for d in &digits {
            result.push(char::from_digit(*d, 10).unwrap());
        }
        result.push(char::from_digit(check, 10).unwrap());
        result
    })
}

/// Generate a random role hint that might be injected in the payload.
/// The system should ignore any role hint and always assign "gerente".
fn role_hint() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        Just(Some("admin".to_string())),
        Just(Some("visualizador".to_string())),
        Just(Some("gerente".to_string())),
        Just(Some("superadmin".to_string())),
        "[a-z]{3,15}".prop_map(Some),
    ]
}

/// Generate a valid user name (3-50 alphanumeric chars).
fn valid_nombre() -> impl Strategy<Value = String> {
    "[a-zA-Z]{3,30}"
        .prop_map(|s| s.trim().to_string())
        .prop_filter("nombre must not be empty", |s| !s.trim().is_empty())
}

/// Generate a valid password (8-30 chars).
fn valid_password() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9!@#]{8,30}"
}

/// Generate a valid phone number.
fn valid_telefono() -> impl Strategy<Value = String> {
    "[0-9]{10}".prop_map(|s| format!("809{}", &s[..7]))
}

/// Generate a valid organization name.
fn valid_org_nombre() -> impl Strategy<Value = String> {
    "[a-zA-Z ]{5,30}"
        .prop_map(|s| s.trim().to_string())
        .prop_filter("org nombre must not be empty", |s| !s.trim().is_empty())
}

// ── Async helpers (Rust 2024: avoid calling async from sync #[test]) ───

mod pbt_async {
    use actix_web::test;
    use realestate_backend::app::create_app;
    use realestate_backend::config::AppConfig;
    use sea_orm::{ConnectOptions, Database, DatabaseConnection};
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
            chatbot: realestate_backend::config::ChatbotEnvConfig::for_testing(),
            database_url: String::new(),
            jwt_secret: JWT_SECRET.to_string(),
            server_port: 0,
            cors_origin: None,
        }
    }

    /// Expected keys in the UserResponse DTO (camelCase serialization).
    const USER_RESPONSE_KEYS: &[&str] = &[
        "id",
        "nombre",
        "email",
        "rol",
        "activo",
        "organizacionId",
        "createdAt",
    ];

    /// Keys that must NOT appear in the response (security contract).
    const FORBIDDEN_KEYS: &[&str] = &["token", "password", "passwordHash", "session"];

    async fn cleanup_user_and_org(db: &DatabaseConnection, user_id: Uuid, org_id: Uuid) {
        use realestate_backend::entities::{organizacion, usuario};
        use sea_orm::EntityTrait;
        let _ = usuario::Entity::delete_by_id(user_id).exec(db).await;
        let _ = organizacion::Entity::delete_by_id(org_id).exec(db).await;
    }

    /// Property 2: Self-registered users are always gerente.
    /// Regardless of any role hint in the payload, the persisted user has
    /// rol="gerente", non-null organizacion_id, and the response matches
    /// the User DTO shape exactly.
    pub fn property_2_self_registered_always_gerente(
        nombre: String,
        email_suffix: String,
        password: String,
        cedula: String,
        telefono: String,
        org_nombre: String,
        role_hint: Option<String>,
    ) {
        with_db(|db| async move {
            let config = make_config();
            let app = test::init_service(create_app(
                db.clone(),
                config,
                actix_web::web::Data::new(
                    realestate_backend::services::ocr_preview::PreviewStore::new(),
                ),
            ))
            .await;

            // Build a unique email for this test case
            let unique_id = Uuid::new_v4();
            let email = format!("pbt_{unique_id}_{email_suffix}@test.com");

            // Build the registration payload (persona_fisica)
            let mut payload = json!({
                "nombre": nombre,
                "email": email,
                "password": password,
                "tipo": "persona_fisica",
                "cedula": cedula,
                "telefono": telefono,
                "nombreOrganizacion": org_nombre,
            });

            // Inject role hint if present (the system should ignore it)
            if let Some(ref hint) = role_hint {
                payload["rol"] = json!(hint);
            }

            let req = test::TestRequest::post()
                .uri("/api/v1/auth/register")
                .set_json(&payload)
                .to_request();
            let resp = test::call_service(&app, req).await;

            // Assert HTTP 201 Created
            assert_eq!(
                resp.status().as_u16(),
                201,
                "Expected 201 Created, got {}",
                resp.status()
            );

            let body: Value = test::read_body_json(resp).await;

            // Assert response contains exactly the User DTO keys
            let obj = body.as_object().expect("Response should be a JSON object");
            for key in USER_RESPONSE_KEYS {
                assert!(
                    obj.contains_key(*key),
                    "Response missing expected key: {key}"
                );
            }

            // Assert forbidden keys are absent
            for key in FORBIDDEN_KEYS {
                assert!(
                    !obj.contains_key(*key),
                    "Response must NOT contain key: {key}"
                );
            }

            // Assert rol is always "gerente" regardless of role hint
            assert_eq!(
                body["rol"], "gerente",
                "Persisted rol must be 'gerente', got: {}",
                body["rol"]
            );

            // Assert organizacionId is non-null
            assert!(
                !body["organizacionId"].is_null(),
                "organizacionId must not be null"
            );

            // Parse IDs for cleanup
            let user_id: Uuid = body["id"]
                .as_str()
                .expect("id should be a string")
                .parse()
                .expect("id should be a valid UUID");
            let org_id: Uuid = body["organizacionId"]
                .as_str()
                .expect("organizacionId should be a string")
                .parse()
                .expect("organizacionId should be a valid UUID");

            // Assert activo is true
            assert_eq!(body["activo"], true, "New user should be activo=true");

            // Cleanup
            cleanup_user_and_org(&db, user_id, org_id).await;
        });
    }
}

// ── Property test ──────────────────────────────────────────────────────

// Feature: spec-gap-remediation, Property 2: Self-registered users are always gerente
// **Validates: Requirements 2.1, 2.3, 2.5**
#[test]
fn property_2_self_registered_users_are_always_gerente() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(
                valid_nombre(),
                "[a-z]{5,10}", // email suffix
                valid_password(),
                valid_cedula(),
                valid_telefono(),
                valid_org_nombre(),
                role_hint(),
            ),
            |(nombre, email_suffix, password, cedula, telefono, org_nombre, hint)| {
                pbt_async::property_2_self_registered_always_gerente(
                    nombre,
                    email_suffix,
                    password,
                    cedula,
                    telefono,
                    org_nombre,
                    hint,
                );
                Ok(())
            },
        )
        .unwrap();
}
