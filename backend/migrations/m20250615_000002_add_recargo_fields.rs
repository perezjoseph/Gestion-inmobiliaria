use sea_orm_migration::prelude::*;

use super::m20250408_000004_create_contratos::Contratos;
use super::m20250408_000005_create_pagos::Pagos;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250615_000002_add_recargo_fields"
    }
}

#[derive(Iden)]
enum RecargoFields {
    RecargoPorcentaje,
    DiasGracia,
    Recargo,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add recargo_porcentaje column to contratos (DECIMAL(5,2) NULLABLE)
        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .add_column(
                        ColumnDef::new(RecargoFields::RecargoPorcentaje).decimal_len(5, 2),
                    )
                    .to_owned(),
            )
            .await?;

        // Add dias_gracia column to contratos (INTEGER NULLABLE)
        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .add_column(ColumnDef::new(RecargoFields::DiasGracia).integer())
                    .to_owned(),
            )
            .await?;

        // Add recargo column to pagos (DECIMAL(12,2) NULLABLE)
        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .add_column(ColumnDef::new(RecargoFields::Recargo).decimal_len(12, 2))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop columns in reverse order
        manager
            .alter_table(
                Table::alter()
                    .table(Pagos::Table)
                    .drop_column(RecargoFields::Recargo)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .drop_column(RecargoFields::DiasGracia)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .drop_column(RecargoFields::RecargoPorcentaje)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
