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
            .execute_unprepared(
                "DO $$ BEGIN
                    IF NOT pg_has_role('metrics_exporter', 'pg_monitor', 'MEMBER') THEN
                        BEGIN
                            GRANT pg_monitor TO metrics_exporter;
                        EXCEPTION WHEN insufficient_privilege THEN
                            RAISE NOTICE 'skipping GRANT pg_monitor: migrating role lacks ADMIN on pg_monitor (already granted cluster-wide by the privileged environment)';
                        END;
                    END IF;
                END $$;",
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "DO $$ BEGIN
                    BEGIN
                        EXECUTE format('GRANT CONNECT ON DATABASE %I TO metrics_exporter', current_database());
                    EXCEPTION WHEN insufficient_privilege THEN
                        RAISE NOTICE 'skipping GRANT CONNECT: migrating role is not the owner of %', current_database();
                    END;
                END $$;",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DO $$ BEGIN
                    BEGIN
                        EXECUTE format('REVOKE CONNECT ON DATABASE %I FROM metrics_exporter', current_database());
                    EXCEPTION WHEN insufficient_privilege THEN
                        RAISE NOTICE 'skipping REVOKE CONNECT: migrating role is not the owner of %', current_database();
                    END;
                END $$;",
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "DO $$ BEGIN
                    BEGIN
                        REVOKE pg_monitor FROM metrics_exporter;
                    EXCEPTION WHEN insufficient_privilege THEN
                        RAISE NOTICE 'skipping REVOKE pg_monitor: migrating role lacks ADMIN on pg_monitor';
                    END;
                END $$;",
            )
            .await?;

        Ok(())
    }
}
