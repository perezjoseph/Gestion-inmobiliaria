use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260618_000001_fix_check_constraints"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("ALTER TABLE pagos DROP CONSTRAINT IF EXISTS chk_pagos_estado")
            .await?;

        db.execute_unprepared(
            "ALTER TABLE pagos ADD CONSTRAINT chk_pagos_estado \
             CHECK (estado IN ('pendiente', 'pagado', 'atrasado', 'cancelado'))",
        )
        .await?;

        db.execute_unprepared(
            "ALTER TABLE contratos DROP CONSTRAINT IF EXISTS chk_contratos_estado",
        )
        .await?;

        db.execute_unprepared(
            "ALTER TABLE contratos ADD CONSTRAINT chk_contratos_estado \
             CHECK (estado IN ('activo', 'vencido', 'cancelado', 'finalizado', 'terminado'))",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
