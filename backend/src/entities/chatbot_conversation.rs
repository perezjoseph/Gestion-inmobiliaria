use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chatbot_conversation")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub organizacion_id: Uuid,
    pub sender_phone: String,
    pub inquilino_id: Option<Uuid>,
    pub role: String,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    pub message_type: String,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub metadata: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
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
        belongs_to = "super::inquilino::Entity",
        from = "Column::InquilinoId",
        to = "super::inquilino::Column::Id"
    )]
    Inquilino,
    #[sea_orm(has_many = "super::chatbot_receipt_extraction::Entity")]
    ChatbotReceiptExtractions,
}

impl Related<super::organizacion::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organizacion.def()
    }
}

impl Related<super::inquilino::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Inquilino.def()
    }
}

impl Related<super::chatbot_receipt_extraction::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ChatbotReceiptExtractions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
