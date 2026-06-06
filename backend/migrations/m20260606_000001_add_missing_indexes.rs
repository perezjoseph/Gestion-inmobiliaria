use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260606_000001_add_missing_indexes"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add missing index on inquilino_id FK in solicitudes_mantenimiento
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_solicitudes_mant_inquilino_id")
                    .table(SolicitudesMantenimiento::Table)
                    .col(SolicitudesMantenimiento::InquilinoId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_solicitudes_mant_inquilino_id")
                    .table(SolicitudesMantenimiento::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum SolicitudesMantenimiento {
    Table,
    InquilinoId,
}
