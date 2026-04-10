use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250413_000001_create_organizaciones"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Organizaciones::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Organizaciones::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(Organizaciones::Tipo)
                            .string_len(20)
                            .not_null()
                            .check(
                                Expr::col(Organizaciones::Tipo)
                                    .is_in(["persona_fisica", "persona_juridica"]),
                            ),
                    )
                    .col(
                        ColumnDef::new(Organizaciones::Nombre)
                            .string_len(200)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Organizaciones::Estado)
                            .string_len(20)
                            .not_null()
                            .default("activo")
                            .check(Expr::col(Organizaciones::Estado).is_in(["activo", "inactivo"])),
                    )
                    // persona_fisica fields
                    .col(
                        ColumnDef::new(Organizaciones::Cedula)
                            .string_len(11)
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Organizaciones::Telefono).string_len(20))
                    .col(ColumnDef::new(Organizaciones::EmailOrganizacion).string_len(255))
                    // persona_juridica fields
                    .col(
                        ColumnDef::new(Organizaciones::Rnc)
                            .string_len(9)
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Organizaciones::RazonSocial).string_len(200))
                    .col(ColumnDef::new(Organizaciones::NombreComercial).string_len(200))
                    .col(ColumnDef::new(Organizaciones::DireccionFiscal).text())
                    .col(ColumnDef::new(Organizaciones::RepresentanteLegal).string_len(200))
                    .col(ColumnDef::new(Organizaciones::DgiiData).json_binary())
                    // timestamps
                    .col(
                        ColumnDef::new(Organizaciones::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(Organizaciones::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Organizaciones::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Organizaciones {
    Table,
    Id,
    Tipo,
    Nombre,
    Estado,
    Cedula,
    Telefono,
    EmailOrganizacion,
    Rnc,
    RazonSocial,
    NombreComercial,
    DireccionFiscal,
    RepresentanteLegal,
    DgiiData,
    CreatedAt,
    UpdatedAt,
}
