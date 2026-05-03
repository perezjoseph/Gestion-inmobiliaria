use actix_web::web;
use realestate_backend::app;
use realestate_backend::config::AppConfig;
use realestate_backend::services::ocr_preview::PreviewStore;

#[path = "../migrations/mod.rs"]
pub mod migrations;

use sea_orm::Database;
use sea_orm_migration::MigratorTrait;
use tracing_subscriber::EnvFilter;

#[allow(clippy::expect_used)]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env().expect("Error cargando configuración");

    let db = Database::connect(config.connect_options())
        .await
        .expect("Error conectando a la base de datos");

    migrations::Migrator::up(&db, None)
        .await
        .expect("Error ejecutando migraciones");

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

    // Iniciar scheduler de tareas de fondo
    let scheduler_db = db.clone();
    realestate_backend::services::background_jobs::iniciar_scheduler(scheduler_db);

    tracing::info!("Servidor iniciando en 0.0.0.0:{}", port);

    actix_web::HttpServer::new(move || {
        app::create_app(db.clone(), config.clone(), preview_store.clone())
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
