use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260510_000001_make_auditoria_usuario_nullable"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the existing FK constraint
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .table(RegistrosAuditoria::Table)
                    .name("fk_registros_auditoria_usuario")
                    .to_owned(),
            )
            .await?;

        // Make usuario_id nullable
        manager
            .alter_table(
                Table::alter()
                    .table(RegistrosAuditoria::Table)
                    .modify_column(ColumnDef::new(RegistrosAuditoria::UsuarioId).uuid().null())
                    .to_owned(),
            )
            .await?;

        // Re-add FK constraint (now on a nullable column, NULL values skip the check)
        manager
            .alter_table(
                Table::alter()
                    .table(RegistrosAuditoria::Table)
                    .add_foreign_key(
                        &TableForeignKey::new()
                            .name("fk_registros_auditoria_usuario")
                            .from_tbl(RegistrosAuditoria::Table)
                            .from_col(RegistrosAuditoria::UsuarioId)
                            .to_tbl(Usuarios::Table)
                            .to_col(Usuarios::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop FK
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .table(RegistrosAuditoria::Table)
                    .name("fk_registros_auditoria_usuario")
                    .to_owned(),
            )
            .await?;

        // Make usuario_id NOT NULL again (will fail if NULLs exist)
        manager
            .alter_table(
                Table::alter()
                    .table(RegistrosAuditoria::Table)
                    .modify_column(
                        ColumnDef::new(RegistrosAuditoria::UsuarioId)
                            .uuid()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Re-add FK
        manager
            .alter_table(
                Table::alter()
                    .table(RegistrosAuditoria::Table)
                    .add_foreign_key(
                        &TableForeignKey::new()
                            .name("fk_registros_auditoria_usuario")
                            .from_tbl(RegistrosAuditoria::Table)
                            .from_col(RegistrosAuditoria::UsuarioId)
                            .to_tbl(Usuarios::Table)
                            .to_col(Usuarios::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum RegistrosAuditoria {
    Table,
    UsuarioId,
}

#[derive(Iden)]
enum Usuarios {
    Table,
    Id,
}
