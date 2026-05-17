use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PreviewIndex::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PreviewIndex::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PreviewIndex::PreviewId).uuid().not_null())
                    .col(
                        ColumnDef::new(PreviewIndex::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PreviewIndex::EntityType)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(ColumnDef::new(PreviewIndex::EntityId).uuid().not_null())
                    .col(
                        ColumnDef::new(PreviewIndex::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_preview_index_organizacion")
                            .from(PreviewIndex::Table, PreviewIndex::OrganizacionId)
                            .to(Organizacion::Table, Organizacion::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique index on (preview_id, organizacion_id) for idempotency lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_preview_index_preview_org")
                    .table(PreviewIndex::Table)
                    .col(PreviewIndex::PreviewId)
                    .col(PreviewIndex::OrganizacionId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PreviewIndex::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum PreviewIndex {
    Table,
    Id,
    PreviewId,
    OrganizacionId,
    EntityType,
    EntityId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Organizacion {
    Table,
    Id,
}
