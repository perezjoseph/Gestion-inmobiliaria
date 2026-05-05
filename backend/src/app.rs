use actix_cors::Cors;
use actix_web::HttpResponse;
use actix_web::error::ResponseError;
use actix_web::http::header;
use actix_web::web;
use actix_web::web::JsonConfig;
use sea_orm::DatabaseConnection;
use tracing_actix_web::TracingLogger;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::routes;
use crate::services::ocr_preview::PreviewStore;

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

async fn serve_upload(
    _claims: crate::services::auth::Claims,
    path: web::Path<String>,
) -> Result<actix_files::NamedFile, AppError> {
    let upload_dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    let requested_path = path.into_inner();

    // Reject obvious traversal attempts before filesystem access
    if requested_path.contains("..") {
        return Err(AppError::Forbidden);
    }

    let full_path = format!("{upload_dir}/{requested_path}");
    let canonical_dir = std::fs::canonicalize(&upload_dir)
        .map_err(|_| AppError::NotFound("Directorio no encontrado".to_string()))?;
    let canonical_file = std::fs::canonicalize(&full_path)
        .map_err(|_| AppError::NotFound("Archivo no encontrado".to_string()))?;

    if !canonical_file.starts_with(&canonical_dir) {
        return Err(AppError::Forbidden);
    }

    actix_files::NamedFile::open_async(&canonical_file)
        .await
        .map_err(|_| AppError::NotFound("Archivo no encontrado".to_string()))
}

fn build_cors(config: &AppConfig) -> Cors {
    config.cors_origin.as_deref().map_or_else(
        || {
            tracing::error!(
                "CORS_ORIGIN no configurado — usando política permisiva. No usar en producción."
            );
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

#[allow(clippy::literal_string_with_formatting_args)]
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

    actix_web::App::new()
        .wrap(crate::middleware::security_headers::SecurityHeaders)
        .wrap(TracingLogger::default())
        .wrap(cors)
        .app_data(web::Data::new(db))
        .app_data(web::Data::new(config))
        .app_data(preview_store)
        .app_data(json_cfg)
        .route("/health", web::get().to(health))
        .configure(routes::configure)
        .route("/uploads/{path:.*}", web::get().to(serve_upload))
}
