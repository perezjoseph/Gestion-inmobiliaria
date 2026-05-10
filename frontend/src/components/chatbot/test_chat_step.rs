use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Headers, Request, RequestInit, Response};
use yew::prelude::*;

use crate::components::common::error_banner::ErrorBanner;
use crate::services::api::get_token;
use crate::services::chatbot::test_chat_stream_url;
use crate::types::chatbot::TestChatRequest;

#[derive(Clone, PartialEq)]
struct ChatMessage {
    role: String,
    content: String,
}

#[component]
pub fn TestChatStep() -> Html {
    let messages = use_state(Vec::<ChatMessage>::new);
    let input = use_state(String::new);
    let loading = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let chat_ref = use_node_ref();

    // Scroll to bottom when messages change
    {
        let chat_ref = chat_ref.clone();
        let msg_len = messages.len();
        use_effect_with(msg_len, move |_| {
            if let Some(el) = chat_ref.cast::<web_sys::HtmlElement>() {
                el.set_scroll_top(el.scroll_height());
            }
        });
    }

    let on_input = {
        let input = input.clone();
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlInputElement = e.target_unchecked_into();
            input.set(el.value());
        })
    };

    let on_reset = {
        let messages = messages.clone();
        let input = input.clone();
        let error = error.clone();
        Callback::from(move |_: MouseEvent| {
            messages.set(Vec::new());
            input.set(String::new());
            error.set(None);
        })
    };

    let on_send = {
        let messages = messages.clone();
        let input = input.clone();
        let loading = loading.clone();
        let error = error.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let msg = (*input).trim().to_string();
            if msg.is_empty() || *loading {
                return;
            }

            let messages = messages.clone();
            let input = input.clone();
            let loading = loading.clone();
            let error = error.clone();

            let mut new_msgs = (*messages).clone();
            new_msgs.push(ChatMessage {
                role: "user".to_string(),
                content: msg.clone(),
            });
            new_msgs.push(ChatMessage {
                role: "assistant".to_string(),
                content: String::new(),
            });
            messages.set(new_msgs);
            input.set(String::new());
            loading.set(true);

            spawn_local(async move {
                match stream_chat_response(&msg).await {
                    Ok(receiver) => {
                        consume_stream(receiver, messages.clone(), error.clone()).await;
                    }
                    Err(e) => {
                        let mut updated = (*messages).clone();
                        if let Some(last) = updated.last() {
                            if last.role == "assistant" && last.content.is_empty() {
                                updated.pop();
                            }
                        }
                        messages.set(updated);
                        error.set(Some(e));
                    }
                }
                loading.set(false);
            });
        })
    };

    let has_messages = !messages.is_empty();

    html! {
        <section class="flex flex-col" style="height: 100%; min-height: 420px;">
            <header class="flex items-center justify-between mb-3">
                <div>
                    <h3 class="text-sm font-semibold text-[var(--text-primary)]">
                        {"Probar conversación"}
                    </h3>
                    <p class="text-xs text-[var(--text-tertiary)] mt-0.5">
                        {"Simule un mensaje de inquilino con la configuración actual."}
                    </p>
                </div>
                if has_messages {
                    <button
                        type="button"
                        class="gi-btn gi-btn-secondary text-xs"
                        onclick={on_reset}
                        aria-label="Reiniciar conversación"
                    >
                        {"Reiniciar"}
                    </button>
                }
            </header>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <div
                ref={chat_ref}
                class="flex flex-col gap-2 mb-3 overflow-y-auto rounded-lg flex-1"
                style="border: 1px solid var(--border-default); background: var(--surface-raised); padding: var(--space-3); min-height: 260px;"
            >
                if messages.is_empty() {
                    <div class="flex flex-col items-center justify-center gap-2 m-auto text-center" style="max-width: 260px;">
                        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="color: var(--text-tertiary);">
                            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
                        </svg>
                        <p class="text-sm text-[var(--text-secondary)]">
                            {"Escriba un mensaje de ejemplo"}
                        </p>
                        <p class="text-xs text-[var(--text-tertiary)]">
                            {"Ej: ¿Cuánto debo este mes? o ¿Cuándo vence mi pago?"}
                        </p>
                    </div>
                }
                {for (*messages).iter().map(|msg| {
                    let is_user = msg.role == "user";
                    html! {
                        <ChatBubble is_user={is_user} content={AttrValue::from(msg.content.clone())} />
                    }
                })}
                if *loading {
                    <div
                        class="self-start rounded-lg text-sm inline-flex items-center gap-1"
                        style="padding: var(--space-2) var(--space-3); background: var(--surface-base); border: 1px solid var(--border-subtle); color: var(--text-tertiary);"
                    >
                        <TypingDot delay={0} />
                        <TypingDot delay={160} />
                        <TypingDot delay={320} />
                    </div>
                }
            </div>

            <form onsubmit={on_send} class="flex gap-2">
                <input
                    type="text"
                    class="gi-input flex-1"
                    placeholder="Escriba como si fuera un inquilino"
                    value={(*input).clone()}
                    oninput={on_input}
                    aria-label="Mensaje de prueba"
                />
                <button
                    type="submit"
                    class="gi-btn gi-btn-primary"
                    disabled={*loading}
                >
                    {"Enviar"}
                </button>
            </form>
        </section>
    }
}

// ---------------------------------------------------------------------------
// Typing indicator dot
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct TypingDotProps {
    delay: u32,
}

#[component]
fn TypingDot(props: &TypingDotProps) -> Html {
    html! {
        <span
            class="inline-block rounded-full"
            style={format!(
                "width: 6px; height: 6px; background: var(--text-tertiary); animation: gi-pulse 1.4s {}ms infinite ease-in-out;",
                props.delay,
            )}
            aria-hidden="true"
        />
    }
}

// ---------------------------------------------------------------------------
// ChatBubble — renders thinking blocks as collapsible "Pensando" sections
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct ChatBubbleProps {
    is_user: bool,
    content: AttrValue,
}

#[component]
fn ChatBubble(props: &ChatBubbleProps) -> Html {
    let (align, style) = if props.is_user {
        (
            "self-end",
            "padding: var(--space-2) var(--space-3); background: var(--color-primary-500); color: var(--text-on-primary); max-width: 80%;",
        )
    } else {
        (
            "self-start",
            "padding: var(--space-2) var(--space-3); background: var(--surface-base); color: var(--text-primary); max-width: 80%; border: 1px solid var(--border-subtle);",
        )
    };

    // For user messages or messages without <think>, render plain
    if props.is_user || !props.content.contains("<think>") {
        return html! {
            <div class={classes!(align, "rounded-lg", "text-sm", "leading-relaxed")} style={style}>
                {&props.content}
            </div>
        };
    }

    // Parse thinking and visible content
    let (thinking, visible) = split_think_content(&props.content);

    html! {
        <div class={classes!(align, "rounded-lg", "text-sm", "leading-relaxed")} style={style}>
            if !thinking.is_empty() {
                <ThinkingDisclosure content={AttrValue::from(thinking)} />
            }
            if !visible.is_empty() {
                <span>{visible}</span>
            }
        </div>
    }
}

// ---------------------------------------------------------------------------
// ThinkingDisclosure — collapsible "Pensando" section
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct ThinkingDisclosureProps {
    content: AttrValue,
}

#[component]
fn ThinkingDisclosure(props: &ThinkingDisclosureProps) -> Html {
    let expanded = use_state(|| false);

    let on_toggle = {
        let expanded = expanded.clone();
        Callback::from(move |_: MouseEvent| {
            expanded.set(!*expanded);
        })
    };

    html! {
        <div class="mb-2" style="border-bottom: 1px solid var(--border-subtle); padding-bottom: var(--space-2);">
            <button
                type="button"
                class="flex items-center gap-1 bg-transparent border-none cursor-pointer p-0"
                style="color: var(--text-tertiary); font-size: var(--text-xs);"
                onclick={on_toggle}
                aria-expanded={(*expanded).to_string()}
            >
                <svg
                    width="12" height="12" viewBox="0 0 24 24" fill="none"
                    stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                    style={if *expanded { "transform: rotate(90deg); transition: transform 150ms;" } else { "transition: transform 150ms;" }}
                >
                    <polyline points="9 18 15 12 9 6"/>
                </svg>
                {"Pensando"}
            </button>
            if *expanded {
                <div
                    class="mt-1 text-xs leading-relaxed"
                    style="color: var(--text-tertiary); white-space: pre-wrap; max-height: 200px; overflow-y: auto;"
                >
                    {&props.content}
                </div>
            }
        </div>
    }
}

/// Splits content into (`thinking_text`, `visible_text`).
/// Handles both complete `<think>...</think>` and unclosed `<think>...` (still streaming).
fn split_think_content(text: &str) -> (String, String) {
    let mut thinking = String::new();
    let mut visible = String::new();
    let mut remaining: &str = text;

    loop {
        if let Some(start) = remaining.find("<think>") {
            visible.push_str(&remaining[..start]);
            remaining = &remaining[start + 7..];

            if let Some(end) = remaining.find("</think>") {
                thinking.push_str(&remaining[..end]);
                remaining = &remaining[end + 8..];
            } else {
                // Unclosed — still streaming thinking
                thinking.push_str(remaining);
                break;
            }
        } else {
            visible.push_str(remaining);
            break;
        }
    }

    (thinking.trim().to_string(), visible.trim().to_string())
}

// ---------------------------------------------------------------------------
// Streaming plumbing
// ---------------------------------------------------------------------------

/// Initiates the streaming fetch request and returns the Response object.
#[allow(clippy::future_not_send)]
async fn stream_chat_response(message: &str) -> Result<Response, String> {
    let url = test_chat_stream_url();

    let request_body = TestChatRequest {
        message: message.to_string(),
        config_override: None,
    };
    let body_json =
        serde_json::to_string(&request_body).map_err(|e| format!("Error serializando: {e}"))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&JsValue::from_str(&body_json));

    let headers = Headers::new().map_err(|_| "Error creando headers".to_string())?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|_| "Error configurando headers".to_string())?;

    if let Some(token) = get_token() {
        headers
            .set("Authorization", &format!("Bearer {token}"))
            .map_err(|_| "Error configurando auth".to_string())?;
    }
    opts.set_headers(&headers);

    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|_| "Error creando request".to_string())?;

    let window = web_sys::window().ok_or("No window disponible")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| "Error de red al conectar con el servidor".to_string())?;

    let response: Response = resp_value
        .dyn_into()
        .map_err(|_| "Respuesta inválida".to_string())?;

    if !response.ok() {
        let status = response.status();
        if status == 401 {
            return Err("Sesión expirada. Redirigiendo al inicio de sesión.".to_string());
        }
        return Err(format!("Error del servidor (código {status})"));
    }

    Ok(response)
}

/// Consumes the SSE stream from the response.
#[allow(clippy::future_not_send)]
async fn consume_stream(
    response: Response,
    messages: UseStateHandle<Vec<ChatMessage>>,
    error: UseStateHandle<Option<String>>,
) {
    let Some(body) = response.body() else {
        error.set(Some("Respuesta sin cuerpo".to_string()));
        return;
    };

    let reader: web_sys::ReadableStreamDefaultReader = body.get_reader().unchecked_into();

    let Ok(decoder) =
        web_sys::TextDecoder::new().or_else(|_| web_sys::TextDecoder::new_with_label("utf-8"))
    else {
        error.set(Some("Error creando TextDecoder".to_string()));
        return;
    };

    let mut buffer = String::new();

    loop {
        let Ok(result) = JsFuture::from(reader.read()).await else {
            error.set(Some("Error leyendo stream".to_string()));
            break;
        };

        let done = js_sys::Reflect::get(&result, &JsValue::from_str("done"))
            .unwrap_or(JsValue::TRUE)
            .as_bool()
            .unwrap_or(true);

        if done {
            break;
        }

        let Ok(value) = js_sys::Reflect::get(&result, &JsValue::from_str("value")) else {
            break;
        };

        let chunk_text = if let Some(uint8) = value.dyn_ref::<js_sys::Uint8Array>() {
            decoder.decode_with_buffer_source(uint8).unwrap_or_default()
        } else {
            continue;
        };

        buffer.push_str(&chunk_text);

        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim().to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() == "[DONE]" {
                    return;
                }

                if let Some(token) = parse_sse_token(data) {
                    let mut updated = (*messages).clone();
                    if let Some(last) = updated.last_mut() {
                        if last.role == "assistant" {
                            last.content.push_str(&token);
                        }
                    }
                    messages.set(updated);
                }
            }
        }
    }
}

/// Parses a single SSE data payload and extracts the content delta token.
fn parse_sse_token(data: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(data).ok()?;
    parsed
        .get("choices")?
        .get(0)?
        .get("delta")?
        .get("content")?
        .as_str()
        .map(String::from)
}
