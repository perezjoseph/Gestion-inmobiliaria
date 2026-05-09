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
            <h3 style="font-size: var(--text-base); font-weight: 600; margin-bottom: var(--space-4);">
                {"Probar Chatbot"}
            </h3>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <div style="border: 1px solid var(--border-default); border-radius: var(--radius-md); height: 300px; overflow-y: auto; padding: var(--space-3); margin-bottom: var(--space-3); display: flex; flex-direction: column; gap: var(--space-2);">
                if messages.is_empty() {
                    <p style="font-size: var(--text-sm); color: var(--text-tertiary); text-align: center; margin: auto;">
                        {"Envíe un mensaje para probar el chatbot"}
                    </p>
                }
                {for (*messages).iter().map(|msg| {
                    let is_user = msg.role == "user";
                    html! {
                        <div style={format!(
                            "max-width: 80%; padding: var(--space-2) var(--space-3); border-radius: var(--radius-md); font-size: var(--text-sm); {}",
                            if is_user {
                                "align-self: flex-end; background: var(--color-primary); color: white;"
                            } else {
                                "align-self: flex-start; background: var(--surface-raised); color: var(--text-primary);"
                            }
                        )}>
                            {msg.content.clone()}
                        </div>
                    }
                })}
                if *loading {
                    <div style="align-self: flex-start; padding: var(--space-2) var(--space-3); border-radius: var(--radius-md); background: var(--surface-raised); font-size: var(--text-sm); color: var(--text-tertiary);">
                        {"Escribiendo..."}
                    </div>
                }
            </div>

            <form onsubmit={on_send} style="display: flex; gap: var(--space-2);">
                <input
                    type="text"
                    class="gi-input"
                    placeholder="Escriba un mensaje de prueba..."
                    value={(*input).clone()}
                    oninput={on_input}
                    style="flex: 1;"
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
