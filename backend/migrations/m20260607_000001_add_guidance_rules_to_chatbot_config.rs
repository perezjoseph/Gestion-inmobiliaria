use sea_orm_migration::prelude::*;

use super::m20260509_000001_create_chatbot_tables::ChatbotConfig;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ChatbotConfig::Table)
                    .add_column(
                        ColumnDef::new(ChatbotConfigGuidance::GuidanceRules)
                            .json_binary()
                            .not_null()
                            .default("[]"),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ChatbotConfig::Table)
                    .drop_column(ChatbotConfigGuidance::GuidanceRules)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum ChatbotConfigGuidance {
    GuidanceRules,
}
