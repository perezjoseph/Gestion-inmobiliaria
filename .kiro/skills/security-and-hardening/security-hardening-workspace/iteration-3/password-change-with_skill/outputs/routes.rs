// In routes.rs, within the existing /usuarios scope, add the new route.
// The /usuarios scope already has a write_governor_conf rate limiter applied.
//
// Existing:
//   .service(
//       web::scope("/usuarios")
//           .wrap(Governor::new(&write_governor_conf))
//           .route("", web::get().to(handlers::usuarios::list))
//           .route("/{id}/rol", web::put().to(handlers::usuarios::cambiar_rol))
//           .route("/{id}/activar", web::put().to(handlers::usuarios::activar))
//           .route("/{id}/desactivar", web::put().to(handlers::usuarios::desactivar)),
//   )
//
// Updated (add one line):
//   .service(
//       web::scope("/usuarios")
//           .wrap(Governor::new(&write_governor_conf))
//           .route("", web::get().to(handlers::usuarios::list))
//           .route("/{id}/rol", web::put().to(handlers::usuarios::cambiar_rol))
//           .route("/{id}/activar", web::put().to(handlers::usuarios::activar))
//           .route("/{id}/desactivar", web::put().to(handlers::usuarios::desactivar))
//           .route("/{id}/cambiar-password", web::post().to(handlers::usuarios::cambiar_password)),
//   )

// The route registration diff:
// Add this line after the /{id}/desactivar route:
//
//     .route("/{id}/cambiar-password", web::post().to(handlers::usuarios::cambiar_password))
//
// This inherits the write_governor_conf rate limiter (2s/req, burst 20)
// which is appropriate for a password-change operation that involves argon2 hashing.
