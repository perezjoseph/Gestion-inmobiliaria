use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::middleware::rbac::AdminOnly;
use crate::models::fiscal::ActualizarTipoFiscalRequest;
use crate::services::fiscal;

/// PUT /api/v1/organizacion/fiscal/tipo-fiscal
///
/// Updates the `tipo_fiscal` for the admin's organization.
/// Validates the identifier (RNC or cédula) before persisting.
pub async fn actualizar_tipo_fiscal(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    body: web::Json<ActualizarTipoFiscalRequest>,
) -> Result<HttpResponse, AppError> {
    let org_id = admin.0.organizacion_id;
    let req = body.into_inner();

    let updated = fiscal::actualizar_tipo_fiscal(
        db.get_ref(),
        org_id,
        req.tipo_fiscal,
        req.identificador.as_deref(),
    )
    .await?;

    let response = fiscal::obtener_estado_fiscal_from_model(&updated);
    Ok(HttpResponse::Ok().json(response))
}

/// GET /api/v1/organizacion/fiscal/estado
///
/// Returns the current fiscal state of the admin's organization.
pub async fn obtener_estado_fiscal(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
) -> Result<HttpResponse, AppError> {
    let org_id = admin.0.organizacion_id;
    let response = fiscal::obtener_estado_fiscal(db.get_ref(), org_id).await?;
    Ok(HttpResponse::Ok().json(response))
}
