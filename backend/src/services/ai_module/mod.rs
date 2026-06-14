#[cfg(feature = "evals")]
pub mod evals;
pub mod guardrail_hook;
#[cfg(test)]
mod guardrail_hook_pbt;
pub mod tools;

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use regex::Regex;
use rig::agent::AgentBuilder;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use rust_decimal::Decimal;
use schemars::JsonSchema;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::ChatbotEnvConfig;
use crate::entities::{contrato, pago};
use crate::errors::AppError;
use crate::models::chatbot::{
    AgentConfig, BalanceResponse, Capabilities, FaqEntry, GuardrailOverrides, GuidanceCategory,
    GuidanceRule, ToolRegistrationStrategy, format_currency,
};
use crate::services::ocr_client::OcrClient;
use crate::services::ovms_provider::OvmsCompletionModel;

pub use tools::{ExtractReceiptInput, ExtractReceiptTool, InlineBase64MediaStore, PaymentReceipt};

#[derive(Clone)]
pub struct GuardrailConfig {
    pub max_receipt_amount_dop: Decimal,
    pub max_receipt_amount_usd: Decimal,
    pub max_description_length: usize,
    pub blocked_output_patterns: Vec<Regex>,
}

impl Default for GuardrailConfig {
    fn default() -> Self {
        Self {
            max_receipt_amount_dop: Decimal::new(10_000_000, 2),
            max_receipt_amount_usd: Decimal::new(500_000, 2),
            max_description_length: 1000,
            blocked_output_patterns: vec![],
        }
    }
}

impl From<GuardrailOverrides> for GuardrailConfig {
    fn from(overrides: GuardrailOverrides) -> Self {
        let default = Self::default();

        let max_receipt_amount_dop = overrides
            .max_receipt_amount_dop
            .map_or(default.max_receipt_amount_dop, |v| {
                Decimal::try_from(v).unwrap_or(default.max_receipt_amount_dop)
            });

        let max_receipt_amount_usd = overrides
            .max_receipt_amount_usd
            .map_or(default.max_receipt_amount_usd, |v| {
                Decimal::try_from(v).unwrap_or(default.max_receipt_amount_usd)
            });

        let blocked_output_patterns = overrides
            .blocked_patterns
            .unwrap_or_default()
            .iter()
            .filter_map(|pattern| match Regex::new(pattern) {
                Ok(re) => Some(re),
                Err(e) => {
                    tracing::warn!(
                        pattern = %pattern,
                        error = %e,
                        "Invalid regex in blocked_patterns, skipping"
                    );
                    None
                }
            })
            .collect();

        Self {
            max_receipt_amount_dop,
            max_receipt_amount_usd,
            max_description_length: default.max_description_length,
            blocked_output_patterns,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserMessage {
    pub content: String,
    pub image_base64: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConversationEntry {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentResponse {
    pub reply: String,
    pub tools_invoked: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_receipt: Option<PaymentReceipt>,
}

#[derive(Debug, Clone)]
pub struct ChatbotPersona {
    pub tone: Option<String>,
    pub greeting: Option<String>,
    pub system_prompt: Option<String>,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
    pub name: String,
}

pub struct AiModule {
    pub(crate) model: OvmsCompletionModel,
    pub(crate) timeout_secs: u64,
}

pub struct ProcessMessageContext<'a> {
    pub config: &'a ChatbotPersona,
    pub tenant_context: Option<&'a TenantContext>,
    pub faqs: &'a [FaqEntry],
    pub policies: Option<&'a str>,
    pub handoff_keywords: &'a [String],
    pub capabilities: &'a Capabilities,
    pub history: &'a [ConversationEntry],
    pub user_message: &'a UserMessage,
    pub db: &'a DatabaseConnection,
    pub organizacion_id: Uuid,
    pub sender_phone: &'a str,
    pub guidance_rules: &'a [GuidanceRule],
    pub sender_policy: &'a str,
}

impl AiModule {
    pub fn new(config: &ChatbotEnvConfig) -> Result<Self, anyhow::Error> {
        let model = OvmsCompletionModel::new(
            &config.vllm_chat_model,
            &config.vllm_endpoint,
            config.vllm_api_key.clone(),
        );

        Ok(Self {
            model,
            timeout_secs: config.ai_chat_timeout_secs,
        })
    }

    pub async fn process_message(
        &self,
        ctx: &ProcessMessageContext<'_>,
    ) -> Result<AgentResponse, AppError> {
        use rig::agent::PromptRequest;
        use rig::message::{self, UserContent};
        use rig::one_or_many::OneOrMany;

        let system_prompt = compose_system_prompt(
            ctx.config,
            ctx.tenant_context,
            ctx.faqs,
            ctx.policies,
            ctx.handoff_keywords,
            ctx.guidance_rules,
        );

        let resolved = AgentConfig::default().resolve();

        let captured_receipt: Arc<Mutex<Option<PaymentReceipt>>> = Arc::new(Mutex::new(None));
        let tools_invoked: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        let guardrail_config: GuardrailConfig = resolved.guardrails.clone().into();

        let hook = guardrail_hook::RentalGuardrailHook {
            captured_receipt: captured_receipt.clone(),
            tools_invoked: tools_invoked.clone(),
            organizacion_id: ctx.organizacion_id,
            guardrail_config,
        };

        let capabilities_for_tools = match resolved.tool_registration {
            ToolRegistrationStrategy::Selective => ctx.capabilities,
            ToolRegistrationStrategy::AllWithHookGating => &Capabilities::all(),
        };
        let tools = build_tools(
            capabilities_for_tools,
            ctx.db,
            ctx.organizacion_id,
            ctx.sender_phone,
            ctx.sender_policy,
            ctx.user_message.image_base64.as_deref(),
        );

        let mut builder = AgentBuilder::new(self.model.clone())
            .preamble(&system_prompt)
            .hook(hook)
            .default_max_turns(resolved.max_turns);

        if let Some(temp) = resolved.temperature {
            builder = builder.temperature(temp);
        }
        if let Some(max_tok) = resolved.max_tokens {
            builder = builder.max_tokens(max_tok);
        }

        let agent = builder.tools(tools).build();

        let user_prompt = ctx.user_message.image_base64.as_ref().map_or_else(
            || ctx.user_message.content.clone(),
            |_| {
                if ctx.user_message.content.is_empty() {
                    "[Imagen adjunta]".to_string()
                } else {
                    format!("{}\n[Imagen adjunta]", ctx.user_message.content)
                }
            },
        );

        let chat_history: Vec<message::Message> = ctx
            .history
            .iter()
            .filter_map(|entry| match entry.role.as_str() {
                "user" => Some(message::Message::User {
                    content: OneOrMany::one(UserContent::text(&entry.content)),
                }),
                "assistant" => Some(message::Message::Assistant {
                    id: None,
                    content: OneOrMany::one(message::AssistantContent::text(&entry.content)),
                }),
                _ => None,
            })
            .collect();

        let timeout_duration = Duration::from_secs(self.timeout_secs);

        crate::metrics::AI_REQUEST_ATTEMPTS.inc();

        let result = tokio::time::timeout(timeout_duration, async {
            PromptRequest::from_agent(&agent, user_prompt)
                .with_history(chat_history)
                .max_turns(resolved.max_turns)
                .await
        })
        .await;

        match result {
            Ok(Ok(reply)) => {
                if reply.trim().is_empty() {
                    tracing::error!(
                        organizacion_id = %ctx.organizacion_id,
                        "El modelo devolvió una respuesta vacía"
                    );
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "Respuesta vacía inesperada del modelo"
                    )));
                }

                let tools_list = tools_invoked
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();
                let receipt = captured_receipt
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();

                Ok(AgentResponse {
                    reply,
                    tools_invoked: tools_list,
                    extracted_receipt: receipt,
                })
            }
            Ok(Err(e)) => {
                let error_msg = e.to_string();

                if error_msg.contains("INFERENCE_COLD_START") {
                    tracing::info!(
                        organizacion_id = %ctx.organizacion_id,
                        "vLLM en cold-start, devolviendo mensaje de espera"
                    );
                    return Ok(AgentResponse {
                        reply: "El asistente se está iniciando. Por favor, intente de nuevo en 1-2 minutos."
                            .to_string(),
                        tools_invoked: vec![],
                        extracted_receipt: None,
                    });
                }

                if error_msg.contains("Response blocked by safety filter") {
                    tracing::warn!(
                        organizacion_id = %ctx.organizacion_id,
                        "Respuesta bloqueada por filtro de seguridad"
                    );
                    return Ok(AgentResponse {
                        reply: "Lo siento, no puedo proporcionar esa información. \
                            ¿Puedo ayudarte con algo más?"
                            .to_string(),
                        tools_invoked: tools_invoked
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .clone(),
                        extracted_receipt: None,
                    });
                }

                tracing::error!(
                    organizacion_id = %ctx.organizacion_id,
                    error = %e,
                    "Error en el agente AI"
                );
                Err(AppError::Internal(anyhow::anyhow!(
                    "Error en agente AI: {e}"
                )))
            }
            Err(_elapsed) => {
                tracing::warn!(
                    organizacion_id = %ctx.organizacion_id,
                    timeout_secs = self.timeout_secs,
                    "Timeout en la solicitud al modelo OVMS"
                );
                Ok(AgentResponse {
                    reply: "Lo siento, el servicio está temporalmente no disponible. \
                        Por favor, intente de nuevo en unos momentos."
                        .to_string(),
                    tools_invoked: vec![],
                    extracted_receipt: None,
                })
            }
        }
    }
}

pub fn compose_system_prompt(
    config: &ChatbotPersona,
    tenant_context: Option<&TenantContext>,
    faqs: &[FaqEntry],
    policies: Option<&str>,
    handoff_keywords: &[String],
    guidance_rules: &[GuidanceRule],
) -> String {
    let mut sections: Vec<String> = Vec::with_capacity(8);

    if let Some(tone) = &config.tone {
        sections.push(format!("Tono: {tone}"));
    }
    sections.push(format!("Idioma: {}", config.language));

    if let Some(greeting) = &config.greeting {
        sections.push(format!("Saludo: {greeting}"));
    }

    let rules_section = format_guidance_rules(guidance_rules);
    if !rules_section.is_empty() {
        sections.push(rules_section);
    }

    if !faqs.is_empty() {
        let mut faq_section = String::from("Preguntas frecuentes:");
        for entry in faqs {
            let _ = write!(faq_section, "\nP: {}\nR: {}", entry.question, entry.answer);
        }
        sections.push(faq_section);
    }

    if let Some(policies_text) = policies {
        if !policies_text.is_empty() {
            sections.push(format!("Políticas:\n{policies_text}"));
        }
    }

    if let Some(ctx) = tenant_context {
        sections.push(format!("Inquilino: {}", ctx.name));
    }

    if !handoff_keywords.is_empty() {
        sections.push(format!(
            "Palabras clave para transferir a humano: {}",
            handoff_keywords.join(", ")
        ));
    }

    sections.join("\n\n")
}

fn format_guidance_rules(rules: &[GuidanceRule]) -> String {
    let enabled: Vec<&GuidanceRule> = rules.iter().filter(|r| r.enabled).collect();

    if enabled.is_empty() {
        return String::new();
    }

    let categories = [
        (
            GuidanceCategory::EstiloComunicacion,
            "Estilo de comunicación",
        ),
        (
            GuidanceCategory::ContextoClarificacion,
            "Contexto y clarificación",
        ),
        (GuidanceCategory::Escalamiento, "Escalamiento"),
        (GuidanceCategory::Politicas, "Políticas"),
    ];

    let mut section = String::from("## Reglas de comportamiento");

    for (category, header) in &categories {
        let mut cat_rules: Vec<&GuidanceRule> = enabled
            .iter()
            .filter(|r| &r.category == category)
            .copied()
            .collect();

        if cat_rules.is_empty() {
            continue;
        }

        cat_rules.sort_by_key(|r| r.sort_order);

        let _ = write!(section, "\n\n### {header}");
        for rule in cat_rules {
            let _ = write!(section, "\n- {}", rule.instruction);
        }
    }

    section
}

#[derive(Debug, thiserror::Error)]
pub enum ChatbotToolError {
    #[error("Error de servicio: {0}")]
    Service(String),
    #[error("Inquilino no resuelto")]
    TenantNotResolved,
    #[error("Error de validación: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct QueryBalanceInput {
    pub inquilino_id: String,
    pub organizacion_id: String,
}

#[derive(Clone)]
pub struct QueryBalanceTool {
    pub db: DatabaseConnection,
    pub sender_phone: String,
    pub sender_policy: String,
    pub organizacion_id: Uuid,
}

impl Tool for QueryBalanceTool {
    const NAME: &'static str = "query_balance";

    type Error = ChatbotToolError;
    type Args = QueryBalanceInput;
    type Output = BalanceResponse;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name(),
            description: "Consulta el balance pendiente de un inquilino. Devuelve los pagos pendientes y atrasados con totales por moneda. Usa esta herramienta cuando el usuario pregunta cuánto debe.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(Self::Args)).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let inquilino_id = Uuid::parse_str(&args.inquilino_id)
            .map_err(|e| ChatbotToolError::Validation(format!("inquilino_id inválido: {e}")))?;
        let org_id = Uuid::parse_str(&args.organizacion_id)
            .map_err(|e| ChatbotToolError::Validation(format!("organizacion_id inválido: {e}")))?;

        crate::services::chatbot::query_tenant_balance_with_sender_check(
            &self.db,
            inquilino_id,
            org_id,
            &self.sender_phone,
            &self.sender_policy,
        )
        .await
        .map_err(|e| ChatbotToolError::Service(e.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct CreateMaintenanceRequestInput {
    pub inquilino_id: String,
    pub organizacion_id: String,
    pub description: String,
    pub priority: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MaintenanceRequestResult {
    pub request_id: Uuid,
    pub status: String,
    pub priority: String,
    pub message: String,
}

#[derive(Clone)]
pub struct CreateMaintenanceRequestTool {
    pub db: DatabaseConnection,
}

impl Tool for CreateMaintenanceRequestTool {
    const NAME: &'static str = "create_maintenance_request";

    type Error = ChatbotToolError;
    type Args = CreateMaintenanceRequestInput;
    type Output = MaintenanceRequestResult;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name(),
            description: "Crea una solicitud de mantenimiento para el inquilino. Usa esta herramienta cuando el usuario reporta un problema o avería en su propiedad.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(Self::Args)).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let inquilino_id = Uuid::parse_str(&args.inquilino_id)
            .map_err(|e| ChatbotToolError::Validation(format!("inquilino_id inválido: {e}")))?;
        let org_id = Uuid::parse_str(&args.organizacion_id)
            .map_err(|e| ChatbotToolError::Validation(format!("organizacion_id inválido: {e}")))?;

        let record = crate::services::chatbot::create_maintenance_from_chat(
            &self.db,
            inquilino_id,
            org_id,
            &args.description,
            args.priority.as_deref(),
        )
        .await
        .map_err(|e| ChatbotToolError::Service(e.to_string()))?;

        Ok(MaintenanceRequestResult {
            request_id: record.id,
            status: record.estado,
            priority: record.prioridad,
            message: "Solicitud de mantenimiento creada exitosamente".to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct GetPaymentHistoryInput {
    pub inquilino_id: String,
    pub organizacion_id: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct PaymentHistoryEntry {
    pub amount: Decimal,
    pub currency: String,
    pub formatted_amount: String,
    pub status: String,
    pub due_date: String,
    pub payment_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentHistoryResult {
    pub payments: Vec<PaymentHistoryEntry>,
    pub total_count: usize,
}

#[derive(Clone)]
pub struct GetPaymentHistoryTool {
    pub db: DatabaseConnection,
    pub organizacion_id: Uuid,
    pub sender_phone: String,
}

impl Tool for GetPaymentHistoryTool {
    const NAME: &'static str = "get_payment_history";

    type Error = ChatbotToolError;
    type Args = GetPaymentHistoryInput;
    type Output = PaymentHistoryResult;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name(),
            description: "Consulta el historial de pagos recientes de un inquilino. Usa esta herramienta cuando el usuario pregunta por sus pagos anteriores o recibos.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(Self::Args)).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        use sea_orm::QuerySelect;

        let inquilino_id = Uuid::parse_str(&args.inquilino_id)
            .map_err(|e| ChatbotToolError::Validation(format!("inquilino_id inválido: {e}")))?;
        let org_id = Uuid::parse_str(&args.organizacion_id)
            .map_err(|e| ChatbotToolError::Validation(format!("organizacion_id inválido: {e}")))?;

        let limit = u64::from(args.limit.unwrap_or(10).min(50));

        let contrato_ids: Vec<Uuid> = contrato::Entity::find()
            .filter(contrato::Column::InquilinoId.eq(inquilino_id))
            .filter(contrato::Column::OrganizacionId.eq(org_id))
            .select_only()
            .column(contrato::Column::Id)
            .into_tuple()
            .all(&self.db)
            .await
            .map_err(|e| ChatbotToolError::Service(format!("Error consultando contratos: {e}")))?;

        if contrato_ids.is_empty() {
            return Ok(PaymentHistoryResult {
                payments: vec![],
                total_count: 0,
            });
        }

        let pagos = pago::Entity::find()
            .filter(pago::Column::ContratoId.is_in(contrato_ids))
            .filter(pago::Column::OrganizacionId.eq(org_id))
            .order_by_desc(pago::Column::FechaVencimiento)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| ChatbotToolError::Service(format!("Error consultando pagos: {e}")))?;

        let total_count = pagos.len();
        let payments: Vec<PaymentHistoryEntry> = pagos
            .iter()
            .map(|p| PaymentHistoryEntry {
                amount: p.monto,
                currency: p.moneda.clone(),
                formatted_amount: format_currency(p.monto, &p.moneda),
                status: p.estado.clone(),
                due_date: p.fecha_vencimiento.format("%Y-%m-%d").to_string(),
                payment_date: p.fecha_pago.map(|d| d.format("%Y-%m-%d").to_string()),
            })
            .collect();

        Ok(PaymentHistoryResult {
            payments,
            total_count,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct HandoffToHumanInput {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HandoffResult {
    pub status: String,
    pub message: String,
}

#[derive(Clone)]
pub struct HandoffToHumanTool {
    pub db: DatabaseConnection,
    pub organizacion_id: Uuid,
    pub sender_phone: String,
}

impl Tool for HandoffToHumanTool {
    const NAME: &'static str = "handoff_to_human";

    type Error = ChatbotToolError;
    type Args = HandoffToHumanInput;
    type Output = HandoffResult;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name(),
            description: "Transfiere la conversación a un operador humano. Usa esta herramienta cuando el usuario solicita hablar con una persona o cuando no puedes resolver su consulta.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(Self::Args)).unwrap_or_default(),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        crate::services::chatbot::set_handoff(&self.db, self.organizacion_id, &self.sender_phone)
            .await
            .map_err(|e| ChatbotToolError::Service(format!("Error setting handoff: {e}")))?;

        Ok(HandoffResult {
            status: "awaiting_human".to_string(),
            message: "Su conversación ha sido transferida a un operador humano. Será atendido a la brevedad.".to_string(),
        })
    }
}

pub fn get_enabled_tools(capabilities: &Capabilities) -> Vec<&'static str> {
    let mut tools = Vec::with_capacity(5);

    if capabilities.receipt_ocr {
        tools.push(ExtractReceiptTool::NAME);
    }
    if capabilities.balance_queries {
        tools.push(QueryBalanceTool::NAME);
        tools.push(GetPaymentHistoryTool::NAME);
    }
    if capabilities.maintenance_requests {
        tools.push(CreateMaintenanceRequestTool::NAME);
    }
    if capabilities.human_handoff {
        tools.push(HandoffToHumanTool::NAME);
    }

    tools
}

pub fn build_tools(
    capabilities: &Capabilities,
    db: &DatabaseConnection,
    organizacion_id: Uuid,
    sender_phone: &str,
    sender_policy: &str,
    _image_base64: Option<&str>,
) -> Vec<Box<dyn rig::tool::ToolDyn>> {
    let mut tools: Vec<Box<dyn rig::tool::ToolDyn>> = Vec::new();

    if capabilities.receipt_ocr {
        if let Ok(ocr) = OcrClient::new() {
            tools.push(Box::new(ExtractReceiptTool {
                media_store: InlineBase64MediaStore,
                ocr,
            }));
        }
    }

    if capabilities.balance_queries {
        tools.push(Box::new(QueryBalanceTool {
            db: db.clone(),
            sender_phone: sender_phone.to_string(),
            sender_policy: sender_policy.to_string(),
            organizacion_id,
        }));
        tools.push(Box::new(GetPaymentHistoryTool {
            db: db.clone(),
            organizacion_id,
            sender_phone: sender_phone.to_string(),
        }));
    }

    if capabilities.maintenance_requests {
        tools.push(Box::new(CreateMaintenanceRequestTool { db: db.clone() }));
    }

    if capabilities.human_handoff {
        tools.push(Box::new(HandoffToHumanTool {
            db: db.clone(),
            organizacion_id,
            sender_phone: sender_phone.to_string(),
        }));
    }

    tools
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn compose_system_prompt_includes_all_elements() {
        let config = ChatbotPersona {
            tone: Some("profesional".to_string()),
            greeting: Some("Bienvenido a su asistente".to_string()),
            system_prompt: Some("Eres un asistente de gestión inmobiliaria.".to_string()),
            language: "es-DO".to_string(),
        };
        let tenant = TenantContext {
            name: "Juan Pérez".to_string(),
        };
        let faqs = vec![
            FaqEntry {
                question: "¿Cuándo se paga?".to_string(),
                answer: "El día 5 de cada mes.".to_string(),
            },
            FaqEntry {
                question: "¿Aceptan transferencia?".to_string(),
                answer: "Sí, Banreservas y Popular.".to_string(),
            },
        ];
        let policies = "No se permiten mascotas. Horario de ruido: 10pm-7am.";
        let handoff_keywords = vec!["hablar con humano".to_string(), "agente".to_string()];

        let prompt = compose_system_prompt(
            &config,
            Some(&tenant),
            &faqs,
            Some(policies),
            &handoff_keywords,
            &[],
        );

        assert!(prompt.contains("profesional"));
        assert!(prompt.contains("Bienvenido a su asistente"));
        assert!(prompt.contains("Juan Pérez"));
        assert!(prompt.contains("¿Cuándo se paga?"));
        assert!(prompt.contains("El día 5 de cada mes."));
        assert!(prompt.contains("¿Aceptan transferencia?"));
        assert!(prompt.contains("Sí, Banreservas y Popular."));
        assert!(prompt.contains("No se permiten mascotas"));
        assert!(prompt.contains("Horario de ruido: 10pm-7am."));
        assert!(prompt.contains("hablar con humano"));
        assert!(prompt.contains("agente"));
    }

    #[test]
    fn compose_system_prompt_without_optional_fields() {
        let config = ChatbotPersona {
            tone: None,
            greeting: None,
            system_prompt: None,
            language: "es-DO".to_string(),
        };

        let prompt = compose_system_prompt(&config, None, &[], None, &[], &[]);

        assert!(prompt.contains("Idioma: es-DO"));
        assert!(!prompt.contains("Preguntas frecuentes:"));
        assert!(!prompt.contains("Políticas:"));
        assert!(!prompt.contains("Palabras clave"));
        assert!(!prompt.contains("Inquilino:"));
        assert!(!prompt.contains("Reglas de comportamiento"));
    }

    #[test]
    fn compose_system_prompt_with_tenant_no_faqs() {
        let config = ChatbotPersona {
            tone: Some("amigable".to_string()),
            greeting: Some("Hola!".to_string()),
            system_prompt: None,
            language: "es-DO".to_string(),
        };
        let tenant = TenantContext {
            name: "María García".to_string(),
        };

        let prompt = compose_system_prompt(&config, Some(&tenant), &[], None, &[], &[]);

        assert!(prompt.contains("amigable"));
        assert!(prompt.contains("Hola!"));
        assert!(prompt.contains("María García"));
        assert!(!prompt.contains("Preguntas frecuentes:"));
    }

    #[test]
    fn compose_system_prompt_with_guidance_rules() {
        use chrono::Utc;

        let config = ChatbotPersona {
            tone: Some("profesional".to_string()),
            greeting: None,
            system_prompt: None,
            language: "es-DO".to_string(),
        };

        let now = Utc::now();
        let rules = vec![
            GuidanceRule {
                id: Uuid::new_v4(),
                category: GuidanceCategory::EstiloComunicacion,
                instruction: "Tratar a todos de usted".to_string(),
                enabled: true,
                is_template: true,
                sort_order: 1,
                created_at: now,
                updated_at: now,
            },
            GuidanceRule {
                id: Uuid::new_v4(),
                category: GuidanceCategory::EstiloComunicacion,
                instruction: "Responder en español".to_string(),
                enabled: true,
                is_template: true,
                sort_order: 2,
                created_at: now,
                updated_at: now,
            },
            GuidanceRule {
                id: Uuid::new_v4(),
                category: GuidanceCategory::Escalamiento,
                instruction: "Transferir si menciona abogado".to_string(),
                enabled: true,
                is_template: true,
                sort_order: 1,
                created_at: now,
                updated_at: now,
            },
            GuidanceRule {
                id: Uuid::new_v4(),
                category: GuidanceCategory::Politicas,
                instruction: "Nunca compartir datos bancarios".to_string(),
                enabled: false,
                is_template: true,
                sort_order: 1,
                created_at: now,
                updated_at: now,
            },
        ];

        let prompt = compose_system_prompt(&config, None, &[], None, &[], &rules);

        assert!(prompt.contains("## Reglas de comportamiento"));
        assert!(prompt.contains("### Estilo de comunicación"));
        assert!(prompt.contains("- Tratar a todos de usted"));
        assert!(prompt.contains("- Responder en español"));
        assert!(prompt.contains("### Escalamiento"));
        assert!(prompt.contains("- Transferir si menciona abogado"));
        assert!(!prompt.contains("Nunca compartir datos bancarios"));
        assert!(!prompt.contains("### Políticas"));
        assert!(!prompt.contains("### Contexto y clarificación"));
    }

    #[test]
    fn compose_system_prompt_all_rules_disabled_omits_header() {
        use chrono::Utc;

        let config = ChatbotPersona {
            tone: None,
            greeting: None,
            system_prompt: None,
            language: "es-DO".to_string(),
        };

        let now = Utc::now();
        let rules = vec![GuidanceRule {
            id: Uuid::new_v4(),
            category: GuidanceCategory::Politicas,
            instruction: "Some rule".to_string(),
            enabled: false,
            is_template: true,
            sort_order: 1,
            created_at: now,
            updated_at: now,
        }];

        let prompt = compose_system_prompt(&config, None, &[], None, &[], &rules);

        assert!(!prompt.contains("Reglas de comportamiento"));
    }

    #[test]
    fn compose_system_prompt_rules_sorted_by_sort_order() {
        use chrono::Utc;

        let config = ChatbotPersona {
            tone: None,
            greeting: None,
            system_prompt: None,
            language: "es-DO".to_string(),
        };

        let now = Utc::now();
        let rules = vec![
            GuidanceRule {
                id: Uuid::new_v4(),
                category: GuidanceCategory::Escalamiento,
                instruction: "Second rule".to_string(),
                enabled: true,
                is_template: false,
                sort_order: 2,
                created_at: now,
                updated_at: now,
            },
            GuidanceRule {
                id: Uuid::new_v4(),
                category: GuidanceCategory::Escalamiento,
                instruction: "First rule".to_string(),
                enabled: true,
                is_template: false,
                sort_order: 1,
                created_at: now,
                updated_at: now,
            },
        ];

        let prompt = compose_system_prompt(&config, None, &[], None, &[], &rules);

        let first_pos = prompt.find("First rule").unwrap();
        let second_pos = prompt.find("Second rule").unwrap();
        assert!(
            first_pos < second_pos,
            "Rules should be sorted by sort_order"
        );
    }

    #[test]
    fn get_enabled_tools_all_enabled() {
        let caps = Capabilities {
            receipt_ocr: true,
            balance_queries: true,
            payment_reminders: true,
            maintenance_requests: true,
            human_handoff: true,
        };

        let tools = get_enabled_tools(&caps);

        assert!(tools.contains(&"extract_receipt"));
        assert!(tools.contains(&"query_balance"));
        assert!(tools.contains(&"get_payment_history"));
        assert!(tools.contains(&"create_maintenance_request"));
        assert!(tools.contains(&"handoff_to_human"));
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn get_enabled_tools_none_enabled() {
        let caps = Capabilities {
            receipt_ocr: false,
            balance_queries: false,
            payment_reminders: false,
            maintenance_requests: false,
            human_handoff: false,
        };

        let tools = get_enabled_tools(&caps);

        assert!(tools.is_empty());
    }

    #[test]
    fn get_enabled_tools_balance_queries_registers_both_tools() {
        let caps = Capabilities {
            receipt_ocr: false,
            balance_queries: true,
            payment_reminders: false,
            maintenance_requests: false,
            human_handoff: false,
        };

        let tools = get_enabled_tools(&caps);

        assert_eq!(tools.len(), 2);
        assert!(tools.contains(&"query_balance"));
        assert!(tools.contains(&"get_payment_history"));
    }

    #[test]
    fn get_enabled_tools_partial_capabilities() {
        let caps = Capabilities {
            receipt_ocr: true,
            balance_queries: false,
            payment_reminders: true,
            maintenance_requests: true,
            human_handoff: false,
        };

        let tools = get_enabled_tools(&caps);

        assert_eq!(tools.len(), 2);
        assert!(tools.contains(&"extract_receipt"));
        assert!(tools.contains(&"create_maintenance_request"));
        assert!(!tools.contains(&"query_balance"));
        assert!(!tools.contains(&"handoff_to_human"));
    }
}
