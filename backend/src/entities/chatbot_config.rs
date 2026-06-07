use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chatbot_config")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub organizacion_id: Uuid,
    pub activo: bool,
    pub connection_status: String,
    pub display_name: Option<String>,
    pub language: String,
    pub tone: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub greeting: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub system_prompt: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub faqs: Option<Json>,
    #[sea_orm(column_type = "Text", nullable)]
    pub policies: Option<String>,
    pub sender_policy: String,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub allowlist: Option<Json>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub capabilities: Option<Json>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub handoff_keywords: Option<Json>,
    pub history_limit: i32,
    pub retention_days: i32,
    #[sea_orm(column_type = "JsonBinary")]
    pub agent_config: Json,
    #[sea_orm(column_type = "JsonBinary")]
    pub guidance_rules: Json,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::organizacion::Entity",
        from = "Column::OrganizacionId",
        to = "super::organizacion::Column::Id"
    )]
    Organizacion,
    #[sea_orm(
        belongs_to = "super::usuario::Entity",
        from = "Column::UpdatedBy",
        to = "super::usuario::Column::Id"
    )]
    UpdatedByUser,
}

impl Related<super::organizacion::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organizacion.def()
    }
}

impl Related<super::usuario::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UpdatedByUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
