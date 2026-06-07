use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260608_000001_add_password_changed_at"
    }
}

#[derive(Iden)]
enum Usuarios {
    Table,
    PasswordChangedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add password_changed_at column with default NOW()
        manager
            .alter_table(
                Table::alter()
                    .table(Usuarios::Table)
                    .add_column(
                        ColumnDef::new(Usuarios::PasswordChangedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Backfill existing rows: set password_changed_at = created_at
        let db = manager.get_connection();
        db.execute_unprepared("UPDATE usuarios SET password_changed_at = created_at")
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Usuarios::Table)
                    .drop_column(Usuarios::PasswordChangedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
