# Cold-Start Deferred Reply Plan

When vLLM is scaling from zero, detect the cold-start condition in the webhook handler, send an immediate "waking up" message, and retry AI processing in a background task with exponential backoff until vLLM responds or max duration elapses.

## Architecture Decision: Typed Error Variant

**Use a typed error variant.** Add `AppError::ServiceUnavailable(String)` rather than checking reply text.

Rationale:
- The current `process_message` returns `Ok(AgentResponse)` with a canned reply for both cold-start and timeout — the handler cannot distinguish "vLLM down" from "real answer" without string matching on user-facing Spanish text.
- A typed variant lets the handler pattern-match cleanly: `Err(AppError::ServiceUnavailable(_))` → spawn deferred retry.
- The existing test-chat endpoint receives the error and can render it as-is (no background spawn needed there — it's synchronous).
- No change to `AgentResponse` struct fields required.

## Affected Files

| Path | Action | What Changes |
|------|--------|--------------|
| `backend/src/errors.rs` | Modify | Add `ServiceUnavailable(String)` variant, map to 503 |
| `backend/src/services/ai_module/mod.rs` | Modify | Return `Err(AppError::ServiceUnavailable(...))` instead of `Ok(AgentResponse{canned})` for cold-start/timeout |
| `backend/src/handlers/chatbot_internal.rs` | Modify | Detect `ServiceUnavailable`, send immediate reply, spawn background retry task |
| `backend/src/services/ai_module/mod.rs` | Modify | Derive `Clone` on `AiModule` |
| `backend/src/metrics.rs` | Modify | Add `COLD_START_RETRIES` and `COLD_START_OUTCOMES` counters |
| `backend/src/handlers/chatbot.rs` | Verify | `test_chat` already handles `Err(e)` — new variant renders as error message. No code change needed. |

## Steps

### Step 1: Add `ServiceUnavailable` to `AppError`

**File:** `backend/src/errors.rs`

Add a new variant:
```rust
#[error("Servicio no disponible: {0}")]
ServiceUnavailable(String),
```

Map it to `StatusCode::SERVICE_UNAVAILABLE` (503) in `status_code()`.

Add the arm in `error_response()`:
```rust
Self::ServiceUnavailable(msg) => ("service_unavailable", msg.clone()),
```

This gives the handler a clean pattern to match against cold-start failures.

---

### Step 2: Derive `Clone` on `AiModule`

**File:** `backend/src/services/ai_module/mod.rs`

Change:
```rust
pub struct AiModule {
```
To:
```rust
#[derive(Clone)]
pub struct AiModule {
```

Both fields (`OvmsCompletionModel` and `u64`) already implement `Clone`. The background task needs a cloned `AiModule` since `ProcessMessageContext` borrows and cannot be moved across the spawn boundary.

---

### Step 3: Return `ServiceUnavailable` from `process_message` for cold-start conditions

**File:** `backend/src/services/ai_module/mod.rs`

In `process_message`, replace the two paths that currently return `Ok(AgentResponse { canned_reply })`:

**a) INFERENCE_COLD_START error (line ~275):**
```rust
if error_msg.contains("INFERENCE_COLD_START") {
    tracing::info!(
        organizacion_id = %ctx.organizacion_id,
        "vLLM en cold-start, retornando ServiceUnavailable"
    );
    return Err(AppError::ServiceUnavailable(
        "vLLM en cold-start".to_string(),
    ));
}
```

**b) Timeout branch (line ~305):**
```rust
Err(_elapsed) => {
    tracing::warn!(
        organizacion_id = %ctx.organizacion_id,
        timeout_secs = self.timeout_secs,
        "Timeout en solicitud al modelo — probable cold-start"
    );
    Err(AppError::ServiceUnavailable(
        "Timeout conectando a vLLM".to_string(),
    ))
}
```

Also handle connection-refused errors that surface as `reqwest` transport errors. In the `Ok(Err(e))` arm, add before the generic error case:
```rust
if error_msg.contains("Connection refused")
    || error_msg.contains("connection refused")
    || error_msg.contains("ConnectError")
{
    tracing::info!(
        organizacion_id = %ctx.organizacion_id,
        "vLLM connection refused — probable cold-start"
    );
    return Err(AppError::ServiceUnavailable(
        "Connection refused a vLLM".to_string(),
    ));
}
```

---

### Step 4: Add cold-start metrics

**File:** `backend/src/metrics.rs`

Add two new counters:
```rust
pub static COLD_START_RETRIES: LazyLock<IntCounter> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter!(Opts::new(
            "cold_start_retries_total",
            "Reintentos de AI durante cold-start de vLLM"
        )),
        "cold_start_retries_total",
    )
});

pub static COLD_START_OUTCOMES: LazyLock<IntCounterVec> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter_vec!(
            Opts::new(
                "cold_start_outcomes_total",
                "Resultados de reintentos cold-start (success/failure)"
            ),
            &["result"]
        ),
        "cold_start_outcomes_total",
    )
});
```

Register them in `init()`:
```rust
let _ = &*COLD_START_RETRIES;
let _ = &*COLD_START_OUTCOMES;
```

---

### Step 5: Implement deferred retry in `incoming_webhook`

**File:** `backend/src/handlers/chatbot_internal.rs`

This is the core change. In the `ai_result` match block, add handling for `ServiceUnavailable`:

```rust
let reply_text = match ai_result {
    Ok(response) => { /* existing success path — unchanged */ }
    Err(AppError::ServiceUnavailable(reason)) => {
        // 1. Persist user message immediately
        chatbot::persist_message(
            db.get_ref(), org_id, &payload.sender_phone,
            inquilino_id, "user", &payload.content,
            &payload.message_type, None,
        ).await?;

        // 2. Send immediate "waking up" reply
        let wakeup_msg = "Dame un momento, estoy despertando... ☕ Te respondo en unos minutos.";
        if let Some(ref client) = baileys {
            let _ = client.send_message(org_id, &payload.sender_phone, wakeup_msg).await;
        }

        // 3. Clone owned data for the background task
        let ai_module_clone = ai_module.clone();
        let db_clone = db.clone();
        let baileys_clone = baileys.clone();
        let phone = payload.sender_phone.clone();
        let content = payload.content.clone();
        let message_type = payload.message_type.clone();
        let persona_clone = persona.clone();
        let faqs_clone = faqs.clone();
        let policies_clone = cfg.policies.clone();
        let handoff_keywords_clone = handoff_keywords.clone();
        let capabilities_clone = capabilities.clone();
        let history_clone = history.clone();
        let user_message_clone = user_message.clone();
        let guidance_rules_clone = guidance_rules.clone();
        let sender_policy_clone = cfg.sender_policy.clone();
        let tenant_context_clone = tenant_context.clone();

        // 4. Spawn background retry task
        tokio::spawn(async move {
            let delays = [30u64, 60, 90, 120, 150, 180];
            let mut success = false;

            for (attempt, delay_secs) in delays.iter().enumerate() {
                tokio::time::sleep(Duration::from_secs(*delay_secs)).await;
                crate::metrics::COLD_START_RETRIES.inc();

                let ctx = ProcessMessageContext {
                    config: &persona_clone,
                    tenant_context: tenant_context_clone.as_ref(),
                    faqs: &faqs_clone,
                    policies: policies_clone.as_deref(),
                    handoff_keywords: &handoff_keywords_clone,
                    capabilities: &capabilities_clone,
                    history: &history_clone,
                    user_message: &user_message_clone,
                    db: db_clone.get_ref(),
                    organizacion_id: org_id,
                    sender_phone: &phone,
                    guidance_rules: &guidance_rules_clone,
                    sender_policy: &sender_policy_clone,
                };

                match ai_module_clone.process_message(&ctx).await {
                    Ok(response) => {
                        // Persist assistant message
                        let _ = chatbot::persist_message(
                            db_clone.get_ref(), org_id, &phone,
                            inquilino_id, "assistant", &response.reply,
                            "text", None,
                        ).await;

                        // Send the AI reply
                        if let Some(ref client) = baileys_clone {
                            let _ = client.send_message(org_id, &phone, &response.reply).await;
                        }

                        crate::metrics::COLD_START_OUTCOMES
                            .with_label_values(&["success"]).inc();
                        success = true;

                        tracing::info!(
                            organizacion_id = %org_id,
                            attempt = attempt + 1,
                            "Cold-start retry exitoso"
                        );
                        break;
                    }
                    Err(AppError::ServiceUnavailable(_)) => {
                        tracing::debug!(
                            organizacion_id = %org_id,
                            attempt = attempt + 1,
                            delay_secs = delay_secs,
                            "Cold-start retry: vLLM aún no disponible"
                        );
                        continue;
                    }
                    Err(e) => {
                        // Non-cold-start error — stop retrying
                        tracing::error!(
                            organizacion_id = %org_id,
                            error = %e,
                            "Cold-start retry falló con error no recuperable"
                        );
                        break;
                    }
                }
            }

            if !success {
                let failure_msg = "Lo siento, no pude procesar tu mensaje. Intente más tarde.";

                let _ = chatbot::persist_message(
                    db_clone.get_ref(), org_id, &phone,
                    inquilino_id, "assistant", failure_msg,
                    "text", None,
                ).await;

                if let Some(ref client) = baileys_clone {
                    let _ = client.send_message(org_id, &phone, failure_msg).await;
                }

                crate::metrics::COLD_START_OUTCOMES
                    .with_label_values(&["failure"]).inc();
            }
        });

        // Return immediately to the webhook caller
        return Ok(HttpResponse::Ok().json(serde_json::json!({"status": "deferred"})));
    }
    Err(e) => { /* existing generic error path — unchanged */ }
};
```

**Key design points:**
- The delays array `[30, 60, 90, 120, 150, 180]` sums to 630s (~10.5 min total wall time from first attempt). Since the cold-start timeout itself is 30s, first retry fires at T+30s after the initial failure. Total window: ~7 min of retry attempts.
- All data is cloned into owned types. `ProcessMessageContext` is reconstructed each loop iteration with references to the owned clones.
- Receipt extraction handling is included in the success path (same as the normal path).
- The spawned future is fire-and-forget. The webhook returns `{"status": "deferred"}` immediately.

---

### Step 6: Add necessary imports and types

**File:** `backend/src/handlers/chatbot_internal.rs`

Add at the top:
```rust
use std::time::Duration;
use crate::services::ai_module::ProcessMessageContext;
```

Ensure `TenantContext` derives `Clone` if it doesn't already (it's `#[derive(Debug, Clone, Serialize, Deserialize)]` — confirmed from reading the file).

---

### Step 7: Ensure `ChatbotPersona`, `UserMessage`, `ConversationEntry` are Clone

**File:** `backend/src/services/ai_module/mod.rs`

From reading the code:
- `UserMessage`: `#[derive(Debug, Clone)]` ✓
- `ConversationEntry`: `#[derive(Debug, Clone)]` ✓
- `ChatbotPersona`: `#[derive(Debug, Clone)]` ✓
- `TenantContext`: `#[derive(Debug, Clone, Serialize, Deserialize)]` ✓

No changes needed here.

---

### Step 8: Verify test-chat compatibility

**File:** `backend/src/handlers/chatbot.rs` — NO CHANGES

The `test_chat` handler already has:
```rust
Err(e) => {
    Ok(HttpResponse::Ok().json(TestChatResponse {
        reply: format!("Error en el pipeline AI: {e}"),
        tools_invoked: vec![],
    }))
}
```

The new `ServiceUnavailable` variant implements `Display` via `#[error(...)]`, so it renders as `"Servicio no disponible: vLLM en cold-start"`. This is acceptable for test-chat — the developer sees the error. No background spawn needed because test-chat is interactive.

---

## Risks & Edge Cases

| Risk | Mitigation |
|------|-----------|
| User sends multiple messages during cold-start → multiple deferred tasks spawned | Each task is independent. They may reply out of order. Acceptable for MVP. Future: add a per-phone debounce/queue. |
| vLLM comes up but is overloaded — first retry succeeds with degraded quality | No mitigation needed. The retry uses the same timeout and model. |
| Background task panics → no reply sent | `tokio::spawn` catches panics. The user already got the "waking up" message. Worst case: no follow-up. Metrics will show the task never completed. |
| Connection pool exhaustion if many deferred tasks retry simultaneously | Retries are staggered by phone/arrival time. At most ~6 DB operations per deferred task. Acceptable for expected load. |
| `inquilino_id` closure capture is `Option<Uuid>` (Copy) | No issue — `Uuid` is Copy. |
| The wakeup message is not persisted in conversation history | It should be. Add a `persist_message` call for the assistant wakeup message right after sending it. |

**Correction to Step 5:** After sending the wakeup message, also persist it:
```rust
let _ = chatbot::persist_message(
    db.get_ref(), org_id, &payload.sender_phone,
    inquilino_id, "assistant", wakeup_msg,
    "text", None,
).await;
```

---

## Verification

1. `cargo build` — confirm no compile errors from new variant and Clone derive.
2. `cargo test` — all existing tests pass (errors.rs tests, chatbot_internal tests, test_chat handler).
3. Manual test: stop vLLM pod (`kubectl scale deployment vllm --replicas=0`), send WhatsApp message, verify:
   - Immediate "Dame un momento..." reply arrives.
   - After `kubectl scale deployment vllm --replicas=1` and warmup, follow-up AI reply arrives.
4. Manual test: send message when vLLM is already running — normal flow unchanged.
5. Check Prometheus: `cold_start_retries_total` and `cold_start_outcomes_total{result="success"}` increment correctly.
6. Verify `/api/v1/chatbot/test-chat` still works — returns error string for cold-start instead of hanging.
