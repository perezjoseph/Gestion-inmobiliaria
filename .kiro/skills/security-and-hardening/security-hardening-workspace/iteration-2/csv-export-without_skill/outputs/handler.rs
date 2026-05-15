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
