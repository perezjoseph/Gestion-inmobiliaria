use gloo_net::http::Request;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

use crate::services::api::BASE_URL;
use crate::types::documento::DocumentoResponse;

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
const ALLOWED_TYPES: &[&str] = &["image/jpeg", "image/png", "application/pdf"];

// ── Document type catalog per entity_type ──────────────────────────────

pub const TIPOS_INQUILINO: &[(&str, &str)] = &[
    ("cedula", "Cédula"),
    ("comprobante_ingresos", "Comprobante de ingresos"),
    ("carta_referencia", "Carta de referencia"),
    ("contrato_trabajo", "Contrato de trabajo"),
    ("carta_no_antecedentes", "Carta de no antecedentes"),
];

pub const TIPOS_PROPIEDAD: &[(&str, &str)] = &[
    ("titulo_propiedad", "Título de propiedad"),
    ("certificacion_no_gravamen", "Certificación de no gravamen"),
    ("plano_catastral", "Plano catastral"),
    ("certificacion_uso_suelo", "Certificación de uso de suelo"),
    ("poliza_seguro", "Póliza de seguro"),
];

pub const TIPOS_CONTRATO: &[(&str, &str)] = &[
    ("contrato_arrendamiento", "Contrato de arrendamiento"),
    ("acta_notarial", "Acta notarial"),
    ("registro_dgii", "Registro DGII"),
    ("addendum", "Addendum"),
];

pub const TIPOS_PAGO: &[(&str, &str)] = &[
    ("recibo_pago", "Recibo de pago"),
    ("comprobante_fiscal_ncf", "Comprobante fiscal (NCF)"),
    ("comprobante_transferencia", "Comprobante de transferencia"),
];

pub const TIPOS_GASTO: &[(&str, &str)] = &[
    ("factura_proveedor", "Factura de proveedor"),
    ("comprobante_fiscal_ncf", "Comprobante fiscal (NCF)"),
    ("recibo_pago", "Recibo de pago"),
];

fn tipos_for_entity(entity_type: &str) -> &'static [(&'static str, &'static str)] {
    match entity_type {
        "inquilino" => TIPOS_INQUILINO,
        "propiedad" => TIPOS_PROPIEDAD,
        "contrato" => TIPOS_CONTRATO,
        "pago" => TIPOS_PAGO,
        "gasto" => TIPOS_GASTO,
        _ => &[],
    }
}

fn estado_label(estado: &str) -> &'static str {
    match estado {
        "verificado" => "Verificado",
        "pendiente" => "Pendiente",
        "rechazado" => "Rechazado",
        "vencido" => "Vencido",
        _ => "Desconocido",
    }
}

fn estado_badge_class(estado: &str) -> &'static str {
    match estado {
        "verificado" => "gi-badge gi-badge-verificado",
        "pendiente" => "gi-badge gi-badge-pendiente",
        "rechazado" => "gi-badge gi-badge-rechazado",
        "vencido" => "gi-badge gi-badge-vencido",
        _ => "gi-badge gi-badge-neutral",
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

fn tipo_doc_label(tipo: &str, entity_type: &str) -> String {
    let tipos = tipos_for_entity(entity_type);
    tipos
        .iter()
        .find(|(val, _)| *val == tipo)
        .map_or_else(|| tipo.replace('_', " "), |(_, label)| (*label).to_string())
}

// ── Props ──────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq, Eq)]
pub struct DocumentGalleryProps {
    pub entity_type: String,
    pub entity_id: String,
    pub token: String,
}

// ── Main component ─────────────────────────────────────────────────────

#[component]
pub fn DocumentGallery(props: &DocumentGalleryProps) -> Html {
    let documents = use_state(Vec::<DocumentoResponse>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let uploading = use_state(|| false);
    let filter_tipo = use_state(String::new);
    let filter_estado = use_state(String::new);
    let selected_tipo_doc = use_state(String::new);
    let numero_documento = use_state(String::new);
    let digitalizing = use_state(|| false);

    // Fetch documents on mount / entity change
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

    // Filter documents client-side
    let filtered_docs: Vec<DocumentoResponse> = documents
        .iter()
        .filter(|d| {
            let tipo_ok = filter_tipo.is_empty()
                || d.tipo_documento
                    .as_deref()
                    .is_some_and(|t| t == filter_tipo.as_str());
            let estado_ok = filter_estado.is_empty()
                || d.estado_verificacion
                    .as_deref()
                    .is_some_and(|e| e == filter_estado.as_str());
            tipo_ok && estado_ok
        })
        .cloned()
        .collect();

    let filtered_docs = Rc::new(filtered_docs);

    // Upload handler
    let on_upload = {
        let documents = documents;
        let error = error.clone();
        let uploading = uploading.clone();
        let entity_type = props.entity_type.clone();
        let entity_id = props.entity_id.clone();
        let token = props.token.clone();
        let selected_tipo_doc = selected_tipo_doc.clone();
        let numero_documento = numero_documento.clone();
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

            if selected_tipo_doc.is_empty() {
                error.set(Some(
                    "Seleccione un tipo de documento antes de subir.".to_string(),
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
            let tipo_doc = (*selected_tipo_doc).clone();
            let num_doc = (*numero_documento).clone();

            uploading.set(true);
            error.set(None);

            spawn_local(async move {
                match upload_document(&entity_type, &entity_id, &token, file, &tipo_doc, &num_doc)
                    .await
                {
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

    // Digitalizar handler
    let on_digitalizar = {
        let error = error.clone();
        let digitalizing = digitalizing.clone();
        let entity_type = props.entity_type.clone();
        let entity_id = props.entity_id.clone();
        let token = props.token.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let Some(files) = input.files() else { return };
            let Some(file) = files.get(0) else { return };

            let error = error.clone();
            let digitalizing = digitalizing.clone();
            let entity_type = entity_type.clone();
            let entity_id = entity_id.clone();
            let token = token.clone();

            digitalizing.set(true);
            error.set(None);

            spawn_local(async move {
                match digitalizar_document(&entity_type, &entity_id, &token, file).await {
                    Ok(resp) => {
                        // Navigate to editor with the digitalized content
                        if let Some(win) = web_sys::window() {
                            let url = format!(
                                "/documentos/editor/{}/{}/{}",
                                entity_type, entity_id, resp.documento_original_id
                            );
                            let _ = win.location().set_href(&url);
                        }
                    }
                    Err(err) => error.set(Some(err)),
                }
                digitalizing.set(false);
            });

            input.set_value("");
        })
    };

    let entity_type_rc: AttrValue = props.entity_type.clone().into();
    let entity_id_rc: AttrValue = props.entity_id.clone().into();

    html! {
        <div class="flex flex-col gap-3">
            <GalleryHeader
                uploading={*uploading}
                digitalizing={*digitalizing}
                on_upload={on_upload}
                on_digitalizar={on_digitalizar}
            />

            <UploadForm
                entity_type={entity_type_rc.clone()}
                selected_tipo_doc={(*selected_tipo_doc).clone()}
                numero_documento={(*numero_documento).clone()}
                on_tipo_change={Callback::from({
                    let selected_tipo_doc = selected_tipo_doc.clone();
                    move |val: String| selected_tipo_doc.set(val)
                })}
                on_numero_change={Callback::from({
                    let numero_documento = numero_documento.clone();
                    move |val: String| numero_documento.set(val)
                })}
            />

            if let Some(err) = (*error).as_ref() {
                <div class="gi-error-banner" role="alert">
                    <span style="font-size: var(--text-sm);">{err}</span>
                </div>
            }

            <FilterBar
                entity_type={entity_type_rc.clone()}
                filter_tipo={(*filter_tipo).clone()}
                filter_estado={(*filter_estado).clone()}
                on_tipo_change={Callback::from({
                    let filter_tipo = filter_tipo.clone();
                    move |val: String| filter_tipo.set(val)
                })}
                on_estado_change={Callback::from({
                    let filter_estado = filter_estado.clone();
                    move |val: String| filter_estado.set(val)
                })}
            />

            if *loading {
                <div style="text-align: center; padding: var(--space-4); color: var(--text-tertiary); font-size: var(--text-sm);">
                    {"Cargando documentos..."}
                </div>
            } else if filtered_docs.is_empty() {
                <div style="text-align: center; padding: var(--space-4); color: var(--text-tertiary); font-size: var(--text-sm);">
                    {"No hay documentos adjuntos."}
                </div>
            } else {
                <DocumentGrid
                    documents={filtered_docs}
                    entity_type={entity_type_rc}
                    entity_id={entity_id_rc}
                />
            }
        </div>
    }
}

// ── Sub-component: GalleryHeader ───────────────────────────────────────

#[derive(Properties, PartialEq)]
struct GalleryHeaderProps {
    uploading: bool,
    digitalizing: bool,
    on_upload: Callback<Event>,
    on_digitalizar: Callback<Event>,
}

#[component]
fn GalleryHeader(props: &GalleryHeaderProps) -> Html {
    html! {
        <div class="flex items-center justify-between">
            <h4 style="font-weight: 600; font-size: var(--text-sm); color: var(--text-primary);">
                {"Documentos Legales"}
            </h4>
            <div class="flex items-center gap-2">
                <label
                    class="gi-btn gi-btn-sm gi-btn-secondary"
                    style={if props.digitalizing { "opacity: 0.5; pointer-events: none;" } else { "" }}
                >
                    { if props.digitalizing { "Digitalizando..." } else { "📷 Digitalizar" } }
                    <input
                        type="file"
                        accept="image/jpeg,image/png,application/pdf"
                        onchange={props.on_digitalizar.clone()}
                        style="display: none;"
                    />
                </label>
                <label
                    class="gi-btn gi-btn-sm gi-btn-secondary"
                    style={if props.uploading { "opacity: 0.5; pointer-events: none;" } else { "" }}
                >
                    { if props.uploading { "Subiendo..." } else { "📎 Subir archivo" } }
                    <input
                        type="file"
                        accept="image/jpeg,image/png,application/pdf"
                        onchange={props.on_upload.clone()}
                        style="display: none;"
                    />
                </label>
            </div>
        </div>
    }
}

// ── Sub-component: UploadForm ──────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct UploadFormProps {
    entity_type: AttrValue,
    selected_tipo_doc: String,
    numero_documento: String,
    on_tipo_change: Callback<String>,
    on_numero_change: Callback<String>,
}

#[component]
fn UploadForm(props: &UploadFormProps) -> Html {
    let tipos = tipos_for_entity(&props.entity_type);
    let show_numero = props.selected_tipo_doc == "comprobante_fiscal_ncf";

    let on_tipo_select = {
        let cb = props.on_tipo_change.clone();
        Callback::from(move |e: Event| {
            let el: HtmlSelectElement = e.target_unchecked_into();
            cb.emit(el.value());
        })
    };

    let on_numero_input = {
        let cb = props.on_numero_change.clone();
        Callback::from(move |e: Event| {
            let el: HtmlInputElement = e.target_unchecked_into();
            cb.emit(el.value());
        })
    };

    html! {
        <div class="flex items-end gap-2 flex-wrap" style="font-size: var(--text-sm);">
            <div>
                <label class="gi-label" style="font-size: var(--text-xs);">{"Tipo de documento"}</label>
                <select onchange={on_tipo_select} class="gi-input" style="font-size: var(--text-xs); padding: var(--space-1) var(--space-2);">
                    <option value="" selected={props.selected_tipo_doc.is_empty()}>{"— Seleccionar —"}</option>
                    { for tipos.iter().map(|(val, label)| {
                        html! {
                            <option value={*val} selected={props.selected_tipo_doc == *val}>{label}</option>
                        }
                    })}
                </select>
            </div>
            if show_numero {
                <div>
                    <label class="gi-label" style="font-size: var(--text-xs);">{"Número NCF"}</label>
                    <input
                        type="text"
                        class="gi-input"
                        style="font-size: var(--text-xs); padding: var(--space-1) var(--space-2);"
                        placeholder="Ej: B0100000001"
                        value={props.numero_documento.clone()}
                        onchange={on_numero_input}
                    />
                </div>
            }
        </div>
    }
}

// ── Sub-component: FilterBar ───────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct FilterBarProps {
    entity_type: AttrValue,
    filter_tipo: String,
    filter_estado: String,
    on_tipo_change: Callback<String>,
    on_estado_change: Callback<String>,
}

#[component]
fn FilterBar(props: &FilterBarProps) -> Html {
    let tipos = tipos_for_entity(&props.entity_type);

    let on_tipo = {
        let cb = props.on_tipo_change.clone();
        Callback::from(move |e: Event| {
            let el: HtmlSelectElement = e.target_unchecked_into();
            cb.emit(el.value());
        })
    };

    let on_estado = {
        let cb = props.on_estado_change.clone();
        Callback::from(move |e: Event| {
            let el: HtmlSelectElement = e.target_unchecked_into();
            cb.emit(el.value());
        })
    };

    html! {
        <div class="flex items-end gap-2 flex-wrap" style="font-size: var(--text-xs);">
            <div>
                <label class="gi-label" style="font-size: var(--text-xs);">{"Tipo"}</label>
                <select onchange={on_tipo} class="gi-input" style="font-size: var(--text-xs); padding: var(--space-1) var(--space-2);">
                    <option value="" selected={props.filter_tipo.is_empty()}>{"Todos"}</option>
                    { for tipos.iter().map(|(val, label)| {
                        html! {
                            <option value={*val} selected={props.filter_tipo == *val}>{label}</option>
                        }
                    })}
                </select>
            </div>
            <div>
                <label class="gi-label" style="font-size: var(--text-xs);">{"Estado"}</label>
                <select onchange={on_estado} class="gi-input" style="font-size: var(--text-xs); padding: var(--space-1) var(--space-2);">
                    <option value="" selected={props.filter_estado.is_empty()}>{"Todos"}</option>
                    <option value="verificado" selected={props.filter_estado == "verificado"}>{"Verificado"}</option>
                    <option value="pendiente" selected={props.filter_estado == "pendiente"}>{"Pendiente"}</option>
                    <option value="rechazado" selected={props.filter_estado == "rechazado"}>{"Rechazado"}</option>
                    <option value="vencido" selected={props.filter_estado == "vencido"}>{"Vencido"}</option>
                </select>
            </div>
        </div>
    }
}

// ── Sub-component: DocumentGrid ────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct DocumentGridProps {
    documents: Rc<Vec<DocumentoResponse>>,
    entity_type: AttrValue,
    entity_id: AttrValue,
}

#[component]
fn DocumentGrid(props: &DocumentGridProps) -> Html {
    html! {
        <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));">
            { for props.documents.iter().map(|doc| {
                html! {
                    <DocumentCard
                        doc={doc.clone()}
                        entity_type={props.entity_type.clone()}
                        entity_id={props.entity_id.clone()}
                    />
                }
            })}
        </div>
    }
}

// ── Sub-component: DocumentCard ────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct DocumentCardProps {
    doc: DocumentoResponse,
    entity_type: AttrValue,
    entity_id: AttrValue,
}

#[component]
fn DocumentCard(props: &DocumentCardProps) -> Html {
    let doc = &props.doc;
    let is_image = doc.mime_type.starts_with("image/");
    let file_url = format!("{}/{}", BASE_URL.trim_end_matches("/api"), doc.file_path);
    let size_label = format_file_size(doc.file_size);
    let estado = doc.estado_verificacion.as_deref().unwrap_or("pendiente");
    let tipo = doc.tipo_documento.as_deref().unwrap_or("otro");
    let tipo_label = tipo_doc_label(tipo, &props.entity_type);
    let editor_url = format!(
        "/documentos/editor/{}/{}/{}",
        props.entity_type, props.entity_id, doc.id
    );

    html! {
        <div class="gi-doc-card">
            <div class="gi-doc-card-preview">
                if is_image {
                    <a href={file_url.clone()} target="_blank" rel="noopener noreferrer">
                        <img
                            src={file_url}
                            alt={doc.filename.clone()}
                            loading="lazy"
                        />
                    </a>
                } else {
                    <a href={file_url} target="_blank" rel="noopener noreferrer"
                       style="font-size: 2rem; text-decoration: none;">
                        {"📄"}
                    </a>
                }
            </div>
            <div class="gi-doc-card-body">
                <div class="gi-doc-card-name" title={doc.filename.clone()}>
                    {&doc.filename}
                </div>
                <div style="font-size: var(--text-xs); color: var(--text-secondary);">
                    {tipo_label}
                </div>
                <div class="gi-doc-card-meta">
                    <span>{size_label}</span>
                    <span class={estado_badge_class(estado)} aria-label={estado_label(estado).to_string()}>
                        {estado_label(estado)}
                    </span>
                </div>
            </div>
            <div class="gi-doc-card-actions">
                <a href={editor_url} class="gi-btn gi-btn-sm gi-btn-secondary" style="font-size: var(--text-xs);">
                    {"✏️ Editar"}
                </a>
            </div>
        </div>
    }
}

// ── API functions ──────────────────────────────────────────────────────

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
    tipo_documento: &str,
    numero_documento: &str,
) -> Result<(), String> {
    let form_data =
        web_sys::FormData::new().map_err(|_| "Error al crear formulario".to_string())?;
    form_data
        .append_with_blob("file", &file)
        .map_err(|_| "Error al adjuntar archivo".to_string())?;
    form_data
        .append_with_str("tipo_documento", tipo_documento)
        .map_err(|_| "Error al agregar tipo de documento".to_string())?;
    if !numero_documento.is_empty() {
        form_data
            .append_with_str("numero_documento", numero_documento)
            .map_err(|_| "Error al agregar número de documento".to_string())?;
    }

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

#[allow(clippy::future_not_send)]
async fn digitalizar_document(
    entity_type: &str,
    entity_id: &str,
    token: &str,
    file: web_sys::File,
) -> Result<crate::types::documento::DigitalizarResponse, String> {
    let form_data =
        web_sys::FormData::new().map_err(|_| "Error al crear formulario".to_string())?;
    form_data
        .append_with_blob("file", &file)
        .map_err(|_| "Error al adjuntar archivo".to_string())?;

    let url = format!("{BASE_URL}/documentos/digitalizar/{entity_type}/{entity_id}");
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
            .unwrap_or_else(|_| "Error al digitalizar documento".into());
        return Err(text);
    }

    response
        .json::<crate::types::documento::DigitalizarResponse>()
        .await
        .map_err(|e| format!("Error al procesar respuesta: {e}"))
}
