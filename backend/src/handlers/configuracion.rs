use actix_web::{HttpResponse, web};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::middleware::rbac::AdminOnly;
use crate::services::configuracion::{
    self, RecargoDefectoResponse, UpdateMonedaRequest, UpdateRecargoDefectoRequest,
};

pub async fn obtener_moneda(
    db: web::Data<DatabaseConnection>,
    claims: crate::services::auth::Claims,
) -> Result<HttpResponse, AppError> {
    let result = configuracion::obtener_moneda(db.get_ref(), claims.organizacion_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn actualizar_moneda(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    body: web::Json<UpdateMonedaRequest>,
) -> Result<HttpResponse, AppError> {
    if body.tasa <= 0.0 {
        return Err(AppError::Validation(
            "La tasa de cambio debe ser un valor positivo".into(),
        ));
    }
    if body.tasa > 1000.0 {
        return Err(AppError::Validation(
            "La tasa de cambio excede el rango permitido".into(),
        ));
    }

    let result = configuracion::actualizar_moneda(
        db.get_ref(),
        body.tasa,
        admin.0.sub,
        admin.0.organizacion_id,
    )
    .await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn obtener_recargo_defecto(
    db: web::Data<DatabaseConnection>,
    claims: crate::services::auth::Claims,
) -> Result<HttpResponse, AppError> {
    let result =
        configuracion::obtener_recargo_defecto(db.get_ref(), claims.organizacion_id).await?;
    let response = RecargoDefectoResponse { porcentaje: result };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn actualizar_recargo_defecto(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    body: web::Json<UpdateRecargoDefectoRequest>,
) -> Result<HttpResponse, AppError> {
    if body.porcentaje < Decimal::ZERO {
        return Err(AppError::Validation(
            "El porcentaje de recargo no puede ser negativo".into(),
        ));
    }
    if body.porcentaje > Decimal::from(100) {
        return Err(AppError::Validation(
            "El porcentaje de recargo no puede exceder 100%".into(),
        ));
    }

    let result = configuracion::actualizar_recargo_defecto(
        db.get_ref(),
        body.porcentaje,
        admin.0.sub,
        admin.0.organizacion_id,
    )
    .await?;
    Ok(HttpResponse::Ok().json(result))
}
