use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GastoRecurrente::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GastoRecurrente::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GastoRecurrente::PropiedadId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(GastoRecurrente::UnidadId).uuid().null())
                    .col(
                        ColumnDef::new(GastoRecurrente::Categoria)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GastoRecurrente::Descripcion)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GastoRecurrente::Monto)
                            .decimal_len(12, 2)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GastoRecurrente::Moneda)
                            .string_len(3)
                            .not_null(),
                    )
                    .col(ColumnDef::new(GastoRecurrente::Proveedor).string().null())
                    .col(
                        ColumnDef::new(GastoRecurrente::Frecuencia)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(ColumnDef::new(GastoRecurrente::DiaDelMes).integer().null())
                    .col(
                        ColumnDef::new(GastoRecurrente::ProximaFecha)
                            .date()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GastoRecurrente::Activo)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(GastoRecurrente::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GastoRecurrente::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GastoRecurrente::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gasto_recurrente_org_activo")
                    .table(GastoRecurrente::Table)
                    .col(GastoRecurrente::OrganizacionId)
                    .col(GastoRecurrente::Activo)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gasto_recurrente_proxima_fecha")
                    .table(GastoRecurrente::Table)
                    .col(GastoRecurrente::ProximaFecha)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GastoRecurrente::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum GastoRecurrente {
    Table,
    Id,
    PropiedadId,
    UnidadId,
    Categoria,
    Descripcion,
    Monto,
    Moneda,
    Proveedor,
    Frecuencia,
    DiaDelMes,
    ProximaFecha,
    Activo,
    OrganizacionId,
    CreatedAt,
    UpdatedAt,
}
