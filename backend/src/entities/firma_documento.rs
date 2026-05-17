use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "firmas_documento")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub documento_id: Uuid,
    pub firmante_tipo: String,
    pub firmante_nombre: String,
    #[sea_orm(column_type = "VarBinary(StringLen::None)", nullable)]
    pub firma_imagen: Option<Vec<u8>>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub firmado_at: Option<DateTimeWithTimeZone>,
    #[sea_orm(unique)]
    pub token: Option<String>,
    pub password_hash: Option<String>,
    pub expira_at: Option<DateTimeWithTimeZone>,
    pub estado: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::documento::Entity",
        from = "Column::DocumentoId",
        to = "super::documento::Column::Id"
    )]
    Documento,
}

impl Related<super::documento::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Documento.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
