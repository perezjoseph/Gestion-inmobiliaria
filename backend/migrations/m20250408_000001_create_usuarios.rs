use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250408_000001_create_usuarios"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Usuarios::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Usuarios::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(Usuarios::Nombre).string_len(100).not_null())
                    .col(
                        ColumnDef::new(Usuarios::Email)
                            .string_len(255)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Usuarios::PasswordHash)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Usuarios::Rol)
                            .string_len(20)
                            .not_null()
                            .check(Expr::col(Usuarios::Rol).is_in([
                                "admin",
                                "gerente",
                                "visualizador",
                            ])),
                    )
                    .col(
                        ColumnDef::new(Usuarios::Activo)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Usuarios::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(Usuarios::UpdatedAt)
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
                    .name("idx_usuarios_email")
                    .table(Usuarios::Table)
                    .col(Usuarios::Email)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Usuarios::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Usuarios {
    Table,
    Id,
    Nombre,
    Email,
    PasswordHash,
    Rol,
    Activo,
    CreatedAt,
    UpdatedAt,
}
