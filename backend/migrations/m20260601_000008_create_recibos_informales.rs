use sea_orm_migration::prelude::*;

use super::m20250408_000005_create_pagos::Pagos;
use super::m20250413_000001_create_organizaciones::Organizaciones;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RecibosInformales::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RecibosInformales::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(RecibosInformales::PagoId).uuid().not_null())
                    .col(
                        ColumnDef::new(RecibosInformales::ReferenciaInterna)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RecibosInformales::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RecibosInformales::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recibos_informales_pago")
                            .from(RecibosInformales::Table, RecibosInformales::PagoId)
                            .to(Pagos::Table, Pagos::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recibos_informales_organizacion")
                            .from(RecibosInformales::Table, RecibosInformales::OrganizacionId)
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique constraint on referencia_interna
        manager
            .create_index(
                Index::create()
                    .name("idx_recibos_informales_referencia_interna")
                    .table(RecibosInformales::Table)
                    .col(RecibosInformales::ReferenciaInterna)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index on pago_id for FK lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_recibos_informales_pago_id")
                    .table(RecibosInformales::Table)
                    .col(RecibosInformales::PagoId)
                    .to_owned(),
            )
            .await?;

        // Index on organizacion_id for FK lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_recibos_informales_organizacion_id")
                    .table(RecibosInformales::Table)
                    .col(RecibosInformales::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RecibosInformales::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum RecibosInformales {
    Table,
    Id,
    PagoId,
    ReferenciaInterna,
    OrganizacionId,
    CreatedAt,
}
