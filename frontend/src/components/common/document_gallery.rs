use gloo_net::http::Request;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::services::api::BASE_URL;
use crate::types::documento::DocumentoResponse;

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
const ALLOWED_TYPES: &[&str] = &["image/jpeg", "image/png", "application/pdf"];

#[derive(Properties, PartialEq, Eq)]
pub struct DocumentGalleryProps {
    pub entity_type: String,
    pub entity_id: String,
    pub token: String,
}

#[component]
pub fn DocumentGallery(props: &DocumentGalleryProps) -> Html {
    let documents = use_state(Vec::<DocumentoResponse>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let uploading = use_state(|| false);

    {
        let documents = documents.clone();
        let loading = loading.clone();
        let error = error.clone();
        let entity_type = props.entity_type.clone();
        let entity_id = props.entity_id.clone();
        let token = props.token.clone();
        use_effect_with((entity_type.clone(), entity_id.clone()), move |_| {
            spawn_local(async move {
                loading.set(true);
                error.set(None);
                match fetch_documents(&entity_type, &entity_id, &token).await {
                    Ok(docs) => documents.set(docs),
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        });
    }

    let on_upload = {
        let documents = documents.clone();
        let error = error.clone();
        let uploading = uploading.clone();
        let entity_type = props.entity_type.clone();
        let entity_id = props.entity_id.clone();
        let token = props.token.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let Some(files) = input.files() else { return };
            let Some(file) = files.get(0) else { return };

            let size = file.size() as u64;
            if size > MAX_FILE_SIZE {
                error.set(Some(format!(
                    "El archivo excede el tamaño máximo de {} MB.",
                    MAX_FILE_SIZE / (1024 * 1024)
                )));
                input.set_value("");
                return;
            }

            let file_type = file.type_();
            if !ALLOWED_TYPES.contains(&file_type.as_str()) {
                error.set(Some(
                    "Tipo de archivo no permitido. Solo se aceptan JPEG, PNG y PDF.".to_string(),
                ));
                input.set_value("");
                return;
            }

            let documents = documents.clone();
            let error = error.clone();
            let uploading = uploading.clone();
            let entity_type = entity_type.clone();
            let entity_id = entity_id.clone();
            let token = token.clone();

            uploading.set(true);
            error.set(None);

            spawn_local(async move {
                match upload_document(&entity_type, &entity_id, &token, file).await {
                    Ok(()) => match fetch_documents(&entity_type, &entity_id, &token).await {
                        Ok(docs) => documents.set(docs),
                        Err(err) => error.set(Some(err)),
                    },
                    Err(err) => error.set(Some(err)),
                }
                uploading.set(false);
            });

            input.set_value("");
        })
    };

    html! {
        <div class="flex flex-col gap-3">
            <div class="flex items-center justify-between">
                <h4 style="font-weight: 600; font-size: var(--text-sm); color: var(--text-primary);">
                    {"Documentos"}
                </h4>
                <label
                    class="gi-btn gi-btn-sm gi-btn-secondary"
                    style={if *uploading { "opacity: 0.5; pointer-events: none;" } else { "" }}
                >
                    { if *uploading { "Subiendo..." } else { "📎 Subir archivo" } }
                    <input
                        type="file"
                        accept="image/jpeg,image/png,application/pdf"
                        onchange={on_upload}
                        style="display: none;"
                    />
                </label>
            </div>

            if let Some(err) = (*error).as_ref() {
                <div class="gi-error-banner" role="alert">
                    <span style="font-size: var(--text-sm);">{err}</span>
                </div>
            }

            if *loading {
                <div style="text-align: center; padding: var(--space-4); color: var(--text-tertiary); font-size: var(--text-sm);">
                    {"Cargando documentos..."}
                </div>
            } else if documents.is_empty() {
                <div style="text-align: center; padding: var(--space-4); color: var(--text-tertiary); font-size: var(--text-sm);">
                    {"No hay documentos adjuntos."}
                </div>
            } else {
                <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));">
                    { for documents.iter().map(render_document) }
                </div>
            }
        </div>
    }
}

fn render_document(doc: &DocumentoResponse) -> Html {
    let is_image = doc.mime_type.starts_with("image/");
    let file_url = format!("{}/{}", BASE_URL.trim_end_matches("/api"), doc.file_path);
    let size_label = format_file_size(doc.file_size);

    if is_image {
        html! {
            <div class="flex flex-col gap-1" style="border: 1px solid var(--border-subtle); border-radius: 8px; overflow: hidden;">
                <a href={file_url.clone()} target="_blank" rel="noopener noreferrer">
                    <img
                        src={file_url}
                        alt={doc.filename.clone()}
                        style="width: 100%; height: 100px; object-fit: cover;"
                        loading="lazy"
                    />
                </a>
                <div style="padding: var(--space-1) var(--space-2);">
                    <div style="font-size: 0.75rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                         title={doc.filename.clone()}>
                        {&doc.filename}
                    </div>
                    <div style="font-size: 0.7rem; color: var(--text-tertiary);">{size_label}</div>
                </div>
            </div>
        }
    } else {
        html! {
            <div class="flex flex-col gap-1" style="border: 1px solid var(--border-subtle); border-radius: 8px; overflow: hidden;">
                <div class="flex items-center justify-center"
                     style="height: 100px; background: var(--surface-raised); font-size: 2rem;">
                    {"📄"}
                </div>
                <div style="padding: var(--space-1) var(--space-2);">
                    <a href={file_url} target="_blank" rel="noopener noreferrer"
                       style="font-size: 0.75rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; display: block;"
                       title={doc.filename.clone()}>
                        {&doc.filename}
                    </a>
                    <div style="font-size: 0.7rem; color: var(--text-tertiary);">{size_label}</div>
                </div>
            </div>
        }
    }
}

fn format_file_size(bytes: i64) -> String {
    let abs = bytes.unsigned_abs();
    if abs < 1024 {
        format!("{abs} B")
    } else if abs < 1024 * 1024 {
        format!("{:.1} KB", abs as f64 / 1024.0)
    } else {
        format!("{:.1} MB", abs as f64 / (1024.0 * 1024.0))
    }
}

#[allow(clippy::future_not_send)]
async fn fetch_documents(
    entity_type: &str,
    entity_id: &str,
    token: &str,
) -> Result<Vec<DocumentoResponse>, String> {
    let url = format!("{BASE_URL}/documentos/{entity_type}/{entity_id}");
    let response = Request::get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;

    if !response.ok() {
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Error desconocido".into());
        return Err(text);
    }

    response
        .json::<Vec<DocumentoResponse>>()
        .await
        .map_err(|e| format!("Error al procesar respuesta: {e}"))
}

#[allow(clippy::future_not_send)]
async fn upload_document(
    entity_type: &str,
    entity_id: &str,
    token: &str,
    file: web_sys::File,
) -> Result<(), String> {
    let form_data =
        web_sys::FormData::new().map_err(|_| "Error al crear formulario".to_string())?;
    form_data
        .append_with_blob("file", &file)
        .map_err(|_| "Error al adjuntar archivo".to_string())?;

    let url = format!("{BASE_URL}/documentos/{entity_type}/{entity_id}");
    let response = Request::post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .body(form_data)
        .map_err(|e| format!("Error al preparar solicitud: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;

    if !response.ok() {
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Error al subir archivo".into());
        return Err(text);
    }

    Ok(())
}
