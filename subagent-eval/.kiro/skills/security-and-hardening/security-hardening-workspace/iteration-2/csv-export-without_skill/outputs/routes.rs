// Add this route inside the existing /api/v1/reportes scope in routes.rs:
//
// .route(
//     "/pagos-export",
//     web::get().to(handlers::reportes::pagos_export),
// )
//
// The full reportes scope would look like:
//
// .service(
//     web::scope("/reportes")
//         .route("/ingresos", web::get().to(handlers::reportes::ingresos))
//         .route("/ingresos/pdf", web::get().to(handlers::reportes::ingresos_pdf))
//         .route("/ingresos/xlsx", web::get().to(handlers::reportes::ingresos_xlsx))
//         .route("/rentabilidad", web::get().to(handlers::reportes::rentabilidad))
//         .route("/rentabilidad/pdf", web::get().to(handlers::reportes::rentabilidad_pdf))
//         .route("/rentabilidad/xlsx", web::get().to(handlers::reportes::rentabilidad_xlsx))
//         .route("/historial-pagos", web::get().to(handlers::reportes::historial_pagos))
//         .route("/ocupacion/tendencia", web::get().to(handlers::reportes::ocupacion_tendencia))
//         .route("/pagos-export", web::get().to(handlers::reportes::pagos_export)),
// )
//
// Additionally, add this DTO to models/reporte.rs:
//
// #[derive(Debug, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct PagosExportQuery {
//     pub fecha_inicio: Option<NaiveDate>,
//     pub fecha_fin: Option<NaiveDate>,
// }
