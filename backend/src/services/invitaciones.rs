use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::entities::invitacion;
use crate::errors::AppError;
use crate::models::invitacion::{CrearInvitacionRequest, InvitacionResponse};

impl From<invitacion::Model> for InvitacionResponse {
    fn from(m: invitacion::Model) -> Self {
        Self {
            id: m.id,
            email: m.email,
            rol: m.rol,
            token: m.token,
            usado: m.usado,
            expires_at: m.expires_at.into(),
            created_at: m.created_at.into(),
        }
    }
}

pub async fn crear(
    db: &DatabaseConnection,
    org_id: Uuid,
    input: CrearInvitacionRequest,
) -> Result<InvitacionResponse, AppError> {
    if input.rol != "gerente" && input.rol != "visualizador" {
        return Err(AppError::BadRequest(
            "El rol debe ser 'gerente' o 'visualizador'".to_string(),
        ));
    }

    let now = Utc::now();
    let expires_at = now + Duration::days(7);
    let id = Uuid::new_v4();
    let token = Uuid::new_v4().to_string();

    let model = invitacion::ActiveModel {
        id: Set(id),
        organizacion_id: Set(org_id),
        email: Set(input.email),
        rol: Set(input.rol),
        token: Set(token),
        usado: Set(false),
        expires_at: Set(expires_at.into()),
        created_at: Set(now.into()),
    };

    let record = model.insert(db).await?;
    Ok(InvitacionResponse::from(record))
}

pub async fn listar(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<Vec<InvitacionResponse>, AppError> {
    let now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
    let records = invitacion::Entity::find()
        .filter(invitacion::Column::OrganizacionId.eq(org_id))
        .filter(invitacion::Column::Usado.eq(false))
        .filter(invitacion::Column::ExpiresAt.gt(now))
        .all(db)
        .await?;

    Ok(records.into_iter().map(InvitacionResponse::from).collect())
}

pub async fn revocar(
    db: &DatabaseConnection,
    org_id: Uuid,
    id: Uuid,
) -> Result<(), AppError> {
    let record = invitacion::Entity::find_by_id(id)
        .filter(invitacion::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Invitación no encontrada".to_string()))?;

    let active: invitacion::ActiveModel = record.into();
    active.delete(db).await?;
    Ok(())
}

pub async fn validar_token(
    db: &DatabaseConnection,
    token: &str,
) -> Result<invitacion::Model, AppError> {
    let record = invitacion::Entity::find()
        .filter(invitacion::Column::Token.eq(token))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Invitación no encontrada".to_string()))?;

    if record.usado {
        return Err(AppError::Conflict(
            "La invitación ya fue utilizada".to_string(),
        ));
    }

    let now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
    if record.expires_at < now {
        return Err(AppError::Gone(
            "La invitación ha expirado".to_string(),
        ));
    }

    Ok(record)
}
