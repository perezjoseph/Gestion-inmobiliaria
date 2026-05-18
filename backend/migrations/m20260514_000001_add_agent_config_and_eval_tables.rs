use sea_orm_migration::prelude::*;

use super::m20250413_000001_create_organizaciones::Organizaciones;
use super::m20260509_000001_create_chatbot_tables::ChatbotConfig;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Add agent_config JSONB column to chatbot_config
        manager
            .alter_table(
                Table::alter()
                    .table(ChatbotConfig::Table)
                    .add_column(
                        ColumnDef::new(ChatbotConfigExtra::AgentConfig)
                            .json_binary()
                            .not_null()
                            .default("{}"),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Create chatbot_eval_suite table
        manager
            .create_table(
                Table::create()
                    .table(ChatbotEvalSuite::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ChatbotEvalSuite::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ChatbotEvalSuite::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChatbotEvalSuite::Name)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(ColumnDef::new(ChatbotEvalSuite::Description).text())
                    .col(
                        ColumnDef::new(ChatbotEvalSuite::Cases)
                            .json_binary()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(ChatbotEvalSuite::Metrics)
                            .json_binary()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(ChatbotEvalSuite::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChatbotEvalSuite::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chatbot_eval_suite_organizacion")
                            .from(ChatbotEvalSuite::Table, ChatbotEvalSuite::OrganizacionId)
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 3. Create chatbot_eval_run table
        manager
            .create_table(
                Table::create()
                    .table(ChatbotEvalRun::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ChatbotEvalRun::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ChatbotEvalRun::SuiteId).uuid().not_null())
                    .col(
                        ColumnDef::new(ChatbotEvalRun::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChatbotEvalRun::Status)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(ColumnDef::new(ChatbotEvalRun::Results).json_binary())
                    .col(ColumnDef::new(ChatbotEvalRun::Summary).json_binary())
                    .col(
                        ColumnDef::new(ChatbotEvalRun::AgentConfigSnapshot)
                            .json_binary()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(ChatbotEvalRun::StartedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ChatbotEvalRun::CompletedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chatbot_eval_run_suite")
                            .from(ChatbotEvalRun::Table, ChatbotEvalRun::SuiteId)
                            .to(ChatbotEvalSuite::Table, ChatbotEvalSuite::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chatbot_eval_run_organizacion")
                            .from(ChatbotEvalRun::Table, ChatbotEvalRun::OrganizacionId)
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 4. Create indexes on FK columns
        manager
            .create_index(
                Index::create()
                    .name("idx_chatbot_eval_suite_organizacion_id")
                    .table(ChatbotEvalSuite::Table)
                    .col(ChatbotEvalSuite::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_chatbot_eval_run_suite_id")
                    .table(ChatbotEvalRun::Table)
                    .col(ChatbotEvalRun::SuiteId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_chatbot_eval_run_organizacion_id")
                    .table(ChatbotEvalRun::Table)
                    .col(ChatbotEvalRun::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ChatbotEvalRun::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ChatbotEvalSuite::Table).to_owned())
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ChatbotConfig::Table)
                    .drop_column(ChatbotConfigExtra::AgentConfig)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

/// Extra column added to ChatbotConfig in this migration.
#[derive(DeriveIden)]
enum ChatbotConfigExtra {
    AgentConfig,
}

#[derive(DeriveIden)]
pub enum ChatbotEvalSuite {
    Table,
    Id,
    OrganizacionId,
    Name,
    Description,
    Cases,
    Metrics,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub enum ChatbotEvalRun {
    Table,
    Id,
    SuiteId,
    OrganizacionId,
    Status,
    Results,
    Summary,
    AgentConfigSnapshot,
    StartedAt,
    CompletedAt,
}
