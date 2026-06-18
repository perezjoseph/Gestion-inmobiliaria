use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::entities::{
    chatbot_config, chatbot_conversation, chatbot_receipt_extraction, contrato, pago,
    preview_index, solicitud_mantenimiento,
};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::chatbot::{
    BalanceResponse, BatchUpdateRequest, Capabilities, ChatbotConfigResponse,
    ChatbotConfigUpdateRequest, Confidence, ConversationListResponse, CreateGuidanceRuleRequest,
    CurrencyTotal, FaqEntry, GuidanceCategory, GuidanceRule, PaymentDetail,
    UpdateGuidanceRuleRequest, format_currency,
};
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::validation::validate_enum;

const SENDER_POLICIES: &[&str] = &["tenants_only", "tenants_and_prospects", "allowlist"];

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
        agent_config: serde_json::json!({}),
        guidance_rules: serde_json::json!([]),
        updated_by: None,
        created_at: Utc::now().into(),
        updated_at: Utc::now().into(),
    }))
}

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
            system_prompt: Set(None),
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
            agent_config: Set(serde_json::json!({})),
            guidance_rules: Set(serde_json::to_value(seed_template_rules())
                .unwrap_or_else(|_| serde_json::json!([]))),
            updated_by: Set(Some(user_id)),
            created_at: Set(now),
            updated_at: Set(now),
        };
        active.insert(db).await?
    };

    config_model_to_response(record)
}

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

pub fn seed_template_rules() -> Vec<GuidanceRule> {
    let now = Utc::now();
    let mut rules = Vec::with_capacity(16);
    let mut sort = 0i32;

    let templates: &[(GuidanceCategory, &[&str])] = &[
        (
            GuidanceCategory::EstiloComunicacion,
            &[
                "Tratar a todos los inquilinos de 'usted', nunca de 'tú'",
                "Incluir siempre el símbolo de moneda (RD$ o US$) al mencionar montos",
                "Mantener mensajes cortos: máximo 3 oraciones por respuesta",
                "Responder siempre en español, sin importar el idioma del mensaje recibido",
            ],
        ),
        (
            GuidanceCategory::ContextoClarificacion,
            &[
                "Antes de compartir cualquier dato financiero, confirmar la identidad del inquilino pidiendo nombre y número de unidad",
                "Si el inquilino pregunta por un balance sin especificar unidad, preguntar cuál unidad antes de responder",
                "Si hay ambigüedad sobre cuál contrato se refiere, listar los contratos activos y pedir que elija",
            ],
        ),
        (
            GuidanceCategory::Escalamiento,
            &[
                "Si el inquilino menciona 'abogado', 'tribunal', 'demanda' o 'acción legal', transferir inmediatamente a un humano sin hacer más preguntas",
                "Si el inquilino reporta una emergencia (inundación, fuga de gas, incendio, fallo eléctrico), transferir a humano inmediatamente",
                "Si el inquilino pide hablar con una persona real o dice 'humano', 'agente' o 'hablar con alguien', respetar su solicitud y transferir",
                "Si el inquilino repite la misma pregunta 3 veces sin obtener la respuesta deseada, ofrecer transferencia a un humano",
            ],
        ),
        (
            GuidanceCategory::Politicas,
            &[
                "Nunca compartir datos bancarios del propietario o la administración",
                "Nunca revelar información personal de otros inquilinos (nombres, balances, unidades)",
                "No confirmar la recepción de un pago sin verificar primero en el sistema",
                "No dar consejos legales ni financieros — derivar al profesional correspondiente",
                "No compartir términos de contrato con personas que no sean parte del contrato",
            ],
        ),
    ];

    for (category, instructions) in templates {
        for instruction in *instructions {
            rules.push(GuidanceRule {
                id: Uuid::new_v4(),
                category: category.clone(),
                instruction: (*instruction).to_string(),
                enabled: true,
                is_template: true,
                sort_order: sort,
                created_at: now,
                updated_at: now,
            });
            sort += 1;
        }
    }

    rules
}

pub(crate) fn enforce_config_role(role: &str) -> Result<(), AppError> {
    match role {
        "admin" | "gerente" => Ok(()),
        _ => Err(AppError::Forbidden(
            "Solo administradores y gerentes pueden modificar la configuración del chatbot"
                .to_string(),
        )),
    }
}

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
        faqs,
        policies: model.policies,
        sender_policy: model.sender_policy,
        allowlist,
        capabilities,
        handoff_keywords,
        history_limit: model.history_limit,
        retention_days: model.retention_days,
        guidance_rules: serde_json::from_value(model.guidance_rules).unwrap_or_default(),
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
        guidance_rules: vec![],
        created_at: now,
        updated_at: now,
    }
}

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
        _ => false,
    }
}

pub fn is_phone_in_allowlist(phone: &str, allowlist: Option<&[String]>) -> bool {
    allowlist.is_some_and(|list| list.iter().any(|entry| entry == phone))
}

pub fn check_sender_policy_no_db(
    sender_policy: &str,
    phone: &str,
    allowlist: Option<&[String]>,
) -> Option<bool> {
    match sender_policy {
        "tenants_only" => None,
        "tenants_and_prospects" => Some(true),
        "allowlist" => Some(is_phone_in_allowlist(phone, allowlist)),
        _ => Some(false),
    }
}

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

pub struct ResolvedTenant {
    pub id: Uuid,
    pub nombre: String,
    pub apellido: String,
}

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

pub fn validate_e164(phone: &str) -> Result<(), AppError> {
    let bytes = phone.as_bytes();

    if bytes.first() != Some(&b'+') {
        return Err(AppError::Validation(
            "Número de teléfono debe comenzar con '+'".to_string(),
        ));
    }

    let digits = &phone[1..];

    if digits.is_empty() || digits.len() > 15 {
        return Err(AppError::Validation(
            "Número E.164 debe tener entre 1 y 15 dígitos después del '+'".to_string(),
        ));
    }

    if digits.starts_with('0') {
        return Err(AppError::Validation(
            "Primer dígito después del '+' no puede ser cero".to_string(),
        ));
    }

    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AppError::Validation(
            "Número E.164 solo puede contener dígitos después del '+'".to_string(),
        ));
    }

    Ok(())
}

pub fn normalize_phone(input: &str, default_country_code: &str) -> Result<String, AppError> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "Número de teléfono no puede estar vacío".to_string(),
        ));
    }

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

    let stripped = raw_digits.trim_start_matches('0');

    if stripped.is_empty() {
        return Err(AppError::Validation(
            "Número de teléfono no contiene dígitos válidos".to_string(),
        ));
    }

    let normalized = if has_plus {
        format!("+{stripped}")
    } else {
        let code = default_country_code.trim_start_matches('+');
        format!("+{code}{stripped}")
    };

    let digit_count = normalized.len() - 1;
    if digit_count < 7 {
        return Err(AppError::Validation(format!(
            "Número de teléfono debe tener al menos 7 dígitos, tiene {digit_count}"
        )));
    }

    validate_e164(&normalized)?;

    Ok(normalized)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampedMessage {
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub fn window_history(messages: &[TimestampedMessage], limit: usize) -> Vec<&TimestampedMessage> {
    let mut sorted: Vec<&TimestampedMessage> = messages.iter().collect();
    sorted.sort_by_key(|m| std::cmp::Reverse(m.created_at));
    sorted.truncate(limit);
    sorted
}

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

pub async fn list_conversations<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    page: u64,
    per_page: u64,
) -> Result<PaginatedResponse<ConversationListResponse>, AppError> {
    use sea_orm::{FromQueryResult, PaginatorTrait, sea_query::Expr};

    #[derive(Debug, FromQueryResult)]
    struct PhoneCount {
        sender_phone: String,
        message_count: i64,
    }

    #[derive(Debug, FromQueryResult)]
    struct LatestMessage {
        sender_phone: String,
        inquilino_id: Option<Uuid>,
        content: String,
        created_at: chrono::DateTime<Utc>,
    }

    let total = chatbot_conversation::Entity::find()
        .filter(chatbot_conversation::Column::OrganizacionId.eq(org_id))
        .select_only()
        .column_as(chatbot_conversation::Column::SenderPhone.count(), "count")
        .group_by(chatbot_conversation::Column::SenderPhone)
        .count(db)
        .await?;

    let offset = (page - 1) * per_page;

    let phone_counts: Vec<PhoneCount> = chatbot_conversation::Entity::find()
        .filter(chatbot_conversation::Column::OrganizacionId.eq(org_id))
        .select_only()
        .column(chatbot_conversation::Column::SenderPhone)
        .column_as(
            Expr::col(chatbot_conversation::Column::Id).count(),
            "message_count",
        )
        .group_by(chatbot_conversation::Column::SenderPhone)
        .order_by_asc(chatbot_conversation::Column::SenderPhone)
        .limit(per_page)
        .offset(offset)
        .into_model::<PhoneCount>()
        .all(db)
        .await?;

    if phone_counts.is_empty() {
        return Ok(PaginatedResponse {
            data: vec![],
            total,
            page,
            per_page,
        });
    }

    let phones: Vec<String> = phone_counts
        .iter()
        .map(|p| p.sender_phone.clone())
        .collect();
    let count_map: HashMap<&str, i64> = phone_counts
        .iter()
        .map(|p| (p.sender_phone.as_str(), p.message_count))
        .collect();

    let latest_msgs: Vec<LatestMessage> = {
        use sea_orm::{DbBackend, Statement};

        let placeholders: Vec<String> = (0..phones.len())
            .map(|i| ["$", &(i + 2).to_string()].concat())
            .collect();
        let in_clause = placeholders.join(", ");

        let sql = [
            "SELECT DISTINCT ON (sender_phone) sender_phone, inquilino_id, content, created_at \
             FROM chatbot_conversation \
             WHERE organizacion_id = $1 AND sender_phone IN (",
            &in_clause,
            ") ORDER BY sender_phone ASC, created_at DESC",
        ]
        .concat();

        let mut params: Vec<sea_orm::Value> = Vec::with_capacity(phones.len() + 1);
        params.push(org_id.into());
        for phone in &phones {
            params.push(phone.clone().into());
        }

        LatestMessage::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            &sql,
            params,
        ))
        .all(db)
        .await?
    };

    let data: Vec<ConversationListResponse> = latest_msgs
        .into_iter()
        .map(|msg| ConversationListResponse {
            sender_phone: msg.sender_phone.clone(),
            inquilino_id: msg.inquilino_id,
            last_message: msg.content,
            last_message_at: msg.created_at,
            message_count: count_map
                .get(msg.sender_phone.as_str())
                .copied()
                .unwrap_or(0),
        })
        .collect();

    Ok(PaginatedResponse {
        data,
        total,
        page,
        per_page,
    })
}

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

pub async fn query_tenant_balance_with_sender_check<C: ConnectionTrait>(
    db: &C,
    inquilino_id: Uuid,
    org_id: Uuid,
    sender_phone: &str,
    sender_policy: &str,
) -> Result<BalanceResponse, AppError> {
    if sender_policy == "tenants_and_prospects" {
        let tenant = find_tenant_by_phone(db, sender_phone, org_id).await?;
        if tenant.is_none() {
            return Err(AppError::Validation(
                "Lo siento, no puedo proporcionarte información de saldo. \
                 Tu número no está vinculado a un inquilino en el sistema."
                    .to_string(),
            ));
        }
    }

    query_tenant_balance(db, inquilino_id, org_id).await
}

pub async fn query_tenant_balance<C: ConnectionTrait>(
    db: &C,
    inquilino_id: Uuid,
    org_id: Uuid,
) -> Result<BalanceResponse, AppError> {
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

    let mut totals: Vec<CurrencyTotal> = totals_map
        .into_iter()
        .map(|(currency, total)| CurrencyTotal {
            formatted_total: format_currency(total, &currency),
            currency,
            total,
        })
        .collect();

    totals.sort_by(|a, b| a.currency.cmp(&b.currency));

    Ok(BalanceResponse { payments, totals })
}

pub async fn record_extraction<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    conversation_id: Uuid,
    inquilino_id: Option<Uuid>,
    contrato_id: Option<Uuid>,
    extracted_data: serde_json::Value,
    _confidence: &Confidence,
) -> Result<chatbot_receipt_extraction::Model, AppError> {
    use sea_orm::ActiveValue::NotSet;

    let now = Utc::now().into();

    let active = chatbot_receipt_extraction::ActiveModel {
        id: Set(Uuid::new_v4()),
        organizacion_id: Set(org_id),
        conversation_id: Set(conversation_id),
        inquilino_id: inquilino_id.map_or(NotSet, |id| Set(Some(id))),
        contrato_id: contrato_id.map_or(NotSet, |id| Set(Some(id))),
        extracted_data: Set(extracted_data),
        status: Set("pending_confirmation".to_string()),
        confirmed_by: NotSet,
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = active.insert(db).await?;
    Ok(record)
}

#[allow(clippy::doc_markdown)]
pub async fn record_extraction_from_agent<C: ConnectionTrait>(
    db: &C,
    receipt: &crate::services::ai_module::PaymentReceipt,
    organizacion_id: Uuid,
    _usuario_id: Option<Uuid>,
) -> Result<chatbot_receipt_extraction::Model, AppError> {
    let extracted_data = serde_json::to_value(receipt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error serializando recibo: {e}")))?;

    let conversation_id = Uuid::new_v4();
    let now = Utc::now().into();
    let conv = crate::entities::chatbot_conversation::ActiveModel {
        id: Set(conversation_id),
        organizacion_id: Set(organizacion_id),
        sender_phone: Set("agent".to_string()),
        inquilino_id: Set(None),
        role: Set("system".to_string()),
        content: Set("Receipt extraction from agent".to_string()),
        message_type: Set("extraction".to_string()),
        metadata: Set(None),
        created_at: Set(now),
    };
    conv.insert(db).await?;

    record_extraction(
        db,
        organizacion_id,
        conversation_id,
        None,
        None,
        extracted_data,
        &receipt.confidence,
    )
    .await
}

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

pub async fn confirm_receipt<C: ConnectionTrait>(
    db: &C,
    extraction_id: Uuid,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<chatbot_receipt_extraction::Model, AppError> {
    let extraction = chatbot_receipt_extraction::Entity::find_by_id(extraction_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "Extracción de recibo no encontrada: {extraction_id}"
            ))
        })?;

    if extraction.organizacion_id != org_id {
        return Err(AppError::NotFound(format!(
            "Extracción de recibo no encontrada: {extraction_id}"
        )));
    }

    if extraction.status != "pending_confirmation" {
        return Err(AppError::Conflict(
            "La extracción ya fue procesada y no está pendiente de confirmación".to_string(),
        ));
    }

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

    let contrato_id = extraction.contrato_id.ok_or_else(|| {
        AppError::Validation(
            "La extracción no tiene un contrato asociado para crear el pago".to_string(),
        )
    })?;

    let now = Utc::now().into();

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
        monto_base: Set(None),
        monto_itbis: Set(None),
        monto_itbis_retenido: Set(None),
        ncf: Set(None),
        fecha_comprobante: Set(None),
        tipo_ncf: Set(None),
        es_parcial: Set(false),
        saldo_pendiente: Set(None),
        tipo_linea: Set("renta".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    pago_active.insert(db).await?;

    let mut active: chatbot_receipt_extraction::ActiveModel = extraction.into();
    active.status = Set("confirmed".to_string());
    active.confirmed_by = Set(Some(user_id));
    active.updated_at = Set(now);
    let updated = active.update(db).await?;

    Ok(updated)
}

pub async fn reject_receipt<C: ConnectionTrait>(
    db: &C,
    extraction_id: Uuid,
    user_id: Uuid,
    reason: Option<&str>,
    org_id: Uuid,
) -> Result<chatbot_receipt_extraction::Model, AppError> {
    let extraction = chatbot_receipt_extraction::Entity::find_by_id(extraction_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "Extracción de recibo no encontrada: {extraction_id}"
            ))
        })?;

    if extraction.organizacion_id != org_id {
        return Err(AppError::NotFound(format!(
            "Extracción de recibo no encontrada: {extraction_id}"
        )));
    }

    if extraction.status != "pending_confirmation" {
        return Err(AppError::Conflict(
            "La extracción ya fue procesada y no está pendiente de confirmación".to_string(),
        ));
    }

    let now = Utc::now().into();

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

const VALID_PRIORITIES: &[&str] = &["baja", "media", "alta", "urgente"];

pub async fn create_maintenance_from_chat<C: ConnectionTrait>(
    db: &C,
    inquilino_id: Uuid,
    org_id: Uuid,
    description: &str,
    priority: Option<&str>,
) -> Result<solicitud_mantenimiento::Model, AppError> {
    let desc_len = description.chars().count();
    if !(2..=1000).contains(&desc_len) {
        return Err(AppError::Validation(
            "La descripción debe tener entre 2 y 1000 caracteres".to_string(),
        ));
    }

    let prioridad = priority.unwrap_or("media");
    validate_enum("prioridad", prioridad, VALID_PRIORITIES)?;

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

const HANDOFF_STATUS_AWAITING: &str = "awaiting_human";
const HANDOFF_STATUS_NONE: &str = "none";

const HANDOFF_METADATA_KEY: &str = "handoff_status";

pub async fn is_handoff_active<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    sender_phone: &str,
) -> Result<bool, AppError> {
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

    let is_active = msg
        .metadata
        .as_ref()
        .and_then(|m| m.get(HANDOFF_METADATA_KEY))
        .and_then(|v| v.as_str())
        .is_some_and(|s| s == HANDOFF_STATUS_AWAITING);

    Ok(is_active)
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceRequestDefaults {
    pub propiedad_id: Uuid,
    pub status: String,
    pub priority: String,
}

pub fn resolve_maintenance_defaults(
    description: &str,
    priority: Option<&str>,
    propiedad_id: Uuid,
) -> Result<MaintenanceRequestDefaults, AppError> {
    let desc_len = description.chars().count();
    if !(2..=1000).contains(&desc_len) {
        return Err(AppError::Validation(
            "La descripción debe tener entre 2 y 1000 caracteres".to_string(),
        ));
    }

    let prioridad = priority.unwrap_or("media");
    validate_enum("prioridad", prioridad, VALID_PRIORITIES)?;

    Ok(MaintenanceRequestDefaults {
        propiedad_id,
        status: "pendiente".to_string(),
        priority: prioridad.to_string(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentType {
    Recibo,
    Gasto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfirmedEntity {
    Pago(Uuid),
    Gasto(Uuid),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrPreviewConfirm {
    pub id: Uuid,
    pub document_type: DocumentType,
    pub monto: Decimal,
    pub moneda: Option<String>,
    pub fecha: Option<chrono::NaiveDate>,
    pub nombre_depositante: Option<String>,
    pub contrato_id: Option<Uuid>,
    pub propiedad_id: Option<Uuid>,
    pub categoria: Option<String>,
    pub descripcion: Option<String>,
    pub metodo_pago: Option<String>,
    pub notas: Option<String>,
}

pub async fn confirmar_preview(
    db: &DatabaseConnection,
    preview: OcrPreviewConfirm,
    organizacion_id: Uuid,
    usuario_id: Uuid,
) -> Result<ConfirmedEntity, AppError> {
    if let Some(existing) = preview_index::Entity::find()
        .filter(preview_index::Column::PreviewId.eq(preview.id))
        .filter(preview_index::Column::OrganizacionId.eq(organizacion_id))
        .one(db)
        .await?
    {
        return match existing.entity_type.as_str() {
            "gasto" => Ok(ConfirmedEntity::Gasto(existing.entity_id)),
            _ => Ok(ConfirmedEntity::Pago(existing.entity_id)),
        };
    }

    if preview.monto <= Decimal::ZERO {
        return Err(AppError::Validation("Datos de OCR inválidos".to_string()));
    }

    let txn = db.begin().await?;

    let result = match preview.document_type {
        DocumentType::Recibo => {
            let contrato_id = preview
                .contrato_id
                .ok_or_else(|| AppError::Validation("Datos de OCR inválidos".to_string()))?;

            let req = crate::models::pago::CreatePagoRequest {
                contrato_id,
                monto: preview.monto,
                moneda: preview.moneda.clone(),
                fecha_pago: preview.fecha,
                fecha_vencimiento: preview.fecha.unwrap_or_else(|| Utc::now().date_naive()),
                metodo_pago: preview.metodo_pago.clone(),
                notas: preview.notas.clone(),
            };

            let pago =
                crate::services::pagos::create(&txn, req, usuario_id, organizacion_id).await?;
            ConfirmedEntity::Pago(pago.id)
        }
        DocumentType::Gasto => {
            let propiedad_id = preview
                .propiedad_id
                .ok_or_else(|| AppError::Validation("Datos de OCR inválidos".to_string()))?;

            let req = crate::models::gasto::CreateGastoRequest {
                propiedad_id,
                unidad_id: None,
                categoria: preview
                    .categoria
                    .clone()
                    .unwrap_or_else(|| "otros".to_string()),
                descripcion: preview
                    .descripcion
                    .clone()
                    .unwrap_or_else(|| "Gasto registrado desde OCR".to_string()),
                monto: preview.monto,
                moneda: preview.moneda.clone().unwrap_or_else(|| "DOP".to_string()),
                fecha_gasto: preview.fecha.unwrap_or_else(|| Utc::now().date_naive()),
                proveedor: None,
                numero_factura: None,
                notas: preview.notas.clone(),
                nic_contrato: None,
                proveedor_servicio: None,
                consumo: None,
                unidad_consumo: None,
                periodo_desde: None,
                periodo_hasta: None,
                numero_cuenta: None,
                periodo_inicio: None,
                periodo_fin: None,
            };

            let gasto =
                crate::services::gastos::create(&txn, req, usuario_id, organizacion_id).await?;
            ConfirmedEntity::Gasto(gasto.id)
        }
    };

    let (entity_type_str, entity_id) = match &result {
        ConfirmedEntity::Pago(id) => ("pago", *id),
        ConfirmedEntity::Gasto(id) => ("gasto", *id),
    };

    let now = Utc::now().into();
    let index_record = preview_index::ActiveModel {
        id: Set(Uuid::new_v4()),
        preview_id: Set(preview.id),
        organizacion_id: Set(organizacion_id),
        entity_type: Set(entity_type_str.to_string()),
        entity_id: Set(entity_id),
        created_at: Set(now),
    };
    index_record.insert(&txn).await?;

    txn.commit().await?;

    Ok(result)
}

const MAX_ACTIVE_RULES: usize = 30;
const MAX_INSTRUCTION_CHARS: usize = 500;

async fn load_guidance_rules(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<(chatbot_config::Model, Vec<GuidanceRule>), AppError> {
    let config = chatbot_config::Entity::find()
        .filter(chatbot_config::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(
                "Configuración de chatbot no encontrada para la organización".to_string(),
            )
        })?;

    let rules: Vec<GuidanceRule> =
        serde_json::from_value(config.guidance_rules.clone()).unwrap_or_default();

    Ok((config, rules))
}

async fn save_guidance_rules(
    db: &DatabaseConnection,
    config: chatbot_config::Model,
    rules: &[GuidanceRule],
) -> Result<(), AppError> {
    let rules_json = serde_json::to_value(rules).map_err(|e| {
        AppError::Internal(anyhow::anyhow!("Error serializando guidance_rules: {e}"))
    })?;

    let mut active: chatbot_config::ActiveModel = config.into();
    active.guidance_rules = Set(rules_json);
    active.updated_at = Set(Utc::now().into());
    active.update(db).await?;
    Ok(())
}

fn validate_instruction(instruction: &str) -> Result<(), AppError> {
    let char_count = instruction.chars().count();
    if char_count == 0 {
        return Err(AppError::Validation(
            "La instrucción no puede estar vacía".to_string(),
        ));
    }
    if char_count > MAX_INSTRUCTION_CHARS {
        return Err(AppError::Validation(format!(
            "La instrucción no puede exceder {MAX_INSTRUCTION_CHARS} caracteres (tiene {char_count})"
        )));
    }
    Ok(())
}

fn count_active(rules: &[GuidanceRule]) -> usize {
    rules.iter().filter(|r| r.enabled).count()
}

pub async fn create_guidance_rule(
    db: &DatabaseConnection,
    org_id: Uuid,
    req: CreateGuidanceRuleRequest,
    usuario_id: Uuid,
) -> Result<GuidanceRule, AppError> {
    validate_instruction(&req.instruction)?;

    let (config, mut rules) = load_guidance_rules(db, org_id).await?;

    let enabled = req.enabled.unwrap_or(true);

    if enabled && count_active(&rules) >= MAX_ACTIVE_RULES {
        return Err(AppError::Validation(format!(
            "No se pueden tener más de {MAX_ACTIVE_RULES} reglas activas"
        )));
    }

    let now = Utc::now();
    #[allow(clippy::cast_possible_wrap)]
    let sort_order = rules.len() as i32;

    let rule = GuidanceRule {
        id: Uuid::new_v4(),
        category: req.category,
        instruction: req.instruction,
        enabled,
        is_template: false,
        sort_order,
        created_at: now,
        updated_at: now,
    };

    rules.push(rule.clone());
    save_guidance_rules(db, config, &rules).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "guidance_rule".to_string(),
            entity_id: rule.id,
            accion: "crear".to_string(),
            cambios: serde_json::json!({
                "category": rule.category,
                "instruction": rule.instruction,
                "enabled": rule.enabled,
            }),
        },
    )
    .await;

    Ok(rule)
}

pub async fn update_guidance_rule(
    db: &DatabaseConnection,
    org_id: Uuid,
    rule_id: Uuid,
    req: UpdateGuidanceRuleRequest,
    usuario_id: Uuid,
) -> Result<GuidanceRule, AppError> {
    let (config, mut rules) = load_guidance_rules(db, org_id).await?;

    let rule_idx = rules
        .iter()
        .position(|r| r.id == rule_id)
        .ok_or_else(|| AppError::NotFound(format!("Regla no encontrada: {rule_id}")))?;

    if rules[rule_idx].is_template && req.instruction.is_some() {
        return Err(AppError::Forbidden(
            "No se puede modificar la instrucción de una regla predefinida".to_string(),
        ));
    }

    if let Some(ref instruction) = req.instruction {
        validate_instruction(instruction)?;
    }

    if req.enabled == Some(true)
        && !rules[rule_idx].enabled
        && count_active(&rules) >= MAX_ACTIVE_RULES
    {
        return Err(AppError::Validation(format!(
            "No se pueden tener más de {MAX_ACTIVE_RULES} reglas activas"
        )));
    }

    let mut cambios = serde_json::Map::new();
    if let Some(ref instruction) = req.instruction {
        cambios.insert("instruction".to_string(), serde_json::json!(instruction));
    }
    if let Some(enabled) = req.enabled {
        cambios.insert("enabled".to_string(), serde_json::json!(enabled));
    }
    if let Some(sort_order) = req.sort_order {
        cambios.insert("sort_order".to_string(), serde_json::json!(sort_order));
    }

    let rule = &mut rules[rule_idx];
    if let Some(instruction) = req.instruction {
        rule.instruction = instruction;
    }
    if let Some(enabled) = req.enabled {
        rule.enabled = enabled;
    }
    if let Some(sort_order) = req.sort_order {
        rule.sort_order = sort_order;
    }
    rule.updated_at = Utc::now();

    let updated = rule.clone();
    save_guidance_rules(db, config, &rules).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "guidance_rule".to_string(),
            entity_id: rule_id,
            accion: "actualizar".to_string(),
            cambios: serde_json::Value::Object(cambios),
        },
    )
    .await;

    Ok(updated)
}

pub async fn delete_guidance_rule(
    db: &DatabaseConnection,
    org_id: Uuid,
    rule_id: Uuid,
    usuario_id: Uuid,
) -> Result<(), AppError> {
    let (config, mut rules) = load_guidance_rules(db, org_id).await?;

    let rule_idx = rules
        .iter()
        .position(|r| r.id == rule_id)
        .ok_or_else(|| AppError::NotFound(format!("Regla no encontrada: {rule_id}")))?;

    if rules[rule_idx].is_template {
        return Err(AppError::Forbidden(
            "No se puede eliminar una regla predefinida".to_string(),
        ));
    }

    let deleted_rule = rules.remove(rule_idx);
    save_guidance_rules(db, config, &rules).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "guidance_rule".to_string(),
            entity_id: rule_id,
            accion: "eliminar".to_string(),
            cambios: serde_json::json!({
                "category": deleted_rule.category,
                "instruction": deleted_rule.instruction,
            }),
        },
    )
    .await;

    Ok(())
}

pub async fn batch_update_guidance_rules(
    db: &DatabaseConnection,
    org_id: Uuid,
    req: BatchUpdateRequest,
) -> Result<Vec<GuidanceRule>, AppError> {
    let (config, mut rules) = load_guidance_rules(db, org_id).await?;

    let now = Utc::now();

    for item in &req.rules {
        let rule_idx = rules
            .iter()
            .position(|r| r.id == item.id)
            .ok_or_else(|| AppError::NotFound(format!("Regla no encontrada: {}", item.id)))?;

        let rule = &mut rules[rule_idx];
        if let Some(enabled) = item.enabled {
            rule.enabled = enabled;
        }
        if let Some(sort_order) = item.sort_order {
            rule.sort_order = sort_order;
        }
        rule.updated_at = now;
    }

    if count_active(&rules) > MAX_ACTIVE_RULES {
        return Err(AppError::Validation(format!(
            "No se pueden tener más de {MAX_ACTIVE_RULES} reglas activas"
        )));
    }

    save_guidance_rules(db, config, &rules).await?;

    let updated_ids: Vec<Uuid> = req.rules.iter().map(|item| item.id).collect();
    let updated_rules: Vec<GuidanceRule> = rules
        .into_iter()
        .filter(|r| updated_ids.contains(&r.id))
        .collect();

    Ok(updated_rules)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

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

    #[test]
    fn validate_config_accepts_valid_full_update() {
        let input = ChatbotConfigUpdateRequest {
            activo: Some(true),
            display_name: Some("Mi Chatbot".to_string()),
            language: Some("es-DO".to_string()),
            tone: Some("profesional".to_string()),
            greeting: Some("Hola!".to_string()),
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
            display_name: Some("a".to_string()),
            tone: Some("b".to_string()),
            history_limit: Some(1),
            retention_days: Some(1),
            ..default_update()
        };
        assert!(validate_config(&input).is_ok());

        let input = ChatbotConfigUpdateRequest {
            display_name: Some("a".repeat(100)),
            tone: Some("b".repeat(50)),
            history_limit: Some(50),
            retention_days: Some(365),
            policies: Some("c".repeat(5000)),
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

    fn default_update() -> ChatbotConfigUpdateRequest {
        ChatbotConfigUpdateRequest {
            activo: None,
            display_name: None,
            language: None,
            tone: None,
            greeting: None,
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
