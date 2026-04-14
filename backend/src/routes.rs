use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::web;

use crate::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    let auth_governor_conf = GovernorConfigBuilder::default()
        .seconds_per_request(6)
        .burst_size(10)
        .finish()
        .unwrap();

    cfg.service(
        web::scope("/api/v1")
            .service(
                web::scope("/auth")
                    .wrap(Governor::new(&auth_governor_conf))
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
                    .route(
                        "/por-vencer",
                        web::get().to(handlers::contratos::por_vencer),
                    )
                    .route("", web::get().to(handlers::contratos::list))
                    .route("", web::post().to(handlers::contratos::create))
                    .route("/{id}", web::get().to(handlers::contratos::get_by_id))
                    .route("/{id}", web::put().to(handlers::contratos::update))
                    .route("/{id}", web::delete().to(handlers::contratos::delete))
                    .route(
                        "/{id}/renovar",
                        web::post().to(handlers::contratos::renovar),
                    )
                    .route(
                        "/{id}/terminar",
                        web::post().to(handlers::contratos::terminar),
                    ),
            )
            .service(
                web::scope("/pagos")
                    .route("", web::get().to(handlers::pagos::list))
                    .route("", web::post().to(handlers::pagos::create))
                    .route("/{id}", web::get().to(handlers::pagos::get_by_id))
                    .route("/{id}", web::put().to(handlers::pagos::update))
                    .route("/{id}", web::delete().to(handlers::pagos::delete))
                    .route(
                        "/{id}/recibo",
                        web::get().to(handlers::recibos::generar_recibo),
                    ),
            )
            .service(
                web::scope("/gastos")
                    .route(
                        "/resumen-categorias",
                        web::get().to(handlers::gastos::resumen_categorias),
                    )
                    .route("", web::get().to(handlers::gastos::list))
                    .route("", web::post().to(handlers::gastos::create))
                    .route("/{id}", web::get().to(handlers::gastos::get_by_id))
                    .route("/{id}", web::put().to(handlers::gastos::update))
                    .route("/{id}", web::delete().to(handlers::gastos::delete)),
            )
            .service(
                web::scope("/mantenimiento")
                    .route("", web::get().to(handlers::mantenimiento::list))
                    .route("", web::post().to(handlers::mantenimiento::create))
                    .route("/{id}", web::get().to(handlers::mantenimiento::get_by_id))
                    .route("/{id}", web::put().to(handlers::mantenimiento::update))
                    .route(
                        "/{id}/estado",
                        web::put().to(handlers::mantenimiento::cambiar_estado),
                    )
                    .route("/{id}", web::delete().to(handlers::mantenimiento::delete))
                    .route(
                        "/{id}/notas",
                        web::post().to(handlers::mantenimiento::agregar_nota),
                    ),
            )
            .service(
                web::scope("/dashboard")
                    .route("/stats", web::get().to(handlers::dashboard::stats))
                    .route(
                        "/ocupacion-tendencia",
                        web::get().to(handlers::dashboard::ocupacion_tendencia),
                    )
                    .route(
                        "/ingresos-comparacion",
                        web::get().to(handlers::dashboard::ingresos_comparacion),
                    )
                    .route(
                        "/pagos-proximos",
                        web::get().to(handlers::dashboard::pagos_proximos),
                    )
                    .route(
                        "/contratos-calendario",
                        web::get().to(handlers::dashboard::contratos_calendario),
                    )
                    .route(
                        "/gastos-comparacion",
                        web::get().to(handlers::dashboard::gastos_comparacion),
                    ),
            )
            .service(web::scope("/auditoria").route("", web::get().to(handlers::auditoria::list)))
            .service(
                web::scope("/usuarios")
                    .route("", web::get().to(handlers::usuarios::list))
                    .route("/{id}/rol", web::put().to(handlers::usuarios::cambiar_rol))
                    .route("/{id}/activar", web::put().to(handlers::usuarios::activar))
                    .route(
                        "/{id}/desactivar",
                        web::put().to(handlers::usuarios::desactivar),
                    ),
            )
            .service(
                web::scope("/perfil")
                    .route("", web::get().to(handlers::perfil::obtener))
                    .route("", web::put().to(handlers::perfil::actualizar))
                    .route(
                        "/password",
                        web::put().to(handlers::perfil::cambiar_password),
                    ),
            )
            .service(web::scope("/notificaciones").route(
                "/pagos-vencidos",
                web::get().to(handlers::notificaciones::pagos_vencidos),
            ))
            .service(
                web::scope("/reportes")
                    .route("/ingresos", web::get().to(handlers::reportes::ingresos))
                    .route(
                        "/ingresos/pdf",
                        web::get().to(handlers::reportes::ingresos_pdf),
                    )
                    .route(
                        "/ingresos/xlsx",
                        web::get().to(handlers::reportes::ingresos_xlsx),
                    )
                    .route(
                        "/rentabilidad",
                        web::get().to(handlers::reportes::rentabilidad),
                    )
                    .route(
                        "/rentabilidad/pdf",
                        web::get().to(handlers::reportes::rentabilidad_pdf),
                    )
                    .route(
                        "/rentabilidad/xlsx",
                        web::get().to(handlers::reportes::rentabilidad_xlsx),
                    )
                    .route(
                        "/historial-pagos",
                        web::get().to(handlers::reportes::historial_pagos),
                    )
                    .route(
                        "/ocupacion/tendencia",
                        web::get().to(handlers::reportes::ocupacion_tendencia),
                    ),
            )
            .service(
                web::scope("/documentos")
                    .route(
                        "/{entity_type}/{entity_id}",
                        web::post().to(handlers::documentos::upload),
                    )
                    .route(
                        "/{entity_type}/{entity_id}",
                        web::get().to(handlers::documentos::listar),
                    ),
            )
            .service(
                web::scope("/configuracion")
                    .route(
                        "/moneda",
                        web::get().to(handlers::configuracion::obtener_moneda),
                    )
                    .route(
                        "/moneda",
                        web::put().to(handlers::configuracion::actualizar_moneda),
                    ),
            )
            .service(
                web::scope("/importar")
                    .route(
                        "/propiedades",
                        web::post().to(handlers::importacion::importar_propiedades),
                    )
                    .route(
                        "/inquilinos",
                        web::post().to(handlers::importacion::importar_inquilinos),
                    )
                    .route(
                        "/pagos",
                        web::post().to(handlers::importacion::importar_pagos),
                    )
                    .route(
                        "/gastos",
                        web::post().to(handlers::importacion::importar_gastos),
                    )
                    .service(
                        web::scope("/ocr")
                            .route(
                                "/confirmar",
                                web::post().to(handlers::importacion::confirmar_preview),
                            )
                            .route(
                                "/preview/{preview_id}",
                                web::delete().to(handlers::importacion::descartar_preview),
                            ),
                    ),
            ),
    );
}
