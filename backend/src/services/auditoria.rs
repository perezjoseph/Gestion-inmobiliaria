use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::registro_auditoria;
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::auditoria::{AuditoriaQuery, AuditoriaResponse};

pub struct CreateAuditoriaEntry {
    pub usuario_id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub accion: String,
    pub cambios: serde_json::Value,
}

impl From<registro_auditoria::Model> for AuditoriaResponse {
    fn from(m: registro_auditoria::Model) -> Self {
        Self {
            id: m.id,
            usuario_id: m.usuario_id,
            entity_type: m.entity_type,
            entity_id: m.entity_id,
            accion: m.accion,
            cambios: m.cambios,
            created_at: m.created_at.into(),
        }
    }
}

pub async fn registrar<C>(db: &C, entry: CreateAuditoriaEntry) -> Result<(), AppError>
where
    C: sea_orm::ConnectionTrait,
{
    let model = registro_auditoria::ActiveModel {
        id: Set(Uuid::new_v4()),
        usuario_id: Set(entry.usuario_id),
        entity_type: Set(entry.entity_type),
        entity_id: Set(entry.entity_id),
        accion: Set(entry.accion),
        cambios: Set(entry.cambios),
        created_at: Set(Utc::now().into()),
    };

    model.insert(db).await?;
    Ok(())
}

pub async fn listar(
    db: &DatabaseConnection,
    query: AuditoriaQuery,
) -> Result<PaginatedResponse<AuditoriaResponse>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let mut select = registro_auditoria::Entity::find();

    if let Some(ref entity_type) = query.entity_type {
        select = select.filter(registro_auditoria::Column::EntityType.eq(entity_type));
    }
    if let Some(entity_id) = query.entity_id {
        select = select.filter(registro_auditoria::Column::EntityId.eq(entity_id));
    }
    if let Some(usuario_id) = query.usuario_id {
        select = select.filter(registro_auditoria::Column::UsuarioId.eq(usuario_id));
    }
    if let Some(fecha_desde) = query.fecha_desde {
        select = select.filter(
            registro_auditoria::Column::CreatedAt
                .gte(fecha_desde.and_hms_opt(0, 0, 0).unwrap_or_default()),
        );
    }
    if let Some(fecha_hasta) = query.fecha_hasta {
        select = select.filter(
            registro_auditoria::Column::CreatedAt
                .lte(fecha_hasta.and_hms_opt(23, 59, 59).unwrap_or_default()),
        );
    }

    let paginator = select
        .order_by_desc(registro_auditoria::Column::CreatedAt)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records.into_iter().map(AuditoriaResponse::from).collect(),
        total,
        page,
        per_page,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_model(entity_type: &str, accion: &str) -> registro_auditoria::Model {
        registro_auditoria::Model {
            id: Uuid::new_v4(),
            usuario_id: Uuid::new_v4(),
            entity_type: entity_type.to_string(),
            entity_id: Uuid::new_v4(),
            accion: accion.to_string(),
            cambios: json!({"campo": "valor"}),
            created_at: Utc::now().fixed_offset(),
        }
    }

    #[test]
    fn create_auditoria_entry_holds_correct_fields() {
        let uid = Uuid::new_v4();
        let eid = Uuid::new_v4();
        let entry = CreateAuditoriaEntry {
            usuario_id: uid,
            entity_type: "propiedad".to_string(),
            entity_id: eid,
            accion: "crear".to_string(),
            cambios: json!({"titulo": "Casa nueva"}),
        };
        assert_eq!(entry.usuario_id, uid);
        assert_eq!(entry.entity_type, "propiedad");
        assert_eq!(entry.entity_id, eid);
        assert_eq!(entry.accion, "crear");
        assert_eq!(entry.cambios["titulo"], "Casa nueva");
    }

    #[test]
    fn from_model_converts_all_fields() {
        let model = make_model("contrato", "actualizar");
        let model_id = model.id;
        let model_usuario_id = model.usuario_id;
        let model_entity_id = model.entity_id;

        let resp = AuditoriaResponse::from(model);
        assert_eq!(resp.id, model_id);
        assert_eq!(resp.usuario_id, model_usuario_id);
        assert_eq!(resp.entity_type, "contrato");
        assert_eq!(resp.entity_id, model_entity_id);
        assert_eq!(resp.accion, "actualizar");
        assert_eq!(resp.cambios["campo"], "valor");
    }

    #[test]
    fn from_model_converts_created_at_to_utc() {
        let model = make_model("pago", "eliminar");
        let resp = AuditoriaResponse::from(model);
        assert_eq!(resp.created_at.timezone(), Utc);
    }

    #[test]
    fn from_model_preserves_complex_cambios() {
        let mut model = make_model("inquilino", "actualizar");
        model.cambios = json!({
            "antes": {"nombre": "Juan"},
            "despues": {"nombre": "Pedro"}
        });
        let resp = AuditoriaResponse::from(model);
        assert_eq!(resp.cambios["antes"]["nombre"], "Juan");
        assert_eq!(resp.cambios["despues"]["nombre"], "Pedro");
    }

    #[test]
    fn create_entry_with_empty_cambios() {
        let entry = CreateAuditoriaEntry {
            usuario_id: Uuid::new_v4(),
            entity_type: "propiedad".to_string(),
            entity_id: Uuid::new_v4(),
            accion: "eliminar".to_string(),
            cambios: json!({}),
        };
        assert!(
            entry
                .cambios
                .as_object()
                .is_none_or(serde_json::Map::is_empty)
        );
    }

    #[test]
    fn from_model_all_entity_types() {
        for entity_type in &["propiedad", "inquilino", "contrato", "pago"] {
            let model = make_model(entity_type, "crear");
            let resp = AuditoriaResponse::from(model);
            assert_eq!(resp.entity_type, *entity_type);
        }
    }

    #[test]
    fn from_model_all_action_types() {
        for accion in &["crear", "actualizar", "eliminar"] {
            let model = make_model("propiedad", accion);
            let resp = AuditoriaResponse::from(model);
            assert_eq!(resp.accion, *accion);
        }
    }
}
