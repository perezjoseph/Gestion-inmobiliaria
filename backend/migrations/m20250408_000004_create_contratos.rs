use sea_orm_migration::prelude::*;

use super::m20250408_000002_create_propiedades::Propiedades;
use super::m20250408_000003_create_inquilinos::Inquilinos;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250408_000004_create_contratos"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Contratos::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Contratos::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(Contratos::PropiedadId).uuid().not_null())
                    .col(ColumnDef::new(Contratos::InquilinoId).uuid().not_null())
                    .col(ColumnDef::new(Contratos::FechaInicio).date().not_null())
                    .col(ColumnDef::new(Contratos::FechaFin).date().not_null())
                    .col(
                        ColumnDef::new(Contratos::MontoMensual)
                            .decimal_len(12, 2)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Contratos::Deposito).decimal_len(12, 2))
                    .col(
                        ColumnDef::new(Contratos::Moneda)
                            .string_len(3)
                            .not_null()
                            .default("DOP"),
                    )
                    .col(
                        ColumnDef::new(Contratos::Estado)
                            .string_len(20)
                            .not_null()
                            .default("activo"),
                    )
                    .col(
                        ColumnDef::new(Contratos::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(Contratos::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_contratos_propiedad")
                            .from(Contratos::Table, Contratos::PropiedadId)
                            .to(Propiedades::Table, Propiedades::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_contratos_inquilino")
                            .from(Contratos::Table, Contratos::InquilinoId)
                            .to(Inquilinos::Table, Inquilinos::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_contratos_propiedad_id")
                    .table(Contratos::Table)
                    .col(Contratos::PropiedadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_contratos_inquilino_id")
                    .table(Contratos::Table)
                    .col(Contratos::InquilinoId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_contratos_estado")
                    .table(Contratos::Table)
                    .col(Contratos::Estado)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Contratos::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Contratos {
    Table,
    Id,
    PropiedadId,
    InquilinoId,
    FechaInicio,
    FechaFin,
    MontoMensual,
    Deposito,
    Moneda,
    Estado,
    CreatedAt,
    UpdatedAt,
}
