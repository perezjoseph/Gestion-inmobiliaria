use actix_web::{HttpResponse, web};
use sea_orm::{DatabaseConnection, TransactionTrait};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::{AdminOnly, WriteAccess};
use crate::models::inquilino::InquilinoSearchQuery;
use crate::models::inquilino::{CreateInquilinoRequest, UpdateInquilinoRequest};
use crate::services::auth::Claims;
use crate::services::inquilinos;

fn validate_nombre(nombre: &str) -> Result<(), AppError> {
    if nombre.trim().is_empty() {
        return Err(AppError::Validation(
            "El nombre no puede estar vacío".into(),
        ));
    }
    if nombre.len() > 100 {
        return Err(AppError::Validation(
            "El nombre no puede exceder 100 caracteres".into(),
        ));
    }
    Ok(())
}

fn validate_apellido(apellido: &str) -> Result<(), AppError> {
    if apellido.trim().is_empty() {
        return Err(AppError::Validation(
            "El apellido no puede estar vacío".into(),
        ));
    }
    if apellido.len() > 100 {
        return Err(AppError::Validation(
            "El apellido no puede exceder 100 caracteres".into(),
        ));
    }
    Ok(())
}

fn validate_cedula(cedula: &str) -> Result<(), AppError> {
    if cedula.trim().is_empty() {
        return Err(AppError::Validation(
            "La cédula no puede estar vacía".into(),
        ));
    }
    if cedula.len() > 20 {
        return Err(AppError::Validation(
            "La cédula no puede exceder 20 caracteres".into(),
        ));
    }
    Ok(())
}

fn validate_email(email: &str) -> Result<(), AppError> {
    if !email.contains('@') || !email.contains('.') {
        return Err(AppError::Validation("Formato de email inválido".into()));
    }
    Ok(())
}

fn validate_telefono(telefono: &str) -> Result<(), AppError> {
    if telefono.len() > 20 {
        return Err(AppError::Validation(
            "El teléfono no puede exceder 20 caracteres".into(),
        ));
    }
    Ok(())
}

fn validate_create_inquilino(dto: &CreateInquilinoRequest) -> Result<(), AppError> {
    validate_nombre(&dto.nombre)?;
    validate_apellido(&dto.apellido)?;
    validate_cedula(&dto.cedula)?;
    if let Some(ref email) = dto.email {
        validate_email(email)?;
    }
    if let Some(ref telefono) = dto.telefono {
        validate_telefono(telefono)?;
    }
    Ok(())
}

fn validate_update_inquilino(dto: &UpdateInquilinoRequest) -> Result<(), AppError> {
    if let Some(ref nombre) = dto.nombre {
        validate_nombre(nombre)?;
    }
    if let Some(ref apellido) = dto.apellido {
        validate_apellido(apellido)?;
    }
    if let Some(ref cedula) = dto.cedula {
        validate_cedula(cedula)?;
    }
    if let Some(ref email) = dto.email {
        validate_email(email)?;
    }
    if let Some(ref telefono) = dto.telefono {
        validate_telefono(telefono)?;
    }
    Ok(())
}

pub async fn list(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<InquilinoSearchQuery>,
) -> Result<HttpResponse, AppError> {
    let q = query.into_inner();
    let term = q.busqueda.or(q.search).filter(|t| t.len() >= 2);
    let result = inquilinos::list(
        db.get_ref(),
        claims.organizacion_id,
        term,
        q.page,
        q.per_page,
    )
    .await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_by_id(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = inquilinos::get_by_id(db.get_ref(), claims.organizacion_id, id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn create(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    body: web::Json<CreateInquilinoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let dto = body.into_inner();
    validate_create_inquilino(&dto)?;
    let txn = db.begin().await?;
    let result = inquilinos::create(&txn, org_id, dto, usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdateInquilinoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let id = path.into_inner();
    let dto = body.into_inner();
    validate_update_inquilino(&dto)?;
    let txn = db.begin().await?;
    let result = inquilinos::update(&txn, org_id, id, dto, usuario_id).await?;
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
    inquilinos::delete(&txn, org_id, id, usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    fn apply_search_filter(busqueda: Option<String>, search: Option<String>) -> Option<String> {
        busqueda.or(search).filter(|t| t.len() >= 2)
    }

    #[test]
    fn search_term_at_least_2_chars_passes() {
        let result = apply_search_filter(Some("ab".to_string()), None);
        assert_eq!(result.as_deref(), Some("ab"));
    }

    #[test]
    fn search_term_single_char_filtered_to_none() {
        let result = apply_search_filter(Some("a".to_string()), None);
        assert!(result.is_none());
    }

    #[test]
    fn search_term_empty_string_filtered_to_none() {
        let result = apply_search_filter(Some(String::new()), None);
        assert!(result.is_none());
    }

    #[test]
    fn busqueda_takes_precedence_over_search() {
        let result = apply_search_filter(Some("García".to_string()), Some("Lopez".to_string()));
        assert_eq!(result.as_deref(), Some("García"));
    }

    #[test]
    fn falls_back_to_search_when_busqueda_is_none() {
        let result = apply_search_filter(None, Some("Lopez".to_string()));
        assert_eq!(result.as_deref(), Some("Lopez"));
    }

    #[test]
    fn both_none_returns_none() {
        let result = apply_search_filter(None, None);
        assert!(result.is_none());
    }

    #[test]
    fn short_fallback_search_filtered_to_none() {
        let result = apply_search_filter(None, Some("x".to_string()));
        assert!(result.is_none());
    }
}
