use sea_orm_migration::prelude::*;

use super::m20250408_000005_create_pagos::Pagos;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260601_000002_add_fiscal_columns_to_pagos"
    }
}

#[derive(Iden)]
enum PagosFiscalColumns {
    MontoBase,
    MontoItbis,
    MontoItbisRetenido,
    Ncf,
    FechaComprobante,
    TipoNcf,
    EsParcial,
    SaldoPendiente,
    TipoLinea,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .add_column(ColumnDef::new(PagosFiscalColumns::MontoBase).decimal_len(12, 2))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .add_column(ColumnDef::new(PagosFiscalColumns::MontoItbis).decimal_len(12, 2))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .add_column(
                        ColumnDef::new(PagosFiscalColumns::MontoItbisRetenido).decimal_len(12, 2),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .add_column(ColumnDef::new(PagosFiscalColumns::Ncf).string_len(11))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .add_column(ColumnDef::new(PagosFiscalColumns::FechaComprobante).date())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .add_column(ColumnDef::new(PagosFiscalColumns::TipoNcf).string())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .add_column(
                        ColumnDef::new(PagosFiscalColumns::EsParcial)
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
                    .table(Pagos::Table)
                    .add_column(
                        ColumnDef::new(PagosFiscalColumns::SaldoPendiente).decimal_len(12, 2),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .add_column(
                        ColumnDef::new(PagosFiscalColumns::TipoLinea)
                            .string()
                            .not_null()
                            .default("renta"),
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
                    .table(Pagos::Table)
                    .drop_column(PagosFiscalColumns::TipoLinea)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .drop_column(PagosFiscalColumns::SaldoPendiente)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .drop_column(PagosFiscalColumns::EsParcial)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .drop_column(PagosFiscalColumns::TipoNcf)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .drop_column(PagosFiscalColumns::FechaComprobante)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .drop_column(PagosFiscalColumns::Ncf)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .drop_column(PagosFiscalColumns::MontoItbisRetenido)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .drop_column(PagosFiscalColumns::MontoItbis)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .drop_column(PagosFiscalColumns::MontoBase)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
