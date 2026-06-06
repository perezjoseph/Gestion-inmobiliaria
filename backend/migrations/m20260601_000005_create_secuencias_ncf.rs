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
                    .table(SecuenciasNcf::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SecuenciasNcf::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(SecuenciasNcf::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SecuenciasNcf::TipoNcf)
                            .string_len(3)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SecuenciasNcf::Prefijo)
                            .char_len(1)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SecuenciasNcf::SiguienteNumero)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SecuenciasNcf::RangoDesde)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SecuenciasNcf::RangoHasta)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SecuenciasNcf::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(SecuenciasNcf::IsEcf)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(SecuenciasNcf::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(SecuenciasNcf::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_secuencias_ncf_organizacion")
                            .from(SecuenciasNcf::Table, SecuenciasNcf::OrganizacionId)
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique constraint: (organizacion_id, tipo_ncf, prefijo)
        manager
            .create_index(
                Index::create()
                    .name("idx_secuencias_ncf_org_tipo_prefijo")
                    .table(SecuenciasNcf::Table)
                    .col(SecuenciasNcf::OrganizacionId)
                    .col(SecuenciasNcf::TipoNcf)
                    .col(SecuenciasNcf::Prefijo)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index on organizacion_id for FK lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_secuencias_ncf_organizacion_id")
                    .table(SecuenciasNcf::Table)
                    .col(SecuenciasNcf::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SecuenciasNcf::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum SecuenciasNcf {
    Table,
    Id,
    OrganizacionId,
    TipoNcf,
    Prefijo,
    SiguienteNumero,
    RangoDesde,
    RangoHasta,
    IsActive,
    IsEcf,
    CreatedAt,
    UpdatedAt,
}
