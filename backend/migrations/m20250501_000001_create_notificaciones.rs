use sea_orm_migration::prelude::*;

use super::m20250408_000001_create_usuarios::Usuarios;
use super::m20250413_000001_create_organizaciones::Organizaciones;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250501_000001_create_notificaciones"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Notificaciones::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Notificaciones::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(Notificaciones::Tipo)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Notificaciones::Titulo)
                            .string_len(500)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Notificaciones::Mensaje)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Notificaciones::Leida)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Notificaciones::EntityType)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Notificaciones::EntityId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Notificaciones::UsuarioId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Notificaciones::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Notificaciones::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notificaciones_usuario")
                            .from(Notificaciones::Table, Notificaciones::UsuarioId)
                            .to(Usuarios::Table, Usuarios::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notificaciones_organizacion")
                            .from(Notificaciones::Table, Notificaciones::OrganizacionId)
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notificaciones_usuario_id")
                    .table(Notificaciones::Table)
                    .col(Notificaciones::UsuarioId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notificaciones_usuario_leida")
                    .table(Notificaciones::Table)
                    .col(Notificaciones::UsuarioId)
                    .col(Notificaciones::Leida)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notificaciones_tipo_entity")
                    .table(Notificaciones::Table)
                    .col(Notificaciones::Tipo)
                    .col(Notificaciones::EntityType)
                    .col(Notificaciones::EntityId)
                    .col(Notificaciones::UsuarioId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notificaciones_organizacion_id")
                    .table(Notificaciones::Table)
                    .col(Notificaciones::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notificaciones_created_at")
                    .table(Notificaciones::Table)
                    .col(Notificaciones::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Notificaciones::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Notificaciones {
    Table,
    Id,
    Tipo,
    Titulo,
    Mensaje,
    Leida,
    EntityType,
    EntityId,
    UsuarioId,
    OrganizacionId,
    CreatedAt,
}
