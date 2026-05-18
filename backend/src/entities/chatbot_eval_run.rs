use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chatbot_eval_run")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub suite_id: Uuid,
    pub organizacion_id: Uuid,
    #[sea_orm(column_type = "String(StringLen::N(20))")]
    pub status: String,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub results: Option<Json>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub summary: Option<Json>,
    #[sea_orm(column_type = "JsonBinary")]
    pub agent_config_snapshot: Json,
    pub started_at: DateTimeWithTimeZone,
    pub completed_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::chatbot_eval_suite::Entity",
        from = "Column::SuiteId",
        to = "super::chatbot_eval_suite::Column::Id"
    )]
    Suite,
    #[sea_orm(
        belongs_to = "super::organizacion::Entity",
        from = "Column::OrganizacionId",
        to = "super::organizacion::Column::Id"
    )]
    Organizacion,
}

impl Related<super::chatbot_eval_suite::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Suite.def()
    }
}

impl Related<super::organizacion::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organizacion.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
