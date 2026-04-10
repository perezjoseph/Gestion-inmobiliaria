use sea_orm_migration::prelude::*;

use super::m20250413_000001_create_organizaciones::Organizaciones;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250413_000003_create_invitaciones"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Invitaciones::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Invitaciones::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(Invitaciones::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Invitaciones::Email)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Invitaciones::Rol)
                            .string_len(20)
                            .not_null()
                            .check(Expr::col(Invitaciones::Rol).is_in(["gerente", "visualizador"])),
                    )
                    .col(
                        ColumnDef::new(Invitaciones::Token)
                            .string_len(255)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Invitaciones::Usado)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Invitaciones::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Invitaciones::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_invitaciones_organizacion")
                            .from(Invitaciones::Table, Invitaciones::OrganizacionId)
                            .to(Organizaciones::Table, Organizaciones::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_invitaciones_token")
                    .table(Invitaciones::Table)
                    .col(Invitaciones::Token)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_invitaciones_organizacion_id")
                    .table(Invitaciones::Table)
                    .col(Invitaciones::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Invitaciones::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Invitaciones {
    Table,
    Id,
    OrganizacionId,
    Email,
    Rol,
    Token,
    Usado,
    ExpiresAt,
    CreatedAt,
}
