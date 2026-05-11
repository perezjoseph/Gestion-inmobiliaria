use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::entities::{
    chatbot_config, chatbot_conversation, chatbot_receipt_extraction, contrato, pago,
    solicitud_mantenimiento,
};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::chatbot::{
    BalanceResponse, Capabilities, ChatbotConfigResponse, ChatbotConfigUpdateRequest, Confidence,
    ConversationListResponse, CurrencyTotal, FaqEntry, PaymentDetail, format_currency,
};
use crate::services::validation::validate_enum;

const SENDER_POLICIES: &[&str] = &["tenants_only", "tenants_and_prospects", "allowlist"];

// --- Configuration Service ---

/// Load chatbot config for an organization, or return a default response if none exists.
pub async fn get_config<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
) -> Result<ChatbotConfigResponse, AppError> {
    let record = chatbot_config::Entity::find()
        .filter(chatbot_config::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?;

    match record {
        Some(model) => Ok(config_model_to_response(model)?),
        None => Ok(default_config_response(org_id)),
    }
}

/// Load the raw chatbot config entity model for an organization.
/// Returns a default model if none exists. Used by the test chat handler
/// to build persona/capabilities from the saved (or default) config.
pub async fn get_config_model<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
) -> Result<chatbot_config::Model, AppError> {
    let record = chatbot_config::Entity::find()
        .filter(chatbot_config::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?;

    Ok(record.unwrap_or_else(|| chatbot_config::Model {
        id: Uuid::new_v4(),
        organizacion_id: org_id,
        activo: false,
        connection_status: "disconnected".to_string(),
        display_name: None,
        language: "es-DO".to_string(),
        tone: None,
        greeting: None,
        system_prompt: None,
        faqs: None,
        policies: None,
        sender_policy: "tenants_only".to_string(),
        allowlist: None,
        capabilities: None,
        handoff_keywords: None,
        history_limit: 10,
        retention_days: 90,
        updated_by: None,
        created_at: Utc::now().into(),
        updated_at: Utc::now().into(),
    }))
}

/// Create or update chatbot config for an organization.
/// Enforces role check: only `admin` or `gerente` may call this.
pub async fn upsert_config<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    input: ChatbotConfigUpdateRequest,
    user_id: Uuid,
    user_role: &str,
) -> Result<ChatbotConfigResponse, AppError> {
    enforce_config_role(user_role)?;
    validate_config(&input)?;

    let existing = chatbot_config::Entity::find()
        .filter(chatbot_config::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?;

    let now = Utc::now().into();

    let record = if let Some(model) = existing {
        let mut active: chatbot_config::ActiveModel = model.into();
        apply_update_fields(&mut active, &input);
        active.updated_by = Set(Some(user_id));
        active.updated_at = Set(now);
        active.update(db).await?
    } else {
        let id = Uuid::new_v4();
        let active = chatbot_config::ActiveModel {
            id: Set(id),
            organizacion_id: Set(org_id),
            activo: Set(input.activo.unwrap_or(false)),
            connection_status: Set("disconnected".to_string()),
            display_name: Set(input.display_name.clone()),
            language: Set(input
                .language
                .clone()
                .unwrap_or_else(|| "es-DO".to_string())),
            tone: Set(input.tone.clone()),
            greeting: Set(input.greeting.clone()),
            system_prompt: Set(input.system_prompt.clone()),
            faqs: Set(input
                .faqs
                .as_ref()
                .map(|f| serde_json::to_value(f).unwrap_or_default())),
            policies: Set(input.policies.clone()),
            sender_policy: Set(input
                .sender_policy
                .clone()
                .unwrap_or_else(|| "tenants_only".to_string())),
            allowlist: Set(input
                .allowlist
                .as_ref()
                .map(|a| serde_json::to_value(a).unwrap_or_default())),
            capabilities: Set(input
                .capabilities
                .as_ref()
                .map(|c| serde_json::to_value(c).unwrap_or_default())),
            handoff_keywords: Set(input
                .handoff_keywords
                .as_ref()
                .map(|k| serde_json::to_value(k).unwrap_or_default())),
            history_limit: Set(input.history_limit.unwrap_or(10)),
            retention_days: Set(input.retention_days.unwrap_or(90)),
            updated_by: Set(Some(user_id)),
            created_at: Set(now),
            updated_at: Set(now),
        };
        active.insert(db).await?
    };

    config_model_to_response(record)
}

// --- Validation ---

/// Validate a chatbot config update request.
/// Rules:
/// - `display_name`: 1–100 characters
/// - `tone`: 1–50 characters
/// - `faqs`: max 50 entries, question max 200 chars, answer max 1000 chars
/// - `policies`: max 5000 characters
/// - `sender_policy`: must be one of the allowed values
/// - `history_limit`: 1–50
/// - `retention_days`: 1–365
pub fn validate_config(input: &ChatbotConfigUpdateRequest) -> Result<(), AppError> {
    if let Some(ref name) = input.display_name {
        if name.is_empty() || name.len() > 100 {
            return Err(AppError::Validation(
                "display_name debe tener entre 1 y 100 caracteres".to_string(),
            ));
        }
    }

    if let Some(ref tone) = input.tone {
        if tone.is_empty() || tone.len() > 50 {
            return Err(AppError::Validation(
                "tone debe tener entre 1 y 50 caracteres".to_string(),
            ));
        }
    }

    if let Some(ref faqs) = input.faqs {
        validate_faqs(faqs)?;
    }

    if let Some(ref policies) = input.policies {
        if policies.len() > 5000 {
            return Err(AppError::Validation(
                "policies no puede exceder 5000 caracteres".to_string(),
            ));
        }
    }

    if let Some(ref sender_policy) = input.sender_policy {
        validate_enum("sender_policy", sender_policy, SENDER_POLICIES)?;
    }

    if let Some(history_limit) = input.history_limit {
        if !(1..=50).contains(&history_limit) {
            return Err(AppError::Validation(
                "history_limit debe estar entre 1 y 50".to_string(),
            ));
        }
    }

    if let Some(retention_days) = input.retention_days {
        if !(1..=365).contains(&retention_days) {
            return Err(AppError::Validation(
                "retention_days debe estar entre 1 y 365".to_string(),
            ));
        }
    }

    Ok(())
}

fn validate_faqs(faqs: &[FaqEntry]) -> Result<(), AppError> {
    if faqs.len() > 50 {
        return Err(AppError::Validation(
            "No se permiten más de 50 FAQs".to_string(),
        ));
    }

    for (i, faq) in faqs.iter().enumerate() {
        if faq.question.is_empty() || faq.question.len() > 200 {
            return Err(AppError::Validation(format!(
                "FAQ #{}: la pregunta debe tener entre 1 y 200 caracteres",
                i + 1
            )));
        }
        if faq.answer.is_empty() || faq.answer.len() > 1000 {
            return Err(AppError::Validation(format!(
                "FAQ #{}: la respuesta debe tener entre 1 y 1000 caracteres",
                i + 1
            )));
        }
    }

    Ok(())
}

// --- Role Enforcement ---

/// Only `admin` or `gerente` roles may modify chatbot configuration.
pub(crate) fn enforce_config_role(role: &str) -> Result<(), AppError> {
    match role {
        "admin" | "gerente" => Ok(()),
        _ => Err(AppError::Forbidden(
            "Solo administradores y gerentes pueden modificar la configuración del chatbot"
                .to_string(),
        )),
    }
}

// --- Helpers ---

fn apply_update_fields(
    active: &mut chatbot_config::ActiveModel,
    input: &ChatbotConfigUpdateRequest,
) {
    if let Some(activo) = input.activo {
        active.activo = Set(activo);
    }
    if let Some(ref display_name) = input.display_name {
        active.display_name = Set(Some(display_name.clone()));
    }
    if let Some(ref language) = input.language {
        active.language = Set(language.clone());
    }
    if let Some(ref tone) = input.tone {
        active.tone = Set(Some(tone.clone()));
    }
    if let Some(ref greeting) = input.greeting {
        active.greeting = Set(Some(greeting.clone()));
    }
    if let Some(ref system_prompt) = input.system_prompt {
        active.system_prompt = Set(Some(system_prompt.clone()));
    }
    if let Some(ref faqs) = input.faqs {
        active.faqs = Set(Some(serde_json::to_value(faqs).unwrap_or_default()));
    }
    if let Some(ref policies) = input.policies {
        active.policies = Set(Some(policies.clone()));
    }
    if let Some(ref sender_policy) = input.sender_policy {
        active.sender_policy = Set(sender_policy.clone());
    }
    if let Some(ref allowlist) = input.allowlist {
        active.allowlist = Set(Some(serde_json::to_value(allowlist).unwrap_or_default()));
    }
    if let Some(ref capabilities) = input.capabilities {
        active.capabilities = Set(Some(serde_json::to_value(capabilities).unwrap_or_default()));
    }
    if let Some(ref handoff_keywords) = input.handoff_keywords {
        active.handoff_keywords = Set(Some(
            serde_json::to_value(handoff_keywords).unwrap_or_default(),
        ));
    }
    if let Some(history_limit) = input.history_limit {
        active.history_limit = Set(history_limit);
    }
    if let Some(retention_days) = input.retention_days {
        active.retention_days = Set(retention_days);
    }
}

pub(crate) fn config_model_to_response(
    model: chatbot_config::Model,
) -> Result<ChatbotConfigResponse, AppError> {
    let faqs: Option<Vec<FaqEntry>> = model
        .faqs
        .as_ref()
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error deserializando FAQs: {e}")))?;

    let allowlist: Option<Vec<String>> = model
        .allowlist
        .as_ref()
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error deserializando allowlist: {e}")))?;

    let capabilities: Capabilities = model
        .capabilities
        .as_ref()
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error deserializando capabilities: {e}")))?
        .unwrap_or(Capabilities {
            receipt_ocr: false,
            balance_queries: false,
            payment_reminders: false,
            maintenance_requests: false,
            human_handoff: false,
        });

    let handoff_keywords: Option<Vec<String>> = model
        .handoff_keywords
        .as_ref()
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|e| {
            AppError::Internal(anyhow::anyhow!(
                "Error deserializando handoff_keywords: {e}"
            ))
        })?;

    Ok(ChatbotConfigResponse {
        id: model.id,
        organizacion_id: model.organizacion_id,
        activo: model.activo,
        connection_status: model.connection_status,
        display_name: model.display_name,
        language: model.language,
        tone: model.tone,
        greeting: model.greeting,
        system_prompt: model.system_prompt,
        faqs,
        policies: model.policies,
        sender_policy: model.sender_policy,
        allowlist,
        capabilities,
        handoff_keywords,
        history_limit: model.history_limit,
        retention_days: model.retention_days,
        created_at: model.created_at.into(),
        updated_at: model.updated_at.into(),
    })
}

fn default_config_response(org_id: Uuid) -> ChatbotConfigResponse {
    let now = Utc::now();
    ChatbotConfigResponse {
        id: Uuid::nil(),
        organizacion_id: org_id,
        activo: false,
        connection_status: "disconnected".to_string(),
        display_name: None,
        language: "es-DO".to_string(),
        tone: None,
        greeting: None,
        system_prompt: None,
        faqs: None,
        policies: None,
        sender_policy: "tenants_only".to_string(),
        allowlist: None,
        capabilities: Capabilities {
            receipt_ocr: false,
            balance_queries: false,
            payment_reminders: false,
            maintenance_requests: false,
            human_handoff: false,
        },
        handoff_keywords: None,
        history_limit: 10,
        retention_days: 90,
        created_at: now,
        updated_at: now,
    }
}

// --- Sender Policy Engine ---

/// Determines whether an incoming sender is authorized to interact with the chatbot
/// based on the organization's configured sender policy.
///
/// Policies:
/// - `tenants_only`: allowed only if the phone matches a tenant in the org (requires DB)
/// - `tenants_and_prospects`: all senders allowed
/// - `allowlist`: allowed only if the phone appears in the org's configured allowlist
/// - Any unrecognized value: denied (fail-closed)
pub async fn is_sender_allowed<C: ConnectionTrait>(
    sender_policy: &str,
    phone: &str,
    allowlist: Option<&[String]>,
    org_id: Uuid,
    db: &C,
) -> bool {
    match sender_policy {
        "tenants_only" => tenant_exists_by_phone(phone, org_id, db).await,
        "tenants_and_prospects" => true,
        "allowlist" => is_phone_in_allowlist(phone, allowlist),
        _ => false, // fail-closed for unrecognized policies
    }
}

/// Pure policy check: returns true iff the phone appears in the allowlist.
/// Returns false if the allowlist is `None` or empty.
pub fn is_phone_in_allowlist(phone: &str, allowlist: Option<&[String]>) -> bool {
    allowlist.is_some_and(|list| list.iter().any(|entry| entry == phone))
}

/// Pure policy check for sender policy without DB access.
///
/// Handles `tenants_and_prospects` (always true), `allowlist` (check list),
/// and any unrecognized policy (always false / fail-closed).
/// For `tenants_only`, returns `None` indicating DB lookup is required.
pub fn check_sender_policy_no_db(
    sender_policy: &str,
    phone: &str,
    allowlist: Option<&[String]>,
) -> Option<bool> {
    match sender_policy {
        "tenants_only" => None, // requires DB lookup
        "tenants_and_prospects" => Some(true),
        "allowlist" => Some(is_phone_in_allowlist(phone, allowlist)),
        _ => Some(false), // fail-closed for unrecognized policies
    }
}

/// Check if a tenant exists with the given phone in the specified organization.
async fn tenant_exists_by_phone<C: ConnectionTrait>(phone: &str, org_id: Uuid, db: &C) -> bool {
    use crate::entities::inquilino;

    inquilino::Entity::find()
        .filter(inquilino::Column::Telefono.eq(phone))
        .filter(inquilino::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await
        .ok()
        .flatten()
        .is_some()
}

/// Resolved tenant info for the incoming webhook pipeline.
pub struct ResolvedTenant {
    pub id: Uuid,
    pub nombre: String,
    pub apellido: String,
}

/// Find a tenant by phone number and organization.
/// Returns `None` if no tenant matches.
pub async fn find_tenant_by_phone<C: ConnectionTrait>(
    db: &C,
    phone: &str,
    org_id: Uuid,
) -> Result<Option<ResolvedTenant>, AppError> {
    use crate::entities::inquilino;

    let record = inquilino::Entity::find()
        .filter(inquilino::Column::Telefono.eq(phone))
        .filter(inquilino::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?;

    Ok(record.map(|t| ResolvedTenant {
        id: t.id,
        nombre: t.nombre,
        apellido: t.apellido,
    }))
}

// --- Phone Number Validation & Normalization ---

/// Validates that a phone number conforms to E.164 format.
///
/// E.164: a `+` followed by 1–15 digits, where the first digit is not zero.
/// Pattern: `^\+[1-9]\d{1,14}$`
pub fn validate_e164(phone: &str) -> Result<(), AppError> {
    let bytes = phone.as_bytes();

    // Must start with '+'
    if bytes.first() != Some(&b'+') {
        return Err(AppError::Validation(
            "Número de teléfono debe comenzar con '+'".to_string(),
        ));
    }

    let digits = &phone[1..];

    // Must have 1–15 digits after the '+'
    if digits.is_empty() || digits.len() > 15 {
        return Err(AppError::Validation(
            "Número E.164 debe tener entre 1 y 15 dígitos después del '+'".to_string(),
        ));
    }

    // First digit must not be zero
    if digits.starts_with('0') {
        return Err(AppError::Validation(
            "Primer dígito después del '+' no puede ser cero".to_string(),
        ));
    }

    // All characters after '+' must be digits
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AppError::Validation(
            "Número E.164 solo puede contener dígitos después del '+'".to_string(),
        ));
    }

    Ok(())
}

/// Normalizes a phone number input to E.164 format.
///
/// Steps:
/// 1. If the input starts with '+', preserve it and strip non-digit chars from the rest.
/// 2. Otherwise, strip all non-digit characters.
/// 3. Remove leading trunk zeros (zeros at the start of the digit string).
/// 4. Prepend the `default_country_code` (e.g., "+1") if the number doesn't already have one.
/// 5. Validate the result is valid E.164.
///
/// Rejects numbers that:
/// - Have fewer than 7 digits after stripping
/// - Contain non-numeric characters other than a leading '+'
pub fn normalize_phone(input: &str, default_country_code: &str) -> Result<String, AppError> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "Número de teléfono no puede estar vacío".to_string(),
        ));
    }

    // Check for invalid characters: only digits, '+', spaces, dashes, dots, parens allowed
    let has_invalid_chars = trimmed.chars().any(|c| {
        !c.is_ascii_digit() && c != '+' && c != '-' && c != ' ' && c != '.' && c != '(' && c != ')'
    });
    if has_invalid_chars {
        return Err(AppError::Validation(
            "Número de teléfono contiene caracteres inválidos".to_string(),
        ));
    }

    let (has_plus, raw_digits) = trimmed.strip_prefix('+').map_or_else(
        || {
            let digits: String = trimmed.chars().filter(char::is_ascii_digit).collect();
            (false, digits)
        },
        |after_plus| {
            let digits: String = after_plus.chars().filter(char::is_ascii_digit).collect();
            (true, digits)
        },
    );

    // Remove leading trunk zeros
    let stripped = raw_digits.trim_start_matches('0');

    if stripped.is_empty() {
        return Err(AppError::Validation(
            "Número de teléfono no contiene dígitos válidos".to_string(),
        ));
    }

    // Build the normalized number
    let normalized = if has_plus {
        format!("+{stripped}")
    } else {
        let code = default_country_code.trim_start_matches('+');
        format!("+{code}{stripped}")
    };

    // Check minimum digit count (at least 7 digits after '+')
    let digit_count = normalized.len() - 1; // subtract the '+'
    if digit_count < 7 {
        return Err(AppError::Validation(format!(
            "Número de teléfono debe tener al menos 7 dígitos, tiene {digit_count}"
        )));
    }

    // Validate the final result as E.164
    validate_e164(&normalized)?;

    Ok(normalized)
}

// --- Conversation History Service ---

/// A lightweight message representation for pure windowing logic.
/// Used in property-based testing to model the DB query behavior without requiring a database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampedMessage {
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Pure function modeling the conversation history windowing logic.
///
/// Given a list of messages (in any order) and a history limit N,
/// returns the min(len, N) most recent messages sorted by `created_at` DESC.
///
/// This mirrors the DB query: `ORDER BY created_at DESC LIMIT N`.
pub fn window_history(messages: &[TimestampedMessage], limit: usize) -> Vec<&TimestampedMessage> {
    let mut sorted: Vec<&TimestampedMessage> = messages.iter().collect();
    sorted.sort_by_key(|m| std::cmp::Reverse(m.created_at));
    sorted.truncate(limit);
    sorted
}

/// Persist a message (user or assistant) in the conversation history.
///
/// Auto-generates a UUID for the record and uses the current timestamp.
pub async fn persist_message<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    sender_phone: &str,
    inquilino_id: Option<Uuid>,
    role: &str,
    content: &str,
    message_type: &str,
    metadata: Option<serde_json::Value>,
) -> Result<chatbot_conversation::Model, AppError> {
    let active = chatbot_conversation::ActiveModel {
        id: Set(Uuid::new_v4()),
        organizacion_id: Set(org_id),
        sender_phone: Set(sender_phone.to_string()),
        inquilino_id: Set(inquilino_id),
        role: Set(role.to_string()),
        content: Set(content.to_string()),
        message_type: Set(message_type.to_string()),
        metadata: Set(metadata),
        created_at: Set(Utc::now().into()),
    };

    let record = active.insert(db).await?;
    Ok(record)
}

/// Load the last N messages for a specific (`org_id`, `sender_phone`) pair,
/// ordered by `created_at` DESC.
pub async fn load_history<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    sender_phone: &str,
    limit: u64,
) -> Result<Vec<chatbot_conversation::Model>, AppError> {
    let messages = chatbot_conversation::Entity::find()
        .filter(chatbot_conversation::Column::OrganizacionId.eq(org_id))
        .filter(chatbot_conversation::Column::SenderPhone.eq(sender_phone))
        .order_by_desc(chatbot_conversation::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await?;

    Ok(messages)
}

/// List recent conversations for an organization (paginated).
///
/// Returns distinct sender phones with their latest message, timestamp, and message count.
pub async fn list_conversations<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    page: u64,
    per_page: u64,
) -> Result<PaginatedResponse<ConversationListResponse>, AppError> {
    use sea_orm::{DbBackend, FromQueryResult, Statement};

    // We need a custom query to get distinct sender_phone with aggregates.
    #[derive(Debug, FromQueryResult)]
    struct ConversationRow {
        sender_phone: String,
        inquilino_id: Option<Uuid>,
        last_message: String,
        last_message_at: chrono::DateTime<Utc>,
        message_count: i64,
    }

    let offset = (page - 1) * per_page;

    // Count distinct sender phones
    let count_sql = format!(
        "SELECT COUNT(DISTINCT sender_phone) as count FROM chatbot_conversation WHERE organizacion_id = '{org_id}'"
    );
    let count_result: Option<i64> = db
        .query_one(Statement::from_string(DbBackend::Postgres, count_sql))
        .await?
        .and_then(|r| r.try_get_by_index::<i64>(0).ok());
    let total = count_result.unwrap_or(0) as u64;

    // Get paginated distinct conversations with latest message
    let query_sql = format!(
        "
        SELECT DISTINCT ON (sender_phone)
            sender_phone,
            inquilino_id,
            content as last_message,
            created_at as last_message_at,
            (SELECT COUNT(*) FROM chatbot_conversation c2
             WHERE c2.organizacion_id = '{org_id}' AND c2.sender_phone = chatbot_conversation.sender_phone) as message_count
        FROM chatbot_conversation
        WHERE organizacion_id = '{org_id}'
        ORDER BY sender_phone, created_at DESC
        LIMIT {per_page} OFFSET {offset}
        "
    );

    let rows: Vec<ConversationRow> =
        ConversationRow::find_by_statement(Statement::from_string(DbBackend::Postgres, query_sql))
            .all(db)
            .await?;

    let data = rows
        .into_iter()
        .map(|r| ConversationListResponse {
            sender_phone: r.sender_phone,
            inquilino_id: r.inquilino_id,
            last_message: r.last_message,
            last_message_at: r.last_message_at,
            message_count: r.message_count,
        })
        .collect();

    Ok(PaginatedResponse {
        data,
        total,
        page,
        per_page,
    })
}

/// Delete conversation messages older than the retention period for an organization.
///
/// Deletes all messages where `created_at < (now - retention_days)`.
/// Returns the number of rows deleted.
pub async fn cleanup_expired<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    retention_days: i64,
) -> Result<u64, AppError> {
    let cutoff = Utc::now() - Duration::days(retention_days);

    let result = chatbot_conversation::Entity::delete_many()
        .filter(chatbot_conversation::Column::OrganizacionId.eq(org_id))
        .filter(chatbot_conversation::Column::CreatedAt.lt(cutoff))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

// --- Balance Query Service ---

/// Queries the outstanding balance for a tenant within an organization.
///
/// Finds all payments with status `pendiente` or `atrasado` belonging to
/// the tenant's contracts, groups totals by currency, and formats amounts
/// with the appropriate currency symbol.
pub async fn query_tenant_balance<C: ConnectionTrait>(
    db: &C,
    inquilino_id: Uuid,
    org_id: Uuid,
) -> Result<BalanceResponse, AppError> {
    // Step 1: Find all contrato IDs for this tenant in this org
    let contrato_ids: Vec<Uuid> = contrato::Entity::find()
        .filter(contrato::Column::InquilinoId.eq(inquilino_id))
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .select_only()
        .column(contrato::Column::Id)
        .into_tuple()
        .all(db)
        .await?;

    if contrato_ids.is_empty() {
        return Ok(BalanceResponse {
            payments: vec![],
            totals: vec![],
        });
    }

    // Step 2: Find all pagos with status pendiente or atrasado for those contracts
    let pagos = pago::Entity::find()
        .filter(pago::Column::ContratoId.is_in(contrato_ids))
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .filter(
            pago::Column::Estado
                .eq("pendiente")
                .or(pago::Column::Estado.eq("atrasado")),
        )
        .order_by_asc(pago::Column::FechaVencimiento)
        .all(db)
        .await?;

    // Step 3: Build payment details and group totals by currency
    let mut totals_map: HashMap<String, Decimal> = HashMap::new();
    let mut payments = Vec::with_capacity(pagos.len());

    for p in &pagos {
        let entry = totals_map.entry(p.moneda.clone()).or_insert(Decimal::ZERO);
        *entry += p.monto;

        payments.push(PaymentDetail {
            amount: p.monto,
            currency: p.moneda.clone(),
            formatted_amount: format_currency(p.monto, &p.moneda),
            due_date: p.fecha_vencimiento,
            status: p.estado.clone(),
        });
    }

    // Step 4: Build per-currency totals
    let mut totals: Vec<CurrencyTotal> = totals_map
        .into_iter()
        .map(|(currency, total)| CurrencyTotal {
            formatted_total: format_currency(total, &currency),
            currency,
            total,
        })
        .collect();

    // Sort for deterministic output (DOP first, then USD)
    totals.sort_by(|a, b| a.currency.cmp(&b.currency));

    Ok(BalanceResponse { payments, totals })
}

// --- Receipt Extraction Service ---

/// Records a receipt extraction result in the `chatbot_receipt_extraction` table.
///
/// Routing logic (per requirements 4.3, 4.4, 4.5):
/// - All extractions are stored with status `pending_confirmation`.
/// - The difference in confidence level affects the reply to the tenant (handled elsewhere),
///   but all extractions queue for landlord confirmation regardless.
///
/// Auto-generates a UUID for the record and uses the current timestamp.
pub async fn record_extraction<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    conversation_id: Uuid,
    inquilino_id: Option<Uuid>,
    contrato_id: Option<Uuid>,
    extracted_data: serde_json::Value,
    _confidence: &Confidence,
) -> Result<chatbot_receipt_extraction::Model, AppError> {
    let now = Utc::now().into();

    let active = chatbot_receipt_extraction::ActiveModel {
        id: Set(Uuid::new_v4()),
        organizacion_id: Set(org_id),
        conversation_id: Set(conversation_id),
        inquilino_id: Set(inquilino_id),
        contrato_id: Set(contrato_id),
        extracted_data: Set(extracted_data),
        status: Set("pending_confirmation".to_string()),
        confirmed_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = active.insert(db).await?;
    Ok(record)
}

// --- Receipt Confirmation Workflow ---

/// Lists all receipt extractions with status `pending_confirmation` for an organization,
/// ordered by `created_at` DESC.
pub async fn list_pending_receipts<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
) -> Result<Vec<chatbot_receipt_extraction::Model>, AppError> {
    let records = chatbot_receipt_extraction::Entity::find()
        .filter(chatbot_receipt_extraction::Column::OrganizacionId.eq(org_id))
        .filter(chatbot_receipt_extraction::Column::Status.eq("pending_confirmation"))
        .order_by_desc(chatbot_receipt_extraction::Column::CreatedAt)
        .all(db)
        .await?;

    Ok(records)
}

/// Confirms a receipt extraction: sets status to `confirmed`, records the confirming user,
/// and creates a Pago record from the extracted data.
///
/// Enforces atomic status transition: rejects if not in `pending_confirmation`.
pub async fn confirm_receipt<C: ConnectionTrait>(
    db: &C,
    extraction_id: Uuid,
    user_id: Uuid,
) -> Result<chatbot_receipt_extraction::Model, AppError> {
    // Load extraction
    let extraction = chatbot_receipt_extraction::Entity::find_by_id(extraction_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "Extracción de recibo no encontrada: {extraction_id}"
            ))
        })?;

    // Enforce atomic status transition
    if extraction.status != "pending_confirmation" {
        return Err(AppError::Conflict(
            "La extracción ya fue procesada y no está pendiente de confirmación".to_string(),
        ));
    }

    // Extract payment data from JSONB
    let data = &extraction.extracted_data;
    let amount: Decimal = data
        .get("amount")
        .and_then(|v| {
            v.as_f64().map_or_else(
                || v.as_str().and_then(|s| Decimal::from_str_exact(s).ok()),
                |f| Decimal::from_str_exact(&format!("{f:.2}")).ok(),
            )
        })
        .ok_or_else(|| {
            AppError::Validation("Datos extraídos no contienen un monto válido".to_string())
        })?;

    let currency = data
        .get("currency")
        .and_then(|v| v.as_str())
        .unwrap_or("DOP")
        .to_string();

    let payment_date = data
        .get("date")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    // We need a contrato_id to create the Pago
    let contrato_id = extraction.contrato_id.ok_or_else(|| {
        AppError::Validation(
            "La extracción no tiene un contrato asociado para crear el pago".to_string(),
        )
    })?;

    let now = Utc::now().into();

    // Create Pago record
    let pago_active = pago::ActiveModel {
        id: Set(Uuid::new_v4()),
        contrato_id: Set(contrato_id),
        monto: Set(amount),
        moneda: Set(currency),
        fecha_pago: Set(payment_date),
        fecha_vencimiento: Set(payment_date.unwrap_or_else(|| Utc::now().date_naive())),
        metodo_pago: Set(Some("transferencia".to_string())),
        estado: Set("pagado".to_string()),
        notas: Set(Some(format!(
            "Pago confirmado desde recibo WhatsApp (extracción: {extraction_id})"
        ))),
        recargo: Set(None),
        organizacion_id: Set(extraction.organizacion_id),
        created_at: Set(now),
        updated_at: Set(now),
    };
    pago_active.insert(db).await?;

    // Update extraction status to confirmed
    let mut active: chatbot_receipt_extraction::ActiveModel = extraction.into();
    active.status = Set("confirmed".to_string());
    active.confirmed_by = Set(Some(user_id));
    active.updated_at = Set(now);
    let updated = active.update(db).await?;

    Ok(updated)
}

/// Rejects a receipt extraction: sets status to `rejected` and persists the rejection reason
/// in the `extracted_data` JSONB (under key `rejection_reason`).
///
/// Enforces atomic status transition: rejects if not in `pending_confirmation`.
pub async fn reject_receipt<C: ConnectionTrait>(
    db: &C,
    extraction_id: Uuid,
    user_id: Uuid,
    reason: Option<&str>,
) -> Result<chatbot_receipt_extraction::Model, AppError> {
    // Load extraction
    let extraction = chatbot_receipt_extraction::Entity::find_by_id(extraction_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "Extracción de recibo no encontrada: {extraction_id}"
            ))
        })?;

    // Enforce atomic status transition
    if extraction.status != "pending_confirmation" {
        return Err(AppError::Conflict(
            "La extracción ya fue procesada y no está pendiente de confirmación".to_string(),
        ));
    }

    let now = Utc::now().into();

    // Store rejection reason in extracted_data JSONB
    let mut updated_data = extraction.extracted_data.clone();
    if let Some(r) = reason {
        if let Some(obj) = updated_data.as_object_mut() {
            obj.insert(
                "rejection_reason".to_string(),
                serde_json::Value::String(r.to_string()),
            );
            obj.insert(
                "rejected_by".to_string(),
                serde_json::Value::String(user_id.to_string()),
            );
        }
    }

    let mut active: chatbot_receipt_extraction::ActiveModel = extraction.into();
    active.status = Set("rejected".to_string());
    active.extracted_data = Set(updated_data);
    active.updated_at = Set(now);
    let updated = active.update(db).await?;

    Ok(updated)
}

// --- Maintenance Request Creation (Chatbot) ---

const VALID_PRIORITIES: &[&str] = &["baja", "media", "alta", "urgente"];

/// Creates a maintenance request (`SolicitudMantenimiento`) from a chatbot interaction.
///
/// Finds the tenant's active contract(s) in the organization, picks the most recently
/// started one, and links the request to that contract's property.
///
/// Defaults: status = `pendiente`, priority = `media` (unless specified).
/// Validates description length: 2–1000 characters.
pub async fn create_maintenance_from_chat<C: ConnectionTrait>(
    db: &C,
    inquilino_id: Uuid,
    org_id: Uuid,
    description: &str,
    priority: Option<&str>,
) -> Result<solicitud_mantenimiento::Model, AppError> {
    // Validate description length (2–1000 chars)
    let desc_len = description.chars().count();
    if !(2..=1000).contains(&desc_len) {
        return Err(AppError::Validation(
            "La descripción debe tener entre 2 y 1000 caracteres".to_string(),
        ));
    }

    // Validate priority if provided
    let prioridad = priority.unwrap_or("media");
    validate_enum("prioridad", prioridad, VALID_PRIORITIES)?;

    // Find active contracts for this tenant in this org, ordered by fecha_inicio DESC
    let active_contract = contrato::Entity::find()
        .filter(contrato::Column::InquilinoId.eq(inquilino_id))
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .filter(contrato::Column::Estado.eq("activo"))
        .order_by_desc(contrato::Column::FechaInicio)
        .one(db)
        .await?;

    let contract = active_contract.ok_or_else(|| {
        AppError::Validation("No se encontró un contrato activo para este inquilino".to_string())
    })?;

    let now = Utc::now().into();

    let active = solicitud_mantenimiento::ActiveModel {
        id: Set(Uuid::new_v4()),
        propiedad_id: Set(contract.propiedad_id),
        unidad_id: Set(None),
        inquilino_id: Set(Some(inquilino_id)),
        titulo: Set(description.chars().take(100).collect::<String>()),
        descripcion: Set(Some(description.to_string())),
        estado: Set("pendiente".to_string()),
        prioridad: Set(prioridad.to_string()),
        nombre_proveedor: Set(None),
        telefono_proveedor: Set(None),
        email_proveedor: Set(None),
        costo_monto: Set(None),
        costo_moneda: Set(None),
        fecha_inicio: Set(None),
        fecha_fin: Set(None),
        organizacion_id: Set(org_id),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = active.insert(db).await?;
    Ok(record)
}

// --- Human Handoff State Management ---

/// Handoff status values for a conversation.
const HANDOFF_STATUS_AWAITING: &str = "awaiting_human";
const HANDOFF_STATUS_NONE: &str = "none";

/// Metadata key used to store handoff status in conversation messages.
const HANDOFF_METADATA_KEY: &str = "handoff_status";

/// Checks whether a human handoff is currently active for a given (`org_id`, `sender_phone`) pair.
///
/// Looks at the most recent conversation message with handoff metadata and checks
/// if its status is `awaiting_human`.
pub async fn is_handoff_active<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    sender_phone: &str,
) -> Result<bool, AppError> {
    // Find the most recent message that has handoff metadata for this conversation
    let recent = chatbot_conversation::Entity::find()
        .filter(chatbot_conversation::Column::OrganizacionId.eq(org_id))
        .filter(chatbot_conversation::Column::SenderPhone.eq(sender_phone))
        .filter(chatbot_conversation::Column::MessageType.eq("handoff_status"))
        .order_by_desc(chatbot_conversation::Column::CreatedAt)
        .one(db)
        .await?;

    let Some(msg) = recent else {
        return Ok(false);
    };

    // Check the metadata for handoff_status
    let is_active = msg
        .metadata
        .as_ref()
        .and_then(|m| m.get(HANDOFF_METADATA_KEY))
        .and_then(|v| v.as_str())
        .is_some_and(|s| s == HANDOFF_STATUS_AWAITING);

    Ok(is_active)
}

/// Sets the handoff status to `awaiting_human` for a (`org_id`, `sender_phone`) pair.
///
/// Persists a special marker message in the conversation history with
/// `message_type = "handoff_status"` and metadata `{"handoff_status": "awaiting_human"}`.
pub async fn set_handoff<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    sender_phone: &str,
) -> Result<chatbot_conversation::Model, AppError> {
    let metadata = serde_json::json!({
        HANDOFF_METADATA_KEY: HANDOFF_STATUS_AWAITING,
    });

    persist_message(
        db,
        org_id,
        sender_phone,
        None,
        "system",
        "Conversación transferida a operador humano",
        "handoff_status",
        Some(metadata),
    )
    .await
}

/// Clears the handoff status for a (`org_id`, `sender_phone`) pair, resuming AI processing.
///
/// Persists a marker message with `message_type = "handoff_status"` and
/// metadata `{"handoff_status": "none"}`.
///
/// Only `admin` or `gerente` roles may call this.
pub async fn clear_handoff<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    sender_phone: &str,
    user_role: &str,
) -> Result<chatbot_conversation::Model, AppError> {
    enforce_config_role(user_role)?;

    let metadata = serde_json::json!({
        HANDOFF_METADATA_KEY: HANDOFF_STATUS_NONE,
    });

    persist_message(
        db,
        org_id,
        sender_phone,
        None,
        "system",
        "Handoff finalizado, AI reanudado",
        "handoff_status",
        Some(metadata),
    )
    .await
}

// --- Pure Maintenance Request Defaults (for PBT) ---

/// Result of validating and resolving maintenance request defaults.
/// Models the pure logic of `create_maintenance_from_chat` without DB access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceRequestDefaults {
    /// The property ID from the contract.
    pub propiedad_id: Uuid,
    /// Always `"pendiente"`.
    pub status: String,
    /// Defaults to `"media"` unless a valid priority is explicitly provided.
    pub priority: String,
}

/// Pure validation and default resolution for maintenance requests.
///
/// Given a description and optional priority, validates inputs and returns
/// the resolved defaults (status, priority) along with the linked property.
///
/// - `description`: must be 2–1000 characters
/// - `priority`: if `None`, defaults to `"media"`; if `Some`, must be one of `VALID_PRIORITIES`
/// - `propiedad_id`: the property from the tenant's active contract
///
/// Returns `Err` if validation fails, `Ok(MaintenanceRequestDefaults)` otherwise.
pub fn resolve_maintenance_defaults(
    description: &str,
    priority: Option<&str>,
    propiedad_id: Uuid,
) -> Result<MaintenanceRequestDefaults, AppError> {
    // Validate description length (2–1000 chars)
    let desc_len = description.chars().count();
    if !(2..=1000).contains(&desc_len) {
        return Err(AppError::Validation(
            "La descripción debe tener entre 2 y 1000 caracteres".to_string(),
        ));
    }

    // Validate and resolve priority
    let prioridad = priority.unwrap_or("media");
    validate_enum("prioridad", prioridad, VALID_PRIORITIES)?;

    Ok(MaintenanceRequestDefaults {
        propiedad_id,
        status: "pendiente".to_string(),
        priority: prioridad.to_string(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ── validate_e164 tests ──────────────────────────────────────────

    #[test]
    fn validate_e164_accepts_valid_dr_number() {
        assert!(validate_e164("+18091234567").is_ok());
    }

    #[test]
    fn validate_e164_accepts_minimum_length() {
        assert!(validate_e164("+1").is_ok());
    }

    #[test]
    fn validate_e164_accepts_maximum_length() {
        assert!(validate_e164("+123456789012345").is_ok());
    }

    #[test]
    fn validate_e164_rejects_missing_plus() {
        assert!(validate_e164("18091234567").is_err());
    }

    #[test]
    fn validate_e164_rejects_leading_zero_after_plus() {
        assert!(validate_e164("+0123456789").is_err());
    }

    #[test]
    fn validate_e164_rejects_too_many_digits() {
        assert!(validate_e164("+1234567890123456").is_err());
    }

    #[test]
    fn validate_e164_rejects_empty_after_plus() {
        assert!(validate_e164("+").is_err());
    }

    #[test]
    fn validate_e164_rejects_non_digit_chars() {
        assert!(validate_e164("+1809-123-4567").is_err());
    }

    #[test]
    fn validate_e164_rejects_spaces() {
        assert!(validate_e164("+1 809 123 4567").is_err());
    }

    #[test]
    fn validate_e164_rejects_empty_string() {
        assert!(validate_e164("").is_err());
    }

    // ── normalize_phone tests ────────────────────────────────────────

    #[test]
    fn normalize_strips_dashes_and_prepends_country_code() {
        let result = normalize_phone("809-123-4567", "+1").unwrap();
        assert_eq!(result, "+18091234567");
    }

    #[test]
    fn normalize_strips_spaces_and_parens() {
        let result = normalize_phone("(809) 123 4567", "+1").unwrap();
        assert_eq!(result, "+18091234567");
    }

    #[test]
    fn normalize_removes_trunk_zero() {
        let result = normalize_phone("0809-123-4567", "+1").unwrap();
        assert_eq!(result, "+18091234567");
    }

    #[test]
    fn normalize_preserves_plus_prefix() {
        let result = normalize_phone("+1-809-123-4567", "+1").unwrap();
        assert_eq!(result, "+18091234567");
    }

    #[test]
    fn normalize_with_plus_removes_trunk_zeros() {
        let result = normalize_phone("+001809123456", "+1").unwrap();
        assert_eq!(result, "+1809123456");
    }

    #[test]
    fn normalize_rejects_fewer_than_7_digits() {
        let result = normalize_phone("12345", "+1");
        assert!(result.is_err());
    }

    #[test]
    fn normalize_rejects_invalid_characters() {
        let result = normalize_phone("abc1234567", "+1");
        assert!(result.is_err());
    }

    #[test]
    fn normalize_rejects_empty_input() {
        let result = normalize_phone("", "+1");
        assert!(result.is_err());
    }

    #[test]
    fn normalize_rejects_all_zeros() {
        let result = normalize_phone("0000000", "+1");
        assert!(result.is_err());
    }

    #[test]
    fn normalize_already_valid_e164_unchanged() {
        let result = normalize_phone("+18091234567", "+1").unwrap();
        assert_eq!(result, "+18091234567");
    }

    #[test]
    fn normalize_trims_whitespace() {
        let result = normalize_phone("  +18091234567  ", "+1").unwrap();
        assert_eq!(result, "+18091234567");
    }

    // ── Configuration validation tests ───────────────────────────────

    #[test]
    fn validate_config_accepts_valid_full_update() {
        let input = ChatbotConfigUpdateRequest {
            activo: Some(true),
            display_name: Some("Mi Chatbot".to_string()),
            language: Some("es-DO".to_string()),
            tone: Some("profesional".to_string()),
            greeting: Some("Hola!".to_string()),
            system_prompt: Some("Eres un asistente.".to_string()),
            faqs: Some(vec![FaqEntry {
                question: "¿Cómo pago?".to_string(),
                answer: "Puede pagar por transferencia.".to_string(),
            }]),
            policies: Some("Política de pagos...".to_string()),
            sender_policy: Some("tenants_only".to_string()),
            allowlist: None,
            capabilities: Some(Capabilities {
                receipt_ocr: true,
                balance_queries: true,
                payment_reminders: false,
                maintenance_requests: true,
                human_handoff: false,
            }),
            handoff_keywords: Some(vec!["hablar con humano".to_string()]),
            history_limit: Some(20),
            retention_days: Some(180),
        };
        assert!(validate_config(&input).is_ok());
    }

    #[test]
    fn validate_config_accepts_empty_partial_update() {
        let input = ChatbotConfigUpdateRequest {
            activo: None,
            display_name: None,
            language: None,
            tone: None,
            greeting: None,
            system_prompt: None,
            faqs: None,
            policies: None,
            sender_policy: None,
            allowlist: None,
            capabilities: None,
            handoff_keywords: None,
            history_limit: None,
            retention_days: None,
        };
        assert!(validate_config(&input).is_ok());
    }

    #[test]
    fn validate_config_rejects_empty_display_name() {
        let input = ChatbotConfigUpdateRequest {
            display_name: Some(String::new()),
            ..default_update()
        };
        let err = validate_config(&input).unwrap_err();
        assert!(err.to_string().contains("display_name"));
    }

    #[test]
    fn validate_config_rejects_display_name_over_100() {
        let input = ChatbotConfigUpdateRequest {
            display_name: Some("a".repeat(101)),
            ..default_update()
        };
        let err = validate_config(&input).unwrap_err();
        assert!(err.to_string().contains("display_name"));
    }

    #[test]
    fn validate_config_rejects_empty_tone() {
        let input = ChatbotConfigUpdateRequest {
            tone: Some(String::new()),
            ..default_update()
        };
        let err = validate_config(&input).unwrap_err();
        assert!(err.to_string().contains("tone"));
    }

    #[test]
    fn validate_config_rejects_tone_over_50() {
        let input = ChatbotConfigUpdateRequest {
            tone: Some("a".repeat(51)),
            ..default_update()
        };
        let err = validate_config(&input).unwrap_err();
        assert!(err.to_string().contains("tone"));
    }

    #[test]
    fn validate_config_rejects_more_than_50_faqs() {
        let faqs: Vec<FaqEntry> = (0..51)
            .map(|i| FaqEntry {
                question: format!("Pregunta {i}"),
                answer: format!("Respuesta {i}"),
            })
            .collect();
        let input = ChatbotConfigUpdateRequest {
            faqs: Some(faqs),
            ..default_update()
        };
        let err = validate_config(&input).unwrap_err();
        assert!(err.to_string().contains("50"));
    }

    #[test]
    fn validate_config_rejects_faq_question_over_200() {
        let input = ChatbotConfigUpdateRequest {
            faqs: Some(vec![FaqEntry {
                question: "a".repeat(201),
                answer: "Respuesta".to_string(),
            }]),
            ..default_update()
        };
        let err = validate_config(&input).unwrap_err();
        assert!(err.to_string().contains("pregunta"));
    }

    #[test]
    fn validate_config_rejects_faq_answer_over_1000() {
        let input = ChatbotConfigUpdateRequest {
            faqs: Some(vec![FaqEntry {
                question: "Pregunta".to_string(),
                answer: "a".repeat(1001),
            }]),
            ..default_update()
        };
        let err = validate_config(&input).unwrap_err();
        assert!(err.to_string().contains("respuesta"));
    }

    #[test]
    fn validate_config_rejects_empty_faq_question() {
        let input = ChatbotConfigUpdateRequest {
            faqs: Some(vec![FaqEntry {
                question: String::new(),
                answer: "Respuesta".to_string(),
            }]),
            ..default_update()
        };
        assert!(validate_config(&input).is_err());
    }

    #[test]
    fn validate_config_rejects_empty_faq_answer() {
        let input = ChatbotConfigUpdateRequest {
            faqs: Some(vec![FaqEntry {
                question: "Pregunta".to_string(),
                answer: String::new(),
            }]),
            ..default_update()
        };
        assert!(validate_config(&input).is_err());
    }

    #[test]
    fn validate_config_rejects_policies_over_5000() {
        let input = ChatbotConfigUpdateRequest {
            policies: Some("a".repeat(5001)),
            ..default_update()
        };
        let err = validate_config(&input).unwrap_err();
        assert!(err.to_string().contains("5000"));
    }

    #[test]
    fn validate_config_rejects_invalid_sender_policy() {
        let input = ChatbotConfigUpdateRequest {
            sender_policy: Some("everyone".to_string()),
            ..default_update()
        };
        let err = validate_config(&input).unwrap_err();
        assert!(err.to_string().contains("sender_policy"));
    }

    #[test]
    fn validate_config_accepts_all_valid_sender_policies() {
        for policy in SENDER_POLICIES {
            let input = ChatbotConfigUpdateRequest {
                sender_policy: Some(policy.to_string()),
                ..default_update()
            };
            assert!(
                validate_config(&input).is_ok(),
                "Failed for policy: {policy}"
            );
        }
    }

    #[test]
    fn validate_config_rejects_history_limit_zero() {
        let input = ChatbotConfigUpdateRequest {
            history_limit: Some(0),
            ..default_update()
        };
        assert!(validate_config(&input).is_err());
    }

    #[test]
    fn validate_config_rejects_history_limit_over_50() {
        let input = ChatbotConfigUpdateRequest {
            history_limit: Some(51),
            ..default_update()
        };
        assert!(validate_config(&input).is_err());
    }

    #[test]
    fn validate_config_rejects_retention_days_zero() {
        let input = ChatbotConfigUpdateRequest {
            retention_days: Some(0),
            ..default_update()
        };
        assert!(validate_config(&input).is_err());
    }

    #[test]
    fn validate_config_rejects_retention_days_over_365() {
        let input = ChatbotConfigUpdateRequest {
            retention_days: Some(366),
            ..default_update()
        };
        assert!(validate_config(&input).is_err());
    }

    #[test]
    fn validate_config_accepts_boundary_values() {
        let input = ChatbotConfigUpdateRequest {
            display_name: Some("a".to_string()), // min 1
            tone: Some("b".to_string()),         // min 1
            history_limit: Some(1),              // min
            retention_days: Some(1),             // min
            ..default_update()
        };
        assert!(validate_config(&input).is_ok());

        let input = ChatbotConfigUpdateRequest {
            display_name: Some("a".repeat(100)), // max 100
            tone: Some("b".repeat(50)),          // max 50
            history_limit: Some(50),             // max
            retention_days: Some(365),           // max
            policies: Some("c".repeat(5000)),    // max
            ..default_update()
        };
        assert!(validate_config(&input).is_ok());
    }

    #[test]
    fn enforce_config_role_allows_admin() {
        assert!(enforce_config_role("admin").is_ok());
    }

    #[test]
    fn enforce_config_role_allows_gerente() {
        assert!(enforce_config_role("gerente").is_ok());
    }

    #[test]
    fn enforce_config_role_rejects_visualizador() {
        let err = enforce_config_role("visualizador").unwrap_err();
        assert!(err.to_string().contains("administradores"));
    }

    #[test]
    fn enforce_config_role_rejects_unknown_role() {
        assert!(enforce_config_role("superuser").is_err());
    }

    #[test]
    fn default_config_response_has_expected_defaults() {
        let org_id = Uuid::new_v4();
        let resp = default_config_response(org_id);
        assert_eq!(resp.organizacion_id, org_id);
        assert!(!resp.activo);
        assert_eq!(resp.connection_status, "disconnected");
        assert_eq!(resp.language, "es-DO");
        assert_eq!(resp.sender_policy, "tenants_only");
        assert_eq!(resp.history_limit, 10);
        assert_eq!(resp.retention_days, 90);
        assert!(!resp.capabilities.receipt_ocr);
        assert!(!resp.capabilities.balance_queries);
    }

    // Helper to create a default empty update request for tests
    fn default_update() -> ChatbotConfigUpdateRequest {
        ChatbotConfigUpdateRequest {
            activo: None,
            display_name: None,
            language: None,
            tone: None,
            greeting: None,
            system_prompt: None,
            faqs: None,
            policies: None,
            sender_policy: None,
            allowlist: None,
            capabilities: None,
            handoff_keywords: None,
            history_limit: None,
            retention_days: None,
        }
    }
}
