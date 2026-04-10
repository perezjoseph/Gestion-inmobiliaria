use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250408_000003_create_inquilinos"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Inquilinos::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Inquilinos::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(Inquilinos::Nombre)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Inquilinos::Apellido)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Inquilinos::Email).string_len(255))
                    .col(ColumnDef::new(Inquilinos::Telefono).string_len(20))
                    .col(
                        ColumnDef::new(Inquilinos::Cedula)
                            .string_len(20)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Inquilinos::ContactoEmergencia).string_len(200))
                    .col(ColumnDef::new(Inquilinos::Notas).text())
                    .col(
                        ColumnDef::new(Inquilinos::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(Inquilinos::UpdatedAt)
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
                    .name("idx_inquilinos_cedula")
                    .table(Inquilinos::Table)
                    .col(Inquilinos::Cedula)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Inquilinos::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Inquilinos {
    Table,
    Id,
    Nombre,
    Apellido,
    Email,
    Telefono,
    Cedula,
    ContactoEmergencia,
    Notas,
    CreatedAt,
    UpdatedAt,
}
