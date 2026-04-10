use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "gastos")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub unidad_id: Option<Uuid>,
    pub categoria: String,
    pub descripcion: String,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub monto: Decimal,
    pub moneda: String,
    pub fecha_gasto: Date,
    pub estado: String,
    pub proveedor: Option<String>,
    pub numero_factura: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub notas: Option<String>,
    pub organizacion_id: Uuid,
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
    #[sea_orm(
        belongs_to = "super::unidad::Entity",
        from = "Column::UnidadId",
        to = "super::unidad::Column::Id"
    )]
    Unidad,
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

impl Related<super::unidad::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Unidad.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
