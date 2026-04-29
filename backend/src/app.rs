use actix_cors::Cors;
use actix_files::Files;
use actix_web::error::ResponseError;
use actix_web::http::header;
use actix_web::web;
use actix_web::web::JsonConfig;
use actix_web::HttpResponse;
use sea_orm::DatabaseConnection;
use tracing_actix_web::TracingLogger;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::routes;
use crate::services::ocr_preview::PreviewStore;

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

fn build_cors(config: &AppConfig) -> Cors {
    config.cors_origin.as_deref().map_or_else(
        || {
            tracing::warn!("CORS_ORIGIN no configurado — usando política permisiva. No usar en producción.");
            Cors::permissive()
        },
        |origin| {
            Cors::default()
                .allowed_origin(origin)
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
                .max_age(3600)
        },
    )
}

pub fn create_app(
    db: DatabaseConnection,
    config: AppConfig,
    preview_store: web::Data<PreviewStore>,
) -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let cors = build_cors(&config);

    let json_cfg = JsonConfig::default()
        .limit(1_048_576) // 1 MB
        .error_handler(|err, _req| {
            let message = err.to_string();
            actix_web::error::InternalError::from_response(
                err,
                AppError::Validation(message).error_response(),
            )
            .into()
        });

    let upload_dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());

    actix_web::App::new()
        .wrap(TracingLogger::default())
        .wrap(cors)
        .app_data(web::Data::new(db))
        .app_data(web::Data::new(config))
        .app_data(preview_store)
        .app_data(json_cfg)
        .route("/health", web::get().to(health))
        .configure(routes::configure)
        .service(Files::new("/uploads", &upload_dir))
}
