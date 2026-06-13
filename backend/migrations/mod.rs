#![allow(clippy::unnecessary_literal_bound)]
pub mod m20250408_000001_create_usuarios;
pub mod m20250408_000002_create_propiedades;
pub mod m20250408_000003_create_inquilinos;
pub mod m20250408_000004_create_contratos;
pub mod m20250408_000005_create_pagos;
pub mod m20250409_000001_create_registros_auditoria;
pub mod m20250409_000002_create_documentos;
pub mod m20250409_000003_create_configuracion;
pub mod m20250409_000004_add_documentos_columns;
pub mod m20250410_000001_create_unidades;
pub mod m20250411_000001_create_solicitudes_mantenimiento;
pub mod m20250411_000002_create_notas_mantenimiento;
pub mod m20250412_000001_create_gastos;
pub mod m20250413_000001_create_organizaciones;
pub mod m20250413_000002_add_organizacion_id;
pub mod m20250413_000003_create_invitaciones;
pub mod m20250430_000001_extend_documentos_legal;
pub mod m20250430_000002_add_documentos_editor;
pub mod m20250430_000003_create_plantillas_documento;
pub mod m20250501_000001_create_notificaciones;
pub mod m20250601_000001_create_ejecuciones_tareas;
pub mod m20250615_000001_add_deposit_tracking_to_contratos;
pub mod m20250615_000002_add_recargo_fields;
pub mod m20250620_000001_create_firmas_documento;
pub mod m20260415_001_add_documento_origen_id;
pub mod m20260415_002_add_gasto_utility_fields;
pub mod m20260509_000001_create_chatbot_tables;
pub mod m20260510_000001_make_auditoria_usuario_nullable;
pub mod m20260510_000002_create_whatsapp_auth_tables;
pub mod m20260512_000001_add_utility_fields_to_gastos;
pub mod m20260512_000002_create_desahucios;
pub mod m20260512_000003_create_responsabilidad_servicios;
pub mod m20260512_000004_create_cache_dgii;
pub mod m20260513_000001_create_preview_index;
pub mod m20260514_000001_add_agent_config_and_eval_tables;
pub mod m20260601_000001_add_tipo_fiscal_to_organizaciones;
pub mod m20260601_000002_add_fiscal_columns_to_pagos;
pub mod m20260601_000003_add_catastral_to_propiedades;
pub mod m20260601_000004_create_cuotas_condominio;
pub mod m20260601_000005_create_secuencias_ncf;
pub mod m20260601_000006_create_reportes_dgii;
pub mod m20260601_000007_create_configuraciones_ipi;
pub mod m20260601_000008_create_recibos_informales;
pub mod m20260601_000009_create_copropietarios;
pub mod m20260606_000001_add_missing_indexes;
pub mod m20260606_000002_create_gastos_recurrentes;
pub mod m20260606_000003_create_mantenimiento_programado;
pub mod m20260607_000001_add_guidance_rules_to_chatbot_config;
pub mod m20260607_000002_seed_guidance_rule_templates;
pub mod m20260608_000001_add_password_changed_at;
pub mod m20260609_000001_add_organizacion_id_to_plantillas;
pub mod m20260609_000002_add_performance_indexes;
pub mod m20260613_000001_add_overdue_payments_indexes;

use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250408_000001_create_usuarios::Migration),
            Box::new(m20250408_000002_create_propiedades::Migration),
            Box::new(m20250408_000003_create_inquilinos::Migration),
            Box::new(m20250408_000004_create_contratos::Migration),
            Box::new(m20250408_000005_create_pagos::Migration),
            Box::new(m20250409_000001_create_registros_auditoria::Migration),
            Box::new(m20250409_000002_create_documentos::Migration),
            Box::new(m20250409_000003_create_configuracion::Migration),
            Box::new(m20250409_000004_add_documentos_columns::Migration),
            Box::new(m20250410_000001_create_unidades::Migration),
            Box::new(m20250411_000001_create_solicitudes_mantenimiento::Migration),
            Box::new(m20250411_000002_create_notas_mantenimiento::Migration),
            Box::new(m20250412_000001_create_gastos::Migration),
            Box::new(m20250413_000001_create_organizaciones::Migration),
            Box::new(m20250413_000002_add_organizacion_id::Migration),
            Box::new(m20250413_000003_create_invitaciones::Migration),
            Box::new(m20250430_000001_extend_documentos_legal::Migration),
            Box::new(m20250430_000002_add_documentos_editor::Migration),
            Box::new(m20250430_000003_create_plantillas_documento::Migration),
            Box::new(m20250501_000001_create_notificaciones::Migration),
            Box::new(m20250601_000001_create_ejecuciones_tareas::Migration),
            Box::new(m20250615_000001_add_deposit_tracking_to_contratos::Migration),
            Box::new(m20250615_000002_add_recargo_fields::Migration),
            Box::new(m20250620_000001_create_firmas_documento::Migration),
            Box::new(m20260415_001_add_documento_origen_id::Migration),
            Box::new(m20260509_000001_create_chatbot_tables::Migration),
            Box::new(m20260510_000001_make_auditoria_usuario_nullable::Migration),
            Box::new(m20260510_000002_create_whatsapp_auth_tables::Migration),
            Box::new(m20260512_000001_add_utility_fields_to_gastos::Migration),
            Box::new(m20260512_000002_create_desahucios::Migration),
            Box::new(m20260512_000003_create_responsabilidad_servicios::Migration),
            Box::new(m20260512_000004_create_cache_dgii::Migration),
            Box::new(m20260415_002_add_gasto_utility_fields::Migration),
            Box::new(m20260513_000001_create_preview_index::Migration),
            Box::new(m20260514_000001_add_agent_config_and_eval_tables::Migration),
            Box::new(m20260606_000001_add_missing_indexes::Migration),
            Box::new(m20260606_000002_create_gastos_recurrentes::Migration),
            Box::new(m20260606_000003_create_mantenimiento_programado::Migration),
            Box::new(m20260601_000001_add_tipo_fiscal_to_organizaciones::Migration),
            Box::new(m20260601_000002_add_fiscal_columns_to_pagos::Migration),
            Box::new(m20260601_000003_add_catastral_to_propiedades::Migration),
            Box::new(m20260601_000004_create_cuotas_condominio::Migration),
            Box::new(m20260601_000005_create_secuencias_ncf::Migration),
            Box::new(m20260601_000006_create_reportes_dgii::Migration),
            Box::new(m20260601_000007_create_configuraciones_ipi::Migration),
            Box::new(m20260601_000008_create_recibos_informales::Migration),
            Box::new(m20260601_000009_create_copropietarios::Migration),
            Box::new(m20260607_000001_add_guidance_rules_to_chatbot_config::Migration),
            Box::new(m20260607_000002_seed_guidance_rule_templates::Migration),
            Box::new(m20260608_000001_add_password_changed_at::Migration),
            Box::new(m20260609_000001_add_organizacion_id_to_plantillas::Migration),
            Box::new(m20260609_000002_add_performance_indexes::Migration),
            Box::new(m20260613_000001_add_overdue_payments_indexes::Migration),
        ]
    }
}
