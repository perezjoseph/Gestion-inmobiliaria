use sea_orm_migration::prelude::*;

use super::m20250413_000001_create_organizaciones::Organizaciones;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260601_000001_add_tipo_fiscal_to_organizaciones"
    }
}

#[derive(Iden)]
enum TipoFiscalColumns {
    TipoFiscal,
    RegimenPagos,
    FechaInicioOperaciones,
    IsEcfCertificado,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add tipo_fiscal column (VARCHAR NOT NULL DEFAULT 'informal')
        manager
            .alter_table(
                Table::alter()
                    .table(Organizaciones::Table)
                    .add_column(
                        ColumnDef::new(TipoFiscalColumns::TipoFiscal)
                            .string()
                            .not_null()
                            .default("informal"),
                    )
                    .to_owned(),
            )
            .await?;

        // Add regimen_pagos column (VARCHAR NULL)
        manager
            .alter_table(
                Table::alter()
                    .table(Organizaciones::Table)
                    .add_column(ColumnDef::new(TipoFiscalColumns::RegimenPagos).string())
                    .to_owned(),
            )
            .await?;

        // Add fecha_inicio_operaciones column (DATE NULL)
        manager
            .alter_table(
                Table::alter()
                    .table(Organizaciones::Table)
                    .add_column(ColumnDef::new(TipoFiscalColumns::FechaInicioOperaciones).date())
                    .to_owned(),
            )
            .await?;

        // Add is_ecf_certificado column (BOOLEAN DEFAULT false)
        manager
            .alter_table(
                Table::alter()
                    .table(Organizaciones::Table)
                    .add_column(
                        ColumnDef::new(TipoFiscalColumns::IsEcfCertificado)
                            .boolean()
                            .not_null()
                            .default(false),
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
                    .table(Organizaciones::Table)
                    .drop_column(TipoFiscalColumns::IsEcfCertificado)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Organizaciones::Table)
                    .drop_column(TipoFiscalColumns::FechaInicioOperaciones)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Organizaciones::Table)
                    .drop_column(TipoFiscalColumns::RegimenPagos)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Organizaciones::Table)
                    .drop_column(TipoFiscalColumns::TipoFiscal)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
