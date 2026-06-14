use actix_web::{HttpResponse, web};
use futures_util::StreamExt;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::chatbot::{
    BatchUpdateRequest, Capabilities, ChatbotConfigUpdateRequest, ClearHandoffRequest,
    ConversationMessage, CreateGuidanceRuleRequest, FaqEntry, HandoffStatusResponse,
    ReceiptConfirmRequest, ReceiptExtractionResponse, TestChatRequest, TestChatResponse,
    UpdateGuidanceRuleRequest,
};
use crate::services::ai_module::{
    AiModule, ChatbotPersona, ConversationEntry, ProcessMessageContext, UserMessage,
    compose_system_prompt,
};
use crate::services::baileys_client::BaileysClient;
use crate::services::chatbot;
use crate::services::ovms_provider::OvmsCompletionModel;

pub async fn get_config(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
) -> Result<HttpResponse, AppError> {
    let config = chatbot::get_config(db.get_ref(), claims.0.organizacion_id).await?;
    Ok(HttpResponse::Ok().json(config))
}

pub async fn update_config(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
    body: web::Json<ChatbotConfigUpdateRequest>,
) -> Result<HttpResponse, AppError> {
    let result = chatbot::upsert_config(
        db.get_ref(),
        claims.0.organizacion_id,
        body.into_inner(),
        claims.0.sub,
        &claims.0.rol,
    )
    .await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn connect(
    baileys: Option<web::Data<BaileysClient>>,
    claims: WriteAccess,
) -> Result<HttpResponse, AppError> {
    let client = baileys.ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "BaileysClient no configurado — servicio WhatsApp no disponible"
        ))
    })?;

    let realm_id = claims.0.organizacion_id;
    let response = client.start_session(realm_id).await?;

    let body = serde_json::to_value(&response)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error serializando respuesta: {e}")))?;

    Ok(HttpResponse::Ok().json(normalize_baileys_response(&body)))
}

pub async fn disconnect(
    baileys: Option<web::Data<BaileysClient>>,
    claims: WriteAccess,
) -> Result<HttpResponse, AppError> {
    let client = baileys.ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "BaileysClient no configurado — servicio WhatsApp no disponible"
        ))
    })?;

    let realm_id = claims.0.organizacion_id;
    let response = client.stop_session(realm_id).await?;

    let body = serde_json::to_value(&response)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error serializando respuesta: {e}")))?;

    Ok(HttpResponse::Ok().json(normalize_baileys_response(&body)))
}

pub async fn status(
    baileys: Option<web::Data<BaileysClient>>,
    claims: WriteAccess,
) -> Result<HttpResponse, AppError> {
    let realm_id = claims.0.organizacion_id;

    let Some(client) = baileys else {
        tracing::warn!(
            organizacion_id = %realm_id,
            "BaileysClient no configurado — reportando estado desconectado"
        );
        return Ok(HttpResponse::Ok().json(disconnected_status()));
    };

    match client.get_status(realm_id).await {
        Ok(response) => {
            let body = serde_json::to_value(&response).map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Error serializando respuesta: {e}"))
            })?;
            Ok(HttpResponse::Ok().json(normalize_baileys_response(&body)))
        }
        Err(e) => {
            tracing::warn!(
                organizacion_id = %realm_id,
                error = %e,
                "Servicio Baileys no disponible — reportando estado desconectado"
            );
            Ok(HttpResponse::Ok().json(disconnected_status()))
        }
    }
}

fn disconnected_status() -> serde_json::Value {
    serde_json::json!({
        "status": "disconnected",
        "qrCode": serde_json::Value::Null,
        "connectedPhone": serde_json::Value::Null,
        "connectedAt": serde_json::Value::Null,
    })
}

#[derive(Debug, Deserialize)]
pub struct ConversationListQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

pub async fn list_conversations(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
    query: web::Query<ConversationListQuery>,
) -> Result<HttpResponse, AppError> {
    let org_id = claims.0.organizacion_id;
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let result = chatbot::list_conversations(db.get_ref(), org_id, page, per_page).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_conversation_history(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
    path: web::Path<String>,
    query: web::Query<ConversationListQuery>,
) -> Result<HttpResponse, AppError> {
    let org_id = claims.0.organizacion_id;
    let phone = path.into_inner();
    let limit = query.per_page.unwrap_or(50).clamp(1, 100);

    let messages = chatbot::load_history(db.get_ref(), org_id, &phone, limit).await?;

    let response: Vec<ConversationMessage> = messages
        .into_iter()
        .map(|m| ConversationMessage {
            id: m.id,
            sender_phone: m.sender_phone,
            inquilino_id: m.inquilino_id,
            role: m.role,
            content: m.content,
            message_type: m.message_type,
            created_at: m.created_at.into(),
        })
        .collect();

    Ok(HttpResponse::Ok().json(response))
}

pub async fn list_pending_receipts(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
) -> Result<HttpResponse, AppError> {
    let org_id = claims.0.organizacion_id;
    let records = chatbot::list_pending_receipts(db.get_ref(), org_id).await?;

    let response: Vec<ReceiptExtractionResponse> =
        records.into_iter().map(extraction_to_response).collect();

    Ok(HttpResponse::Ok().json(response))
}

pub async fn confirm_receipt(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let extraction_id = path.into_inner();
    let user_id = claims.0.sub;

    let updated = chatbot::confirm_receipt(db.get_ref(), extraction_id, user_id).await?;
    Ok(HttpResponse::Ok().json(extraction_to_response(updated)))
}

pub async fn reject_receipt(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<ReceiptConfirmRequest>,
) -> Result<HttpResponse, AppError> {
    let extraction_id = path.into_inner();
    let user_id = claims.0.sub;
    let reason = body.rejection_reason.as_deref();

    let updated = chatbot::reject_receipt(db.get_ref(), extraction_id, user_id, reason).await?;
    Ok(HttpResponse::Ok().json(extraction_to_response(updated)))
}

fn normalize_baileys_response(body: &serde_json::Value) -> serde_json::Value {
    let status = body
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("disconnected");

    let qr_code = body.get("qr").and_then(|v| v.as_str()).map(|raw| {
        raw.strip_prefix("data:image/png;base64,")
            .unwrap_or(raw)
            .to_string()
    });

    let connected_phone = body
        .get("connectedPhone")
        .and_then(|v| v.as_str())
        .map(String::from);

    let connected_at = body
        .get("connectedAt")
        .and_then(|v| v.as_str())
        .map(String::from);

    serde_json::json!({
        "status": status,
        "qrCode": qr_code,
        "connectedPhone": connected_phone,
        "connectedAt": connected_at,
    })
}

fn extraction_to_response(
    model: crate::entities::chatbot_receipt_extraction::Model,
) -> ReceiptExtractionResponse {
    let data = &model.extracted_data;

    let amount = data
        .get("amount")
        .and_then(|v| {
            v.as_f64().map_or_else(
                || {
                    v.as_str()
                        .and_then(|s| rust_decimal::Decimal::from_str_exact(s).ok())
                },
                |f| rust_decimal::Decimal::from_str_exact(&format!("{f:.2}")).ok(),
            )
        })
        .unwrap_or(rust_decimal::Decimal::ZERO);

    ReceiptExtractionResponse {
        id: model.id,
        organizacion_id: model.organizacion_id,
        conversation_id: model.conversation_id,
        inquilino_id: model.inquilino_id,
        contrato_id: model.contrato_id,
        bank: data.get("bank").and_then(|v| v.as_str()).map(String::from),
        amount,
        currency: data
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("DOP")
            .to_string(),
        date: data.get("date").and_then(|v| v.as_str()).map(String::from),
        reference: data
            .get("reference")
            .and_then(|v| v.as_str())
            .map(String::from),
        sender_name: data
            .get("sender_name")
            .and_then(|v| v.as_str())
            .map(String::from),
        recipient: data
            .get("recipient")
            .and_then(|v| v.as_str())
            .map(String::from),
        confidence: data
            .get("confidence")
            .and_then(|v| v.as_str())
            .unwrap_or("low")
            .to_string(),
        status: model.status,
        confirmed_by: model.confirmed_by,
        created_at: model.created_at.into(),
        updated_at: model.updated_at.into(),
    }
}

pub async fn test_chat(
    db: web::Data<DatabaseConnection>,
    app_config: web::Data<AppConfig>,
    claims: WriteAccess,
    body: web::Json<TestChatRequest>,
) -> Result<HttpResponse, AppError> {
    let chatbot_env = &app_config.chatbot;

    let org_id = claims.0.organizacion_id;
    let request = body.into_inner();

    let saved_config = chatbot::get_config_model(db.get_ref(), org_id).await?;

    let (persona, faqs, policies, capabilities, handoff_keywords, _) =
        if let Some(overrides) = &request.config_override {
            let persona = ChatbotPersona {
                tone: overrides.tone.clone().or_else(|| saved_config.tone.clone()),
                greeting: overrides
                    .greeting
                    .clone()
                    .or_else(|| saved_config.greeting.clone()),
                system_prompt: overrides
                    .system_prompt
                    .clone()
                    .or_else(|| saved_config.system_prompt.clone()),
                language: overrides
                    .language
                    .clone()
                    .unwrap_or_else(|| saved_config.language.clone()),
            };

            let faqs: Vec<FaqEntry> = overrides.faqs.clone().unwrap_or_else(|| {
                saved_config
                    .faqs
                    .as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default()
            });

            let policies = overrides
                .policies
                .clone()
                .or_else(|| saved_config.policies.clone());

            let capabilities: Capabilities = overrides.capabilities.clone().unwrap_or_else(|| {
                saved_config
                    .capabilities
                    .as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or(Capabilities {
                        receipt_ocr: false,
                        balance_queries: false,
                        payment_reminders: false,
                        maintenance_requests: false,
                        human_handoff: false,
                    })
            });

            let handoff_keywords: Vec<String> =
                overrides.handoff_keywords.clone().unwrap_or_else(|| {
                    saved_config
                        .handoff_keywords
                        .as_ref()
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default()
                });

            let history_limit = overrides
                .history_limit
                .unwrap_or(saved_config.history_limit) as u64;

            (
                persona,
                faqs,
                policies,
                capabilities,
                handoff_keywords,
                history_limit,
            )
        } else {
            let persona = ChatbotPersona {
                tone: saved_config.tone.clone(),
                greeting: saved_config.greeting.clone(),
                system_prompt: saved_config.system_prompt.clone(),
                language: saved_config.language.clone(),
            };

            let faqs: Vec<FaqEntry> = saved_config
                .faqs
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let policies = saved_config.policies.clone();

            let capabilities: Capabilities = saved_config
                .capabilities
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or(Capabilities {
                    receipt_ocr: false,
                    balance_queries: false,
                    payment_reminders: false,
                    maintenance_requests: false,
                    human_handoff: false,
                });

            let handoff_keywords: Vec<String> = saved_config
                .handoff_keywords
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let history_limit = saved_config.history_limit as u64;

            (
                persona,
                faqs,
                policies,
                capabilities,
                handoff_keywords,
                history_limit,
            )
        };

    let history: Vec<ConversationEntry> = request
        .history
        .iter()
        .map(|h| ConversationEntry {
            role: h.role.clone(),
            content: h.content.clone(),
        })
        .collect();

    let user_message = UserMessage {
        content: request.message,
        image_base64: None,
    };

    let ai_module = AiModule::new(chatbot_env)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error inicializando AI module: {e}")))?;

    let guidance_rules: Vec<crate::models::chatbot::GuidanceRule> =
        serde_json::from_value(saved_config.guidance_rules.clone()).unwrap_or_default();

    let ctx = ProcessMessageContext {
        config: &persona,
        tenant_context: None,
        faqs: &faqs,
        policies: policies.as_deref(),
        handoff_keywords: &handoff_keywords,
        capabilities: &capabilities,
        history: &history,
        user_message: &user_message,
        db: db.get_ref(),
        organizacion_id: org_id,
        sender_phone: "test",
        guidance_rules: &guidance_rules,
        sender_policy: "tenants_only",
    };

    match ai_module.process_message(&ctx).await {
        Ok(response) => Ok(HttpResponse::Ok().json(TestChatResponse {
            reply: response.reply,
            tools_invoked: response.tools_invoked,
        })),
        Err(e) => {
            tracing::error!(
                organizacion_id = %org_id,
                error = %e,
                "Error en test chat AI pipeline"
            );
            Ok(HttpResponse::Ok().json(TestChatResponse {
                reply: format!("Error en el pipeline AI: {e}"),
                tools_invoked: vec![],
            }))
        }
    }
}

pub async fn test_chat_stream(
    db: web::Data<DatabaseConnection>,
    app_config: web::Data<AppConfig>,
    claims: WriteAccess,
    body: web::Json<TestChatRequest>,
) -> Result<HttpResponse, AppError> {
    let chatbot_env = &app_config.chatbot;
    let org_id = claims.0.organizacion_id;
    let request = body.into_inner();

    let saved_config = chatbot::get_config_model(db.get_ref(), org_id).await?;

    let (persona, faqs, policies, handoff_keywords) =
        if let Some(overrides) = &request.config_override {
            let persona = ChatbotPersona {
                tone: overrides.tone.clone().or_else(|| saved_config.tone.clone()),
                greeting: overrides
                    .greeting
                    .clone()
                    .or_else(|| saved_config.greeting.clone()),
                system_prompt: overrides
                    .system_prompt
                    .clone()
                    .or_else(|| saved_config.system_prompt.clone()),
                language: overrides
                    .language
                    .clone()
                    .unwrap_or_else(|| saved_config.language.clone()),
            };

            let faqs: Vec<FaqEntry> = overrides.faqs.clone().unwrap_or_else(|| {
                saved_config
                    .faqs
                    .as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default()
            });

            let policies = overrides
                .policies
                .clone()
                .or_else(|| saved_config.policies.clone());

            let handoff_keywords: Vec<String> =
                overrides.handoff_keywords.clone().unwrap_or_else(|| {
                    saved_config
                        .handoff_keywords
                        .as_ref()
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default()
                });

            (persona, faqs, policies, handoff_keywords)
        } else {
            let persona = ChatbotPersona {
                tone: saved_config.tone.clone(),
                greeting: saved_config.greeting.clone(),
                system_prompt: saved_config.system_prompt.clone(),
                language: saved_config.language.clone(),
            };

            let faqs: Vec<FaqEntry> = saved_config
                .faqs
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let policies = saved_config.policies.clone();

            let handoff_keywords: Vec<String> = saved_config
                .handoff_keywords
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            (persona, faqs, policies, handoff_keywords)
        };

    let guidance_rules: Vec<crate::models::chatbot::GuidanceRule> =
        serde_json::from_value(saved_config.guidance_rules.clone()).unwrap_or_default();

    let system_prompt = compose_system_prompt(
        &persona,
        None,
        &faqs,
        policies.as_deref(),
        &handoff_keywords,
        &guidance_rules,
    );

    let mut chat_history: Vec<rig::completion::Message> = Vec::with_capacity(request.history.len());
    for h in &request.history {
        let msg = match h.role.as_str() {
            "assistant" => rig::completion::Message::assistant(&h.content),
            _ => rig::completion::Message::user(&h.content),
        };
        chat_history.push(msg);
    }

    let user_msg = rig::completion::Message::user(&request.message);
    chat_history.push(user_msg);

    let chat_history_one_or_many = rig::one_or_many::OneOrMany::many(chat_history)
        .map_err(|_| AppError::Validation("Se requiere al menos un mensaje".to_string()))?;

    let completion_request = rig::completion::CompletionRequest {
        model: None,
        preamble: Some(system_prompt),
        chat_history: chat_history_one_or_many,
        documents: vec![],
        tools: vec![],
        temperature: Some(0.7),
        max_tokens: Some(1024),
        tool_choice: None,
        additional_params: None,
        output_schema: None,
    };

    let model = OvmsCompletionModel::new(
        &chatbot_env.vllm_chat_model,
        &chatbot_env.vllm_endpoint,
        chatbot_env.vllm_api_key.clone(),
    );

    crate::metrics::AI_REQUEST_ATTEMPTS.inc();

    let mut streaming_response = rig::completion::CompletionModel::stream(
        &model,
        completion_request,
    )
    .await
    .map_err(|e| {
        let error_msg = e.to_string();
        if error_msg.contains("INFERENCE_COLD_START") {
            tracing::info!(organizacion_id = %org_id, "vLLM en cold-start (stream)");
            AppError::Internal(anyhow::anyhow!(
                "El asistente se está iniciando. Por favor, intente de nuevo en 1-2 minutos."
            ))
        } else {
            tracing::error!(organizacion_id = %org_id, error = %e, "Error en OVMS stream");
            AppError::Internal(anyhow::anyhow!("Error conectando al servicio AI: {e}"))
        }
    })?;

    let sse_stream = async_stream::stream! {
        while let Some(chunk) = streaming_response.next().await {
            match chunk {
                Ok(rig::streaming::StreamedAssistantContent::Text(text)) => {
                    let data = serde_json::json!({
                        "choices": [{
                            "delta": { "content": text.text },
                            "index": 0,
                            "finish_reason": serde_json::Value::Null
                        }]
                    });
                    let event = format!("data: {data}\n\n");
                    yield Ok::<_, actix_web::Error>(actix_web::web::Bytes::from(event));
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!(organizacion_id = %org_id, error = %e, "Error en stream chunk");
                    break;
                }
            }
        }
        let done = "data: [DONE]\n\n".to_string();
        yield Ok::<_, actix_web::Error>(actix_web::web::Bytes::from(done));
    };

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .streaming(sse_stream))
}

pub async fn clear_handoff(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
    body: web::Json<ClearHandoffRequest>,
) -> Result<HttpResponse, AppError> {
    let org_id = claims.0.organizacion_id;
    let role = &claims.0.rol;

    chatbot::clear_handoff(db.get_ref(), org_id, &body.sender_phone, role).await?;

    Ok(HttpResponse::Ok().json(HandoffStatusResponse {
        organizacion_id: org_id,
        sender_phone: body.sender_phone.clone(),
        handoff_status: "none".to_string(),
    }))
}

pub async fn create_guidance_rule_handler(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
    body: web::Json<CreateGuidanceRuleRequest>,
) -> Result<HttpResponse, AppError> {
    let rule = chatbot::create_guidance_rule(
        db.get_ref(),
        claims.0.organizacion_id,
        body.into_inner(),
        claims.0.sub,
    )
    .await?;
    Ok(HttpResponse::Created().json(rule))
}

pub async fn update_guidance_rule_handler(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdateGuidanceRuleRequest>,
) -> Result<HttpResponse, AppError> {
    let rule_id = path.into_inner();
    let rule = chatbot::update_guidance_rule(
        db.get_ref(),
        claims.0.organizacion_id,
        rule_id,
        body.into_inner(),
        claims.0.sub,
    )
    .await?;
    Ok(HttpResponse::Ok().json(rule))
}

pub async fn delete_guidance_rule_handler(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let rule_id = path.into_inner();
    chatbot::delete_guidance_rule(
        db.get_ref(),
        claims.0.organizacion_id,
        rule_id,
        claims.0.sub,
    )
    .await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn batch_update_guidance_rules_handler(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
    body: web::Json<BatchUpdateRequest>,
) -> Result<HttpResponse, AppError> {
    let rules = chatbot::batch_update_guidance_rules(
        db.get_ref(),
        claims.0.organizacion_id,
        body.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(rules))
}
