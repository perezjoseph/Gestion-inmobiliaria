// Add this route inside the existing `/api/v1/reportes` scope in routes.rs:
//
// .service(
//     web::scope("/reportes")
//         .wrap(Governor::new(&write_governor_conf))  // Rate limit exports (skill: expensive operation)
//         .route("/ingresos", web::get().to(handlers::reportes::ingresos))
//         .route("/ingresos/pdf", web::get().to(handlers::reportes::ingresos_pdf))
//         .route("/ingresos/xlsx", web::get().to(handlers::reportes::ingresos_xlsx))
//         .route("/rentabilidad", web::get().to(handlers::reportes::rentabilidad))
//         .route("/rentabilidad/pdf", web::get().to(handlers::reportes::rentabilidad_pdf))
//         .route("/rentabilidad/xlsx", web::get().to(handlers::reportes::rentabilidad_xlsx))
//         .route("/historial-pagos", web::get().to(handlers::reportes::historial_pagos))
//         .route("/ocupacion/tendencia", web::get().to(handlers::reportes::ocupacion_tendencia))
//         .route("/pagos-export", web::get().to(handlers::reportes::pagos_export))  // NEW
// )

// Key change: wrap the /reportes scope with write_governor to rate-limit all export endpoints.
// The existing scope did NOT have a rate limiter — this is a security improvement.
//
// If adding the governor to the entire scope is too disruptive (it would rate-limit
// the JSON endpoints too), an alternative is to nest the export under a sub-scope:
//
// .service(
//     web::scope("/reportes")
//         .route("/ingresos", web::get().to(handlers::reportes::ingresos))
//         // ... existing routes ...
//         .service(
//             web::scope("")
//                 .wrap(Governor::new(&write_governor_conf))
//                 .route("/pagos-export", web::get().to(handlers::reportes::pagos_export))
//                 .route("/ingresos/pdf", web::get().to(handlers::reportes::ingresos_pdf))
//                 .route("/ingresos/xlsx", web::get().to(handlers::reportes::ingresos_xlsx))
//                 .route("/rentabilidad/pdf", web::get().to(handlers::reportes::rentabilidad_pdf))
//                 .route("/rentabilidad/xlsx", web::get().to(handlers::reportes::rentabilidad_xlsx))
//         )
// )

// Minimal addition (just the new route line to add to the existing scope):
.route("/pagos-export", web::get().to(handlers::reportes::pagos_export))
