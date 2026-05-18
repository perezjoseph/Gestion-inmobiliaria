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
    AgentConfig, BalanceResponse, Capabilities, FaqEntry, GuardrailOverrides,
    ToolRegistrationStrategy, format_currency,
};
use crate::services::ocr_client::OcrClient;
use crate::services::ovms_provider::OvmsCompletionModel;

// Re-export tool types for backward compatibility
pub use tools::{ExtractReceiptInput, ExtractReceiptTool, InlineBase64MediaStore, PaymentReceipt};

// =============================================================================
// GuardrailConfig — validation limits for the guardrail hook
// =============================================================================

/// Configuration for argument validation and output safety in the guardrail hook.
#[derive(Clone)]
pub struct GuardrailConfig {
    /// Maximum allowed payment amount for receipt extraction (DOP).
    pub max_receipt_amount_dop: Decimal,
    /// Maximum allowed payment amount for receipt extraction (USD).
    pub max_receipt_amount_usd: Decimal,
    /// Maximum description length for maintenance requests.
    pub max_description_length: usize,
    /// Blocked output patterns (compiled regexes for safety filtering).
    pub blocked_output_patterns: Vec<Regex>,
}

impl Default for GuardrailConfig {
    fn default() -> Self {
        Self {
            max_receipt_amount_dop: Decimal::new(10_000_000, 2), // RD$100,000
            max_receipt_amount_usd: Decimal::new(500_000, 2),    // US$5,000
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

// =============================================================================
// Types for process_message entry point
// =============================================================================

/// A user message passed to the AI module (text + optional image attachment).
#[derive(Debug, Clone)]
pub struct UserMessage {
    /// Text content of the message.
    pub content: String,
    /// Optional base64-encoded image attachment.
    pub image_base64: Option<String>,
}

/// A conversation history entry with role and content.
#[derive(Debug, Clone)]
pub struct ConversationEntry {
    /// `"user"` or `"assistant"`.
    pub role: String,
    /// Message content.
    pub content: String,
}

/// Response from the AI agent after processing a message.
#[derive(Debug, Clone, Serialize)]
pub struct AgentResponse {
    /// The text reply to send back to the user.
    pub reply: String,
    /// Names of tools that were invoked during processing.
    pub tools_invoked: Vec<String>,
    /// If `extract_receipt` was successfully invoked, carries the extracted receipt data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_receipt: Option<PaymentReceipt>,
}

// =============================================================================
// Persona and context types
// =============================================================================

/// Subset of chatbot persona configuration used for system prompt composition.
#[derive(Debug, Clone)]
pub struct ChatbotPersona {
    pub tone: Option<String>,
    pub greeting: Option<String>,
    pub system_prompt: Option<String>,
    pub language: String,
}

/// Resolved tenant information used to personalize AI responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
    pub name: String,
}

/// AI module that wraps an [`OvmsCompletionModel`] for Rig's native agent loop.
/// Stateless — receives all context as parameters.
pub struct AiModule {
    pub(crate) model: OvmsCompletionModel,
    pub(crate) timeout_secs: u64,
}

/// Context passed to [`AiModule::process_message`] grouping all per-invocation parameters.
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
    /// Per-organization agent configuration. Defaults to `AgentConfig::default()`.
    pub agent_config: AgentConfig,
}

impl AiModule {
    /// Creates a new [`AiModule`] from the chatbot environment configuration.
    /// Uses the custom [`OvmsCompletionModel`] that handles OVMS's missing `id` field.
    pub fn new(config: &ChatbotEnvConfig) -> Result<Self, anyhow::Error> {
        let model = OvmsCompletionModel::new(&config.ovms_chat_model, &config.ovms_endpoint);

        Ok(Self {
            model,
            timeout_secs: config.ai_chat_timeout_secs,
        })
    }

    /// Single entry point for all messages (text or image).
    ///
    /// Uses Rig's native agent loop with the OVMS provider. The LLM decides
    /// which tools to invoke based on content. This method:
    /// 1. Composes the system prompt from persona config + tenant context
    /// 2. Resolves `AgentConfig` and builds the guardrail hook with shared state
    /// 3. Builds a Rig agent via `AgentBuilder` with tools, hook, and config
    /// 4. Invokes the agent with timeout
    /// 5. Extracts side-effects from shared state and returns `AgentResponse`
    pub async fn process_message(
        &self,
        ctx: &ProcessMessageContext<'_>,
    ) -> Result<AgentResponse, AppError> {
        use rig::agent::PromptRequest;
        use rig::message::{self, UserContent};
        use rig::one_or_many::OneOrMany;

        // 1. Compose system prompt
        let system_prompt = compose_system_prompt(
            ctx.config,
            ctx.tenant_context,
            ctx.faqs,
            ctx.policies,
            ctx.handoff_keywords,
        );

        // 2. Resolve AgentConfig
        let resolved = ctx.agent_config.resolve();

        // 3. Build shared state for the hook
        let captured_receipt: Arc<Mutex<Option<PaymentReceipt>>> = Arc::new(Mutex::new(None));
        let tools_invoked: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        // 4. Construct GuardrailConfig from resolved guardrails
        let guardrail_config: GuardrailConfig = resolved.guardrails.clone().into();

        // 5. Construct RentalGuardrailHook
        let hook = guardrail_hook::RentalGuardrailHook {
            captured_receipt: captured_receipt.clone(),
            tools_invoked: tools_invoked.clone(),
            organizacion_id: ctx.organizacion_id,
            guardrail_config,
        };

        // 6. Build tools based on tool_registration strategy
        let capabilities_for_tools = match resolved.tool_registration {
            ToolRegistrationStrategy::Selective => ctx.capabilities,
            ToolRegistrationStrategy::AllWithHookGating => &Capabilities::all(),
        };
        let tools = build_tools(
            capabilities_for_tools,
            ctx.db,
            ctx.organizacion_id,
            ctx.sender_phone,
            ctx.user_message.image_base64.as_deref(),
        );

        // 7. Build agent via AgentBuilder
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

        // 8. Compose the user prompt (include image reference if present)
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

        // 9. Convert conversation history to Rig Message format
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

        // 10. Invoke agent with timeout
        let timeout_duration = Duration::from_secs(self.timeout_secs);

        let result = tokio::time::timeout(timeout_duration, async {
            PromptRequest::from_agent(&agent, user_prompt)
                .with_history(chat_history)
                .max_turns(resolved.max_turns)
                .await
        })
        .await;

        // 11. Handle result and extract side-effects from shared state
        match result {
            Ok(Ok(reply)) => {
                // Check for empty reply (Req 1.6)
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

                // Handle HookAction::Terminate from safety filter (Req 5.3)
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

/// Composes a system prompt from the organization's persona configuration,
/// FAQs, policies, tenant context, and handoff keywords.
///
/// The resulting prompt is a structured string that includes all provided
/// elements so the LLM has full context for generating responses.
pub fn compose_system_prompt(
    config: &ChatbotPersona,
    tenant_context: Option<&TenantContext>,
    faqs: &[FaqEntry],
    policies: Option<&str>,
    handoff_keywords: &[String],
) -> String {
    let mut sections: Vec<String> = Vec::with_capacity(8);

    // Base system prompt override (if provided)
    if let Some(system_prompt) = &config.system_prompt {
        sections.push(system_prompt.clone());
    }

    // Language and tone
    if let Some(tone) = &config.tone {
        sections.push(format!("Tono: {tone}"));
    }
    sections.push(format!("Idioma: {}", config.language));

    // Tenant context
    if let Some(ctx) = tenant_context {
        sections.push(format!("Inquilino: {}", ctx.name));
    }

    // Greeting
    if let Some(greeting) = &config.greeting {
        sections.push(format!("Saludo: {greeting}"));
    }

    // FAQs
    if !faqs.is_empty() {
        let mut faq_section = String::from("Preguntas frecuentes:");
        for entry in faqs {
            let _ = write!(faq_section, "\nP: {}\nR: {}", entry.question, entry.answer);
        }
        sections.push(faq_section);
    }

    // Policies
    if let Some(policies_text) = policies {
        if !policies_text.is_empty() {
            sections.push(format!("Políticas:\n{policies_text}"));
        }
    }

    // Handoff keywords
    if !handoff_keywords.is_empty() {
        sections.push(format!(
            "Palabras clave para transferir a humano: {}",
            handoff_keywords.join(", ")
        ));
    }

    sections.join("\n\n")
}

// =============================================================================
// Rig Tool Definitions (other tools remain inline)
// =============================================================================

/// Error type for all chatbot AI tools (except `ExtractReceiptTool` which has its own).
#[derive(Debug, thiserror::Error)]
pub enum ChatbotToolError {
    #[error("Error de servicio: {0}")]
    Service(String),
    #[error("Inquilino no resuelto")]
    TenantNotResolved,
    #[error("Error de validación: {0}")]
    Validation(String),
}

// --- QueryBalanceTool ---

/// Input the LLM provides when it wants to query a tenant's balance.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct QueryBalanceInput {
    /// UUID del inquilino cuyo balance se desea consultar
    pub inquilino_id: String,
    /// UUID de la organización
    pub organizacion_id: String,
}

/// Rig tool that calls [`query_tenant_balance`](super::chatbot::query_tenant_balance).
#[derive(Clone)]
pub struct QueryBalanceTool {
    pub db: DatabaseConnection,
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

        crate::services::chatbot::query_tenant_balance(&self.db, inquilino_id, org_id)
            .await
            .map_err(|e| ChatbotToolError::Service(e.to_string()))
    }
}

// --- CreateMaintenanceRequestTool ---

/// Input the LLM provides when creating a maintenance request.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct CreateMaintenanceRequestInput {
    /// UUID del inquilino que reporta el problema
    pub inquilino_id: String,
    /// UUID de la organización
    pub organizacion_id: String,
    /// Descripción del problema de mantenimiento (2–1000 caracteres)
    pub description: String,
    /// Prioridad de la solicitud: `baja`, `media`, `alta` o `urgente`
    pub priority: Option<String>,
}

/// Output returned to the LLM after creating a maintenance request.
#[derive(Debug, Serialize)]
pub struct MaintenanceRequestResult {
    pub request_id: Uuid,
    pub status: String,
    pub priority: String,
    pub message: String,
}

/// Rig tool that calls [`create_maintenance_from_chat`](super::chatbot::create_maintenance_from_chat).
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

// --- GetPaymentHistoryTool ---

/// Input the LLM provides when querying payment history.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct GetPaymentHistoryInput {
    /// UUID del inquilino
    pub inquilino_id: String,
    /// UUID de la organización
    pub organizacion_id: String,
    /// Cantidad máxima de pagos a devolver (por defecto 10)
    pub limit: Option<u32>,
}

/// A single payment record in the history response.
#[derive(Debug, Serialize)]
pub struct PaymentHistoryEntry {
    pub amount: Decimal,
    pub currency: String,
    pub formatted_amount: String,
    pub status: String,
    pub due_date: String,
    pub payment_date: Option<String>,
}

/// Output returned to the LLM with recent payment history.
#[derive(Debug, Serialize)]
pub struct PaymentHistoryResult {
    pub payments: Vec<PaymentHistoryEntry>,
    pub total_count: usize,
}

/// Rig tool that queries recent payments for a tenant.
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

        // Find all contrato IDs for this tenant in this org
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

        // Query pagos for those contracts, ordered by due date descending
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

// --- HandoffToHumanTool ---

/// Input the LLM provides when handing off to a human operator.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct HandoffToHumanInput {
    /// Razón por la cual se transfiere a un operador humano
    pub reason: Option<String>,
}

/// Output returned to the LLM after flagging for human handoff.
#[derive(Debug, Serialize)]
pub struct HandoffResult {
    pub status: String,
    pub message: String,
}

/// Rig tool that flags the conversation for human operator handoff.
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
        // Set conversation handoff_status to `awaiting_human`
        crate::services::chatbot::set_handoff(&self.db, self.organizacion_id, &self.sender_phone)
            .await
            .map_err(|e| ChatbotToolError::Service(format!("Error setting handoff: {e}")))?;

        Ok(HandoffResult {
            status: "awaiting_human".to_string(),
            message: "Su conversación ha sido transferida a un operador humano. Será atendido a la brevedad.".to_string(),
        })
    }
}

// =============================================================================
// Capability-Gated Tool Registration
// =============================================================================

/// Returns the list of tool names that should be registered on the AI agent
/// based on the organization's enabled capabilities.
///
/// Mapping:
/// - `receipt_ocr` → [`ExtractReceiptTool`] (`"extract_receipt"`)
/// - `balance_queries` → [`QueryBalanceTool`] (`"query_balance"`) + [`GetPaymentHistoryTool`] (`"get_payment_history"`)
/// - `maintenance_requests` → [`CreateMaintenanceRequestTool`] (`"create_maintenance_request"`)
/// - `human_handoff` → [`HandoffToHumanTool`] (`"handoff_to_human"`)
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

/// Builds the tool vector based on enabled capabilities.
/// Only enabled tools are registered — the LLM never sees disabled tool definitions.
pub fn build_tools(
    capabilities: &Capabilities,
    db: &DatabaseConnection,
    organizacion_id: Uuid,
    sender_phone: &str,
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
        tools.push(Box::new(QueryBalanceTool { db: db.clone() }));
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
        );

        // Tone keyword present
        assert!(prompt.contains("profesional"));
        // Greeting present
        assert!(prompt.contains("Bienvenido a su asistente"));
        // System prompt override present
        assert!(prompt.contains("Eres un asistente de gestión inmobiliaria."));
        // Tenant name present
        assert!(prompt.contains("Juan Pérez"));
        // All FAQ Q&A pairs present
        assert!(prompt.contains("¿Cuándo se paga?"));
        assert!(prompt.contains("El día 5 de cada mes."));
        assert!(prompt.contains("¿Aceptan transferencia?"));
        assert!(prompt.contains("Sí, Banreservas y Popular."));
        // Policies text present
        assert!(prompt.contains("No se permiten mascotas"));
        assert!(prompt.contains("Horario de ruido: 10pm-7am."));
        // Handoff keywords present
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

        let prompt = compose_system_prompt(&config, None, &[], None, &[]);

        assert!(prompt.contains("Idioma: es-DO"));
        // Should not contain optional section headers when empty
        assert!(!prompt.contains("Preguntas frecuentes:"));
        assert!(!prompt.contains("Políticas:"));
        assert!(!prompt.contains("Palabras clave"));
        assert!(!prompt.contains("Inquilino:"));
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

        let prompt = compose_system_prompt(&config, Some(&tenant), &[], None, &[]);

        assert!(prompt.contains("amigable"));
        assert!(prompt.contains("Hola!"));
        assert!(prompt.contains("María García"));
        assert!(!prompt.contains("Preguntas frecuentes:"));
    }

    // --- get_enabled_tools tests ---

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
        // payment_reminders has no tool mapping
        assert!(!tools.contains(&"query_balance"));
        assert!(!tools.contains(&"handoff_to_human"));
    }
}
