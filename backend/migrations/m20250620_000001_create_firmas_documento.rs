use sea_orm_migration::prelude::*;

use super::m20250409_000002_create_documentos::Documentos;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250620_000001_create_firmas_documento"
    }
}

#[derive(Iden)]
pub enum FirmasDocumento {
    Table,
    Id,
    DocumentoId,
    FirmanteTipo,
    FirmanteNombre,
    FirmaImagen,
    IpAddress,
    UserAgent,
    FirmadoAt,
    Token,
    PasswordHash,
    ExpiraAt,
    Estado,
    CreatedAt,
}

#[derive(Iden)]
enum DocumentosSellado {
    Sellado,
    SelladoAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create firmas_documento table
        manager
            .create_table(
                Table::create()
                    .table(FirmasDocumento::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FirmasDocumento::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(FirmasDocumento::DocumentoId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FirmasDocumento::FirmanteTipo)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FirmasDocumento::FirmanteNombre)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(FirmasDocumento::FirmaImagen).binary())
                    .col(ColumnDef::new(FirmasDocumento::IpAddress).string())
                    .col(ColumnDef::new(FirmasDocumento::UserAgent).text())
                    .col(ColumnDef::new(FirmasDocumento::FirmadoAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(FirmasDocumento::Token).string())
                    .col(ColumnDef::new(FirmasDocumento::PasswordHash).string())
                    .col(ColumnDef::new(FirmasDocumento::ExpiraAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(FirmasDocumento::Estado)
                            .string()
                            .not_null()
                            .default("pendiente"),
                    )
                    .col(
                        ColumnDef::new(FirmasDocumento::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_firmas_documento_documento_id")
                            .from(FirmasDocumento::Table, FirmasDocumento::DocumentoId)
                            .to(Documentos::Table, Documentos::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index on documento_id (FK column)
        manager
            .create_index(
                Index::create()
                    .name("idx_firmas_documento_documento_id")
                    .table(FirmasDocumento::Table)
                    .col(FirmasDocumento::DocumentoId)
                    .to_owned(),
            )
            .await?;

        // Unique partial index on token WHERE token IS NOT NULL
        let db = manager.get_connection();
        db.execute_unprepared(
            "CREATE UNIQUE INDEX idx_firmas_documento_token ON firmas_documento(token) WHERE token IS NOT NULL",
        )
        .await?;

        // Add sellado and sellado_at columns to documentos table
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .add_column(
                        ColumnDef::new(DocumentosSellado::Sellado)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .add_column(
                        ColumnDef::new(DocumentosSellado::SelladoAt).timestamp_with_time_zone(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop sellado columns from documentos
        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .drop_column(DocumentosSellado::SelladoAt)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Documentos::Table)
                    .drop_column(DocumentosSellado::Sellado)
                    .to_owned(),
            )
            .await?;

        // Drop firmas_documento table (indexes are dropped automatically)
        manager
            .drop_table(Table::drop().table(FirmasDocumento::Table).to_owned())
            .await
    }
}
