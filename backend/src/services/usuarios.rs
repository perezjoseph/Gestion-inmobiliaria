use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::usuario;
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::usuario::UsuarioResponse;
use crate::services::validation::validate_enum;

const ROLES: &[&str] = &["admin", "gerente", "visualizador"];

impl From<usuario::Model> for UsuarioResponse {
    fn from(m: usuario::Model) -> Self {
        Self {
            id: m.id,
            nombre: m.nombre,
            email: m.email,
            rol: m.rol,
            activo: m.activo,
            organizacion_id: m.organizacion_id,
            created_at: m.created_at.into(),
        }
    }
}

pub async fn listar(
    db: &DatabaseConnection,
    org_id: Uuid,
    page: u64,
    per_page: u64,
) -> Result<PaginatedResponse<UsuarioResponse>, AppError> {
    let page = page.max(1);
    let per_page = per_page.clamp(1, 100);

    let paginator = usuario::Entity::find()
        .filter(usuario::Column::OrganizacionId.eq(org_id))
        .order_by_desc(usuario::Column::CreatedAt)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records.into_iter().map(UsuarioResponse::from).collect(),
        total,
        page,
        per_page,
    })
}

pub async fn cambiar_rol(
    db: &DatabaseConnection,
    id: Uuid,
    org_id: Uuid,
    nuevo_rol: &str,
) -> Result<UsuarioResponse, AppError> {
    validate_enum("rol", nuevo_rol, ROLES)?;

    let record = usuario::Entity::find_by_id(id)
        .filter(usuario::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Usuario no encontrado".to_string()))?;

    let mut active: usuario::ActiveModel = record.into();
    active.rol = Set(nuevo_rol.to_string());

    let updated = active.update(db).await?;
    Ok(UsuarioResponse::from(updated))
}

pub async fn desactivar(
    db: &DatabaseConnection,
    id: Uuid,
    org_id: Uuid,
) -> Result<UsuarioResponse, AppError> {
    let record = usuario::Entity::find_by_id(id)
        .filter(usuario::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Usuario no encontrado".to_string()))?;

    let mut active: usuario::ActiveModel = record.into();
    active.activo = Set(false);

    let updated = active.update(db).await?;
    Ok(UsuarioResponse::from(updated))
}

pub async fn activar(
    db: &DatabaseConnection,
    id: Uuid,
    org_id: Uuid,
) -> Result<UsuarioResponse, AppError> {
    let record = usuario::Entity::find_by_id(id)
        .filter(usuario::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Usuario no encontrado".to_string()))?;

    let mut active: usuario::ActiveModel = record.into();
    active.activo = Set(true);

    let updated = active.update(db).await?;
    Ok(UsuarioResponse::from(updated))
}
