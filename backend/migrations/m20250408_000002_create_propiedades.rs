use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250408_000002_create_propiedades"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Propiedades::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Propiedades::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(Propiedades::Titulo)
                            .string_len(200)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Propiedades::Descripcion).text())
                    .col(
                        ColumnDef::new(Propiedades::Direccion)
                            .string_len(300)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Propiedades::Ciudad)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Propiedades::Provincia)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Propiedades::TipoPropiedad)
                            .string_len(20)
                            .not_null()
                            .check(Expr::col(Propiedades::TipoPropiedad).is_in([
                                "casa",
                                "apartamento",
                                "comercial",
                                "terreno",
                            ])),
                    )
                    .col(ColumnDef::new(Propiedades::Habitaciones).integer())
                    .col(ColumnDef::new(Propiedades::Banos).integer())
                    .col(ColumnDef::new(Propiedades::AreaM2).decimal_len(10, 2))
                    .col(
                        ColumnDef::new(Propiedades::Precio)
                            .decimal_len(12, 2)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Propiedades::Moneda)
                            .string_len(3)
                            .not_null()
                            .default("DOP"),
                    )
                    .col(
                        ColumnDef::new(Propiedades::Estado)
                            .string_len(20)
                            .not_null()
                            .default("disponible"),
                    )
                    .col(
                        ColumnDef::new(Propiedades::Imagenes)
                            .json_binary()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(Propiedades::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(Propiedades::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_propiedades_ciudad")
                    .table(Propiedades::Table)
                    .col(Propiedades::Ciudad)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_propiedades_provincia")
                    .table(Propiedades::Table)
                    .col(Propiedades::Provincia)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_propiedades_tipo_propiedad")
                    .table(Propiedades::Table)
                    .col(Propiedades::TipoPropiedad)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_propiedades_estado")
                    .table(Propiedades::Table)
                    .col(Propiedades::Estado)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Propiedades::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Propiedades {
    Table,
    Id,
    Titulo,
    Descripcion,
    Direccion,
    Ciudad,
    Provincia,
    TipoPropiedad,
    Habitaciones,
    Banos,
    #[iden = "area_m2"]
    AreaM2,
    Precio,
    Moneda,
    Estado,
    Imagenes,
    CreatedAt,
    UpdatedAt,
}
