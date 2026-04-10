use actix_web::web;

use crate::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(
                web::scope("/auth")
                    .route("/register", web::post().to(handlers::auth::register))
                    .route("/login", web::post().to(handlers::auth::login)),
            )
            .service(
                web::scope("/propiedades")
                    .route("", web::get().to(handlers::propiedades::list))
                    .route("", web::post().to(handlers::propiedades::create))
                    .route("/{id}", web::get().to(handlers::propiedades::get_by_id))
                    .route("/{id}", web::put().to(handlers::propiedades::update))
                    .route("/{id}", web::delete().to(handlers::propiedades::delete)),
            )
            .service(
                web::scope("/inquilinos")
                    .route("", web::get().to(handlers::inquilinos::list))
                    .route("", web::post().to(handlers::inquilinos::create))
                    .route("/{id}", web::get().to(handlers::inquilinos::get_by_id))
                    .route("/{id}", web::put().to(handlers::inquilinos::update))
                    .route("/{id}", web::delete().to(handlers::inquilinos::delete)),
            )
            .service(
                web::scope("/contratos")
                    .route("", web::get().to(handlers::contratos::list))
                    .route("", web::post().to(handlers::contratos::create))
                    .route("/{id}", web::get().to(handlers::contratos::get_by_id))
                    .route("/{id}", web::put().to(handlers::contratos::update))
                    .route("/{id}", web::delete().to(handlers::contratos::delete)),
            )
            .service(
                web::scope("/pagos")
                    .route("", web::get().to(handlers::pagos::list))
                    .route("", web::post().to(handlers::pagos::create))
                    .route("/{id}", web::get().to(handlers::pagos::get_by_id))
                    .route("/{id}", web::put().to(handlers::pagos::update))
                    .route("/{id}", web::delete().to(handlers::pagos::delete)),
            )
            .service(
                web::scope("/dashboard").route("/stats", web::get().to(handlers::dashboard::stats)),
            ),
    );
}
