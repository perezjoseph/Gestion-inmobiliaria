use sea_orm_migration::prelude::*;

use super::m20250409_000002_create_documentos::Documentos;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250430_000002_add_documentos_editor"
    }
}

#[derive(Iden)]
enum DocumentosEditor {
    ContenidoEditable,
    UpdatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add contenido_editable JSONB column
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .add_column(ColumnDef::new(DocumentosEditor::ContenidoEditable).json_binary())
                    .to_owned(),
            )
            .await?;

        // Add updated_at TIMESTAMPTZ column
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .add_column(
                        ColumnDef::new(DocumentosEditor::UpdatedAt).timestamp_with_time_zone(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .drop_column(DocumentosEditor::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .drop_column(DocumentosEditor::ContenidoEditable)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
