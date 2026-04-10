use actix_web::{HttpResponse, web};
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, TransactionTrait};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::{AdminOnly, WriteAccess};
use crate::models::propiedad::{
    CreatePropiedadRequest, PropiedadListQuery, UpdatePropiedadRequest,
};
use crate::services::auth::Claims;
use crate::services::propiedades;

const VALID_ESTADOS: &[&str] = &["disponible", "ocupada", "mantenimiento"];
const VALID_MONEDAS: &[&str] = &["DOP", "USD"];

fn validate_titulo(titulo: &str) -> Result<(), AppError> {
    if titulo.trim().is_empty() {
        return Err(AppError::Validation(
            "El título no puede estar vacío".into(),
        ));
    }
    if titulo.len() > 200 {
        return Err(AppError::Validation(
            "El título no puede exceder 200 caracteres".into(),
        ));
    }
    Ok(())
}

fn validate_direccion(direccion: &str) -> Result<(), AppError> {
    if direccion.trim().is_empty() {
        return Err(AppError::Validation(
            "La dirección no puede estar vacía".into(),
        ));
    }
    if direccion.len() > 500 {
        return Err(AppError::Validation(
            "La dirección no puede exceder 500 caracteres".into(),
        ));
    }
    Ok(())
}

fn validate_precio(precio: &Decimal) -> Result<(), AppError> {
    if precio <= &Decimal::ZERO {
        return Err(AppError::Validation(
            "El precio debe ser un valor positivo".into(),
        ));
    }
    Ok(())
}

fn validate_estado(estado: &str) -> Result<(), AppError> {
    if !VALID_ESTADOS.contains(&estado) {
        return Err(AppError::Validation(format!(
            "Estado inválido. Valores permitidos: {}",
            VALID_ESTADOS.join(", ")
        )));
    }
    Ok(())
}

fn validate_moneda(moneda: &str) -> Result<(), AppError> {
    if !VALID_MONEDAS.contains(&moneda) {
        return Err(AppError::Validation(format!(
            "Moneda inválida. Valores permitidos: {}",
            VALID_MONEDAS.join(", ")
        )));
    }
    Ok(())
}

fn validate_create(dto: &CreatePropiedadRequest) -> Result<(), AppError> {
    validate_titulo(&dto.titulo)?;
    validate_direccion(&dto.direccion)?;
    validate_precio(&dto.precio)?;
    if let Some(ref estado) = dto.estado {
        validate_estado(estado)?;
    }
    if let Some(ref moneda) = dto.moneda {
        validate_moneda(moneda)?;
    }
    Ok(())
}

fn validate_update(dto: &UpdatePropiedadRequest) -> Result<(), AppError> {
    if let Some(ref titulo) = dto.titulo {
        validate_titulo(titulo)?;
    }
    if let Some(ref direccion) = dto.direccion {
        validate_direccion(direccion)?;
    }
    if let Some(ref precio) = dto.precio {
        validate_precio(precio)?;
    }
    if let Some(ref estado) = dto.estado {
        validate_estado(estado)?;
    }
    if let Some(ref moneda) = dto.moneda {
        validate_moneda(moneda)?;
    }
    Ok(())
}

pub async fn list(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<PropiedadListQuery>,
) -> Result<HttpResponse, AppError> {
    let result =
        propiedades::list(db.get_ref(), claims.organizacion_id, query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_by_id(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = propiedades::get_by_id(db.get_ref(), claims.organizacion_id, id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn create(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    body: web::Json<CreatePropiedadRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let dto = body.into_inner();
    validate_create(&dto)?;
    let txn = db.begin().await?;
    let result = propiedades::create(&txn, org_id, dto, usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdatePropiedadRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let id = path.into_inner();
    let dto = body.into_inner();
    validate_update(&dto)?;
    let txn = db.begin().await?;
    let result = propiedades::update(&txn, org_id, id, dto, usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn delete(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = admin.0.sub;
    let org_id = admin.0.organizacion_id;
    let id = path.into_inner();
    let txn = db.begin().await?;
    propiedades::delete(&txn, org_id, id, usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}
