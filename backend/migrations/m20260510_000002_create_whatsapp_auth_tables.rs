use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260510_000002_create_whatsapp_auth_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create whatsapp_auth_creds table — one row per organization's device identity
        manager
            .create_table(
                Table::create()
                    .table(WhatsappAuthCreds::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WhatsappAuthCreds::RealmId)
                            .string_len(100)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WhatsappAuthCreds::CredsData)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WhatsappAuthCreds::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .to_owned(),
            )
            .await?;

        // Create whatsapp_auth_keys table — Signal Protocol key-value store
        manager
            .create_table(
                Table::create()
                    .table(WhatsappAuthKeys::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WhatsappAuthKeys::RealmId)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WhatsappAuthKeys::Category)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WhatsappAuthKeys::KeyId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WhatsappAuthKeys::KeyData)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WhatsappAuthKeys::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .to_owned(),
            )
            .await?;

        // Composite primary key for keys table
        manager
            .create_index(
                Index::create()
                    .name("pk_whatsapp_auth_keys")
                    .table(WhatsappAuthKeys::Table)
                    .col(WhatsappAuthKeys::RealmId)
                    .col(WhatsappAuthKeys::Category)
                    .col(WhatsappAuthKeys::KeyId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index for listing all keys by realm (used on session restore)
        manager
            .create_index(
                Index::create()
                    .name("idx_whatsapp_auth_keys_realm")
                    .table(WhatsappAuthKeys::Table)
                    .col(WhatsappAuthKeys::RealmId)
                    .to_owned(),
            )
            .await?;

        // Create the dedicated PG role with restricted access
        manager
            .get_connection()
            .execute_unprepared(
                "DO $$ BEGIN
                    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'whatsapp_session_rw') THEN
                        CREATE ROLE whatsapp_session_rw LOGIN;
                    END IF;
                END $$;",
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "GRANT SELECT, INSERT, UPDATE, DELETE ON whatsapp_auth_creds TO whatsapp_session_rw;",
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "GRANT SELECT, INSERT, UPDATE, DELETE ON whatsapp_auth_keys TO whatsapp_session_rw;",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "REVOKE ALL ON whatsapp_auth_creds FROM whatsapp_session_rw;",
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "REVOKE ALL ON whatsapp_auth_keys FROM whatsapp_session_rw;",
            )
            .await?;

        manager
            .drop_table(Table::drop().table(WhatsappAuthKeys::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(WhatsappAuthCreds::Table).to_owned())
            .await?;

        manager
            .get_connection()
            .execute_unprepared("DROP ROLE IF EXISTS whatsapp_session_rw;")
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
pub enum WhatsappAuthCreds {
    Table,
    RealmId,
    CredsData,
    UpdatedAt,
}

#[derive(Iden)]
pub enum WhatsappAuthKeys {
    Table,
    RealmId,
    Category,
    KeyId,
    KeyData,
    UpdatedAt,
}
