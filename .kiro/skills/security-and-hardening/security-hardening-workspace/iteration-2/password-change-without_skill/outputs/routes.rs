// In backend/src/routes.rs, add the new route to the existing /usuarios scope.
//
// Current /usuarios scope:
//
//   .service(
//       web::scope("/usuarios")
//           .wrap(Governor::new(&write_governor_conf))
//           .route("", web::get().to(handlers::usuarios::list))
//           .route("/{id}/rol", web::put().to(handlers::usuarios::cambiar_rol))
//           .route("/{id}/activar", web::put().to(handlers::usuarios::activar))
//           .route("/{id}/desactivar", web::put().to(handlers::usuarios::desactivar)),
//   )
//
// Updated /usuarios scope (add the cambiar-password route):
//
//   .service(
//       web::scope("/usuarios")
//           .wrap(Governor::new(&write_governor_conf))
//           .route("", web::get().to(handlers::usuarios::list))
//           .route("/{id}/rol", web::put().to(handlers::usuarios::cambiar_rol))
//           .route("/{id}/activar", web::put().to(handlers::usuarios::activar))
//           .route("/{id}/desactivar", web::put().to(handlers::usuarios::desactivar))
//           .route("/{id}/cambiar-password", web::post().to(handlers::usuarios::cambiar_password)),
//   )
//
// The route uses POST since it's a command/action, not an idempotent update of a resource field.
// Rate limiting is already applied via the write_governor_conf wrapper on the scope.
