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

            // Add user message immediately
            let mut new_msgs = (*messages).clone();
            new_msgs.push(ChatMessage {
                role: "user".to_string(),
                content: msg.clone(),
            });
            // Add empty assistant message that will be filled by streaming
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
                        // Remove the empty assistant message on connection error
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

    html! {
        <div class="gi-card" style="padding: var(--space-5);">
            <h3 class="text-base font-semibold mb-4">
                {"Probar Chatbot"}
            </h3>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <div
                ref={chat_ref}
                class="flex flex-col gap-2 mb-3 overflow-y-auto"
                style="border: 1px solid var(--border-default); border-radius: var(--radius-md); height: 300px; padding: var(--space-3);"
            >
                if messages.is_empty() {
                    <p class="text-sm text-[var(--text-tertiary)] text-center m-auto">
                        {"Envíe un mensaje para probar el chatbot"}
                    </p>
                }
                {for (*messages).iter().map(|msg| {
                    let is_user = msg.role == "user";
                    html! {
                        <ChatBubble is_user={is_user} content={AttrValue::from(msg.content.clone())} />
                    }
                })}
                if *loading {
                    <div class="self-start rounded-lg text-sm"
                        style="padding: var(--space-2) var(--space-3); background: var(--surface-raised); color: var(--text-tertiary);">
                        {"Escribiendo..."}
                    </div>
                }
            </div>

            <form onsubmit={on_send} class="flex gap-2">
                <input
                    type="text"
                    class="gi-input flex-1"
                    placeholder="Escriba un mensaje de prueba..."
                    value={(*input).clone()}
                    oninput={on_input}
                />
                <button
                    type="submit"
                    class="gi-btn gi-btn-primary"
                    disabled={*loading}
                >
                    {"Enviar"}
                </button>
            </form>
        </div>
    }
}

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

/// Consumes the SSE stream from the response, parsing tokens and appending them
/// to the last assistant message in state.
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

        // Process complete SSE lines from the buffer
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
                    // Append token to the last assistant message
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
/// Expected format: `{"choices":[{"delta":{"content":"token"}}]}`
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

// Sub-component for chat bubbles
#[derive(Properties, PartialEq)]
struct ChatBubbleProps {
    is_user: bool,
    content: AttrValue,
}

#[component]
fn ChatBubble(props: &ChatBubbleProps) -> Html {
    let class = if props.is_user {
        "self-end rounded-lg text-sm max-w-[80%]"
    } else {
        "self-start rounded-lg text-sm max-w-[80%]"
    };

    let style = if props.is_user {
        "padding: var(--space-2) var(--space-3); background: var(--color-primary-500); color: white;"
    } else {
        "padding: var(--space-2) var(--space-3); background: var(--surface-raised); color: var(--text-primary);"
    };

    html! {
        <div class={class} style={style}>
            {&props.content}
        </div>
    }
}
