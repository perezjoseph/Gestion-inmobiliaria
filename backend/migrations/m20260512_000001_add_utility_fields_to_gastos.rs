use sea_orm_migration::prelude::*;

use super::m20250412_000001_create_gastos::Gastos;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260512_000001_add_utility_fields_to_gastos"
    }
}

#[derive(Iden)]
enum UtilityFields {
    NicContrato,
    ProveedorServicio,
    Consumo,
    UnidadConsumo,
    PeriodoDesde,
    PeriodoHasta,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add nic_contrato column (VARCHAR(50) NULL)
        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .add_column(ColumnDef::new(UtilityFields::NicContrato).string_len(50))
                    .to_owned(),
            )
            .await?;

        // Add proveedor_servicio column (VARCHAR(20) NULL)
        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .add_column(ColumnDef::new(UtilityFields::ProveedorServicio).string_len(20))
                    .to_owned(),
            )
            .await?;

        // Add consumo column (DECIMAL(12,4) NULL)
        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .add_column(ColumnDef::new(UtilityFields::Consumo).decimal_len(12, 4))
                    .to_owned(),
            )
            .await?;

        // Add unidad_consumo column (VARCHAR(5) NULL)
        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .add_column(ColumnDef::new(UtilityFields::UnidadConsumo).string_len(5))
                    .to_owned(),
            )
            .await?;

        // Add periodo_desde column (DATE NULL)
        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .add_column(ColumnDef::new(UtilityFields::PeriodoDesde).date())
                    .to_owned(),
            )
            .await?;

        // Add periodo_hasta column (DATE NULL)
        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .add_column(ColumnDef::new(UtilityFields::PeriodoHasta).date())
                    .to_owned(),
            )
            .await?;

        // Create index on proveedor_servicio
        manager
            .create_index(
                Index::create()
                    .name("idx_gastos_proveedor_servicio")
                    .table(Gastos::Table)
                    .col(UtilityFields::ProveedorServicio)
                    .to_owned(),
            )
            .await?;

        // Create composite index on (unidad_id, proveedor_servicio)
        manager
            .create_index(
                Index::create()
                    .name("idx_gastos_unidad_proveedor")
                    .table(Gastos::Table)
                    .col(Gastos::UnidadId)
                    .col(UtilityFields::ProveedorServicio)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_gastos_unidad_proveedor")
                    .table(Gastos::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_gastos_proveedor_servicio")
                    .table(Gastos::Table)
                    .to_owned(),
            )
            .await?;

        // Drop columns in reverse order
        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .drop_column(UtilityFields::PeriodoHasta)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .drop_column(UtilityFields::PeriodoDesde)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .drop_column(UtilityFields::UnidadConsumo)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .drop_column(UtilityFields::Consumo)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .drop_column(UtilityFields::ProveedorServicio)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Gastos::Table)
                    .drop_column(UtilityFields::NicContrato)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
