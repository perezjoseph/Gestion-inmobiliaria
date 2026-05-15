use sea_orm_migration::prelude::*;

use super::m20250412_000001_create_gastos::Gastos;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260415_002_add_gasto_utility_fields"
    }
}

#[derive(Iden)]
enum GastoUtility {
    NumeroCuenta,
    PeriodoInicio,
    PeriodoFin,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add numero_cuenta column (TEXT NULL)
        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .add_column(ColumnDef::new(GastoUtility::NumeroCuenta).text())
                    .to_owned(),
            )
            .await?;

        // Add periodo_inicio column (DATE NULL)
        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .add_column(ColumnDef::new(GastoUtility::PeriodoInicio).date())
                    .to_owned(),
            )
            .await?;

        // Add periodo_fin column (DATE NULL)
        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .add_column(ColumnDef::new(GastoUtility::PeriodoFin).date())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .drop_column(GastoUtility::PeriodoFin)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .drop_column(GastoUtility::PeriodoInicio)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .drop_column(GastoUtility::NumeroCuenta)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
