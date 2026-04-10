use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "propiedades")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub titulo: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub descripcion: Option<String>,
    pub direccion: String,
    pub ciudad: String,
    pub provincia: String,
    pub tipo_propiedad: String,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    #[sea_orm(column_type = "Decimal(Some((10, 2)))", nullable)]
    pub area_m2: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub precio: Decimal,
    pub moneda: String,
    pub estado: String,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub imagenes: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::contrato::Entity")]
    Contratos,
}

impl Related<super::contrato::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Contratos.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
