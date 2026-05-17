use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "cache_dgii")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub cedula_rnc: String,
    pub nombre_razon_social: String,
    pub nombre_comercial: Option<String>,
    pub estado: String,
    pub regimen_de_pagos: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub actividad_economica: Option<String>,
    pub raw_response: Json,
    pub organizacion_id: Uuid,
    pub cached_at: DateTimeWithTimeZone,
    pub expires_at: DateTimeWithTimeZone,
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
}

impl Related<super::organizacion::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organizacion.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
