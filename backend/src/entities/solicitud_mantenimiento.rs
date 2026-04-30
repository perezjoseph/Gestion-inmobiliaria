use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "solicitudes_mantenimiento")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub unidad_id: Option<Uuid>,
    pub inquilino_id: Option<Uuid>,
    pub titulo: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub descripcion: Option<String>,
    pub estado: String,
    pub prioridad: String,
    pub nombre_proveedor: Option<String>,
    pub telefono_proveedor: Option<String>,
    pub email_proveedor: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))", nullable)]
    pub costo_monto: Option<Decimal>,
    pub costo_moneda: Option<String>,
    pub fecha_inicio: Option<DateTimeWithTimeZone>,
    pub fecha_fin: Option<DateTimeWithTimeZone>,
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
    #[sea_orm(has_many = "super::nota_mantenimiento::Entity")]
    Notas,
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

impl Related<super::nota_mantenimiento::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Notas.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
