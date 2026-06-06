use sea_orm_migration::prelude::*;

use super::m20250413_000001_create_organizaciones::Organizaciones;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ConfiguracionesIpi::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ConfiguracionesIpi::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(ConfiguracionesIpi::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConfiguracionesIpi::UmbralIpi)
                            .decimal_len(14, 2)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConfiguracionesIpi::Anio)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConfiguracionesIpi::FechaPago1)
                            .date()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConfiguracionesIpi::FechaPago2)
                            .date()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConfiguracionesIpi::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(ConfiguracionesIpi::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_configuraciones_ipi_organizacion")
                            .from(
                                ConfiguracionesIpi::Table,
                                ConfiguracionesIpi::OrganizacionId,
                            )
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index on organizacion_id for FK lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_configuraciones_ipi_organizacion_id")
                    .table(ConfiguracionesIpi::Table)
                    .col(ConfiguracionesIpi::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ConfiguracionesIpi::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum ConfiguracionesIpi {
    Table,
    Id,
    OrganizacionId,
    UmbralIpi,
    Anio,
    FechaPago1,
    FechaPago2,
    CreatedAt,
    UpdatedAt,
}
