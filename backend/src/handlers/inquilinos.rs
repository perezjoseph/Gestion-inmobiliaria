use actix_web::{HttpResponse, web};
use sea_orm::{DatabaseConnection, TransactionTrait};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::{AdminOnly, WriteAccess};
use crate::models::inquilino::InquilinoSearchQuery;
use crate::models::inquilino::{CreateInquilinoRequest, UpdateInquilinoRequest};
use crate::services::auth::Claims;
use crate::services::inquilinos;

pub async fn list(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    query: web::Query<InquilinoSearchQuery>,
) -> Result<HttpResponse, AppError> {
    let q = query.into_inner();
    let term = q.busqueda.or(q.search).filter(|t| t.len() >= 2);
    let result = inquilinos::list(db.get_ref(), term, q.page, q.per_page).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_by_id(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = inquilinos::get_by_id(db.get_ref(), id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn create(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    body: web::Json<CreateInquilinoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let txn = db.begin().await?;
    let result = inquilinos::create(&txn, body.into_inner(), usuario_id).await?;
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
    let id = path.into_inner();
    let txn = db.begin().await?;
    let result = inquilinos::update(&txn, id, body.into_inner(), usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn delete(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = admin.0.sub;
    let id = path.into_inner();
    let txn = db.begin().await?;
    inquilinos::delete(&txn, id, usuario_id).await?;
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
