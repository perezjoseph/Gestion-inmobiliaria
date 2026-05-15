use chrono::NaiveDate;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::entities::pago;
use crate::errors::AppError;

/// Maximum rows to export in a single CSV request (skill: Data Export Security — cap row count).
const MAX_EXPORT_ROWS: usize = 50_000;

/// Default lookback period when no date boundaries are provided (skill: require at least one date
/// boundary — default to last 12 months if none provided).
const DEFAULT_LOOKBACK_MONTHS: i64 = 12;

pub async fn exportar_pagos_csv(
    db: &DatabaseConnection,
    org_id: Uuid,
    fecha_inicio: Option<NaiveDate>,
    fecha_fin: Option<NaiveDate>,
) -> Result<Vec<u8>, AppError> {
    // Apply default date boundary if neither date is provided (skill guidance)
    let today = chrono::Utc::now().date_naive();
    let effective_inicio = fecha_inicio.unwrap_or_else(|| {
        today - chrono::Duration::days(DEFAULT_LOOKBACK_MONTHS * 30)
    });
    let effective_fin = fecha_fin.unwrap_or(today);

    // Query pagos scoped to tenant (skill: Multi-Tenant Query Scoping)
    let pagos = pago::Entity::find()
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .filter(pago::Column::FechaVencimiento.gte(effective_inicio))
        .filter(pago::Column::FechaVencimiento.lte(effective_fin))
        .order_by_asc(pago::Column::FechaVencimiento)
        .all(db)
        .await?;

    // Cap row count (skill: Data Export Security — cap at 50k)
    if pagos.len() > MAX_EXPORT_ROWS {
        return Err(AppError::Validation(format!(
            "La exportación excede el límite de {} registros. Reduzca el rango de fechas.",
            MAX_EXPORT_ROWS
        )));
    }

    // Build CSV in memory
    let mut csv_buf = Vec::with_capacity(pagos.len() * 128);

    // UTF-8 BOM for Excel compatibility
    csv_buf.extend_from_slice(b"\xEF\xBB\xBF");

    // Header row
    csv_buf.extend_from_slice(
        b"id,contrato_id,monto,moneda,fecha_vencimiento,fecha_pago,metodo_pago,estado,recargo,notas\n",
    );

    for p in &pagos {
        // Escape CSV fields that may contain commas or quotes
        let notas = csv_escape(p.notas.as_deref().unwrap_or(""));
        let metodo = csv_escape(p.metodo_pago.as_deref().unwrap_or(""));
        let recargo = p
            .recargo
            .map(|r| r.to_string())
            .unwrap_or_default();
        let fecha_pago = p
            .fecha_pago
            .map(|d| d.to_string())
            .unwrap_or_default();

        let line = format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            p.id,
            p.contrato_id,
            p.monto,
            p.moneda,
            p.fecha_vencimiento,
            fecha_pago,
            metodo,
            p.estado,
            recargo,
            notas,
        );
        csv_buf.extend_from_slice(line.as_bytes());
    }

    Ok(csv_buf)
}

/// Escapes a field value for CSV output. Wraps in quotes if the value contains
/// commas, quotes, or newlines. Doubles any internal quotes per RFC 4180.
fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
