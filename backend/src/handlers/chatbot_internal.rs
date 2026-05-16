use actix_web::{HttpRequest, HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::chatbot::{Capabilities, FaqEntry, IncomingWebhookPayload, SendMessageRequest};
use crate::services::ai_module::{
    AiModule, ChatbotPersona, ConversationEntry, ProcessMessageContext, TenantContext, UserMessage,
};
use crate::services::chatbot;
use crate::services::crypto::constant_time_eq;

/// Hash a phone number for safe logging (no PII in logs).
fn hash_phone(phone: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    phone.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// POST `/internal/whatsapp/incoming`
///
/// Receives incoming messages from the Baileys sidecar.
/// Validates the `X-Internal-Token` header using constant-time comparison.
/// Then runs the full message processing pipeline:
/// 1. Load `ChatbotConfig` for the org
/// 2. Check `config.activo` — if false, return 200 silently
/// 3. Apply sender policy — if denied, return 200 silently (log for monitoring)
/// 4. Resolve tenant by phone + `org_id`
/// 5. Load conversation history
/// 6. Invoke AI module
/// 7. Persist user message + assistant reply
/// 8. Send reply via `Baileys` Service
/// 9. If AI fails: persist user message, send error reply
#[allow(clippy::future_not_send)]
pub async fn incoming_webhook(
    req: HttpRequest,
    config: web::Data<AppConfig>,
    db: web::Data<DatabaseConnection>,
    payload: web::Json<IncomingWebhookPayload>,
) -> Result<HttpResponse, AppError> {
    let chatbot_env = &config.chatbot;

    // --- Token validation ---
    let token = req
        .headers()
        .get("X-Internal-Token")
        .and_then(|v| v.to_str().ok());

    let Some(provided_token) = token else {
        tracing::warn!("Webhook interno rechazado: header X-Internal-Token ausente");
        return Err(AppError::Unauthorized(None));
    };

    if !constant_time_eq(
        provided_token.as_bytes(),
        chatbot_env.baileys_internal_token.as_bytes(),
    ) {
        tracing::warn!("Webhook interno rechazado: token inválido");
        return Err(AppError::Unauthorized(None));
    }

    // Token valid — proceed with message processing pipeline
    let payload = payload.into_inner();
    let org_id = payload.realm_id;

    // Step 1: Load ChatbotConfig for the org
    let cfg = chatbot::get_config_model(db.get_ref(), org_id).await?;

    // Step 2: Check config.activo — if false, silently discard (Requirement 9.7)
    if !cfg.activo {
        tracing::debug!(organizacion_id = %org_id, "Chatbot inactivo, descartando mensaje");
        return Ok(HttpResponse::Ok().json(serde_json::json!({"status": "discarded"})));
    }

    // Self-message bypass: skip sender policy for messages from the bot's own number
    let is_self_message = payload.session_phone.as_deref() == Some(&payload.sender_phone);
    if is_self_message {
        tracing::debug!("Self-message detected, bypassing sender policy");
    }

    // Step 3: Apply sender policy (Requirement 2.4)
    if !is_self_message {
        let allowlist: Option<Vec<String>> = cfg
            .allowlist
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        let allowlist_slice = allowlist.as_deref();

        let is_allowed = chatbot::is_sender_allowed(
            &cfg.sender_policy,
            &payload.sender_phone,
            allowlist_slice,
            org_id,
            db.get_ref(),
        )
        .await;

        if !is_allowed {
            tracing::info!(
                organizacion_id = %org_id,
                sender_phone_hash = %hash_phone(&payload.sender_phone),
                "Sender no autorizado por política '{}', descartando mensaje",
                cfg.sender_policy
            );
            return Ok(HttpResponse::Ok().json(serde_json::json!({"status": "discarded"})));
        }
    }

    // Step 4: Resolve tenant by phone + org_id
    let tenant = chatbot::find_tenant_by_phone(db.get_ref(), &payload.sender_phone, org_id).await?;

    let tenant_context = tenant.as_ref().map(|t| TenantContext {
        name: format!("{} {}", t.nombre, t.apellido),
    });
    let inquilino_id = tenant.as_ref().map(|t| t.id);

    // Step 4.5: Check handoff status — if active, persist message but skip AI (Requirement 11.4)
    let handoff_active =
        chatbot::is_handoff_active(db.get_ref(), org_id, &payload.sender_phone).await?;

    if handoff_active {
        // Persist the user message for human operator review, but do NOT invoke AI
        chatbot::persist_message(
            db.get_ref(),
            org_id,
            &payload.sender_phone,
            inquilino_id,
            "user",
            &payload.content,
            &payload.message_type,
            None,
        )
        .await?;

        tracing::debug!(
            organizacion_id = %org_id,
            sender_phone_hash = %hash_phone(&payload.sender_phone),
            "Handoff activo, mensaje persistido sin invocar AI"
        );

        return Ok(HttpResponse::Ok().json(serde_json::json!({"status": "handoff_active"})));
    }

    // Step 5: Load conversation history
    let history_limit = cfg.history_limit as u64;
    let history_records =
        chatbot::load_history(db.get_ref(), org_id, &payload.sender_phone, history_limit).await?;

    let history: Vec<ConversationEntry> = history_records
        .iter()
        .rev() // load_history returns DESC, we need chronological order
        .map(|m| ConversationEntry {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    // Step 6: Invoke AI module
    let ai_module = AiModule::new(chatbot_env)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error inicializando AI module: {e}")))?;

    let persona = ChatbotPersona {
        tone: cfg.tone.clone(),
        greeting: cfg.greeting.clone(),
        system_prompt: cfg.system_prompt.clone(),
        language: cfg.language.clone(),
    };

    let faqs: Vec<FaqEntry> = cfg
        .faqs
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let capabilities: Capabilities = cfg
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

    let handoff_keywords: Vec<String> = cfg
        .handoff_keywords
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let user_message = UserMessage {
        content: payload.content.clone(),
        image_base64: if payload.message_type == "image" {
            Some(payload.content.clone())
        } else {
            None
        },
    };

    let ctx = ProcessMessageContext {
        config: &persona,
        tenant_context: tenant_context.as_ref(),
        faqs: &faqs,
        policies: cfg.policies.as_deref(),
        handoff_keywords: &handoff_keywords,
        capabilities: &capabilities,
        history: &history,
        user_message: &user_message,
        db: db.get_ref(),
        organizacion_id: org_id,
        sender_phone: &payload.sender_phone,
    };

    let ai_result = ai_module.process_message(&ctx).await;

    // Step 7 & 8 & 9: Handle AI result
    let reply_text = match ai_result {
        Ok(response) => {
            // Persist user message
            chatbot::persist_message(
                db.get_ref(),
                org_id,
                &payload.sender_phone,
                inquilino_id,
                "user",
                &payload.content,
                &payload.message_type,
                None,
            )
            .await?;

            // Persist assistant reply
            chatbot::persist_message(
                db.get_ref(),
                org_id,
                &payload.sender_phone,
                inquilino_id,
                "assistant",
                &response.reply,
                "text",
                None,
            )
            .await?;

            // Post-loop: if extract_receipt was invoked successfully, persist the extraction
            // (Requirement 8.3)
            if response
                .tools_invoked
                .contains(&"extract_receipt".to_string())
            {
                if let Some(ref receipt) = response.extracted_receipt {
                    if let Err(e) = chatbot::record_extraction_from_agent(
                        db.get_ref(),
                        receipt,
                        org_id,
                        inquilino_id,
                    )
                    .await
                    {
                        tracing::error!(
                            organizacion_id = %org_id,
                            error = %e,
                            "Error persistiendo extracción de recibo del agente"
                        );
                    }
                }
            }

            response.reply
        }
        Err(e) => {
            tracing::error!(
                organizacion_id = %org_id,
                error = %e,
                "Error en AI module, persistiendo mensaje de usuario y enviando error"
            );

            // Persist user message even on AI failure (Requirement 7.6)
            let _ = chatbot::persist_message(
                db.get_ref(),
                org_id,
                &payload.sender_phone,
                inquilino_id,
                "user",
                &payload.content,
                &payload.message_type,
                None,
            )
            .await;

            // Error reply to sender
            "Lo siento, no pude procesar tu mensaje en este momento. \
             Por favor, intenta de nuevo más tarde."
                .to_string()
        }
    };

    // Send reply via Baileys Service
    let send_url = format!(
        "{}/sessions/{}/send",
        chatbot_env.baileys_service_url, org_id
    );

    let send_body = SendMessageRequest {
        recipient_phone: payload.sender_phone.clone(),
        content: reply_text,
        message_type: "text".to_string(),
    };

    let client = reqwest::Client::new();
    let send_result = client.post(&send_url).json(&send_body).send().await;

    if let Err(e) = send_result {
        tracing::error!(
            organizacion_id = %org_id,
            error = %e,
            "Error enviando respuesta via Baileys"
        );
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "processed"})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_equal_values() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(constant_time_eq(b"", b""));
        assert!(constant_time_eq(
            b"a-long-secret-token-value-12345",
            b"a-long-secret-token-value-12345"
        ));
    }

    #[test]
    fn constant_time_eq_different_values() {
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"abc", b"abd"));
    }

    #[test]
    fn constant_time_eq_different_lengths() {
        assert!(!constant_time_eq(b"short", b"longer"));
        assert!(!constant_time_eq(b"a", b""));
    }
}
