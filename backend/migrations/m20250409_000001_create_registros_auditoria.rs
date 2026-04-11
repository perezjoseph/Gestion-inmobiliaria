use sea_orm_migration::prelude::*;

use super::m20250408_000001_create_usuarios::Usuarios;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250409_000001_create_registros_auditoria"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RegistrosAuditoria::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RegistrosAuditoria::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(RegistrosAuditoria::UsuarioId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RegistrosAuditoria::EntityType)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RegistrosAuditoria::EntityId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RegistrosAuditoria::Accion)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RegistrosAuditoria::Cambios)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RegistrosAuditoria::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_registros_auditoria_usuario")
                            .from(RegistrosAuditoria::Table, RegistrosAuditoria::UsuarioId)
                            .to(Usuarios::Table, Usuarios::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_registros_auditoria_usuario_id")
                    .table(RegistrosAuditoria::Table)
                    .col(RegistrosAuditoria::UsuarioId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_registros_auditoria_entity_type")
                    .table(RegistrosAuditoria::Table)
                    .col(RegistrosAuditoria::EntityType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_registros_auditoria_entity_id")
                    .table(RegistrosAuditoria::Table)
                    .col(RegistrosAuditoria::EntityId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_registros_auditoria_created_at")
                    .table(RegistrosAuditoria::Table)
                    .col(RegistrosAuditoria::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RegistrosAuditoria::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum RegistrosAuditoria {
    Table,
    Id,
    UsuarioId,
    EntityType,
    EntityId,
    Accion,
    Cambios,
    CreatedAt,
}
