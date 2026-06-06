use sea_orm_migration::prelude::*;

use super::m20250408_000002_create_propiedades::Propiedades;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260601_000003_add_catastral_to_propiedades"
    }
}

#[derive(Iden)]
enum PropiedadesCatastral {
    ValorCatastral,
    ExentoIpi,
    MotivoExencion,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add valor_catastral column (DECIMAL(14,2) NULL)
        manager
            .alter_table(
                Table::alter()
                    .table(Propiedades::Table)
                    .add_column(
                        ColumnDef::new(PropiedadesCatastral::ValorCatastral).decimal_len(14, 2),
                    )
                    .to_owned(),
            )
            .await?;

        // Add exento_ipi column (BOOLEAN DEFAULT false)
        manager
            .alter_table(
                Table::alter()
                    .table(Propiedades::Table)
                    .add_column(
                        ColumnDef::new(PropiedadesCatastral::ExentoIpi)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // Add motivo_exencion column (VARCHAR NULL)
        manager
            .alter_table(
                Table::alter()
                    .table(Propiedades::Table)
                    .add_column(ColumnDef::new(PropiedadesCatastral::MotivoExencion).string())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Propiedades::Table)
                    .drop_column(PropiedadesCatastral::MotivoExencion)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Propiedades::Table)
                    .drop_column(PropiedadesCatastral::ExentoIpi)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Propiedades::Table)
                    .drop_column(PropiedadesCatastral::ValorCatastral)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
