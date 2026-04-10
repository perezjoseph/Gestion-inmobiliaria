use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "contratos")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub inquilino_id: Uuid,
    pub fecha_inicio: Date,
    pub fecha_fin: Date,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub monto_mensual: Decimal,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))", nullable)]
    pub deposito: Option<Decimal>,
    pub moneda: String,
    pub estado: String,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub documentos: Option<Json>,
    pub organizacion_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub estado_deposito: Option<String>,
    pub fecha_cobro_deposito: Option<DateTimeWithTimeZone>,
    pub fecha_devolucion_deposito: Option<DateTimeWithTimeZone>,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))", nullable)]
    pub monto_retenido: Option<Decimal>,
    pub motivo_retencion: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((5, 2)))", nullable)]
    pub recargo_porcentaje: Option<Decimal>,
    pub dias_gracia: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::propiedad::Entity",
        from = "Column::PropiedadId",
        to = "super::propiedad::Column::Id"
    )]
    Propiedad,
    #[sea_orm(
        belongs_to = "super::inquilino::Entity",
        from = "Column::InquilinoId",
        to = "super::inquilino::Column::Id"
    )]
    Inquilino,
    #[sea_orm(has_many = "super::pago::Entity")]
    Pagos,
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

impl Related<super::propiedad::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Propiedad.def()
    }
}

impl Related<super::inquilino::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Inquilino.def()
    }
}

impl Related<super::pago::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Pagos.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
