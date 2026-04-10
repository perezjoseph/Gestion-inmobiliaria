use actix_web::{HttpResponse, web};
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, TransactionTrait};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::{AdminOnly, WriteAccess};
use crate::models::mantenimiento::{
    CambiarEstadoRequest, CreateNotaRequest, CreateSolicitudRequest, SolicitudListQuery,
    UpdateSolicitudRequest,
};
use crate::services::auth::Claims;
use crate::services::mantenimiento;

const VALID_ESTADOS: &[&str] = &["pendiente", "en_progreso", "completado"];
const VALID_PRIORIDADES: &[&str] = &["baja", "media", "alta", "urgente"];
const VALID_MONEDAS: &[&str] = &["DOP", "USD"];

fn validate_descripcion(descripcion: &str) -> Result<(), AppError> {
    if descripcion.len() > 1000 {
        return Err(AppError::Validation(
            "La descripción no puede exceder 1000 caracteres".into(),
        ));
    }
    Ok(())
}

fn validate_costo(costo: &Decimal) -> Result<(), AppError> {
    if costo <= &Decimal::ZERO {
        return Err(AppError::Validation(
            "El costo debe ser un valor positivo".into(),
        ));
    }
    Ok(())
}

fn validate_create_solicitud(dto: &CreateSolicitudRequest) -> Result<(), AppError> {
    if let Some(ref descripcion) = dto.descripcion {
        validate_descripcion(descripcion)?;
    }
    if let Some(ref prioridad) = dto.prioridad {
        if !VALID_PRIORIDADES.contains(&prioridad.as_str()) {
            return Err(AppError::Validation(format!(
                "Prioridad inválida. Valores permitidos: {}",
                VALID_PRIORIDADES.join(", ")
            )));
        }
    }
    if let Some(ref costo) = dto.costo_monto {
        validate_costo(costo)?;
    }
    if let Some(ref moneda) = dto.costo_moneda {
        if !VALID_MONEDAS.contains(&moneda.as_str()) {
            return Err(AppError::Validation(format!(
                "Moneda inválida. Valores permitidos: {}",
                VALID_MONEDAS.join(", ")
            )));
        }
    }
    Ok(())
}

fn validate_update_solicitud(dto: &UpdateSolicitudRequest) -> Result<(), AppError> {
    if let Some(ref descripcion) = dto.descripcion {
        validate_descripcion(descripcion)?;
    }
    if let Some(ref prioridad) = dto.prioridad {
        if !VALID_PRIORIDADES.contains(&prioridad.as_str()) {
            return Err(AppError::Validation(format!(
                "Prioridad inválida. Valores permitidos: {}",
                VALID_PRIORIDADES.join(", ")
            )));
        }
    }
    if let Some(ref costo) = dto.costo_monto {
        validate_costo(costo)?;
    }
    if let Some(ref moneda) = dto.costo_moneda {
        if !VALID_MONEDAS.contains(&moneda.as_str()) {
            return Err(AppError::Validation(format!(
                "Moneda inválida. Valores permitidos: {}",
                VALID_MONEDAS.join(", ")
            )));
        }
    }
    Ok(())
}

fn validate_cambiar_estado(dto: &CambiarEstadoRequest) -> Result<(), AppError> {
    if !VALID_ESTADOS.contains(&dto.estado.as_str()) {
        return Err(AppError::Validation(format!(
            "Estado inválido. Valores permitidos: {}",
            VALID_ESTADOS.join(", ")
        )));
    }
    Ok(())
}

pub async fn list(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<SolicitudListQuery>,
) -> Result<HttpResponse, AppError> {
    let result =
        mantenimiento::list(db.get_ref(), claims.organizacion_id, query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_by_id(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = mantenimiento::get_by_id(db.get_ref(), claims.organizacion_id, id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn create(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    body: web::Json<CreateSolicitudRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let organizacion_id = access.0.organizacion_id;
    let dto = body.into_inner();
    validate_create_solicitud(&dto)?;
    let txn = db.begin().await?;
    let result = mantenimiento::create(&txn, dto, usuario_id, organizacion_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdateSolicitudRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let id = path.into_inner();
    let dto = body.into_inner();
    validate_update_solicitud(&dto)?;
    let txn = db.begin().await?;
    let result = mantenimiento::update(&txn, org_id, id, dto, usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn cambiar_estado(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<CambiarEstadoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let id = path.into_inner();
    let dto = body.into_inner();
    validate_cambiar_estado(&dto)?;
    let txn = db.begin().await?;
    let result = mantenimiento::cambiar_estado(&txn, org_id, id, &dto.estado, usuario_id).await?;
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
    mantenimiento::delete(&txn, org_id, id, usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn agregar_nota(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<CreateNotaRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let solicitud_id = path.into_inner();
    let txn = db.begin().await?;
    let result = mantenimiento::agregar_nota(
        &txn,
        org_id,
        solicitud_id,
        body.into_inner().contenido,
        usuario_id,
    )
    .await?;
    txn.commit().await?;
    Ok(HttpResponse::Created().json(result))
}
