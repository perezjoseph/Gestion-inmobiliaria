use sea_orm_migration::prelude::*;

use super::m20250408_000002_create_propiedades::Propiedades;
use super::m20250410_000001_create_unidades::Unidades;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250412_000001_create_gastos"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Gastos::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Gastos::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(Gastos::PropiedadId).uuid().not_null())
                    .col(ColumnDef::new(Gastos::UnidadId).uuid())
                    .col(ColumnDef::new(Gastos::Categoria).string_len(30).not_null())
                    .col(
                        ColumnDef::new(Gastos::Descripcion)
                            .string_len(500)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Gastos::Monto).decimal_len(12, 2).not_null())
                    .col(
                        ColumnDef::new(Gastos::Moneda)
                            .string_len(3)
                            .not_null()
                            .default("DOP"),
                    )
                    .col(ColumnDef::new(Gastos::FechaGasto).date().not_null())
                    .col(
                        ColumnDef::new(Gastos::Estado)
                            .string_len(20)
                            .not_null()
                            .default("pendiente"),
                    )
                    .col(ColumnDef::new(Gastos::Proveedor).string_len(200))
                    .col(ColumnDef::new(Gastos::NumeroFactura).string_len(100))
                    .col(ColumnDef::new(Gastos::Notas).text())
                    .col(
                        ColumnDef::new(Gastos::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(Gastos::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_gastos_propiedad")
                            .from(Gastos::Table, Gastos::PropiedadId)
                            .to(Propiedades::Table, Propiedades::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_gastos_unidad")
                            .from(Gastos::Table, Gastos::UnidadId)
                            .to(Unidades::Table, Unidades::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gastos_propiedad_id")
                    .table(Gastos::Table)
                    .col(Gastos::PropiedadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gastos_unidad_id")
                    .table(Gastos::Table)
                    .col(Gastos::UnidadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gastos_categoria")
                    .table(Gastos::Table)
                    .col(Gastos::Categoria)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gastos_estado")
                    .table(Gastos::Table)
                    .col(Gastos::Estado)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gastos_fecha_gasto")
                    .table(Gastos::Table)
                    .col(Gastos::FechaGasto)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Gastos::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Gastos {
    Table,
    Id,
    PropiedadId,
    UnidadId,
    Categoria,
    Descripcion,
    Monto,
    Moneda,
    FechaGasto,
    Estado,
    Proveedor,
    NumeroFactura,
    Notas,
    CreatedAt,
    UpdatedAt,
}
