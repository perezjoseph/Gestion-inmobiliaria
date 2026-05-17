use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "documentos")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub filename: String,
    pub file_path: String,
    pub mime_type: String,
    pub file_size: i64,
    pub uploaded_by: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub tipo_documento: String,
    pub estado_verificacion: String,
    pub fecha_vencimiento: Option<Date>,
    pub verificado_por: Option<Uuid>,
    pub fecha_verificacion: Option<DateTimeWithTimeZone>,
    pub notas_verificacion: Option<String>,
    pub numero_documento: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub contenido_editable: Option<Json>,
    pub updated_at: Option<DateTimeWithTimeZone>,
    pub sellado: bool,
    pub sellado_at: Option<DateTimeWithTimeZone>,
    pub documento_origen_id: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::firma_documento::Entity")]
    FirmaDocumento,
}

impl Related<super::firma_documento::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FirmaDocumento.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
