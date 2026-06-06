use actix_web::{HttpResponse, web};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::condominios::{CrearCuotaRequest, UpdateCuotaRequest};
use crate::services::condominios;

pub async fn crear_cuota_handler(
    db: web::Data<sea_orm::DatabaseConnection>,
    user: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<CrearCuotaRequest>,
) -> Result<HttpResponse, AppError> {
    let propiedad_id = path.into_inner();
    let org_id = user.0.organizacion_id;
    let mut input = body.into_inner();
    input.propiedad_id = propiedad_id;
    let result = condominios::crear_cuota(db.get_ref(), input, org_id).await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn actualizar_cuota_handler(
    db: web::Data<sea_orm::DatabaseConnection>,
    user: WriteAccess,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<UpdateCuotaRequest>,
) -> Result<HttpResponse, AppError> {
    let (_propiedad_id, cuota_id) = path.into_inner();
    let org_id = user.0.organizacion_id;
    let input = body.into_inner();
    let result = condominios::actualizar_cuota(db.get_ref(), cuota_id, input, org_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn listar_cuotas_handler(
    db: web::Data<sea_orm::DatabaseConnection>,
    user: WriteAccess,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let propiedad_id = path.into_inner();
    let org_id = user.0.organizacion_id;
    let result = condominios::listar_cuotas(db.get_ref(), propiedad_id, org_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn eliminar_cuota_handler(
    db: web::Data<sea_orm::DatabaseConnection>,
    user: WriteAccess,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, AppError> {
    let (_propiedad_id, cuota_id) = path.into_inner();
    let org_id = user.0.organizacion_id;
    condominios::eliminar_cuota(db.get_ref(), cuota_id, org_id).await?;
    Ok(HttpResponse::NoContent().finish())
}
