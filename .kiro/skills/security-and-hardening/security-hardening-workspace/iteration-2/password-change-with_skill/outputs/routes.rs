// Add this route inside the existing `/usuarios` scope in routes.rs:
//
// .service(
//     web::scope("/usuarios")
//         .wrap(Governor::new(&write_governor_conf))
//         .route("", web::get().to(handlers::usuarios::list))
//         .route("/{id}/rol", web::put().to(handlers::usuarios::cambiar_rol))
//         .route("/{id}/activar", web::put().to(handlers::usuarios::activar))
//         .route(
//             "/{id}/desactivar",
//             web::put().to(handlers::usuarios::desactivar),
//         )
//         // NEW: password change endpoint
//         .route(
//             "/{id}/cambiar-password",
//             web::post().to(handlers::usuarios::cambiar_password),
//         ),
// )

// Full scope block for reference:
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
        .route(
            "/{id}/cambiar-password",
            web::post().to(handlers::usuarios::cambiar_password),
        ),
)
