use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use serde_json::json;

use crate::errors::AppError;
use crate::models::reporte::{HistorialPagosQuery, IngresoReportQuery, RentabilidadReportQuery};
use crate::services::auth::Claims;
use crate::services::reportes;

pub async fn ingresos(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<IngresoReportQuery>,
) -> Result<HttpResponse, AppError> {
    let summary =
        reportes::generar_reporte_ingresos(db.get_ref(), claims.organizacion_id, query.into_inner(), claims.email).await?;
    Ok(HttpResponse::Ok().json(summary))
}

pub async fn ingresos_pdf(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<IngresoReportQuery>,
) -> Result<HttpResponse, AppError> {
    let summary =
        reportes::generar_reporte_ingresos(db.get_ref(), claims.organizacion_id, query.into_inner(), claims.email).await?;
    let bytes = reportes::exportar_pdf(&summary)?;
    Ok(HttpResponse::Ok()
        .content_type("application/pdf")
        .insert_header((
            "Content-Disposition",
            "attachment; filename=\"reporte-ingresos.pdf\"",
        ))
        .body(bytes))
}

pub async fn ingresos_xlsx(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<IngresoReportQuery>,
) -> Result<HttpResponse, AppError> {
    let summary =
        reportes::generar_reporte_ingresos(db.get_ref(), claims.organizacion_id, query.into_inner(), claims.email).await?;
    let bytes = reportes::exportar_xlsx(&summary)?;
    Ok(HttpResponse::Ok()
        .content_type("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .insert_header((
            "Content-Disposition",
            "attachment; filename=\"reporte-ingresos.xlsx\"",
        ))
        .body(bytes))
}

pub async fn historial_pagos(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<HistorialPagosQuery>,
) -> Result<HttpResponse, AppError> {
    let params = query.into_inner();
    let entries =
        reportes::historial_pagos(db.get_ref(), claims.organizacion_id, params.fecha_desde, params.fecha_hasta).await?;
    Ok(HttpResponse::Ok().json(entries))
}

pub async fn ocupacion_tendencia(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
) -> Result<HttpResponse, AppError> {
    let tasa = reportes::calcular_tasa_ocupacion(db.get_ref(), claims.organizacion_id).await?;
    Ok(HttpResponse::Ok().json(json!({ "tasaOcupacion": tasa })))
}

pub async fn rentabilidad(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<RentabilidadReportQuery>,
) -> Result<HttpResponse, AppError> {
    let summary =
        reportes::generar_reporte_rentabilidad(db.get_ref(), claims.organizacion_id, query.into_inner(), claims.email)
            .await?;
    Ok(HttpResponse::Ok().json(summary))
}

pub async fn rentabilidad_pdf(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<RentabilidadReportQuery>,
) -> Result<HttpResponse, AppError> {
    let summary =
        reportes::generar_reporte_rentabilidad(db.get_ref(), claims.organizacion_id, query.into_inner(), claims.email)
            .await?;
    let bytes = reportes::exportar_rentabilidad_pdf(&summary)?;
    Ok(HttpResponse::Ok()
        .content_type("application/pdf")
        .insert_header((
            "Content-Disposition",
            "attachment; filename=\"reporte-rentabilidad.pdf\"",
        ))
        .body(bytes))
}

pub async fn rentabilidad_xlsx(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<RentabilidadReportQuery>,
) -> Result<HttpResponse, AppError> {
    let summary =
        reportes::generar_reporte_rentabilidad(db.get_ref(), claims.organizacion_id, query.into_inner(), claims.email)
            .await?;
    let bytes = reportes::exportar_rentabilidad_xlsx(&summary)?;
    Ok(HttpResponse::Ok()
        .content_type("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .insert_header((
            "Content-Disposition",
            "attachment; filename=\"reporte-rentabilidad.xlsx\"",
        ))
        .body(bytes))
}
