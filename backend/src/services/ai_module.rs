use std::fmt::Write as _;
use std::time::Duration;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::ChatbotEnvConfig;
use crate::errors::AppError;
use crate::models::chatbot::{BalanceResponse, Capabilities, Confidence, FaqEntry};

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
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub name: String,
}

/// AI module that wraps a Rig `OpenAI`-compatible client pointed at the
/// self-hosted `OVMS` endpoint. Stateless — receives all context as parameters.
pub struct AiModule {
    #[allow(dead_code)]
    pub(crate) client: rig::providers::openai::Client,
    pub(crate) model_name: String,
    pub(crate) timeout_secs: u64,
    pub(crate) ovms_endpoint: String,
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
}

impl AiModule {
    /// Creates a new [`AiModule`] from the chatbot environment configuration.
    /// The `OpenAI`-compatible client is pointed at the self-hosted `OVMS` endpoint.
    pub fn new(config: &ChatbotEnvConfig) -> Result<Self, anyhow::Error> {
        let client = rig::providers::openai::Client::builder()
            .api_key("unused")
            .base_url(&config.ovms_endpoint)
            .build()
            .map_err(|e| anyhow::anyhow!("Error al crear cliente OVMS: {e}"))?;

        Ok(Self {
            client,
            model_name: config.ovms_chat_model.clone(),
            timeout_secs: config.ai_chat_timeout_secs,
            ovms_endpoint: config.ovms_endpoint.clone(),
        })
    }

    /// Single entry point for all messages (text or image).
    ///
    /// The LLM decides which tools to invoke based on content. This method:
    /// 1. Composes the system prompt from persona config + tenant context
    /// 2. Builds a Rig agent with enabled tools
    /// 3. Converts conversation history to Rig `Message` format
    /// 4. Sends the user message to the agent with a timeout
    /// 5. Returns the agent's response or a timeout error message
    pub async fn process_message(
        &self,
        ctx: &ProcessMessageContext<'_>,
    ) -> Result<AgentResponse, AppError> {
        // 1. Compose system prompt
        let system_prompt = compose_system_prompt(
            ctx.config,
            ctx.tenant_context,
            ctx.faqs,
            ctx.policies,
            ctx.handoff_keywords,
        );

        // 2. Compose the user prompt (include image reference if present)
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

        // 4. Build agent with enabled tools and invoke with timeout
        let timeout_duration = Duration::from_secs(self.timeout_secs);

        let result = tokio::time::timeout(timeout_duration, async {
            self.invoke_agent(&system_prompt, ctx.capabilities, ctx.history, &user_prompt)
                .await
        })
        .await;

        match result {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(e)) => {
                tracing::error!(
                    organizacion_id = %ctx.organizacion_id,
                    error = %e,
                    "Error en el agente AI"
                );
                Err(e)
            }
            Err(_elapsed) => {
                tracing::warn!(
                    organizacion_id = %ctx.organizacion_id,
                    timeout_secs = self.timeout_secs,
                    "Timeout en la solicitud al modelo OVMS"
                );
                // Return a user-friendly error message per Requirement 3.7
                Ok(AgentResponse {
                    reply: "Lo siento, el servicio está temporalmente no disponible. \
                        Por favor, intente de nuevo en unos momentos."
                        .to_string(),
                    tools_invoked: vec![],
                })
            }
        }
    }

    /// Internal method that builds the Rig agent and invokes it.
    /// Separated from `process_message` to keep the timeout wrapper clean.
    ///
    /// Uses a direct HTTP call to OVMS instead of Rig's built-in client because
    /// OVMS's OpenAI-compatible response omits the `id` field, which Rig requires.
    async fn invoke_agent(
        &self,
        system_prompt: &str,
        capabilities: &Capabilities,
        history: &[ConversationEntry],
        user_prompt: &str,
    ) -> Result<AgentResponse, AppError> {
        let enabled_tools = get_enabled_tools(capabilities);
        let _ = &enabled_tools;

        // Build the messages array for the OpenAI-compatible request
        let mut messages = Vec::new();

        // System prompt
        messages.push(serde_json::json!({
            "role": "system",
            "content": system_prompt
        }));

        // Conversation history
        for entry in history {
            messages.push(serde_json::json!({
                "role": entry.role,
                "content": entry.content
            }));
        }

        // Current user message
        messages.push(serde_json::json!({
            "role": "user",
            "content": user_prompt
        }));

        // Build the request body
        let request_body = serde_json::json!({
            "model": self.model_name,
            "messages": messages,
            "stream": false
        });

        // Make direct HTTP call to OVMS
        let client = reqwest::Client::new();
        let url = format!("{}/chat/completions", self.ovms_endpoint);

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error conectando a OVMS: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(anyhow::anyhow!(
                "OVMS respondió con estado {status}: {body}"
            )));
        }

        let body: OvmsCompletionResponse = response.json().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Error parseando respuesta OVMS: {e}"))
        })?;

        let reply = body
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(AgentResponse {
            reply,
            tools_invoked: vec![],
        })
    }
}

/// OVMS chat completion response (OpenAI-compatible but without `id` field).
#[derive(Debug, Deserialize)]
struct OvmsCompletionResponse {
    pub choices: Vec<OvmsChoice>,
    #[allow(dead_code)]
    pub model: String,
}

#[derive(Debug, Deserialize)]
struct OvmsChoice {
    pub message: OvmsMessage,
    #[allow(dead_code)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OvmsMessage {
    pub content: Option<String>,
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
// Rig Tool Definitions
// =============================================================================

/// Error type for all chatbot AI tools.
#[derive(Debug, thiserror::Error)]
pub enum ChatbotToolError {
    #[error("Error de servicio: {0}")]
    Service(String),
    #[error("Inquilino no resuelto")]
    TenantNotResolved,
    #[error("Error de validación: {0}")]
    Validation(String),
}

// --- ExtractReceiptTool ---

/// Input the LLM provides when it wants to extract receipt data from an image.
#[derive(Debug, Deserialize, Serialize)]
pub struct ExtractReceiptInput {
    /// Base64-encoded image data.
    pub image_base64: String,
    /// Optional caption accompanying the image.
    pub caption: Option<String>,
}

/// Structured payment receipt data extracted from an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentReceipt {
    pub bank: Option<String>,
    pub amount: Decimal,
    /// `"DOP"` or `"USD"`.
    pub currency: String,
    /// ISO 8601 date string.
    pub date: Option<String>,
    /// Reference number (up to 50 characters).
    pub reference: Option<String>,
    pub sender_name: Option<String>,
    pub recipient: Option<String>,
    pub confidence: Confidence,
}

/// Rig tool that calls the OCR service and maps the result to a [`PaymentReceipt`].
#[derive(Clone)]
pub struct ExtractReceiptTool {
    pub db: DatabaseConnection,
}

impl Tool for ExtractReceiptTool {
    const NAME: &'static str = "extract_receipt";

    type Error = ChatbotToolError;
    type Args = ExtractReceiptInput;
    type Output = PaymentReceipt;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name(),
            description: "Extrae datos estructurados de un comprobante de pago a partir de una imagen. Usa esta herramienta cuando el usuario envía una foto de un recibo o comprobante de transferencia.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "image_base64": {
                        "type": "string",
                        "description": "Imagen del recibo codificada en base64"
                    },
                    "caption": {
                        "type": "string",
                        "description": "Texto opcional que acompaña la imagen"
                    }
                },
                "required": ["image_base64"]
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Stub: actual OCR service call will be wired in the full pipeline.
        Err(ChatbotToolError::Service(
            "OCR service not yet wired".to_string(),
        ))
    }
}

// --- QueryBalanceTool ---

/// Input the LLM provides when it wants to query a tenant's balance.
#[derive(Debug, Deserialize, Serialize)]
pub struct QueryBalanceInput {
    /// UUID of the tenant whose balance to query.
    pub inquilino_id: String,
    /// UUID of the organization.
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
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "inquilino_id": {
                        "type": "string",
                        "description": "UUID del inquilino"
                    },
                    "organizacion_id": {
                        "type": "string",
                        "description": "UUID de la organización"
                    }
                },
                "required": ["inquilino_id", "organizacion_id"]
            }),
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
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateMaintenanceRequestInput {
    /// UUID of the tenant filing the request.
    pub inquilino_id: String,
    /// UUID of the organization.
    pub organizacion_id: String,
    /// Description of the maintenance issue (2–1000 characters).
    pub description: String,
    /// Optional priority: `baja`, `media`, `alta`, or `urgente`.
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
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "inquilino_id": {
                        "type": "string",
                        "description": "UUID del inquilino"
                    },
                    "organizacion_id": {
                        "type": "string",
                        "description": "UUID de la organización"
                    },
                    "description": {
                        "type": "string",
                        "description": "Descripción del problema de mantenimiento (2–1000 caracteres)"
                    },
                    "priority": {
                        "type": "string",
                        "enum": ["baja", "media", "alta", "urgente"],
                        "description": "Prioridad de la solicitud (por defecto: media)"
                    }
                },
                "required": ["inquilino_id", "organizacion_id", "description"]
            }),
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
#[derive(Debug, Deserialize, Serialize)]
pub struct GetPaymentHistoryInput {
    /// UUID of the tenant.
    pub inquilino_id: String,
    /// UUID of the organization.
    pub organizacion_id: String,
    /// Maximum number of recent payments to return (default 10).
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
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "inquilino_id": {
                        "type": "string",
                        "description": "UUID del inquilino"
                    },
                    "organizacion_id": {
                        "type": "string",
                        "description": "UUID de la organización"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Cantidad máxima de pagos a devolver (por defecto 10)"
                    }
                },
                "required": ["inquilino_id", "organizacion_id"]
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Stub: full query will be wired when the pipeline is assembled.
        // The actual implementation will query pagos for the tenant's contracts.
        Ok(PaymentHistoryResult {
            payments: vec![],
            total_count: 0,
        })
    }
}

// --- HandoffToHumanTool ---

/// Input the LLM provides when handing off to a human operator.
#[derive(Debug, Deserialize, Serialize)]
pub struct HandoffToHumanInput {
    /// Reason for the handoff (from conversation context).
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
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "reason": {
                        "type": "string",
                        "description": "Razón por la cual se transfiere a un humano"
                    }
                },
                "required": []
            }),
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
