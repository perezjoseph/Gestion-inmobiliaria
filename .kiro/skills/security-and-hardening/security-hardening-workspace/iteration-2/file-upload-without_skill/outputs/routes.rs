// Route registration for the new POST /api/v1/documentos/upload endpoint.
//
// This would be added inside the existing `/documentos` scope in routes.rs,
// BEFORE the dynamic `/{entity_type}/{entity_id}` routes to avoid path conflicts.
//
// In the existing `web::scope("/documentos")` block in routes.rs, add:

.route(
    "/upload",
    web::post().to(handlers::documentos::upload_documento),
)

// Full context — the documentos scope would look like:
//
// .service(
//     web::scope("/documentos")
//         .wrap(Governor::new(&write_governor_conf))
//         // Static paths first
//         .route("/por-vencer", web::get().to(handlers::documentos::por_vencer))
//         .route("/plantillas", web::get().to(handlers::documentos::listar_plantillas))
//         .route("/upload", web::post().to(handlers::documentos::upload_documento))  // <-- NEW
//         // ... rest of existing routes ...
//         // Dynamic paths (must come last)
//         .route("/{entity_type}/{entity_id}", web::post().to(handlers::documentos::upload))
//         .route("/{entity_type}/{entity_id}", web::get().to(handlers::documentos::listar))
// )
//
// The route is placed among the static paths to ensure it matches before the
// dynamic `/{entity_type}/{entity_id}` pattern.
