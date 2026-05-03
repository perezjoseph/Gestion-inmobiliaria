use sea_orm_migration::prelude::*;

use super::m20250408_000001_create_usuarios::Usuarios;
use super::m20250409_000002_create_documentos::Documentos;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250430_000001_extend_documentos_legal"
    }
}

#[derive(Iden)]
enum DocumentosLegal {
    TipoDocumento,
    EstadoVerificacion,
    FechaVencimiento,
    VerificadoPor,
    FechaVerificacion,
    NotasVerificacion,
    NumeroDocumento,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add tipo_documento column
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .add_column(
                        ColumnDef::new(DocumentosLegal::TipoDocumento)
                            .string_len(50)
                            .not_null()
                            .default("otro"),
                    )
                    .to_owned(),
            )
            .await?;

        // Add estado_verificacion column
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .add_column(
                        ColumnDef::new(DocumentosLegal::EstadoVerificacion)
                            .string_len(20)
                            .not_null()
                            .default("pendiente"),
                    )
                    .to_owned(),
            )
            .await?;

        // Add fecha_vencimiento column
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .add_column(ColumnDef::new(DocumentosLegal::FechaVencimiento).date())
                    .to_owned(),
            )
            .await?;

        // Add verificado_por column
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .add_column(ColumnDef::new(DocumentosLegal::VerificadoPor).uuid())
                    .to_owned(),
            )
            .await?;

        // Add FK for verificado_por -> usuarios(id)
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_documentos_verificado_por")
                    .from(Documentos::Table, DocumentosLegal::VerificadoPor)
                    .to(Usuarios::Table, Usuarios::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        // Add fecha_verificacion column
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .add_column(
                        ColumnDef::new(DocumentosLegal::FechaVerificacion)
                            .timestamp_with_time_zone(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add notas_verificacion column
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .add_column(ColumnDef::new(DocumentosLegal::NotasVerificacion).text())
                    .to_owned(),
            )
            .await?;

        // Add numero_documento column
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .add_column(ColumnDef::new(DocumentosLegal::NumeroDocumento).string_len(100))
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_documentos_tipo_documento")
                    .table(Documentos::Table)
                    .col(DocumentosLegal::TipoDocumento)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_documentos_estado_verificacion")
                    .table(Documentos::Table)
                    .col(DocumentosLegal::EstadoVerificacion)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_documentos_fecha_vencimiento")
                    .table(Documentos::Table)
                    .col(DocumentosLegal::FechaVencimiento)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_documentos_fecha_vencimiento")
                    .table(Documentos::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_documentos_estado_verificacion")
                    .table(Documentos::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_documentos_tipo_documento")
                    .table(Documentos::Table)
                    .to_owned(),
            )
            .await?;

        // Drop FK
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_documentos_verificado_por")
                    .table(Documentos::Table)
                    .to_owned(),
            )
            .await?;

        // Drop columns in reverse order
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .drop_column(DocumentosLegal::NumeroDocumento)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .drop_column(DocumentosLegal::NotasVerificacion)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .drop_column(DocumentosLegal::FechaVerificacion)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .drop_column(DocumentosLegal::VerificadoPor)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .drop_column(DocumentosLegal::FechaVencimiento)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .drop_column(DocumentosLegal::EstadoVerificacion)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .drop_column(DocumentosLegal::TipoDocumento)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
