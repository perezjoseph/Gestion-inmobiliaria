use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;

use crate::entities::organizacion;
use crate::errors::AppError;
use crate::models::organizacion::{OrganizacionResponse, UpdateOrganizacionRequest};

impl From<organizacion::Model> for OrganizacionResponse {
    fn from(m: organizacion::Model) -> Self {
        Self {
            id: m.id,
            tipo: m.tipo,
            nombre: m.nombre,
            estado: m.estado,
            cedula: m.cedula,
            telefono: m.telefono,
            email_organizacion: m.email_organizacion,
            rnc: m.rnc,
            razon_social: m.razon_social,
            nombre_comercial: m.nombre_comercial,
            direccion_fiscal: m.direccion_fiscal,
            representante_legal: m.representante_legal,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}

pub async fn get_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<OrganizacionResponse, AppError> {
    let record = organizacion::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;
    Ok(OrganizacionResponse::from(record))
}

pub async fn update<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
    input: UpdateOrganizacionRequest,
) -> Result<OrganizacionResponse, AppError> {
    let existing = organizacion::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;

    let mut active: organizacion::ActiveModel = existing.into();

    if let Some(nombre) = input.nombre {
        active.nombre = Set(nombre);
    }
    if let Some(telefono) = input.telefono {
        active.telefono = Set(Some(telefono));
    }
    if let Some(email_organizacion) = input.email_organizacion {
        active.email_organizacion = Set(Some(email_organizacion));
    }
    if let Some(nombre_comercial) = input.nombre_comercial {
        active.nombre_comercial = Set(Some(nombre_comercial));
    }
    if let Some(direccion_fiscal) = input.direccion_fiscal {
        active.direccion_fiscal = Set(Some(direccion_fiscal));
    }
    if let Some(representante_legal) = input.representante_legal {
        active.representante_legal = Set(Some(representante_legal));
    }
    if let Some(dgii_data) = input.dgii_data {
        active.dgii_data = Set(Some(dgii_data));
    }

    active.updated_at = Set(Utc::now().into());

    let updated = active.update(db).await?;
    Ok(OrganizacionResponse::from(updated))
}
