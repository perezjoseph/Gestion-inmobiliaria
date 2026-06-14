use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260614_000001_create_metrics_exporter_role"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DO $$ BEGIN
                    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'metrics_exporter') THEN
                        CREATE ROLE metrics_exporter LOGIN;
                    END IF;
                END $$;",
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared("GRANT pg_monitor TO metrics_exporter;")
            .await?;

        manager
            .get_connection()
            .execute_unprepared("GRANT CONNECT ON DATABASE realestate TO metrics_exporter;")
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("REVOKE CONNECT ON DATABASE realestate FROM metrics_exporter;")
            .await?;

        manager
            .get_connection()
            .execute_unprepared("REVOKE pg_monitor FROM metrics_exporter;")
            .await?;

        manager
            .get_connection()
            .execute_unprepared("DROP ROLE IF EXISTS metrics_exporter;")
            .await?;

        Ok(())
    }
}
