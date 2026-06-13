use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::models::reporte::PagosExportQuery;
use crate::services::auth::Claims;
use crate::services::reportes;

pub async fn pagos_export(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<PagosExportQuery>,
) -> Result<HttpResponse, AppError> {
    let params = query.into_inner();

    // Validate date range if both provided
    if let (Some(inicio), Some(fin)) = (params.fecha_inicio, params.fecha_fin) {
        if inicio > fin {
            return Err(AppError::Validation(
                "fecha_inicio no puede ser posterior a fecha_fin".into(),
            ));
        }
        // Cap date range at 2 years per skill guidance (Data Export Security)
        let max_days = 365 * 2;
        if (fin - inicio).num_days() > max_days {
            return Err(AppError::Validation(
                "El rango de fechas no puede exceder 2 años".into(),
            ));
        }
    }

    // Log export event (skill: Data Export Security — log every export)
    tracing::info!(
        user_email = %claims.email,
        org_id = %claims.organizacion_id,
        fecha_inicio = ?params.fecha_inicio,
        fecha_fin = ?params.fecha_fin,
        "Exportación CSV de pagos solicitada"
    );

    let csv_bytes = reportes::exportar_pagos_csv(
        db.get_ref(),
        claims.organizacion_id,
        params.fecha_inicio,
        params.fecha_fin,
    )
    .await?;

    Ok(HttpResponse::Ok()
        .content_type("text/csv; charset=utf-8")
        .insert_header((
            "Content-Disposition",
            "attachment; filename=\"pagos-export.csv\"",
        ))
        .body(csv_bytes))
}
