use sea_orm_migration::prelude::*;

use super::m20250408_000001_create_usuarios::Usuarios;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250409_000002_create_documentos"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Documentos::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Documentos::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(Documentos::EntityType)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Documentos::EntityId).uuid().not_null())
                    .col(
                        ColumnDef::new(Documentos::Filename)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Documentos::FilePath)
                            .string_len(500)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Documentos::MimeType)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Documentos::FileSize)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Documentos::UploadedBy).uuid().not_null())
                    .col(
                        ColumnDef::new(Documentos::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_documentos_uploaded_by")
                            .from(Documentos::Table, Documentos::UploadedBy)
                            .to(Usuarios::Table, Usuarios::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_documentos_entity_type")
                    .table(Documentos::Table)
                    .col(Documentos::EntityType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_documentos_entity_id")
                    .table(Documentos::Table)
                    .col(Documentos::EntityId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_documentos_uploaded_by")
                    .table(Documentos::Table)
                    .col(Documentos::UploadedBy)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Documentos::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Documentos {
    Table,
    Id,
    EntityType,
    EntityId,
    Filename,
    FilePath,
    MimeType,
    FileSize,
    UploadedBy,
    CreatedAt,
}
