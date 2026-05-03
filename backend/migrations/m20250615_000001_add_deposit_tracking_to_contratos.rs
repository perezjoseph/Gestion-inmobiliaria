use sea_orm_migration::prelude::*;

use super::m20250408_000004_create_contratos::Contratos;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250615_000001_add_deposit_tracking_to_contratos"
    }
}

#[derive(Iden)]
enum DepositTracking {
    EstadoDeposito,
    FechaCobroDeposito,
    FechaDevolucionDeposito,
    MontoRetenido,
    MotivoRetencion,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add estado_deposito column (VARCHAR(20) NULLABLE)
        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .add_column(ColumnDef::new(DepositTracking::EstadoDeposito).string_len(20))
                    .to_owned(),
            )
            .await?;

        // Add fecha_cobro_deposito column (TIMESTAMP WITH TIME ZONE NULLABLE)
        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .add_column(
                        ColumnDef::new(DepositTracking::FechaCobroDeposito)
                            .timestamp_with_time_zone(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add fecha_devolucion_deposito column (TIMESTAMP WITH TIME ZONE NULLABLE)
        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .add_column(
                        ColumnDef::new(DepositTracking::FechaDevolucionDeposito)
                            .timestamp_with_time_zone(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add monto_retenido column (DECIMAL(12,2) NULLABLE)
        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .add_column(
                        ColumnDef::new(DepositTracking::MontoRetenido).decimal_len(12, 2),
                    )
                    .to_owned(),
            )
            .await?;

        // Add motivo_retencion column (TEXT NULLABLE)
        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .add_column(ColumnDef::new(DepositTracking::MotivoRetencion).text())
                    .to_owned(),
            )
            .await?;

        // Backfill: set estado_deposito = 'pendiente' for existing contratos with deposito > 0
        let db = manager.get_connection();
        db.execute_unprepared(
            "UPDATE contratos SET estado_deposito = 'pendiente' WHERE deposito IS NOT NULL AND deposito > 0",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop columns in reverse order
        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .drop_column(DepositTracking::MotivoRetencion)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .drop_column(DepositTracking::MontoRetenido)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .drop_column(DepositTracking::FechaDevolucionDeposito)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .drop_column(DepositTracking::FechaCobroDeposito)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .drop_column(DepositTracking::EstadoDeposito)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
