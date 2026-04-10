use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ejecuciones_tareas")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub nombre_tarea: String,
    pub iniciado_en: DateTimeWithTimeZone,
    pub duracion_ms: i64,
    pub exitosa: bool,
    pub registros_afectados: i64,
    #[sea_orm(column_type = "Text", nullable)]
    pub mensaje_error: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
