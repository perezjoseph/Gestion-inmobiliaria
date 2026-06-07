use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::web;

use crate::handlers;
use crate::middleware::rate_limit::FallbackPeerIpKeyExtractor;

pub fn configure(cfg: &mut web::ServiceConfig) {
    #[allow(clippy::unwrap_used)]
    let auth_governor_conf = GovernorConfigBuilder::default()
        .key_extractor(FallbackPeerIpKeyExtractor)
        .seconds_per_request(6)
        .burst_size(10)
        .finish()
        .unwrap();

    #[allow(clippy::unwrap_used)]
    let write_governor_conf = GovernorConfigBuilder::default()
        .key_extractor(FallbackPeerIpKeyExtractor)
        .seconds_per_request(2)
        .burst_size(20)
        .finish()
        .unwrap();

    #[allow(clippy::unwrap_used)]
    let firmas_governor_conf = GovernorConfigBuilder::default()
        .key_extractor(FallbackPeerIpKeyExtractor)
        .seconds_per_request(6)
        .burst_size(5)
        .finish()
        .unwrap();

    #[allow(clippy::unwrap_used)]
    let webhook_governor_conf = GovernorConfigBuilder::default()
        .key_extractor(FallbackPeerIpKeyExtractor)
        .seconds_per_request(1)
        .burst_size(30)
        .finish()
        .unwrap();

    cfg.service(
        web::scope("/internal/whatsapp")
            .wrap(Governor::new(&webhook_governor_conf))
            .route(
                "/incoming",
                web::post().to(handlers::chatbot_internal::incoming_webhook),
            ),
    );

    cfg.service(
        web::scope("/api/v1")
            .service(
                web::scope("/firmas")
                    .wrap(Governor::new(&firmas_governor_conf))
                    .route(
                        "/{token}/verificar",
                        web::post().to(handlers::firmas::verificar_firma_publica),
                    )
                    .route(
                        "/{token}/firmar",
                        web::post().to(handlers::firmas::firmar_publica),
                    ),
            )
            .service(
                web::scope("/auth")
                    .wrap(Governor::new(&auth_governor_conf))
                    .route("/register", web::post().to(handlers::auth::register))
                    .route("/login", web::post().to(handlers::auth::login)),
            )
            .service(
                web::scope("/organizacion")
                    .wrap(Governor::new(&write_governor_conf))
                    .route("", web::get().to(handlers::organizaciones::get))
                    .route("", web::put().to(handlers::organizaciones::update))
                    .service(
                        web::scope("/fiscal")
                            .route(
                                "/tipo-fiscal",
                                web::put().to(handlers::fiscal::actualizar_tipo_fiscal),
                            )
                            .route(
                                "/estado",
                                web::get().to(handlers::fiscal::obtener_estado_fiscal),
                            ),
                    ),
            )
            .service(
                web::scope("/invitaciones")
                    .wrap(Governor::new(&write_governor_conf))
                    .route("", web::post().to(handlers::invitaciones::crear))
                    .route("", web::get().to(handlers::invitaciones::listar))
                    .route("/{id}", web::delete().to(handlers::invitaciones::revocar)),
            )
            .service(
                web::scope("/propiedades")
                    .wrap(Governor::new(&write_governor_conf))
                    .route("", web::get().to(handlers::propiedades::list))
                    .route("", web::post().to(handlers::propiedades::create))
                    .route("/{id}", web::get().to(handlers::propiedades::get_by_id))
                    .route("/{id}", web::put().to(handlers::propiedades::update))
                    .route("/{id}", web::delete().to(handlers::propiedades::delete))
                    .service(
                        web::scope("/{propiedad_id}/unidades")
                            .route("", web::get().to(handlers::unidades::list))
                            .route("", web::post().to(handlers::unidades::create))
                            .route("/{id}", web::get().to(handlers::unidades::get_by_id))
                            .route("/{id}", web::put().to(handlers::unidades::update))
                            .route("/{id}", web::delete().to(handlers::unidades::delete))
                            .route(
                                "/{id}/servicios",
                                web::get()
                                    .to(handlers::servicios_publicos::obtener_responsabilidades),
                            )
                            .route(
                                "/{id}/servicios",
                                web::put().to(
                                    handlers::servicios_publicos::actualizar_responsabilidad_unidad,
                                ),
                            ),
                    )
                    .service(
                        web::scope("/{propiedad_id}/condominios")
                            .route(
                                "",
                                web::post().to(handlers::condominios::crear_cuota_handler),
                            )
                            .route(
                                "",
                                web::get().to(handlers::condominios::listar_cuotas_handler),
                            )
                            .route(
                                "/{cuota_id}",
                                web::put().to(handlers::condominios::actualizar_cuota_handler),
                            )
                            .route(
                                "/{cuota_id}",
                                web::delete().to(handlers::condominios::eliminar_cuota_handler),
                            ),
                    ),
            )
            .service(
                web::scope("/inquilinos")
                    .wrap(Governor::new(&write_governor_conf))
                    .route("", web::get().to(handlers::inquilinos::list))
                    .route("", web::post().to(handlers::inquilinos::create))
                    .route("/{id}", web::get().to(handlers::inquilinos::get_by_id))
                    .route("/{id}", web::put().to(handlers::inquilinos::update))
                    .route("/{id}", web::delete().to(handlers::inquilinos::delete)),
            )
            .service(
                web::scope("/contratos")
                    .wrap(Governor::new(&write_governor_conf))
                    .route(
                        "/por-vencer",
                        web::get().to(handlers::contratos::por_vencer),
                    )
                    .route("", web::get().to(handlers::contratos::list))
                    .route("", web::post().to(handlers::contratos::create))
                    .route(
                        "/{id}/deposito",
                        web::put().to(handlers::contratos::cambiar_estado_deposito),
                    )
                    .route("/{id}", web::get().to(handlers::contratos::get_by_id))
                    .route("/{id}", web::put().to(handlers::contratos::update))
                    .route("/{id}", web::delete().to(handlers::contratos::delete))
                    .route(
                        "/{id}/renovar",
                        web::post().to(handlers::contratos::renovar),
                    )
                    .route(
                        "/{id}/sugerir-renovacion",
                        web::get().to(handlers::contratos::sugerir_renovacion),
                    )
                    .route(
                        "/{id}/terminar",
                        web::post().to(handlers::contratos::terminar),
                    )
                    .route(
                        "/{id}/pagos/preview",
                        web::get().to(handlers::contratos::preview_pagos),
                    )
                    .route(
                        "/{id}/pagos/generar",
                        web::post().to(handlers::contratos::generar_pagos),
                    )
                    .route(
                        "/{id}/servicios",
                        web::put()
                            .to(handlers::servicios_publicos::actualizar_responsabilidad_contrato),
                    ),
            )
            .service(
                web::scope("/pagos")
                    .wrap(Governor::new(&write_governor_conf))
                    .route(
                        "/bulk/marcar-pagado",
                        web::post().to(handlers::pagos::bulk_marcar_pagado),
                    )
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
                    .wrap(Governor::new(&write_governor_conf))
                    .route(
                        "/resumen-categorias",
                        web::get().to(handlers::gastos::resumen_categorias),
                    )
                    .route(
                        "/recurrentes",
                        web::get().to(handlers::gastos_recurrentes::list),
                    )
                    .route(
                        "/recurrentes",
                        web::post().to(handlers::gastos_recurrentes::create),
                    )
                    .route(
                        "/recurrentes/{id}",
                        web::get().to(handlers::gastos_recurrentes::get_by_id),
                    )
                    .route(
                        "/recurrentes/{id}",
                        web::put().to(handlers::gastos_recurrentes::update),
                    )
                    .route(
                        "/recurrentes/{id}",
                        web::delete().to(handlers::gastos_recurrentes::delete),
                    )
                    .route("", web::get().to(handlers::gastos::list))
                    .route("", web::post().to(handlers::gastos::create))
                    .route("/{id}", web::get().to(handlers::gastos::get_by_id))
                    .route("/{id}", web::put().to(handlers::gastos::update))
                    .route("/{id}", web::delete().to(handlers::gastos::delete)),
            )
            .service(
                web::scope("/mantenimiento")
                    .wrap(Governor::new(&write_governor_conf))
                    .route(
                        "/programado",
                        web::get().to(handlers::mantenimiento_programado::list),
                    )
                    .route(
                        "/programado",
                        web::post().to(handlers::mantenimiento_programado::create),
                    )
                    .route(
                        "/programado/{id}",
                        web::get().to(handlers::mantenimiento_programado::get_by_id),
                    )
                    .route(
                        "/programado/{id}",
                        web::put().to(handlers::mantenimiento_programado::update),
                    )
                    .route(
                        "/programado/{id}",
                        web::delete().to(handlers::mantenimiento_programado::delete),
                    )
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
                    .wrap(Governor::new(&write_governor_conf))
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
            .service(
                web::scope("/auditoria")
                    .wrap(Governor::new(&write_governor_conf))
                    .route("", web::get().to(handlers::auditoria::list)),
            )
            .service(
                web::scope("/usuarios")
                    .wrap(Governor::new(&write_governor_conf))
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
                    .wrap(Governor::new(&write_governor_conf))
                    .route("", web::get().to(handlers::perfil::obtener))
                    .route("", web::put().to(handlers::perfil::actualizar))
                    .route(
                        "/password",
                        web::put().to(handlers::perfil::cambiar_password),
                    ),
            )
            .service(
                web::scope("/notificaciones")
                    .wrap(Governor::new(&write_governor_conf))
                    .route(
                        "/pagos-vencidos",
                        web::get().to(handlers::notificaciones::pagos_vencidos),
                    )
                    .route(
                        "/no-leidas/conteo",
                        web::get().to(handlers::notificaciones::conteo_no_leidas),
                    )
                    .route(
                        "/leer-todas",
                        web::put().to(handlers::notificaciones::marcar_todas_leidas),
                    )
                    .route(
                        "/generar",
                        web::post().to(handlers::notificaciones::generar),
                    )
                    .route(
                        "/{id}/leer",
                        web::put().to(handlers::notificaciones::marcar_leida),
                    )
                    .route("", web::get().to(handlers::notificaciones::listar)),
            )
            .service(
                web::scope("/reportes")
                    .wrap(Governor::new(&write_governor_conf))
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
                    .wrap(Governor::new(&write_governor_conf))
                    // Static paths first
                    .route(
                        "/por-vencer",
                        web::get().to(handlers::documentos::por_vencer),
                    )
                    .route(
                        "/plantillas",
                        web::get().to(handlers::documentos::listar_plantillas),
                    )
                    .route(
                        "/plantillas",
                        web::post().to(handlers::documentos::crear_plantilla),
                    )
                    .route(
                        "/plantillas/{id}",
                        web::get().to(handlers::documentos::obtener_plantilla),
                    )
                    .route(
                        "/plantillas/{id}",
                        web::put().to(handlers::documentos::actualizar_plantilla),
                    )
                    .route(
                        "/plantillas/{id}",
                        web::delete().to(handlers::documentos::eliminar_plantilla),
                    )
                    // Parameterized static paths
                    .route(
                        "/plantillas/{id}/rellenar/{entity_type}/{entity_id}",
                        web::get().to(handlers::documentos::rellenar_plantilla),
                    )
                    .route(
                        "/cumplimiento/resumen",
                        web::get().to(handlers::documentos::cumplimiento_resumen),
                    )
                    .route(
                        "/cumplimiento/{entity_type}/{entity_id}",
                        web::get().to(handlers::documentos::cumplimiento),
                    )
                    .route(
                        "/digitalizar/{entity_type}/{entity_id}",
                        web::post().to(handlers::documentos::digitalizar),
                    )
                    // Dynamic paths
                    .route(
                        "/{id}/verificar",
                        web::put().to(handlers::documentos::verificar),
                    )
                    .route(
                        "/{id}/contenido",
                        web::put().to(handlers::documentos::guardar_contenido),
                    )
                    .route(
                        "/{id}/exportar-pdf",
                        web::get().to(handlers::documentos::exportar_pdf),
                    )
                    .route(
                        "/{id}/exportar-docx",
                        web::get().to(handlers::documentos::exportar_docx),
                    )
                    // Signature routes (authenticated)
                    .route("/{id}/firmar", web::post().to(handlers::firmas::firmar))
                    .route(
                        "/{id}/solicitar-firma",
                        web::post().to(handlers::firmas::solicitar_firma),
                    )
                    .route(
                        "/{id}/firmas",
                        web::get().to(handlers::firmas::listar_firmas),
                    )
                    .route("/{id}", web::delete().to(handlers::documentos::eliminar))
                    // Existing routes (most dynamic — two path segments)
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
                web::scope("/chatbot")
                    .wrap(Governor::new(&write_governor_conf))
                    .route("/config", web::get().to(handlers::chatbot::get_config))
                    .route("/config", web::put().to(handlers::chatbot::update_config))
                    .route("/connect", web::post().to(handlers::chatbot::connect))
                    .route("/disconnect", web::post().to(handlers::chatbot::disconnect))
                    .route("/status", web::get().to(handlers::chatbot::status))
                    .route("/test", web::post().to(handlers::chatbot::test_chat))
                    .route(
                        "/test/stream",
                        web::post().to(handlers::chatbot::test_chat_stream),
                    )
                    .route(
                        "/handoff/clear",
                        web::post().to(handlers::chatbot::clear_handoff),
                    )
                    .route(
                        "/conversations",
                        web::get().to(handlers::chatbot::list_conversations),
                    )
                    .route(
                        "/conversations/{phone}",
                        web::get().to(handlers::chatbot::get_conversation_history),
                    )
                    .route(
                        "/receipts/pending",
                        web::get().to(handlers::chatbot::list_pending_receipts),
                    )
                    .route(
                        "/receipts/{id}/confirm",
                        web::post().to(handlers::chatbot::confirm_receipt),
                    )
                    .route(
                        "/receipts/{id}/reject",
                        web::post().to(handlers::chatbot::reject_receipt),
                    ),
            )
            .service(
                web::scope("/chatbot/guidance-rules")
                    .wrap(Governor::new(&write_governor_conf))
                    .route(
                        "",
                        web::post().to(handlers::chatbot::create_guidance_rule_handler),
                    )
                    .route(
                        "/batch",
                        web::put().to(handlers::chatbot::batch_update_guidance_rules_handler),
                    )
                    .route(
                        "/{id}",
                        web::put().to(handlers::chatbot::update_guidance_rule_handler),
                    )
                    .route(
                        "/{id}",
                        web::delete().to(handlers::chatbot::delete_guidance_rule_handler),
                    ),
            )
            .service({
                #[allow(unused_mut)]
                let mut evals_scope =
                    web::scope("/chatbot/evals").wrap(Governor::new(&write_governor_conf));
                #[cfg(feature = "evals")]
                {
                    evals_scope = evals_scope
                        .route(
                            "/suites",
                            web::get().to(handlers::chatbot_evals::list_suites),
                        )
                        .route(
                            "/suites",
                            web::post().to(handlers::chatbot_evals::create_suite),
                        )
                        .route("/run", web::post().to(handlers::chatbot_evals::run_eval))
                        .route("/runs", web::get().to(handlers::chatbot_evals::list_runs))
                        .route(
                            "/runs/{id}",
                            web::get().to(handlers::chatbot_evals::get_run),
                        );
                }
                evals_scope
            })
            .service(
                web::scope("/configuracion")
                    .wrap(Governor::new(&write_governor_conf))
                    .route(
                        "/moneda",
                        web::get().to(handlers::configuracion::obtener_moneda),
                    )
                    .route(
                        "/moneda",
                        web::put().to(handlers::configuracion::actualizar_moneda),
                    )
                    .route(
                        "/recargo",
                        web::get().to(handlers::configuracion::obtener_recargo_defecto),
                    )
                    .route(
                        "/recargo",
                        web::put().to(handlers::configuracion::actualizar_recargo_defecto),
                    )
                    .route("/ipc", web::get().to(handlers::ipc::obtener_ipc))
                    .route("/ipc", web::put().to(handlers::ipc::actualizar_ipc)),
            )
            .service(
                web::scope("/importar")
                    .wrap(Governor::new(&write_governor_conf))
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
            )
            .service(
                web::scope("/ocr")
                    .wrap(Governor::new(&write_governor_conf))
                    .route("/extract", web::post().to(handlers::ocr::ocr_extract)),
            )
            .service(
                web::scope("/tareas")
                    .wrap(Governor::new(&write_governor_conf))
                    .route(
                        "/historial",
                        web::get().to(handlers::background_jobs::historial),
                    )
                    .route(
                        "/{nombre}/ejecutar",
                        web::post().to(handlers::background_jobs::ejecutar_tarea),
                    ),
            )
            .service(
                web::scope("/desahucios")
                    .wrap(Governor::new(&write_governor_conf))
                    .route("", web::get().to(handlers::desahucios::list))
                    .route("", web::post().to(handlers::desahucios::create))
                    .route("/{id}", web::put().to(handlers::desahucios::update)),
            )
            .service(
                web::scope("/dgii")
                    .wrap(Governor::new(&write_governor_conf))
                    .route("/consulta", web::get().to(handlers::dgii::consultar_rnc))
                    .route(
                        "/consulta/nombre",
                        web::get().to(handlers::dgii::consultar_nombre),
                    )
                    .route(
                        "/cache/{rnc}",
                        web::delete().to(handlers::dgii::invalidar_cache),
                    ),
            )
            .service(
                web::scope("/recibos-informales")
                    .wrap(Governor::new(&write_governor_conf))
                    .route("", web::post().to(handlers::recibos_informales::crear)),
            )
            .service(
                web::scope("/ncf")
                    .wrap(Governor::new(&write_governor_conf))
                    .route(
                        "/secuencias",
                        web::get().to(handlers::ncf::listar_secuencias),
                    )
                    .route(
                        "/configurar-rango",
                        web::post().to(handlers::ncf::configurar_rango_handler),
                    )
                    .route("/alertas", web::get().to(handlers::ncf::obtener_alertas)),
            )
            .service(
                web::scope("/reportes-dgii")
                    .wrap(Governor::new(&write_governor_conf))
                    .route(
                        "/607",
                        web::post().to(handlers::reportes_dgii::generar_607_handler),
                    )
                    .route(
                        "/606",
                        web::post().to(handlers::reportes_dgii::generar_606_handler),
                    )
                    .route(
                        "/preview/{tipo}/{periodo}",
                        web::get().to(handlers::reportes_dgii::preview_reporte),
                    )
                    .route(
                        "/{id}/estado",
                        web::put().to(handlers::reportes_dgii::actualizar_estado),
                    ),
            )
            .service(
                web::scope("/ipi")
                    .wrap(Governor::new(&write_governor_conf))
                    .route(
                        "/calculo",
                        web::get().to(handlers::ipi::calcular_ipi_handler),
                    )
                    .route("/umbral", web::put().to(handlers::ipi::actualizar_umbral))
                    .route(
                        "/copropietarios/{propiedad_id}",
                        web::get().to(handlers::ipi::listar_copropietarios),
                    )
                    .route(
                        "/copropietarios",
                        web::post().to(handlers::ipi::crear_copropietario),
                    ),
            )
            .service(
                web::scope("/indexacion")
                    .wrap(Governor::new(&write_governor_conf))
                    .route(
                        "/propuesta/{contrato_id}",
                        web::get().to(handlers::indexacion::obtener_propuesta),
                    )
                    .route(
                        "/aprobar/{contrato_id}",
                        web::post().to(handlers::indexacion::aprobar_renovacion_handler),
                    )
                    .route(
                        "/proximos-vencer",
                        web::get().to(handlers::indexacion::proximos_vencer),
                    ),
            ),
    );
}
