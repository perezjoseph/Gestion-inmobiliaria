use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::responsabilidad_servicio::UpdateResponsabilidadRequest;
use crate::services::servicios_publicos;

pub async fn obtener_responsabilidades(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, AppError> {
    let org_id = access.0.organizacion_id;
    let (_propiedad_id, unidad_id) = path.into_inner();

    let result =
        servicios_publicos::obtener_responsabilidades(db.get_ref(), org_id, unidad_id).await?;

    Ok(HttpResponse::Ok().json(result))
}

pub async fn actualizar_responsabilidad_unidad(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<UpdateResponsabilidadRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let (_propiedad_id, unidad_id) = path.into_inner();

    let result = servicios_publicos::actualizar_responsabilidad_unidad(
        db.get_ref(),
        org_id,
        unidad_id,
        body.into_inner(),
        usuario_id,
    )
    .await?;

    Ok(HttpResponse::Ok().json(result))
}

pub async fn actualizar_responsabilidad_contrato(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdateResponsabilidadRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let contrato_id = path.into_inner();

    let result = servicios_publicos::actualizar_responsabilidad_contrato(
        db.get_ref(),
        org_id,
        contrato_id,
        body.into_inner(),
        usuario_id,
    )
    .await?;

    Ok(HttpResponse::Ok().json(result))
}
