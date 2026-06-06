use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "reportes_dgii")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub organizacion_id: Uuid,
    pub tipo_reporte: String,
    #[sea_orm(column_type = "Char(Some(6))")]
    pub periodo: String,
    pub estado: String,
    pub cantidad_registros: i32,
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub monto_total: Decimal,
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub itbis_total: Decimal,
    #[sea_orm(column_type = "Text")]
    pub contenido: String,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub registros_excluidos: Option<Json>,
    pub generated_by: Uuid,
    pub generated_at: DateTimeWithTimeZone,
    pub submitted_at: Option<DateTimeWithTimeZone>,
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
        from = "Column::GeneratedBy",
        to = "super::usuario::Column::Id"
    )]
    Usuario,
}

impl Related<super::organizacion::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organizacion.def()
    }
}

impl Related<super::usuario::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Usuario.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
