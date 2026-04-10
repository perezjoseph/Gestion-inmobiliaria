use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::error_banner::ErrorBanner;
use crate::services::chatbot;
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

            let mut new_msgs = (*messages).clone();
            new_msgs.push(ChatMessage {
                role: "user".to_string(),
                content: msg.clone(),
            });
            messages.set(new_msgs);
            input.set(String::new());
            loading.set(true);

            spawn_local(async move {
                let request = TestChatRequest {
                    message: msg,
                    config_override: None,
                };
                match chatbot::test_chat(&request).await {
                    Ok(resp) => {
                        let mut updated = (*messages).clone();
                        updated.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: resp.reply,
                        });
                        messages.set(updated);
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e)),
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
