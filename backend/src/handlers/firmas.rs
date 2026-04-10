use actix_web::{HttpRequest, HttpResponse, web};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::firma::{
    FirmarConTokenRequest, FirmarRequest, SolicitarFirmaRequest, VerificarTokenRequest,
};
use crate::services::auth::Claims;
use crate::services::firmas;

/// Extract client IP from X-Forwarded-For header, falling back to peer address.
fn extract_ip(req: &HttpRequest) -> String {
    req.headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map_or_else(
            || {
                req.peer_addr()
                    .map_or_else(|| "unknown".to_string(), |addr| addr.ip().to_string())
            },
            |s| s.trim().to_string(),
        )
}

/// Extract User-Agent from request headers.
fn extract_user_agent(req: &HttpRequest) -> String {
    req.headers()
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}

/// POST `/documentos/{id}/firmar` — Authenticated manager signing.
#[allow(clippy::future_not_send)]
pub async fn firmar(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<FirmarRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let documento_id = path.into_inner();
    let ip_address = extract_ip(&req);
    let user_agent = extract_user_agent(&req);
    let claims = &access.0;

    let result = firmas::firmar_autenticado(
        db.get_ref(),
        documento_id,
        &claims.email,
        &claims.rol,
        &body.firma_imagen,
        ip_address,
        user_agent,
    )
    .await?;

    Ok(HttpResponse::Ok().json(result))
}

/// POST `/documentos/{id}/solicitar-firma` — Request tenant signature.
pub async fn solicitar_firma(
    db: web::Data<DatabaseConnection>,
    _access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<SolicitarFirmaRequest>,
) -> Result<HttpResponse, AppError> {
    let documento_id = path.into_inner();

    let result = firmas::solicitar_firma(db.get_ref(), documento_id, &body).await?;

    Ok(HttpResponse::Created().json(result))
}

/// GET `/documentos/{id}/firmas` — List all signatures for a document.
pub async fn listar_firmas(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let documento_id = path.into_inner();

    let result = firmas::listar_firmas(db.get_ref(), documento_id).await?;

    Ok(HttpResponse::Ok().json(result))
}

/// POST `/firmas/{token}/verificar` — Public token verification (no auth).
pub async fn verificar_firma_publica(
    db: web::Data<DatabaseConnection>,
    path: web::Path<String>,
    body: web::Json<VerificarTokenRequest>,
) -> Result<HttpResponse, AppError> {
    let token = path.into_inner();

    let result = firmas::verificar_token(db.get_ref(), &token, &body.password).await?;

    Ok(HttpResponse::Ok().json(result))
}

/// POST `/firmas/{token}/firmar` — Public tenant signing (no auth).
#[allow(clippy::future_not_send)]
pub async fn firmar_publica(
    db: web::Data<DatabaseConnection>,
    path: web::Path<String>,
    body: web::Json<FirmarConTokenRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let token = path.into_inner();
    let ip_address = extract_ip(&req);
    let user_agent = extract_user_agent(&req);

    let result = firmas::firmar_con_token(
        db.get_ref(),
        &token,
        &body.password,
        &body.firma_imagen,
        ip_address,
        user_agent,
    )
    .await?;

    Ok(HttpResponse::Ok().json(result))
}
