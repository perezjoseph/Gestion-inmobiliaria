use sea_orm_migration::prelude::*;

use super::m20250408_000001_create_usuarios::Usuarios;
use super::m20250408_000003_create_inquilinos::Inquilinos;
use super::m20250408_000004_create_contratos::Contratos;
use super::m20250413_000001_create_organizaciones::Organizaciones;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260509_000001_create_chatbot_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create chatbot_config table
        manager
            .create_table(
                Table::create()
                    .table(ChatbotConfig::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ChatbotConfig::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(ChatbotConfig::OrganizacionId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(ChatbotConfig::Activo)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(ChatbotConfig::ConnectionStatus)
                            .string_len(20)
                            .not_null()
                            .default("disconnected"),
                    )
                    .col(ColumnDef::new(ChatbotConfig::DisplayName).string_len(100))
                    .col(
                        ColumnDef::new(ChatbotConfig::Language)
                            .string_len(10)
                            .not_null()
                            .default("es-DO"),
                    )
                    .col(ColumnDef::new(ChatbotConfig::Tone).string_len(50))
                    .col(ColumnDef::new(ChatbotConfig::Greeting).text())
                    .col(ColumnDef::new(ChatbotConfig::SystemPrompt).text())
                    .col(ColumnDef::new(ChatbotConfig::Faqs).json_binary())
                    .col(ColumnDef::new(ChatbotConfig::Policies).text())
                    .col(
                        ColumnDef::new(ChatbotConfig::SenderPolicy)
                            .string_len(30)
                            .not_null()
                            .default("tenants_only"),
                    )
                    .col(ColumnDef::new(ChatbotConfig::Allowlist).json_binary())
                    .col(ColumnDef::new(ChatbotConfig::Capabilities).json_binary())
                    .col(ColumnDef::new(ChatbotConfig::HandoffKeywords).json_binary())
                    .col(
                        ColumnDef::new(ChatbotConfig::HistoryLimit)
                            .integer()
                            .not_null()
                            .default(10),
                    )
                    .col(
                        ColumnDef::new(ChatbotConfig::RetentionDays)
                            .integer()
                            .not_null()
                            .default(90),
                    )
                    .col(ColumnDef::new(ChatbotConfig::UpdatedBy).uuid())
                    .col(
                        ColumnDef::new(ChatbotConfig::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(ChatbotConfig::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chatbot_config_organizacion")
                            .from(ChatbotConfig::Table, ChatbotConfig::OrganizacionId)
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chatbot_config_updated_by")
                            .from(ChatbotConfig::Table, ChatbotConfig::UpdatedBy)
                            .to(Usuarios::Table, Usuarios::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create chatbot_conversation table
        manager
            .create_table(
                Table::create()
                    .table(ChatbotConversation::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ChatbotConversation::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(ChatbotConversation::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChatbotConversation::SenderPhone)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(ColumnDef::new(ChatbotConversation::InquilinoId).uuid())
                    .col(
                        ColumnDef::new(ChatbotConversation::Role)
                            .string_len(10)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChatbotConversation::Content)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChatbotConversation::MessageType)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(ColumnDef::new(ChatbotConversation::Metadata).json_binary())
                    .col(
                        ColumnDef::new(ChatbotConversation::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chatbot_conversation_organizacion")
                            .from(
                                ChatbotConversation::Table,
                                ChatbotConversation::OrganizacionId,
                            )
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chatbot_conversation_inquilino")
                            .from(ChatbotConversation::Table, ChatbotConversation::InquilinoId)
                            .to(Inquilinos::Table, Inquilinos::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Composite index for history retrieval: (organizacion_id, sender_phone, created_at DESC)
        manager
            .create_index(
                Index::create()
                    .name("idx_chatbot_conversation_org_phone_created")
                    .table(ChatbotConversation::Table)
                    .col(ChatbotConversation::OrganizacionId)
                    .col(ChatbotConversation::SenderPhone)
                    .col(ChatbotConversation::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // Index for retention cleanup: (organizacion_id, created_at)
        manager
            .create_index(
                Index::create()
                    .name("idx_chatbot_conversation_org_created")
                    .table(ChatbotConversation::Table)
                    .col(ChatbotConversation::OrganizacionId)
                    .col(ChatbotConversation::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // Create chatbot_receipt_extraction table
        manager
            .create_table(
                Table::create()
                    .table(ChatbotReceiptExtraction::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ChatbotReceiptExtraction::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(ChatbotReceiptExtraction::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChatbotReceiptExtraction::ConversationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ChatbotReceiptExtraction::InquilinoId).uuid())
                    .col(ColumnDef::new(ChatbotReceiptExtraction::ContratoId).uuid())
                    .col(
                        ColumnDef::new(ChatbotReceiptExtraction::ExtractedData)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChatbotReceiptExtraction::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending_confirmation"),
                    )
                    .col(ColumnDef::new(ChatbotReceiptExtraction::ConfirmedBy).uuid())
                    .col(
                        ColumnDef::new(ChatbotReceiptExtraction::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(ChatbotReceiptExtraction::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chatbot_receipt_organizacion")
                            .from(
                                ChatbotReceiptExtraction::Table,
                                ChatbotReceiptExtraction::OrganizacionId,
                            )
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chatbot_receipt_conversation")
                            .from(
                                ChatbotReceiptExtraction::Table,
                                ChatbotReceiptExtraction::ConversationId,
                            )
                            .to(ChatbotConversation::Table, ChatbotConversation::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chatbot_receipt_inquilino")
                            .from(
                                ChatbotReceiptExtraction::Table,
                                ChatbotReceiptExtraction::InquilinoId,
                            )
                            .to(Inquilinos::Table, Inquilinos::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chatbot_receipt_contrato")
                            .from(
                                ChatbotReceiptExtraction::Table,
                                ChatbotReceiptExtraction::ContratoId,
                            )
                            .to(Contratos::Table, Contratos::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chatbot_receipt_confirmed_by")
                            .from(
                                ChatbotReceiptExtraction::Table,
                                ChatbotReceiptExtraction::ConfirmedBy,
                            )
                            .to(Usuarios::Table, Usuarios::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index on receipt extraction FK columns
        manager
            .create_index(
                Index::create()
                    .name("idx_chatbot_receipt_organizacion_id")
                    .table(ChatbotReceiptExtraction::Table)
                    .col(ChatbotReceiptExtraction::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_chatbot_receipt_status")
                    .table(ChatbotReceiptExtraction::Table)
                    .col(ChatbotReceiptExtraction::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ChatbotReceiptExtraction::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(ChatbotConversation::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ChatbotConfig::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
pub enum ChatbotConfig {
    Table,
    Id,
    OrganizacionId,
    Activo,
    ConnectionStatus,
    DisplayName,
    Language,
    Tone,
    Greeting,
    SystemPrompt,
    Faqs,
    Policies,
    SenderPolicy,
    Allowlist,
    Capabilities,
    HandoffKeywords,
    HistoryLimit,
    RetentionDays,
    UpdatedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum ChatbotConversation {
    Table,
    Id,
    OrganizacionId,
    SenderPhone,
    InquilinoId,
    Role,
    Content,
    MessageType,
    Metadata,
    CreatedAt,
}

#[derive(Iden)]
pub enum ChatbotReceiptExtraction {
    Table,
    Id,
    OrganizacionId,
    ConversationId,
    InquilinoId,
    ContratoId,
    ExtractedData,
    Status,
    ConfirmedBy,
    CreatedAt,
    UpdatedAt,
}
