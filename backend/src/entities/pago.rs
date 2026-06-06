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
    #[sea_orm(column_type = "Decimal(Some((12, 2)))", nullable)]
    pub recargo: Option<Decimal>,
    pub organizacion_id: Uuid,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))", nullable)]
    pub monto_base: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))", nullable)]
    pub monto_itbis: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))", nullable)]
    pub monto_itbis_retenido: Option<Decimal>,
    pub ncf: Option<String>,
    pub fecha_comprobante: Option<Date>,
    pub tipo_ncf: Option<String>,
    #[sea_orm(default_value = "false")]
    pub es_parcial: bool,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))", nullable)]
    pub saldo_pendiente: Option<Decimal>,
    #[sea_orm(default_value = "renta")]
    pub tipo_linea: String,
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
