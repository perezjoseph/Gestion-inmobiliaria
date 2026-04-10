use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "unidades")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub numero_unidad: String,
    pub piso: Option<i32>,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    #[sea_orm(column_type = "Decimal(Some((10, 2)))", nullable)]
    pub area_m2: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub precio: Decimal,
    pub moneda: String,
    pub estado: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub descripcion: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::propiedad::Entity",
        from = "Column::PropiedadId",
        to = "super::propiedad::Column::Id"
    )]
    Propiedad,
}

impl Related<super::propiedad::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Propiedad.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
