use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::desahucio::{
    CreateDesahucioRequest, DesahucioListQuery, UpdateDesahucioRequest,
};
use crate::services::desahucios;

const VALID_ESTADOS: &[&str] = &["iniciado", "en_progreso", "completado"];

fn validate_create(dto: &CreateDesahucioRequest) -> Result<(), AppError> {
    if dto.motivo.trim().is_empty() {
        return Err(AppError::Validation(
            "El motivo no puede estar vacío".into(),
        ));
    }
    if dto.motivo.len() > 1000 {
        return Err(AppError::Validation(
            "El motivo no puede exceder 1000 caracteres".into(),
        ));
    }
    Ok(())
}

fn validate_update(dto: &UpdateDesahucioRequest) -> Result<(), AppError> {
    if let Some(ref estado) = dto.estado {
        if !VALID_ESTADOS.contains(&estado.as_str()) {
            return Err(AppError::Validation(format!(
                "Estado inválido. Valores permitidos: {}",
                VALID_ESTADOS.join(", ")
            )));
        }
    }
    if let Some(ref motivo) = dto.motivo {
        if motivo.trim().is_empty() {
            return Err(AppError::Validation(
                "El motivo no puede estar vacío".into(),
            ));
        }
        if motivo.len() > 1000 {
            return Err(AppError::Validation(
                "El motivo no puede exceder 1000 caracteres".into(),
            ));
        }
    }
    Ok(())
}

pub async fn create(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    body: web::Json<CreateDesahucioRequest>,
) -> Result<HttpResponse, AppError> {
    let dto = body.into_inner();
    validate_create(&dto)?;

    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;

    let result = desahucios::create(db.get_ref(), dto, usuario_id, org_id).await?;

    Ok(HttpResponse::Created().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdateDesahucioRequest>,
) -> Result<HttpResponse, AppError> {
    let dto = body.into_inner();
    validate_update(&dto)?;

    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let id = path.into_inner();

    let result = desahucios::update(db.get_ref(), org_id, id, dto, usuario_id).await?;

    Ok(HttpResponse::Ok().json(result))
}

pub async fn list(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    query: web::Query<DesahucioListQuery>,
) -> Result<HttpResponse, AppError> {
    let org_id = access.0.organizacion_id;

    let result = desahucios::list(db.get_ref(), org_id, query.into_inner()).await?;

    Ok(HttpResponse::Ok().json(result))
}
