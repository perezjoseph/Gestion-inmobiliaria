use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::entities::organizacion;
use crate::errors::AppError;
use crate::middleware::rbac::AdminOnly;
use crate::models::fiscal::{ActualizarTipoFiscalRequest, EstadoFiscalResponse, TipoFiscal};
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

    let response = build_estado_fiscal_response(&updated);
    Ok(HttpResponse::Ok().json(response))
}

/// GET /api/v1/organizacion/fiscal/estado
///
/// Returns the current fiscal state of the admin's organization.
pub async fn obtener_estado_fiscal(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
) -> Result<HttpResponse, AppError> {
    use sea_orm::EntityTrait;

    let org_id = admin.0.organizacion_id;

    let org = organizacion::Entity::find_by_id(org_id)
        .one(db.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;

    let response = build_estado_fiscal_response(&org);
    Ok(HttpResponse::Ok().json(response))
}

fn build_estado_fiscal_response(org: &organizacion::Model) -> EstadoFiscalResponse {
    let tipo_fiscal = match org.tipo_fiscal.as_str() {
        "persona_juridica" => TipoFiscal::PersonaJuridica,
        "persona_fisica" => TipoFiscal::PersonaFisica,
        _ => TipoFiscal::Informal,
    };

    EstadoFiscalResponse {
        tipo_fiscal,
        rnc: org.rnc.clone(),
        cedula_rnc: org.cedula.clone(),
        razon_social: org.razon_social.clone(),
        regimen_pagos: org.regimen_pagos.clone(),
        fecha_inicio_operaciones: org.fecha_inicio_operaciones,
        is_ecf_certificado: org.is_ecf_certificado,
    }
}
