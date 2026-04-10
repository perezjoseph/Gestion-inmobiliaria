use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::middleware::rbac::AdminOnly;
use crate::services::configuracion::{
    self, RecargoDefectoResponse, UpdateMonedaRequest, UpdateRecargoDefectoRequest,
};

pub async fn obtener_moneda(
    db: web::Data<DatabaseConnection>,
    _claims: crate::services::auth::Claims,
) -> Result<HttpResponse, AppError> {
    let result = configuracion::obtener_moneda(db.get_ref()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn actualizar_moneda(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    body: web::Json<UpdateMonedaRequest>,
) -> Result<HttpResponse, AppError> {
    let result = configuracion::actualizar_moneda(db.get_ref(), body.tasa, admin.0.sub).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn obtener_recargo_defecto(
    db: web::Data<DatabaseConnection>,
    _claims: crate::services::auth::Claims,
) -> Result<HttpResponse, AppError> {
    let result = configuracion::obtener_recargo_defecto(db.get_ref()).await?;
    let response = RecargoDefectoResponse { porcentaje: result };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn actualizar_recargo_defecto(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    body: web::Json<UpdateRecargoDefectoRequest>,
) -> Result<HttpResponse, AppError> {
    let result =
        configuracion::actualizar_recargo_defecto(db.get_ref(), body.porcentaje, admin.0.sub)
            .await?;
    Ok(HttpResponse::Ok().json(result))
}
