// ── Route registration snippet ─────────────────────────────────
//
// Add this inside the `configure()` function in `backend/src/routes.rs`,
// within the existing `/documentos` scope. Place it BEFORE the existing
// `/{entity_type}/{entity_id}` routes (which are catch-all two-segment paths).
//
// The `/documentos` scope already has `write_governor_conf` applied,
// which provides rate limiting (2s/req, burst 20) — appropriate for
// upload operations per the security skill guidance.

// Inside the existing web::scope("/documentos"):
.route(
    "/upload",
    web::post().to(handlers::documentos_upload::upload),
)

// ── Full context showing placement ────────────────────────────
//
// .service(
//     web::scope("/documentos")
//         .wrap(Governor::new(&write_governor_conf))
//         // Static paths first
//         .route("/por-vencer", web::get().to(handlers::documentos::por_vencer))
//         .route("/plantillas", web::get().to(handlers::documentos::listar_plantillas))
//         // ─── NEW ROUTE ───
//         .route("/upload", web::post().to(handlers::documentos_upload::upload))
//         // ─── END NEW ROUTE ───
//         .route(
//             "/plantillas/{id}/rellenar/{entity_type}/{entity_id}",
//             web::get().to(handlers::documentos::rellenar_plantilla),
//         )
//         // ... rest of existing routes ...
//         .route(
//             "/{entity_type}/{entity_id}",
//             web::post().to(handlers::documentos::upload),
//         )
//         .route(
//             "/{entity_type}/{entity_id}",
//             web::get().to(handlers::documentos::listar),
//         ),
// )
//
// ── Handler module registration ───────────────────────────────
//
// In `backend/src/handlers/mod.rs`, add:
//   pub mod documentos_upload;
//
// In `backend/src/services/mod.rs`, add:
//   pub mod documentos_upload;
//
// ── Actix multipart payload config (in app.rs) ────────────────
//
// To enforce the 10 MB limit at the framework level before the handler
// even starts processing, add a `MultipartConfig` to app data:
//
//   use actix_multipart::form::MultipartFormConfig;
//
//   .app_data(
//       actix_multipart::form::MultipartFormConfig::default()
//           .total_limit(10 * 1024 * 1024) // 10 MB total
//   )
//
// This provides defense-in-depth: the framework rejects oversized payloads
// before our handler code runs, reducing DoS surface.
