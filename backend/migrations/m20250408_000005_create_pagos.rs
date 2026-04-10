use sea_orm_migration::prelude::*;

use super::m20250408_000004_create_contratos::Contratos;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250408_000005_create_pagos"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Pagos::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Pagos::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(Pagos::ContratoId).uuid().not_null())
                    .col(ColumnDef::new(Pagos::Monto).decimal_len(12, 2).not_null())
                    .col(
                        ColumnDef::new(Pagos::Moneda)
                            .string_len(3)
                            .not_null()
                            .default("DOP"),
                    )
                    .col(ColumnDef::new(Pagos::FechaPago).date())
                    .col(ColumnDef::new(Pagos::FechaVencimiento).date().not_null())
                    .col(ColumnDef::new(Pagos::MetodoPago).string_len(20))
                    .col(
                        ColumnDef::new(Pagos::Estado)
                            .string_len(20)
                            .not_null()
                            .default("pendiente"),
                    )
                    .col(ColumnDef::new(Pagos::Notas).text())
                    .col(
                        ColumnDef::new(Pagos::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(Pagos::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_pagos_contrato")
                            .from(Pagos::Table, Pagos::ContratoId)
                            .to(Contratos::Table, Contratos::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_pagos_contrato_id")
                    .table(Pagos::Table)
                    .col(Pagos::ContratoId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_pagos_estado")
                    .table(Pagos::Table)
                    .col(Pagos::Estado)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_pagos_fecha_vencimiento")
                    .table(Pagos::Table)
                    .col(Pagos::FechaVencimiento)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Pagos::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Pagos {
    Table,
    Id,
    ContratoId,
    Monto,
    Moneda,
    FechaPago,
    FechaVencimiento,
    MetodoPago,
    Estado,
    Notas,
    CreatedAt,
    UpdatedAt,
}
