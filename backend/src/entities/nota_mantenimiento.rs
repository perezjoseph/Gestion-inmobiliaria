use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "notas_mantenimiento")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub solicitud_id: Uuid,
    pub autor_id: Uuid,
    pub contenido: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::solicitud_mantenimiento::Entity",
        from = "Column::SolicitudId",
        to = "super::solicitud_mantenimiento::Column::Id"
    )]
    SolicitudMantenimiento,
    #[sea_orm(
        belongs_to = "super::usuario::Entity",
        from = "Column::AutorId",
        to = "super::usuario::Column::Id"
    )]
    Usuario,
}

impl Related<super::solicitud_mantenimiento::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SolicitudMantenimiento.def()
    }
}

impl Related<super::usuario::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Usuario.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
