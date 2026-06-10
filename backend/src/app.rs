use std::sync::Arc;

use actix_cors::Cors;
use actix_web::HttpResponse;
use actix_web::error::ResponseError;
use actix_web::http::header;
use actix_web::middleware::Compress;
use actix_web::web;
use actix_web::web::JsonConfig;
use actix_web_prom::PrometheusMetricsBuilder;
use sea_orm::DatabaseConnection;
use tracing_actix_web::TracingLogger;

use crate::config::{AppConfig, SmtpConfig};
use crate::errors::AppError;
use crate::routes;
use crate::services::baileys_client::BaileysClient;
use crate::services::dashboard_cache::DashboardCache;
use crate::services::login_lockout::LoginLockout;
use crate::services::mail::{MailClient, OutgoingMail, SmtpMailClient};
use crate::services::ocr_preview::PreviewStore;
use crate::services::user_security_cache::UserSecurityCache;

async fn health(db: web::Data<DatabaseConnection>) -> HttpResponse {
    use sea_orm::ConnectionTrait;
    let db_ok = db.execute_unprepared("SELECT 1").await.is_ok();
    if db_ok {
        HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
    } else {
        HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({"status": "degraded", "db": "unreachable"}))
    }
}

async fn metrics_handler(_claims: crate::middleware::rbac::AdminOnly) -> HttpResponse {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::default_registry().gather();
    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .unwrap_or_default();
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(buffer)
}

/// Internal metrics endpoint for Prometheus scraping.
/// If `METRICS_TOKEN` is set, requires `Authorization: Bearer <token>`.
/// Protected by `NetworkPolicy` — only monitoring namespace can reach it.
#[allow(clippy::future_not_send)]
async fn internal_metrics(
    config: web::Data<AppConfig>,
    req: actix_web::HttpRequest,
) -> HttpResponse {
    if let Some(ref expected_token) = config.metrics_token {
        let is_valid = req
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .is_some_and(|t| t == expected_token.as_str());
        if !is_valid {
            return HttpResponse::Unauthorized().finish();
        }
    }

    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::default_registry().gather();
    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .unwrap_or_default();
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(buffer)
}

async fn serve_upload(
    _claims: crate::services::auth::Claims,
    path: web::Path<String>,
) -> Result<actix_files::NamedFile, AppError> {
    let upload_dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    let requested_path = path.into_inner();

    // Reject obvious traversal attempts before filesystem access
    if requested_path.contains("..") {
        return Err(AppError::Forbidden("Acceso denegado".to_string()));
    }

    let full_path = format!("{upload_dir}/{requested_path}");
    let upload_dir_clone = upload_dir.clone();
    let full_path_clone = full_path.clone();

    // Perform canonicalization on a blocking thread
    let canonical_file =
        tokio::task::spawn_blocking(move || -> Result<std::path::PathBuf, AppError> {
            let canonical_dir = std::fs::canonicalize(&upload_dir_clone)
                .map_err(|_| AppError::NotFound("Directorio no encontrado".to_string()))?;
            let canonical_file = std::fs::canonicalize(&full_path_clone)
                .map_err(|_| AppError::NotFound("Archivo no encontrado".to_string()))?;

            if !canonical_file.starts_with(&canonical_dir) {
                return Err(AppError::Forbidden("Acceso denegado".to_string()));
            }

            Ok(canonical_file)
        })
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error en tarea: {e}")))??;

    actix_files::NamedFile::open_async(&canonical_file)
        .await
        .map(|f| {
            f.set_content_disposition(actix_web::http::header::ContentDisposition {
                disposition: actix_web::http::header::DispositionType::Attachment,
                parameters: vec![],
            })
        })
        .map_err(|_| AppError::NotFound("Archivo no encontrado".to_string()))
}

fn build_cors(config: &AppConfig) -> Cors {
    config.cors_origin.as_deref().map_or_else(
        || {
            tracing::warn!(
                "CORS_ORIGIN no configurado — usando CORS permisivo (solo desarrollo). \
                 Configure CORS_ORIGIN para restringir orígenes en producción."
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

    #[allow(clippy::panic)] // Fatal: app cannot start without metrics middleware
    let prometheus = PrometheusMetricsBuilder::new("api")
        .build()
        .unwrap_or_else(|e| panic!("Error inicializando métricas Prometheus: {e}"));

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

    // Limit multipart uploads to 20 MB total
    let multipart_cfg = actix_multipart::form::MultipartFormConfig::default()
        .total_limit(20 * 1024 * 1024)
        .memory_limit(2 * 1024 * 1024);

    // Create login lockout tracker and spawn background cleanup task
    let lockout = web::Data::new(LoginLockout::new());
    {
        let lockout_clone = lockout.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5 * 60));
            loop {
                interval.tick().await;
                lockout_clone.cleanup();
            }
        });
    }

    // Construct the mail client from SMTP env vars (graceful fallback if not configured)
    let mail_client: Arc<dyn MailClient> = match SmtpConfig::from_env() {
        Ok(smtp_cfg) => match SmtpMailClient::from_config(&smtp_cfg) {
            Ok(client) => {
                tracing::info!("Cliente SMTP configurado correctamente");
                Arc::new(client)
            }
            Err(e) => {
                tracing::warn!(error = %e, "Error creando cliente SMTP — correos deshabilitados");
                Arc::new(NoopMailClient)
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "SMTP no configurado — correos deshabilitados");
            Arc::new(NoopMailClient)
        }
    };

    // Construct BaileysClient from chatbot config (always available since chatbot is required)
    let baileys_client = match BaileysClient::new(&config.chatbot) {
        Ok(client) => {
            tracing::info!("BaileysClient configurado correctamente");
            Some(web::Data::new(client))
        }
        Err(e) => {
            tracing::warn!(error = %e, "Error creando BaileysClient — endpoints de WhatsApp deshabilitados");
            None
        }
    };

    let mut app = actix_web::App::new()
        .wrap(Compress::default())
        .wrap(prometheus)
        .wrap(crate::middleware::security_headers::SecurityHeaders)
        .wrap(TracingLogger::default())
        .wrap(cors)
        .app_data(web::Data::new(db))
        .app_data(web::Data::new(config))
        .app_data(preview_store)
        .app_data(lockout)
        .app_data(web::Data::new(UserSecurityCache::new()))
        .app_data(web::Data::new(DashboardCache::new()))
        .app_data(web::Data::new(mail_client))
        .app_data(json_cfg)
        .app_data(multipart_cfg)
        .route("/health", web::get().to(health))
        .route("/metrics", web::get().to(metrics_handler))
        .route("/internal/metrics", web::get().to(internal_metrics))
        .configure(routes::configure)
        .route("/uploads/{path:.*}", web::get().to(serve_upload));

    if let Some(baileys) = baileys_client {
        app = app.app_data(baileys);
    }

    app
}

/// No-op mail client used when SMTP is not configured.
/// Logs the email instead of sending it.
struct NoopMailClient;

impl MailClient for NoopMailClient {
    fn send(
        &self,
        msg: OutgoingMail,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), AppError>> + Send + '_>>
    {
        Box::pin(async move {
            tracing::warn!(
                to = %msg.to,
                subject = %msg.subject,
                "Correo no enviado (SMTP no configurado)"
            );
            Ok(())
        })
    }
}
