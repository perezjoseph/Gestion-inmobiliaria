use sea_orm_migration::prelude::*;

use super::m20250413_000001_create_organizaciones::Organizaciones;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260512_000004_create_cache_dgii"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CacheDgii::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CacheDgii::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(CacheDgii::CedulaRnc)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CacheDgii::NombreRazonSocial)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(ColumnDef::new(CacheDgii::NombreComercial).string_len(255))
                    .col(ColumnDef::new(CacheDgii::Estado).string_len(20).not_null())
                    .col(ColumnDef::new(CacheDgii::RegimenDePagos).string_len(50))
                    .col(ColumnDef::new(CacheDgii::ActividadEconomica).text())
                    .col(ColumnDef::new(CacheDgii::RawResponse).json().not_null())
                    .col(ColumnDef::new(CacheDgii::OrganizacionId).uuid().not_null())
                    .col(
                        ColumnDef::new(CacheDgii::CachedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CacheDgii::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CacheDgii::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(CacheDgii::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cache_dgii_organizacion")
                            .from(CacheDgii::Table, CacheDgii::OrganizacionId)
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique constraint on (cedula_rnc, organizacion_id)
        manager
            .create_index(
                Index::create()
                    .name("uq_cache_dgii_rnc_org")
                    .table(CacheDgii::Table)
                    .col(CacheDgii::CedulaRnc)
                    .col(CacheDgii::OrganizacionId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index on cedula_rnc
        manager
            .create_index(
                Index::create()
                    .name("idx_cache_dgii_cedula_rnc")
                    .table(CacheDgii::Table)
                    .col(CacheDgii::CedulaRnc)
                    .to_owned(),
            )
            .await?;

        // Index on organizacion_id
        manager
            .create_index(
                Index::create()
                    .name("idx_cache_dgii_organizacion_id")
                    .table(CacheDgii::Table)
                    .col(CacheDgii::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        // Index on expires_at
        manager
            .create_index(
                Index::create()
                    .name("idx_cache_dgii_expires_at")
                    .table(CacheDgii::Table)
                    .col(CacheDgii::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CacheDgii::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum CacheDgii {
    Table,
    Id,
    CedulaRnc,
    NombreRazonSocial,
    NombreComercial,
    Estado,
    RegimenDePagos,
    ActividadEconomica,
    RawResponse,
    OrganizacionId,
    CachedAt,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
}
