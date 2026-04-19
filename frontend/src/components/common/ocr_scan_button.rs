use gloo_net::http::Request;
use gloo_timers::callback::Timeout;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::services::api::BASE_URL;
use crate::types::ocr::{OcrExtractField, OcrExtractResponse};

fn get_token() -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()
        .flatten()?
        .get_item("jwt_token")
        .ok()
        .flatten()
}

#[derive(Properties, PartialEq)]
pub struct OcrScanButtonProps {
    pub document_type: AttrValue,
    pub on_result: Callback<Vec<OcrExtractField>>,
    pub label: AttrValue,
    #[prop_or_default]
    pub disabled: bool,
}

async fn perform_ocr_extract(
    file: web_sys::File,
    document_type: String,
) -> Result<OcrExtractResponse, String> {
    let form_data =
        web_sys::FormData::new().map_err(|_| "Error al crear formulario".to_string())?;
    let _ = form_data.append_with_blob("file", &file);
    let _ = form_data.append_with_str("document_type", &document_type);

    let url = format!("{BASE_URL}/ocr/extract");
    let mut builder = Request::post(&url);
    if let Some(token) = get_token() {
        builder = builder.header("Authorization", &format!("Bearer {token}"));
    }
    let resp = builder
        .body(form_data)
        .map_err(|e| format!("Error al preparar solicitud: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;

    if resp.status() == 401 {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(storage)) = win.local_storage() {
                let _ = storage.remove_item("jwt_token");
            }
            let _ = win.location().set_href("/");
        }
        return Err("Sesión expirada".into());
    }

    if !resp.ok() {
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "Error desconocido".into());
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(msg) = parsed.get("message").and_then(|v| v.as_str()) {
                return Err(msg.to_string());
            }
        }
        return Err(match resp.status() {
            503 => "Servicio OCR no disponible".into(),
            422 => "Datos inválidos. Revise el archivo e intente de nuevo.".into(),
            _ => format!("Error inesperado (código {})", resp.status()),
        });
    }

    resp.json::<OcrExtractResponse>()
        .await
        .map_err(|e| format!("Error al procesar respuesta: {e}"))
}

#[function_component]
pub fn OcrScanButton(props: &OcrScanButtonProps) -> Html {
    let loading = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let input_ref = use_node_ref();

    let on_click = {
        let input_ref = input_ref.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(input) = input_ref.cast::<web_sys::HtmlInputElement>() {
                input.click();
            }
        })
    };

    let on_file_change = {
        let loading = loading.clone();
        let error = error.clone();
        let on_result = props.on_result.clone();
        let document_type = props.document_type.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let Some(files) = input.files() else { return };
            let Some(file) = files.get(0) else { return };

            let doc_type = document_type.to_string();
            let loading = loading.clone();
            let error = error.clone();
            let on_result = on_result.clone();

            loading.set(true);
            error.set(None);

            spawn_local(async move {
                match perform_ocr_extract(file, doc_type).await {
                    Ok(response) => on_result.emit(response.fields),
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });

            input.set_value("");
        })
    };

    {
        let error = error.clone();
        use_effect_with((*error).clone(), move |err| {
            let handle = if err.is_some() {
                Some(Timeout::new(5_000, move || {
                    error.set(None);
                }))
            } else {
                None
            };
            move || drop(handle)
        });
    }

    let is_disabled = props.disabled || *loading;
    let button_text = if *loading {
        "Escaneando..."
    } else {
        &props.label
    };

    html! {
        <div style="display: inline-flex; flex-direction: column; gap: var(--space-1);">
            <button
                class="gi-btn gi-btn-secondary gi-btn-sm"
                onclick={on_click}
                disabled={is_disabled}
                style={if *loading { "opacity: 0.6; cursor: wait;" } else { "" }}
            >
                {button_text}
            </button>
            <input
                ref={input_ref}
                type="file"
                accept=".jpg,.jpeg,.png,.pdf"
                onchange={on_file_change}
                style="display: none;"
            />
            if let Some(ref err) = *error {
                <div
                    role="alert"
                    style="font-size: var(--text-xs); color: var(--color-error); margin-top: var(--space-1); max-width: 280px;"
                >
                    {err}
                </div>
            }
        </div>
    }
}
