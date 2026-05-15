// Route registration change in backend/src/routes.rs
//
// In the existing `/usuarios` scope, add the new route:

.service(
    web::scope("/usuarios")
        .wrap(Governor::new(&write_governor_conf))
        .route("", web::get().to(handlers::usuarios::list))
        .route("/{id}/rol", web::put().to(handlers::usuarios::cambiar_rol))
        .route("/{id}/activar", web::put().to(handlers::usuarios::activar))
        .route(
            "/{id}/desactivar",
            web::put().to(handlers::usuarios::desactivar),
        )
        // NEW: password change endpoint
        .route(
            "/{id}/cambiar-password",
            web::post().to(handlers::usuarios::cambiar_password),
        ),
)
