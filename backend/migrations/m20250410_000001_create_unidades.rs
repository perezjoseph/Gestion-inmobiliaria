use sea_orm_migration::prelude::*;

use super::m20250408_000002_create_propiedades::Propiedades;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250410_000001_create_unidades"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Unidades::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Unidades::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(Unidades::PropiedadId).uuid().not_null())
                    .col(
                        ColumnDef::new(Unidades::NumeroUnidad)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Unidades::Piso).integer())
                    .col(ColumnDef::new(Unidades::Habitaciones).integer())
                    .col(ColumnDef::new(Unidades::Banos).integer())
                    .col(ColumnDef::new(Unidades::AreaM2).decimal_len(10, 2))
                    .col(
                        ColumnDef::new(Unidades::Precio)
                            .decimal_len(12, 2)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Unidades::Moneda)
                            .string_len(3)
                            .not_null()
                            .default("DOP"),
                    )
                    .col(
                        ColumnDef::new(Unidades::Estado)
                            .string_len(20)
                            .not_null()
                            .default("disponible"),
                    )
                    .col(ColumnDef::new(Unidades::Descripcion).text())
                    .col(
                        ColumnDef::new(Unidades::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(Unidades::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_unidades_propiedad")
                            .from(Unidades::Table, Unidades::PropiedadId)
                            .to(Propiedades::Table, Propiedades::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_unidades_propiedad_id")
                    .table(Unidades::Table)
                    .col(Unidades::PropiedadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_unidades_estado")
                    .table(Unidades::Table)
                    .col(Unidades::Estado)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("uq_unidades_propiedad_numero")
                    .table(Unidades::Table)
                    .col(Unidades::PropiedadId)
                    .col(Unidades::NumeroUnidad)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Unidades::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Unidades {
    Table,
    Id,
    PropiedadId,
    NumeroUnidad,
    Piso,
    Habitaciones,
    Banos,
    #[iden = "area_m2"]
    AreaM2,
    Precio,
    Moneda,
    Estado,
    Descripcion,
    CreatedAt,
    UpdatedAt,
}
