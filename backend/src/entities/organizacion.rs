use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "organizaciones")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tipo: String,
    pub nombre: String,
    pub estado: String,
    pub cedula: Option<String>,
    pub telefono: Option<String>,
    pub email_organizacion: Option<String>,
    pub rnc: Option<String>,
    pub razon_social: Option<String>,
    pub nombre_comercial: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub direccion_fiscal: Option<String>,
    pub representante_legal: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub dgii_data: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::invitacion::Entity")]
    Invitacion,
    #[sea_orm(has_many = "super::propiedad::Entity")]
    Propiedad,
    #[sea_orm(has_many = "super::usuario::Entity")]
    Usuario,
}

impl Related<super::invitacion::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invitacion.def()
    }
}

impl Related<super::propiedad::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Propiedad.def()
    }
}

impl Related<super::usuario::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Usuario.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
