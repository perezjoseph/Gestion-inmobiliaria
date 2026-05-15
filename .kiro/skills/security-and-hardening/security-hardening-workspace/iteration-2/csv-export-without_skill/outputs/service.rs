use chrono::NaiveDate;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::entities::pago;
use crate::errors::AppError;

pub async fn exportar_pagos_csv(
    db: &DatabaseConnection,
    org_id: Uuid,
    fecha_inicio: Option<NaiveDate>,
    fecha_fin: Option<NaiveDate>,
) -> Result<Vec<u8>, AppError> {
    if let (Some(inicio), Some(fin)) = (fecha_inicio, fecha_fin) {
        if inicio > fin {
            return Err(AppError::Validation(
                "fecha_inicio no puede ser posterior a fecha_fin".into(),
            ));
        }
    }

    let mut query = pago::Entity::find()
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .order_by_asc(pago::Column::FechaVencimiento);

    if let Some(inicio) = fecha_inicio {
        query = query.filter(pago::Column::FechaVencimiento.gte(inicio));
    }
    if let Some(fin) = fecha_fin {
        query = query.filter(pago::Column::FechaVencimiento.lte(fin));
    }

    let pagos = query.all(db).await?;

    let mut csv = String::with_capacity(pagos.len() * 128);
    csv.push_str("id,contrato_id,monto,moneda,fecha_vencimiento,fecha_pago,metodo_pago,estado,recargo,notas\n");

    for p in &pagos {
        let fecha_pago = p
            .fecha_pago
            .map(|d| d.to_string())
            .unwrap_or_default();
        let metodo_pago = p.metodo_pago.as_deref().unwrap_or("");
        let recargo = p
            .recargo
            .map(|r| r.to_string())
            .unwrap_or_default();
        let notas = p
            .notas
            .as_deref()
            .unwrap_or("")
            .replace('"', "\"\"");

        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},\"{}\"\n",
            p.id,
            p.contrato_id,
            p.monto,
            p.moneda,
            p.fecha_vencimiento,
            fecha_pago,
            metodo_pago,
            p.estado,
            recargo,
            notas,
        ));
    }

    Ok(csv.into_bytes())
}
