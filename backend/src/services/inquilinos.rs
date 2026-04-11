use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::inquilino;
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::inquilino::{CreateInquilinoRequest, InquilinoResponse, UpdateInquilinoRequest};
use crate::services::auditoria::{self, CreateAuditoriaEntry};

impl From<inquilino::Model> for InquilinoResponse {
    fn from(m: inquilino::Model) -> Self {
        Self {
            id: m.id,
            nombre: m.nombre,
            apellido: m.apellido,
            email: m.email,
            telefono: m.telefono,
            cedula: m.cedula,
            contacto_emergencia: m.contacto_emergencia,
            notas: m.notas,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    input: CreateInquilinoRequest,
    usuario_id: Uuid,
) -> Result<InquilinoResponse, AppError> {
    let existing = inquilino::Entity::find()
        .filter(inquilino::Column::Cedula.eq(&input.cedula))
        .one(db)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(
            "Ya existe un inquilino con esta cédula".to_string(),
        ));
    }

    let now = Utc::now().into();
    let id = Uuid::new_v4();

    let model = inquilino::ActiveModel {
        id: Set(id),
        nombre: Set(input.nombre),
        apellido: Set(input.apellido),
        email: Set(input.email),
        telefono: Set(input.telefono),
        cedula: Set(input.cedula),
        contacto_emergencia: Set(input.contacto_emergencia),
        notas: Set(input.notas),
        documentos: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = model.insert(db).await?;

    auditoria::registrar(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "inquilino".to_string(),
            entity_id: id,
            accion: "crear".to_string(),
            cambios: serde_json::json!(InquilinoResponse::from(record.clone())),
        },
    )
    .await?;

    Ok(InquilinoResponse::from(record))
}

pub async fn get_by_id(db: &DatabaseConnection, id: Uuid) -> Result<InquilinoResponse, AppError> {
    let record = inquilino::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Inquilino no encontrado".to_string()))?;
    Ok(InquilinoResponse::from(record))
}

pub async fn list(
    db: &DatabaseConnection,
    search: Option<String>,
    page: Option<u64>,
    per_page: Option<u64>,
) -> Result<PaginatedResponse<InquilinoResponse>, AppError> {
    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(20).clamp(1, 100);

    let mut select = inquilino::Entity::find();

    if let Some(ref term) = search {
        let condition = Condition::any()
            .add(inquilino::Column::Nombre.contains(term))
            .add(inquilino::Column::Apellido.contains(term))
            .add(inquilino::Column::Cedula.contains(term));
        select = select.filter(condition);
    }

    let paginator = select
        .order_by_asc(inquilino::Column::Apellido)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records.into_iter().map(InquilinoResponse::from).collect(),
        total,
        page,
        per_page,
    })
}

pub async fn update<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
    input: UpdateInquilinoRequest,
    usuario_id: Uuid,
) -> Result<InquilinoResponse, AppError> {
    let existing = inquilino::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Inquilino no encontrado".to_string()))?;

    let mut active: inquilino::ActiveModel = existing.into();

    if let Some(nombre) = input.nombre {
        active.nombre = Set(nombre);
    }
    if let Some(apellido) = input.apellido {
        active.apellido = Set(apellido);
    }
    if let Some(email) = input.email {
        active.email = Set(Some(email));
    }
    if let Some(telefono) = input.telefono {
        active.telefono = Set(Some(telefono));
    }
    if let Some(cedula) = input.cedula {
        active.cedula = Set(cedula);
    }
    if let Some(contacto_emergencia) = input.contacto_emergencia {
        active.contacto_emergencia = Set(Some(contacto_emergencia));
    }
    if let Some(notas) = input.notas {
        active.notas = Set(Some(notas));
    }

    active.updated_at = Set(Utc::now().into());

    let updated = active.update(db).await?;

    auditoria::registrar(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "inquilino".to_string(),
            entity_id: id,
            accion: "actualizar".to_string(),
            cambios: serde_json::json!(InquilinoResponse::from(updated.clone())),
        },
    )
    .await?;

    Ok(InquilinoResponse::from(updated))
}

pub async fn delete<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
    usuario_id: Uuid,
) -> Result<(), AppError> {
    let result = inquilino::Entity::delete_by_id(id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(AppError::NotFound("Inquilino no encontrado".to_string()));
    }

    auditoria::registrar(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "inquilino".to_string(),
            entity_id: id,
            accion: "eliminar".to_string(),
            cambios: serde_json::json!({ "id": id }),
        },
    )
    .await?;

    Ok(())
}
