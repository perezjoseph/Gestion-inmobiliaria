use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250409_000004_add_documentos_columns"
    }
}

#[derive(Iden)]
enum Inquilinos {
    Table,
    Documentos,
}

#[derive(Iden)]
enum Contratos {
    Table,
    Documentos,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Inquilinos::Table)
                    .add_column(
                        ColumnDef::new(Inquilinos::Documentos)
                            .json_binary()
                            .default(SimpleExpr::Custom("'[]'::jsonb".to_owned())),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Contratos::Table)
                    .add_column(
                        ColumnDef::new(Contratos::Documentos)
                            .json_binary()
                            .default(SimpleExpr::Custom("'[]'::jsonb".to_owned())),
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
                    .table(Contratos::Table)
                    .drop_column(Contratos::Documentos)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Inquilinos::Table)
                    .drop_column(Inquilinos::Documentos)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
