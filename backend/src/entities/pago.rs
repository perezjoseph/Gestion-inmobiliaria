use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "pagos")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub contrato_id: Uuid,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub monto: Decimal,
    pub moneda: String,
    pub fecha_pago: Option<Date>,
    pub fecha_vencimiento: Date,
    pub metodo_pago: Option<String>,
    pub estado: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub notas: Option<String>,
    pub organizacion_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::contrato::Entity",
        from = "Column::ContratoId",
        to = "super::contrato::Column::Id"
    )]
    Contrato,
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
        Relation::Contrato.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
