use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::services::recibos_informales;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrearReciboInformalRequest {
    pub pago_id: Uuid,
    pub organizacion_id: Uuid,
}

pub async fn crear(
    db: web::Data<DatabaseConnection>,
    _claims: WriteAccess,
    body: web::Json<CrearReciboInformalRequest>,
) -> Result<HttpResponse, AppError> {
    let req = body.into_inner();
    let model =
        recibos_informales::crear_recibo_informal(db.get_ref(), req.pago_id, req.organizacion_id)
            .await?;
    Ok(HttpResponse::Created().json(model))
}
