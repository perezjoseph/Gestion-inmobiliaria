use chrono::{NaiveDate, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::entities::pago;
use crate::errors::AppError;

/// Maximum number of rows to export in a single CSV request.
/// Prevents unbounded memory usage on large tenants (skill: Data Export Security).
const MAX_EXPORT_ROWS: usize = 50_000;

pub async fn exportar_pagos_csv(
    db: &DatabaseConnection,
    org_id: Uuid,
    fecha_inicio: Option<NaiveDate>,
    fecha_fin: Option<NaiveDate>,
) -> Result<Vec<u8>, AppError> {
    // Default to last 12 months if no date boundary provided (skill: Data Export Security)
    let today = Utc::now().date_naive();
    let effective_inicio = fecha_inicio.unwrap_or_else(|| {
        today
            .checked_sub_months(chrono::Months::new(12))
            .unwrap_or(today)
    });
    let effective_fin = fecha_fin.unwrap_or(today);

    // Tenant-scoped query (skill: Multi-Tenant Query Scoping)
    let pagos = pago::Entity::find()
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .filter(pago::Column::FechaVencimiento.gte(effective_inicio))
        .filter(pago::Column::FechaVencimiento.lte(effective_fin))
        .order_by_asc(pago::Column::FechaVencimiento)
        .all(db)
        .await?;

    // Cap row count to prevent OOM on large datasets
    if pagos.len() > MAX_EXPORT_ROWS {
        return Err(AppError::Validation(format!(
            "La exportación excede el límite de {} registros. Use un rango de fechas más reducido.",
            MAX_EXPORT_ROWS
        )));
    }

    // Build CSV in memory using csv crate
    let mut wtr = csv::Writer::from_writer(Vec::with_capacity(pagos.len() * 128));

    // Write header row
    wtr.write_record([
        "id",
        "contrato_id",
        "monto",
        "moneda",
        "fecha_vencimiento",
        "fecha_pago",
        "metodo_pago",
        "estado",
        "recargo",
        "notas",
    ])
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error escribiendo encabezado CSV: {e}")))?;

    for p in &pagos {
        wtr.write_record([
            p.id.to_string(),
            p.contrato_id.to_string(),
            p.monto.to_string(),
            p.moneda.clone(),
            p.fecha_vencimiento.to_string(),
            p.fecha_pago
                .map(|d| d.to_string())
                .unwrap_or_default(),
            p.metodo_pago.clone().unwrap_or_default(),
            p.estado.clone(),
            p.recargo
                .map(|r| r.to_string())
                .unwrap_or_default(),
            p.notas.clone().unwrap_or_default(),
        ])
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error escribiendo fila CSV: {e}")))?;
    }

    let bytes = wtr
        .into_inner()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error finalizando CSV: {e}")))?;

    Ok(bytes)
}
