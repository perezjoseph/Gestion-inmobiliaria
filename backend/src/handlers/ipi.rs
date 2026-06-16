use actix_web::{HttpResponse, web};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::ipi::ConfiguracionIpiRequest;
use crate::services::ipi;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CrearCopropietarioRequest {
    pub propiedad_id: Uuid,
    pub nombre: String,
    pub cedula_rnc: String,
    pub porcentaje_propiedad: Decimal,
}

pub async fn calcular_ipi_handler(
    db: web::Data<DatabaseConnection>,
    user: WriteAccess,
) -> Result<HttpResponse, AppError> {
    let org_id = user.0.organizacion_id;
    let result = ipi::calcular_ipi(db.get_ref(), org_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn actualizar_umbral(
    db: web::Data<DatabaseConnection>,
    user: WriteAccess,
    body: web::Json<ConfiguracionIpiRequest>,
) -> Result<HttpResponse, AppError> {
    let org_id = user.0.organizacion_id;
    let result = ipi::actualizar_umbral(db.get_ref(), org_id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn listar_copropietarios(
    db: web::Data<DatabaseConnection>,
    user: WriteAccess,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let org_id = user.0.organizacion_id;
    let propiedad_id = path.into_inner();
    let result = ipi::obtener_copropietarios(db.get_ref(), propiedad_id, org_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn crear_copropietario(
    db: web::Data<DatabaseConnection>,
    user: WriteAccess,
    body: web::Json<CrearCopropietarioRequest>,
) -> Result<HttpResponse, AppError> {
    let org_id = user.0.organizacion_id;
    let req = body.into_inner();
    let result = ipi::crear_copropietario(
        db.get_ref(),
        org_id,
        req.propiedad_id,
        req.nombre,
        req.cedula_rnc,
        req.porcentaje_propiedad,
    )
    .await?;
    Ok(HttpResponse::Created().json(result))
}
