use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "inquilinos")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub nombre: String,
    pub apellido: String,
    pub email: Option<String>,
    pub telefono: Option<String>,
    #[sea_orm(unique)]
    pub cedula: String,
    pub contacto_emergencia: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub notas: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub documentos: Option<Json>,
    pub organizacion_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::contrato::Entity")]
    Contratos,
    #[sea_orm(
        belongs_to = "super::organizacion::Entity",
        from = "Column::OrganizacionId",
        to = "super::organizacion::Column::Id"
    )]
    Organizacion,
}

impl Related<super::organizacion::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organizacion.def()
    }
}

impl Related<super::contrato::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Contratos.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
