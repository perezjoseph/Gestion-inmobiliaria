use realestate_backend::app;
use realestate_backend::config::AppConfig;

#[path = "../migrations/mod.rs"]
pub mod migrations;

use sea_orm::Database;
use sea_orm_migration::MigratorTrait;
use tracing_subscriber::EnvFilter;

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

    tracing::info!("Servidor iniciando en 0.0.0.0:{}", port);

    actix_web::HttpServer::new(move || app::create_app(db.clone(), config.clone()))
        .bind(("0.0.0.0", port))?
        .run()
        .await
}
