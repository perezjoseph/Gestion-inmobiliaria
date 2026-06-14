use actix_web::{HttpRequest, HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::chatbot::{Capabilities, FaqEntry, IncomingWebhookPayload};
use crate::services::ai_module::{
    AiModule, ChatbotPersona, ConversationEntry, ProcessMessageContext, TenantContext, UserMessage,
};
use crate::services::baileys_client::BaileysClient;
use crate::services::chatbot;
use crate::services::crypto::constant_time_eq;

fn hash_phone(phone: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    phone.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[allow(clippy::future_not_send)]
pub async fn incoming_webhook(
    req: HttpRequest,
    config: web::Data<AppConfig>,
    db: web::Data<DatabaseConnection>,
    baileys: Option<web::Data<BaileysClient>>,
    payload: web::Json<IncomingWebhookPayload>,
) -> Result<HttpResponse, AppError> {
    let chatbot_env = &config.chatbot;

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

    let payload = payload.into_inner();
    let org_id = payload.realm_id;

    let cfg = chatbot::get_config_model(db.get_ref(), org_id).await?;

    if !cfg.activo {
        tracing::debug!(organizacion_id = %org_id, "Chatbot inactivo, descartando mensaje");
        return Ok(HttpResponse::Ok().json(serde_json::json!({"status": "discarded"})));
    }

    let is_self_message = payload.session_phone.as_deref() == Some(&payload.sender_phone);
    if is_self_message {
        tracing::debug!("Self-message detected, bypassing sender policy");
    }

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

    let tenant = chatbot::find_tenant_by_phone(db.get_ref(), &payload.sender_phone, org_id).await?;

    let tenant_context = tenant.as_ref().map(|t| TenantContext {
        name: format!("{} {}", t.nombre, t.apellido),
    });
    let inquilino_id = tenant.as_ref().map(|t| t.id);

    let handoff_active =
        chatbot::is_handoff_active(db.get_ref(), org_id, &payload.sender_phone).await?;

    if handoff_active {
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

    let history_limit = cfg.history_limit as u64;
    let history_records =
        chatbot::load_history(db.get_ref(), org_id, &payload.sender_phone, history_limit).await?;

    let history: Vec<ConversationEntry> = history_records
        .iter()
        .rev()
        .map(|m| ConversationEntry {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

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

    let guidance_rules: Vec<crate::models::chatbot::GuidanceRule> =
        serde_json::from_value(cfg.guidance_rules.clone()).unwrap_or_default();

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
        guidance_rules: &guidance_rules,
        sender_policy: &cfg.sender_policy,
    };

    let ai_result = ai_module.process_message(&ctx).await;

    let reply_text = match ai_result {
        Ok(response) => {
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

            "Lo siento, no pude procesar tu mensaje en este momento. \
             Por favor, intenta de nuevo más tarde."
                .to_string()
        }
    };

    if let Some(ref client) = baileys {
        if let Err(e) = client
            .send_message(org_id, &payload.sender_phone, &reply_text)
            .await
        {
            tracing::error!(
                organizacion_id = %org_id,
                error = %e,
                "Error enviando respuesta via Baileys"
            );
        }
    } else {
        tracing::error!(
            organizacion_id = %org_id,
            "BaileysClient no disponible — no se pudo enviar respuesta"
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
