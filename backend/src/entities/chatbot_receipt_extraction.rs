use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chatbot_receipt_extraction")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub organizacion_id: Uuid,
    pub conversation_id: Uuid,
    pub inquilino_id: Option<Uuid>,
    pub contrato_id: Option<Uuid>,
    #[sea_orm(column_type = "JsonBinary")]
    pub extracted_data: Json,
    pub status: String,
    pub confirmed_by: Option<Uuid>,
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
        belongs_to = "super::chatbot_conversation::Entity",
        from = "Column::ConversationId",
        to = "super::chatbot_conversation::Column::Id"
    )]
    Conversation,
    #[sea_orm(
        belongs_to = "super::inquilino::Entity",
        from = "Column::InquilinoId",
        to = "super::inquilino::Column::Id"
    )]
    Inquilino,
    #[sea_orm(
        belongs_to = "super::contrato::Entity",
        from = "Column::ContratoId",
        to = "super::contrato::Column::Id"
    )]
    Contrato,
    #[sea_orm(
        belongs_to = "super::usuario::Entity",
        from = "Column::ConfirmedBy",
        to = "super::usuario::Column::Id"
    )]
    ConfirmedByUser,
}

impl Related<super::organizacion::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organizacion.def()
    }
}

impl Related<super::chatbot_conversation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Conversation.def()
    }
}

impl Related<super::inquilino::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Inquilino.def()
    }
}

impl Related<super::contrato::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Contrato.def()
    }
}

impl Related<super::usuario::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConfirmedByUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
