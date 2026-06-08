#![allow(clippy::needless_return)]
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

use crate::migrations;

// ── Strategies ─────────────────────────────────────────────────

/// Generate a non-empty string suitable for nombre/tipo_documento.
fn non_empty_string() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9 _-]{0,29}".prop_map(|s| s.trim().to_string())
}

/// Generate a valid `entity_type` value.
fn valid_entity_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("propiedad".to_string()),
        Just("inquilino".to_string()),
        Just("contrato".to_string()),
        Just("pago".to_string()),
        Just("gasto".to_string()),
    ]
}

/// Generate valid JSON contenido for a template.
fn valid_contenido() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        Just(serde_json::json!({"version": 1, "blocks": []})),
        Just(serde_json::json!({"version": 1, "blocks": [{"type": "paragraph", "text": "Hello"}]})),
        Just(
            serde_json::json!({"version": 1, "blocks": [{"type": "heading", "text": "Title", "level": 1}]})
        ),
        Just(serde_json::json!({"version": 1, "blocks": [
            {"type": "paragraph", "text": "Intro"},
            {"type": "list", "ordered": true, "items": ["Item 1", "Item 2"]}
        ]})),
    ]
}

// ── Async helpers ──────────────────────────────────────────────

mod pbt_async {
    use realestate_backend::models::documento::CrearPlantillaRequest;
    use realestate_backend::services::plantillas;
    use sea_orm::{ConnectOptions, Database, DatabaseConnection};
    use sea_orm_migration::MigratorTrait;

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

    async fn cleanup_plantilla(db: &DatabaseConnection, id: uuid::Uuid) {
        use realestate_backend::entities::plantilla_documento;
        use sea_orm::EntityTrait;
        let _ = plantilla_documento::Entity::delete_by_id(id).exec(db).await;
    }

    /// Property 3: Template CRUD round-trip
    /// Create template with arbitrary valid inputs, read by ID, verify all fields match.
    pub fn p3_template_crud_round_trip(
        nombre: String,
        tipo_documento: String,
        entity_type: String,
        contenido: serde_json::Value,
    ) {
        with_db(|db| async move {
            let org_id = uuid::Uuid::new_v4();
            let input = CrearPlantillaRequest {
                nombre: nombre.clone(),
                tipo_documento: tipo_documento.clone(),
                entity_type: entity_type.clone(),
                contenido: contenido.clone(),
            };

            // Create template via service
            let created = plantillas::crear(&db, org_id, input)
                .await
                .expect("crear should succeed for valid inputs");

            // Read back by ID via service
            let fetched = plantillas::obtener(&db, created.id, org_id)
                .await
                .expect("obtener should find the created template");

            // Verify all fields match
            assert_eq!(fetched.nombre, nombre, "nombre mismatch");
            assert_eq!(
                fetched.tipo_documento, tipo_documento,
                "tipo_documento mismatch"
            );
            assert_eq!(fetched.entity_type, entity_type, "entity_type mismatch");
            assert_eq!(fetched.contenido, contenido, "contenido mismatch");
            assert_eq!(fetched.id, created.id, "id mismatch");

            // Cleanup
            cleanup_plantilla(&db, created.id).await;
        });
    }
}

// Feature: contract-document-signing, Property 3: Template CRUD round-trip
// **Validates: Requirements 2.1, 2.4**
#[test]
fn test_template_crud_round_trip() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(
                non_empty_string(),
                non_empty_string(),
                valid_entity_type(),
                valid_contenido(),
            ),
            |(nombre, tipo_documento, entity_type, contenido)| {
                pbt_async::p3_template_crud_round_trip(
                    nombre,
                    tipo_documento,
                    entity_type,
                    contenido,
                );
                Ok(())
            },
        )
        .unwrap();
}
