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
        ]
    }
}
