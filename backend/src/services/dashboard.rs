use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter,
    QuerySelect,
};
use serde::Serialize;

use crate::entities::{contrato, pago, propiedad};
use crate::errors::AppError;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_propiedades: u64,
    pub tasa_ocupacion: f64,
    pub ingreso_mensual: Decimal,
    pub pagos_atrasados: u64,
}

#[derive(Debug, FromQueryResult)]
struct SumResult {
    total: Option<Decimal>,
}

pub async fn get_stats(db: &DatabaseConnection) -> Result<DashboardStats, AppError> {
    let total_propiedades = propiedad::Entity::find().count(db).await?;

    let ocupadas = propiedad::Entity::find()
        .filter(propiedad::Column::Estado.eq("ocupada"))
        .count(db)
        .await?;

    let tasa_ocupacion = if total_propiedades > 0 {
        (ocupadas as f64 / total_propiedades as f64) * 100.0
    } else {
        0.0
    };

    let ingreso_mensual = contrato::Entity::find()
        .select_only()
        .column_as(contrato::Column::MontoMensual.sum(), "total")
        .filter(contrato::Column::Estado.eq("activo"))
        .into_model::<SumResult>()
        .one(db)
        .await?
        .and_then(|r| r.total)
        .unwrap_or(Decimal::ZERO);

    let pagos_atrasados = pago::Entity::find()
        .filter(pago::Column::Estado.eq("atrasado"))
        .count(db)
        .await?;

    Ok(DashboardStats {
        total_propiedades,
        tasa_ocupacion,
        ingreso_mensual,
        pagos_atrasados,
    })
}
