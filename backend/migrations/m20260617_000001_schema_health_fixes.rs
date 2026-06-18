use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260617_000001_schema_health_fixes"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_gastos_propiedad_id")
                    .table(Gastos::Table)
                    .col(Gastos::PropiedadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_gastos_unidad_id")
                    .table(Gastos::Table)
                    .col(Gastos::UnidadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_unidades_propiedad_id")
                    .table(Unidades::Table)
                    .col(Unidades::PropiedadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_copropietarios_propiedad_id")
                    .table(Copropietarios::Table)
                    .col(Copropietarios::PropiedadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_cuotas_condominio_propiedad_id")
                    .table(CuotasCondominio::Table)
                    .col(CuotasCondominio::PropiedadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_cuotas_condominio_contrato_id")
                    .table(CuotasCondominio::Table)
                    .col(CuotasCondominio::ContratoId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_gasto_recurrente_propiedad_id")
                    .table(GastoRecurrente::Table)
                    .col(GastoRecurrente::PropiedadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_gasto_recurrente_unidad_id")
                    .table(GastoRecurrente::Table)
                    .col(GastoRecurrente::UnidadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_mantenimiento_prog_propiedad_id")
                    .table(MantenimientoProgramado::Table)
                    .col(MantenimientoProgramado::PropiedadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_mantenimiento_prog_unidad_id")
                    .table(MantenimientoProgramado::Table)
                    .col(MantenimientoProgramado::UnidadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_desahucios_contrato_id")
                    .table(Desahucios::Table)
                    .col(Desahucios::ContratoId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_responsabilidad_servicios_unidad_id")
                    .table(ResponsabilidadServicios::Table)
                    .col(ResponsabilidadServicios::UnidadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_responsabilidad_servicios_contrato_id")
                    .table(ResponsabilidadServicios::Table)
                    .col(ResponsabilidadServicios::ContratoId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_notas_mantenimiento_solicitud_id")
                    .table(NotasMantenimiento::Table)
                    .col(NotasMantenimiento::SolicitudId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_notas_mantenimiento_autor_id")
                    .table(NotasMantenimiento::Table)
                    .col(NotasMantenimiento::AutorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_chatbot_conversation_org_sender")
                    .table(ChatbotConversation::Table)
                    .col(ChatbotConversation::OrganizacionId)
                    .col(ChatbotConversation::SenderPhone)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_chatbot_conversation_organizacion_id")
                    .table(ChatbotConversation::Table)
                    .col(ChatbotConversation::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_chatbot_receipt_conversation_id")
                    .table(ChatbotReceiptExtraction::Table)
                    .col(ChatbotReceiptExtraction::ConversationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_chatbot_receipt_inquilino_id")
                    .table(ChatbotReceiptExtraction::Table)
                    .col(ChatbotReceiptExtraction::InquilinoId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_notificaciones_usuario_id")
                    .table(Notificaciones::Table)
                    .col(Notificaciones::UsuarioId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_notificaciones_organizacion_id")
                    .table(Notificaciones::Table)
                    .col(Notificaciones::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_documentos_uploaded_by")
                    .table(Documentos::Table)
                    .col(Documentos::UploadedBy)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_firmas_documento_documento_id")
                    .table(FirmasDocumento::Table)
                    .col(FirmasDocumento::DocumentoId)
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();

        db.execute_unprepared(
            "DO $$ BEGIN \
             IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_contratos_estado') THEN \
             ALTER TABLE contratos ADD CONSTRAINT chk_contratos_estado \
             CHECK (estado IN ('activo', 'vencido', 'cancelado', 'finalizado', 'terminado')); \
             END IF; END $$",
        )
        .await?;

        db.execute_unprepared(
            "DO $$ BEGIN \
             IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_pagos_estado') THEN \
             ALTER TABLE pagos ADD CONSTRAINT chk_pagos_estado \
             CHECK (estado IN ('pendiente', 'pagado', 'atrasado', 'cancelado')); \
             END IF; END $$",
        )
        .await?;

        db.execute_unprepared(
            "DO $$ BEGIN \
             IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_gastos_estado') THEN \
             ALTER TABLE gastos ADD CONSTRAINT chk_gastos_estado \
             CHECK (estado IN ('pendiente', 'pagado', 'cancelado')); \
             END IF; END $$",
        )
        .await?;

        db.execute_unprepared(
            "DO $$ BEGIN \
             IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chk_solicitudes_mant_estado') THEN \
             ALTER TABLE solicitudes_mantenimiento ADD CONSTRAINT chk_solicitudes_mant_estado \
             CHECK (estado IN ('pendiente', 'en_progreso', 'completado')); \
             END IF; END $$",
        )
        .await?;

        db.execute_unprepared(
            "ALTER TABLE registros_auditoria ADD COLUMN IF NOT EXISTS organizacion_id UUID",
        )
        .await?;

        db.execute_unprepared(
            "DO $$ BEGIN \
             IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_registros_auditoria_organizacion') THEN \
             ALTER TABLE registros_auditoria \
             ADD CONSTRAINT fk_registros_auditoria_organizacion \
             FOREIGN KEY (organizacion_id) REFERENCES organizaciones(id) ON DELETE SET NULL; \
             END IF; END $$",
        )
        .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_registros_auditoria_organizacion_id")
                    .table(RegistrosAuditoria::Table)
                    .col(RegistrosAuditoria::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            "ALTER TABLE registros_auditoria DROP CONSTRAINT IF EXISTS fk_registros_auditoria_organizacion",
        )
        .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_registros_auditoria_organizacion_id")
                    .table(RegistrosAuditoria::Table)
                    .to_owned(),
            )
            .await?;

        db.execute_unprepared(
            "ALTER TABLE registros_auditoria DROP COLUMN IF EXISTS organizacion_id",
        )
        .await?;

        db.execute_unprepared(
            "ALTER TABLE solicitudes_mantenimiento DROP CONSTRAINT IF EXISTS chk_solicitudes_mant_estado",
        )
        .await?;

        db.execute_unprepared("ALTER TABLE gastos DROP CONSTRAINT IF EXISTS chk_gastos_estado")
            .await?;

        db.execute_unprepared("ALTER TABLE pagos DROP CONSTRAINT IF EXISTS chk_pagos_estado")
            .await?;

        db.execute_unprepared(
            "ALTER TABLE contratos DROP CONSTRAINT IF EXISTS chk_contratos_estado",
        )
        .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_firmas_documento_documento_id")
                    .table(FirmasDocumento::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_documentos_uploaded_by")
                    .table(Documentos::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_notificaciones_organizacion_id")
                    .table(Notificaciones::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_notificaciones_usuario_id")
                    .table(Notificaciones::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_chatbot_receipt_inquilino_id")
                    .table(ChatbotReceiptExtraction::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_chatbot_receipt_conversation_id")
                    .table(ChatbotReceiptExtraction::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_chatbot_conversation_organizacion_id")
                    .table(ChatbotConversation::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_chatbot_conversation_org_sender")
                    .table(ChatbotConversation::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_notas_mantenimiento_autor_id")
                    .table(NotasMantenimiento::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_notas_mantenimiento_solicitud_id")
                    .table(NotasMantenimiento::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_responsabilidad_servicios_contrato_id")
                    .table(ResponsabilidadServicios::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_responsabilidad_servicios_unidad_id")
                    .table(ResponsabilidadServicios::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_desahucios_contrato_id")
                    .table(Desahucios::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_mantenimiento_prog_unidad_id")
                    .table(MantenimientoProgramado::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_mantenimiento_prog_propiedad_id")
                    .table(MantenimientoProgramado::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_gasto_recurrente_unidad_id")
                    .table(GastoRecurrente::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_gasto_recurrente_propiedad_id")
                    .table(GastoRecurrente::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_cuotas_condominio_contrato_id")
                    .table(CuotasCondominio::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_cuotas_condominio_propiedad_id")
                    .table(CuotasCondominio::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_copropietarios_propiedad_id")
                    .table(Copropietarios::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_unidades_propiedad_id")
                    .table(Unidades::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_gastos_unidad_id")
                    .table(Gastos::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_gastos_propiedad_id")
                    .table(Gastos::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Gastos {
    Table,
    PropiedadId,
    UnidadId,
}

#[derive(DeriveIden)]
enum Unidades {
    Table,
    PropiedadId,
}

#[derive(DeriveIden)]
enum Copropietarios {
    Table,
    PropiedadId,
}

#[derive(DeriveIden)]
enum CuotasCondominio {
    Table,
    PropiedadId,
    ContratoId,
}

#[derive(DeriveIden)]
enum GastoRecurrente {
    Table,
    PropiedadId,
    UnidadId,
}

#[derive(DeriveIden)]
enum MantenimientoProgramado {
    Table,
    PropiedadId,
    UnidadId,
}

#[derive(DeriveIden)]
enum Desahucios {
    Table,
    ContratoId,
}

#[derive(DeriveIden)]
enum ResponsabilidadServicios {
    Table,
    UnidadId,
    ContratoId,
}

#[derive(DeriveIden)]
enum NotasMantenimiento {
    Table,
    SolicitudId,
    AutorId,
}

#[derive(DeriveIden)]
enum ChatbotConversation {
    Table,
    OrganizacionId,
    SenderPhone,
}

#[derive(DeriveIden)]
enum ChatbotReceiptExtraction {
    Table,
    ConversationId,
    InquilinoId,
}

#[derive(DeriveIden)]
enum Notificaciones {
    Table,
    UsuarioId,
    OrganizacionId,
}

#[derive(DeriveIden)]
enum Documentos {
    Table,
    UploadedBy,
}

#[derive(DeriveIden)]
enum FirmasDocumento {
    Table,
    DocumentoId,
}

#[derive(DeriveIden)]
enum RegistrosAuditoria {
    Table,
    OrganizacionId,
}
