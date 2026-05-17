//! Shared test scaffolding for integration tests.
//!
//! Hoist any helper or fixture that appears in 3+ test files here and import
//! it from the call sites. See `.kiro/steering/backend.md` "Tests" section for
//! the rules that govern this module.
//!
//! Helpers are kept reachable across the test crate; warnings are silenced
//! because cargo can't see calls in modules that haven't been migrated yet.
#![allow(dead_code)]

use std::sync::OnceLock;
use std::time::Duration;

use realestate_backend::config::{AppConfig, ChatbotEnvConfig, PoolConfig};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

use crate::migrations;

/// Single canonical JWT secret for tests. Must be at least 32 chars to satisfy
/// the production validator. Never define a second test secret elsewhere — JWTs
/// signed with one secret are unverifiable with another.
pub const JWT_SECRET: &str = "test_secret_key_that_is_long_enough_for_jwt";

/// Reads `DATABASE_URL` from the environment (loading `.env` if present).
/// Returns an empty string if unset, so callers can short-circuit instead of
/// failing the test.
pub fn db_url() -> String {
    dotenvy::dotenv().ok();
    std::env::var("DATABASE_URL").unwrap_or_default()
}

/// Connects to the test database, runs migrations, and returns the connection.
/// Returns `Err` if `DATABASE_URL` is unset or unreachable so that `with_db`
/// can skip cleanly instead of panicking.
pub async fn setup_db() -> Result<DatabaseConnection, String> {
    let url = db_url();
    if url.is_empty() {
        return Err("DATABASE_URL not set".into());
    }
    let mut opts = ConnectOptions::new(url);
    opts.max_connections(5)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(60))
        .acquire_timeout(Duration::from_secs(30));
    let db = Database::connect(opts)
        .await
        .map_err(|e| format!("Failed to connect to database: {e}"))?;
    migrations::Migrator::up(&db, None)
        .await
        .map_err(|e| format!("Failed to run migrations: {e}"))?;
    Ok(db)
}

/// Process-wide tokio runtime + DB connection, lazily initialized once.
/// Subsequent test modules reuse the same runtime to avoid the cost of
/// spinning up tokio per-test.
pub fn shared_rt_and_db() -> Option<&'static (tokio::runtime::Runtime, DatabaseConnection)> {
    static SHARED: OnceLock<Result<(tokio::runtime::Runtime, DatabaseConnection), String>> =
        OnceLock::new();
    SHARED
        .get_or_init(|| {
            let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Runtime error: {e}"))?;
            let db = rt.block_on(setup_db())?;
            Ok((rt, db))
        })
        .as_ref()
        .ok()
}

/// Runs an async test body against the shared DB connection, serialized via
/// `crate::GLOBAL_DB_SERIAL` so DB-touching tests never overlap. Skips silently
/// when `DATABASE_URL` is unset, which keeps `cargo test` green on machines
/// without a database.
pub fn with_db<F, Fut>(f: F)
where
    F: FnOnce(DatabaseConnection) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    if db_url().is_empty() {
        eprintln!("DATABASE_URL not set -- skipping DB integration test");
        return;
    }
    let _guard = crate::GLOBAL_DB_SERIAL
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let Some((rt, db)) = shared_rt_and_db() else {
        eprintln!("DB unavailable -- skipping DB integration test");
        return;
    };
    rt.block_on(f(db.clone()));
}

/// Builds a minimal `AppConfig` for integration tests. Use this anywhere a
/// real `AppConfig` is required — pass the test database URL and rely on
/// defaults for the rest.
pub fn test_app_config(database_url: impl Into<String>) -> AppConfig {
    AppConfig {
        database_url: database_url.into(),
        jwt_secret: JWT_SECRET.to_string(),
        server_port: 0,
        cors_origin: None,
        pool: PoolConfig::default(),
        chatbot: ChatbotEnvConfig::for_testing(),
    }
}
