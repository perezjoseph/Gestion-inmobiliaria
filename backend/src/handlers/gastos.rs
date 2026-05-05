use actix_web::{HttpResponse, web};
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, TransactionTrait};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::gasto::{
    CreateGastoRequest, GastoListQuery, ResumenCategoriasQuery, UpdateGastoRequest,
};
use crate::services::auth::Claims;
use crate::services::gastos;

const VALID_MONEDAS: &[&str] = &["DOP", "USD"];
const VALID_ESTADOS: &[&str] = &["pendiente", "pagado", "cancelado"];

fn validate_create_gasto(dto: &CreateGastoRequest) -> Result<(), AppError> {
    if dto.monto <= Decimal::ZERO {
        return Err(AppError::Validation(
            "El monto debe ser un valor positivo".into(),
        ));
    }
    if dto.categoria.trim().is_empty() || dto.categoria.len() > 100 {
        return Err(AppError::Validation(
            "La categoría no puede estar vacía ni exceder 100 caracteres".into(),
        ));
    }
    if !VALID_MONEDAS.contains(&dto.moneda.as_str()) {
        return Err(AppError::Validation(format!(
            "Moneda inválida. Valores permitidos: {}",
            VALID_MONEDAS.join(", ")
        )));
    }
    Ok(())
}

fn validate_update_gasto(dto: &UpdateGastoRequest) -> Result<(), AppError> {
    if let Some(ref monto) = dto.monto {
        if monto <= &Decimal::ZERO {
            return Err(AppError::Validation(
                "El monto debe ser un valor positivo".into(),
            ));
        }
    }
    if let Some(ref categoria) = dto.categoria {
        if categoria.trim().is_empty() || categoria.len() > 100 {
            return Err(AppError::Validation(
                "La categoría no puede estar vacía ni exceder 100 caracteres".into(),
            ));
        }
    }
    if let Some(ref moneda) = dto.moneda {
        if !VALID_MONEDAS.contains(&moneda.as_str()) {
            return Err(AppError::Validation(format!(
                "Moneda inválida. Valores permitidos: {}",
                VALID_MONEDAS.join(", ")
            )));
        }
    }
    if let Some(ref estado) = dto.estado {
        if !VALID_ESTADOS.contains(&estado.as_str()) {
            return Err(AppError::Validation(format!(
                "Estado inválido. Valores permitidos: {}",
                VALID_ESTADOS.join(", ")
            )));
        }
    }
    Ok(())
}

pub async fn create(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    body: web::Json<CreateGastoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let organizacion_id = access.0.organizacion_id;
    let dto = body.into_inner();
    validate_create_gasto(&dto)?;
    let txn = db.begin().await?;
    let result = gastos::create(&txn, dto, usuario_id, organizacion_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn list(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<GastoListQuery>,
) -> Result<HttpResponse, AppError> {
    let result = gastos::list(db.get_ref(), claims.organizacion_id, query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_by_id(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = gastos::get_by_id(db.get_ref(), claims.organizacion_id, id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdateGastoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let id = path.into_inner();
    let dto = body.into_inner();
    validate_update_gasto(&dto)?;
    let txn = db.begin().await?;
    let result = gastos::update(&txn, org_id, id, dto, usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn delete(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let id = path.into_inner();
    let txn = db.begin().await?;
    gastos::delete(&txn, org_id, id, usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn resumen_categorias(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<ResumenCategoriasQuery>,
) -> Result<HttpResponse, AppError> {
    let result =
        gastos::resumen_categorias(db.get_ref(), claims.organizacion_id, query.into_inner())
            .await?;
    Ok(HttpResponse::Ok().json(result))
}
