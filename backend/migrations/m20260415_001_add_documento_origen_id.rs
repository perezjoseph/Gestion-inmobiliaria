use sea_orm_migration::prelude::*;

use super::m20250408_000004_create_contratos::Contratos;
use super::m20250409_000002_create_documentos::Documentos;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260415_001_add_documento_origen_id"
    }
}

#[derive(Iden)]
enum DocumentosExtra {
    DocumentoOrigenId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add nullable documento_origen_id UUID column
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .add_column(ColumnDef::new(DocumentosExtra::DocumentoOrigenId).uuid())
                    .to_owned(),
            )
            .await?;

        // Add FK: fk_documento_origen_contrato → contratos.id ON DELETE SET NULL
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_documento_origen_contrato")
                    .from(Documentos::Table, DocumentosExtra::DocumentoOrigenId)
                    .to(Contratos::Table, Contratos::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        // Index on the FK column
        manager
            .create_index(
                Index::create()
                    .name("idx_documentos_documento_origen_id")
                    .table(Documentos::Table)
                    .col(DocumentosExtra::DocumentoOrigenId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_documentos_documento_origen_id")
                    .table(Documentos::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_documento_origen_contrato")
                    .table(Documentos::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .drop_column(DocumentosExtra::DocumentoOrigenId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
