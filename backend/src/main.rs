use actix_web::web;
use realestate_backend::app;
use realestate_backend::config::AppConfig;
use realestate_backend::services::ocr_preview::PreviewStore;
use realestate_backend::telemetry;

#[path = "../migrations/mod.rs"]
pub mod migrations;

use sea_orm::Database;
use sea_orm_migration::MigratorTrait;

#[allow(clippy::expect_used)]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let _otel_guard = telemetry::init_telemetry();

    realestate_backend::metrics::init();

    let config = AppConfig::from_env().expect("Error cargando configuración");

    let db = Database::connect(config.connect_options())
        .await
        .expect("Error conectando a la base de datos");

    {
        use sea_orm::ConnectionTrait;
        const MIGRATION_LOCK_ID: i64 = 0x5245_4D49_4752;
        db.execute_unprepared(&format!("SELECT pg_advisory_lock({MIGRATION_LOCK_ID})"))
            .await
            .expect("Error adquiriendo lock de migraciones");

        migrations::Migrator::up(&db, None)
            .await
            .expect("Error ejecutando migraciones");

        db.execute_unprepared(&format!("SELECT pg_advisory_unlock({MIGRATION_LOCK_ID})"))
            .await
            .expect("Error liberando lock de migraciones");
    }

    let port = config.server_port;

    let preview_store = web::Data::new(PreviewStore::new());
    let cleanup_store = preview_store.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            cleanup_store.cleanup_expired();
        }
    });

    let scheduler_db = db.clone();
    let shutdown_token = tokio_util::sync::CancellationToken::new();
    let scheduler_handles = realestate_backend::services::background_jobs::iniciar_scheduler(
        scheduler_db,
        shutdown_token.clone(),
    );

    tracing::info!("Servidor iniciando en 0.0.0.0:{}", port);

    let server = actix_web::HttpServer::new(move || {
        app::create_app(db.clone(), config.clone(), preview_store.clone())
    })
    .workers(2)
    .client_request_timeout(std::time::Duration::from_secs(60))
    .client_disconnect_timeout(std::time::Duration::from_secs(5))
    .keep_alive(std::time::Duration::from_secs(75))
    .bind(("0.0.0.0", port))?
    .run();

    let server_handle = server.handle();

    let shutdown_handle = tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Error registrando señal de shutdown");
        tracing::info!("Señal de shutdown recibida, cerrando servidor...");

        shutdown_token.cancel();
        server_handle.stop(true).await;
    });

    server.await?;

    for handle in scheduler_handles {
        let _ = handle.await;
    }
    let _ = shutdown_handle.await;

    tracing::info!("Servidor cerrado correctamente");
    Ok(())
}
