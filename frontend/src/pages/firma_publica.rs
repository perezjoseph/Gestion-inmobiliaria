use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::signature_canvas::SignatureCanvas;
use crate::services::api::BASE_URL;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentoFirmaResponse {
    #[allow(dead_code)]
    documento_id: String,
    contenido: serde_json::Value,
    firmante_nombre: String,
    #[allow(dead_code)]
    estado: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerificarRequest {
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmarConTokenRequest {
    password: String,
    firma_imagen: String,
}

#[derive(Clone, PartialEq, Eq)]
enum PageState {
    PasswordEntry,
    DocumentReview {
        contenido: serde_json::Value,
        firmante_nombre: String,
    },
    Confirmation,
    Expired,
}

#[derive(Properties, PartialEq, Eq)]
pub struct FirmaPublicaProps {
    pub token: String,
}

#[component]
pub fn FirmaPublica(props: &FirmaPublicaProps) -> Html {
    let state = use_state(|| PageState::PasswordEntry);
    let password = use_state(String::new);
    let error_msg = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);
    let token = props.token.clone();

    let on_verify = {
        let state = state.clone();
        let password = password.clone();
        let error_msg = error_msg.clone();
        let loading = loading.clone();
        let token = token.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let pwd = (*password).clone();
            if pwd.is_empty() {
                return;
            }
            let state = state.clone();
            let error_msg = error_msg.clone();
            let loading = loading.clone();
            let token = token.clone();
            loading.set(true);
            error_msg.set(None);
            spawn_local(async move {
                let url = format!("{BASE_URL}/firmas/{token}/verificar");
                let body = VerificarRequest { password: pwd };
                let json = serde_json::to_string(&body).unwrap_or_default();
                let Ok(req_builder) = Request::post(&url)
                    .header("Content-Type", "application/json")
                    .body(json)
                else {
                    error_msg.set(Some("Error al construir la solicitud.".into()));
                    loading.set(false);
                    return;
                };
                let result = req_builder.send().await;
                loading.set(false);
                match result {
                    Ok(resp) => match resp.status() {
                        200 => {
                            if let Ok(data) = resp.json::<DocumentoFirmaResponse>().await {
                                state.set(PageState::DocumentReview {
                                    contenido: data.contenido,
                                    firmante_nombre: data.firmante_nombre,
                                });
                            } else {
                                error_msg.set(Some(
                                    "Error al procesar la respuesta del servidor.".into(),
                                ));
                            }
                        }
                        401 => {
                            error_msg.set(Some("Contraseña incorrecta".into()));
                        }
                        410 => {
                            state.set(PageState::Expired);
                        }
                        _ => {
                            error_msg.set(Some("Error inesperado. Intente de nuevo.".into()));
                        }
                    },
                    Err(_) => {
                        error_msg.set(Some("Error de red. Verifique su conexión.".into()));
                    }
                }
            });
        })
    };

    let on_sign = {
        let state = state.clone();
        let password = password.clone();
        let error_msg = error_msg.clone();
        let loading = loading.clone();
        Callback::from(move |firma_imagen: String| {
            let pwd = (*password).clone();
            let state = state.clone();
            let error_msg = error_msg.clone();
            let loading = loading.clone();
            let token = token.clone();
            loading.set(true);
            error_msg.set(None);
            spawn_local(async move {
                let url = format!("{BASE_URL}/firmas/{token}/firmar");
                let body = FirmarConTokenRequest {
                    password: pwd,
                    firma_imagen,
                };
                let json = serde_json::to_string(&body).unwrap_or_default();
                let Ok(req_builder) = Request::post(&url)
                    .header("Content-Type", "application/json")
                    .body(json)
                else {
                    error_msg.set(Some("Error al construir la solicitud.".into()));
                    loading.set(false);
                    return;
                };
                let result = req_builder.send().await;
                loading.set(false);
                match result {
                    Ok(resp) => match resp.status() {
                        200 => {
                            state.set(PageState::Confirmation);
                        }
                        410 => {
                            state.set(PageState::Expired);
                        }
                        401 => {
                            error_msg.set(Some("Contraseña incorrecta".into()));
                        }
                        409 => {
                            error_msg.set(Some("Esta firma ya fue procesada.".into()));
                        }
                        _ => {
                            error_msg.set(Some("Error inesperado. Intente de nuevo.".into()));
                        }
                    },
                    Err(_) => {
                        error_msg.set(Some("Error de red. Verifique su conexión.".into()));
                    }
                }
            });
        })
    };

    let on_password_input = {
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            password.set(input.value());
        })
    };

    match &*state {
        PageState::Expired => render_expired(),
        PageState::Confirmation => render_confirmation(),
        PageState::PasswordEntry => render_password_form(
            &on_verify,
            &on_password_input,
            (*error_msg).as_ref(),
            *loading,
        ),
        PageState::DocumentReview {
            contenido,
            firmante_nombre,
        } => render_document_review(
            contenido,
            firmante_nombre,
            &on_sign,
            (*error_msg).as_ref(),
            *loading,
        ),
    }
}

fn render_expired() -> Html {
    html! {
        <div class="gi-login-page">
            <div class="gi-login-container">
                <div class="gi-card gi-p-6" style="text-align: center;">
                    <div style="font-size: 3rem; margin-bottom: var(--space-4);">{"⚠️"}</div>
                    <h1 class="text-display" style="margin-bottom: var(--space-3);">
                        {"Enlace expirado"}
                    </h1>
                    <p style="color: var(--text-secondary);">
                        {"Este enlace de firma ha expirado. Contacte al administrador de la propiedad."}
                    </p>
                </div>
            </div>
        </div>
    }
}

fn render_confirmation() -> Html {
    html! {
        <div class="gi-login-page">
            <div class="gi-login-container">
                <div class="gi-card gi-p-6" style="text-align: center;">
                    <div style="font-size: 3rem; margin-bottom: var(--space-4);">{"✅"}</div>
                    <h1 class="text-display" style="margin-bottom: var(--space-3);">
                        {"Documento firmado exitosamente"}
                    </h1>
                    <p style="color: var(--text-secondary);">
                        {"Su firma ha sido registrada. Puede cerrar esta ventana."}
                    </p>
                </div>
            </div>
        </div>
    }
}

fn render_password_form(
    on_submit: &Callback<SubmitEvent>,
    on_input: &Callback<InputEvent>,
    error_msg: Option<&String>,
    loading: bool,
) -> Html {
    html! {
        <div class="gi-login-page">
            <div class="gi-login-container">
                <div class="gi-card gi-p-6">
                    <div class="gi-login-header">
                        <div class="gi-logo-mark gi-mb-3" style="margin-inline: auto;">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M3 21V9l9-7 9 7v12"/>
                                <path d="M9 21V12h6v9"/>
                            </svg>
                        </div>
                        <h1 class="text-display gi-login-title">
                            {"Firma de Documento"}
                        </h1>
                        <p class="gi-login-subtitle">
                            {"Ingrese la contraseña proporcionada para acceder al documento."}
                        </p>
                    </div>
                    if let Some(err) = error_msg {
                        <div class="gi-error-banner gi-mb-3" role="alert">
                            {err}
                        </div>
                    }
                    <form onsubmit={on_submit.clone()}>
                        <div class="gi-form-group">
                            <label for="firma-password" class="gi-label">{"Contraseña"}</label>
                            <input
                                id="firma-password"
                                type="password"
                                class="gi-input"
                                placeholder="Ingrese la contraseña"
                                oninput={on_input.clone()}
                                disabled={loading}
                                autocomplete="off"
                                required=true
                            />
                        </div>
                        <button
                            type="submit"
                            class="gi-btn gi-btn-primary"
                            style="width: 100%; margin-top: var(--space-3);"
                            disabled={loading}
                        >
                            if loading {
                                {"Verificando..."}
                            } else {
                                {"Acceder al documento"}
                            }
                        </button>
                    </form>
                </div>
            </div>
        </div>
    }
}

fn render_document_review(
    contenido: &serde_json::Value,
    firmante_nombre: &str,
    on_sign: &Callback<String>,
    error_msg: Option<&String>,
    loading: bool,
) -> Html {
    let blocks_html = render_blocks(contenido);
    let on_cancel = Callback::from(|()| {});

    html! {
        <div class="gi-login-page" style="padding: var(--space-5);">
            <div style="max-width: 800px; margin: 0 auto;">
                <div class="gi-card gi-p-6" style="margin-bottom: var(--space-4);">
                    <h1 class="text-display" style="margin-bottom: var(--space-2);">
                        {"Revisión de Documento"}
                    </h1>
                    <p style="color: var(--text-secondary); margin-bottom: var(--space-4);">
                        {format!("Firmante: {firmante_nombre}")}
                    </p>
                    <div class="gi-document-content" style="border: 1px solid var(--border-primary); border-radius: var(--radius-md); padding: var(--space-4); background: var(--surface-raised); max-height: 400px; overflow-y: auto;">
                        {blocks_html}
                    </div>
                </div>
                <div class="gi-card gi-p-6">
                    <h2 style="margin-bottom: var(--space-3);">{"Firmar Documento"}</h2>
                    <p style="color: var(--text-secondary); margin-bottom: var(--space-3);">
                        {"Dibuje su firma en el recuadro a continuación."}
                    </p>
                    if let Some(err) = error_msg {
                        <div class="gi-error-banner gi-mb-3" role="alert">
                            {err}
                        </div>
                    }
                    if loading {
                        <div style="text-align: center; padding: var(--space-4);">
                            <div class="gi-spinner" style="margin: 0 auto var(--space-2);"></div>
                            <span style="color: var(--text-tertiary);">{"Enviando firma..."}</span>
                        </div>
                    } else {
                        <SignatureCanvas on_submit={on_sign.clone()} on_cancel={on_cancel} />
                    }
                </div>
            </div>
        </div>
    }
}

fn render_blocks(contenido: &serde_json::Value) -> Html {
    let blocks = contenido
        .get("blocks")
        .and_then(|b| b.as_array())
        .cloned()
        .unwrap_or_default();

    html! {
        <>
            { for blocks.iter().map(render_block) }
        </>
    }
}

fn render_block(block: &serde_json::Value) -> Html {
    let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let text = block
        .get("data")
        .and_then(|d| d.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    match block_type {
        "heading" => {
            let level = block
                .get("data")
                .and_then(|d| d.get("level"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(1);
            match level {
                1 => html! { <h1>{text}</h1> },
                2 => html! { <h2>{text}</h2> },
                _ => html! { <h3>{text}</h3> },
            }
        }
        "paragraph" => html! { <p>{text}</p> },
        "list" => {
            let items = block
                .get("data")
                .and_then(|d| d.get("items"))
                .and_then(|i| i.as_array())
                .cloned()
                .unwrap_or_default();
            let style = block
                .get("data")
                .and_then(|d| d.get("style"))
                .and_then(|s| s.as_str())
                .unwrap_or("unordered");
            let list_items = items.iter().map(|item| {
                let item_text = item.as_str().unwrap_or("");
                html! { <li>{item_text}</li> }
            });
            if style == "ordered" {
                html! { <ol>{ for list_items }</ol> }
            } else {
                html! { <ul>{ for list_items }</ul> }
            }
        }
        "page_break" => html! { <hr style="margin: var(--space-4) 0;" /> },
        _ => html! {},
    }
}
