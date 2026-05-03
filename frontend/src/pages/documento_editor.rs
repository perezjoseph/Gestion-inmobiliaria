use serde_json::Value;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::components::common::document_editor::DocumentEditor;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::loading::Loading;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_get, api_put};
use crate::types::documento::{DocumentoResponse, PlantillaResponse};

// ── Props ──────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq, Eq)]
pub struct DocumentoEditorPageProps {
    pub entity_type: String,
    pub entity_id: String,
    #[prop_or_default]
    pub documento_id: Option<String>,
}

// ── Page states ────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Eq)]
enum PageMode {
    Loading,
    TemplateSelector,
    Editor,
}

// ── Save request body ──────────────────────────────────────────────────

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GuardarEditorRequest {
    contenido_editable: Value,
}

// ── Main page component ────────────────────────────────────────────────

#[allow(clippy::redundant_clone)]
#[component]
pub fn DocumentoEditorPage(props: &DocumentoEditorPageProps) -> Html {
    let mode = use_state(|| PageMode::Loading);
    let contenido = use_state(|| Option::<Value>::None);
    let documento = use_state(|| Option::<DocumentoResponse>::None);
    let plantillas = use_state(Vec::<PlantillaResponse>::new);
    let error = use_state(|| Option::<String>::None);
    let _saving = use_state(|| false);
    let toasts = use_context::<ToastContext>();

    let entity_type = props.entity_type.clone();
    let entity_id = props.entity_id.clone();
    let documento_id = props.documento_id.clone();

    // On mount: fetch document or load templates
    {
        let mode = mode.clone();
        let contenido = contenido.clone();
        let documento = documento.clone();
        let plantillas = plantillas.clone();
        let error = error.clone();
        let etype = entity_type.clone();
        let eid = entity_id.clone();
        let did = documento_id.clone();

        use_effect_with(
            (etype, eid, did),
            move |(entity_type, entity_id, documento_id)| {
                let entity_type = entity_type.clone();
                let entity_id = entity_id.clone();
                let documento_id = documento_id.clone();
                spawn_local(async move {
                    if let Some(ref doc_id) = documento_id {
                        match fetch_document(&entity_type, &entity_id, doc_id).await {
                            Ok(doc) => {
                                contenido.set(doc.contenido_editable.clone());
                                documento.set(Some(doc));
                                mode.set(PageMode::Editor);
                            }
                            Err(err) => {
                                error.set(Some(err));
                                mode.set(PageMode::Editor);
                            }
                        }
                    } else {
                        match fetch_plantillas(&entity_type).await {
                            Ok(tpls) => {
                                plantillas.set(tpls);
                                mode.set(PageMode::TemplateSelector);
                            }
                            Err(err) => {
                                error.set(Some(err));
                                mode.set(PageMode::TemplateSelector);
                            }
                        }
                    }
                });
            },
        );
    }

    // Save handler
    let on_save = {
        let documento = documento.clone();
        let error = error.clone();
        let toasts = toasts.clone();
        let contenido = contenido.clone();
        Callback::from(move |content: Value| {
            let doc = (*documento).clone();
            let Some(doc) = doc else { return };
            let error = error.clone();
            let toasts = toasts.clone();
            let contenido = contenido.clone();
            let doc_id = doc.id.clone();
            let body = GuardarEditorRequest {
                contenido_editable: content.clone(),
            };
            spawn_local(async move {
                match api_put::<DocumentoResponse, _>(
                    &format!("/documentos/{doc_id}/contenido"),
                    &body,
                )
                .await
                {
                    Ok(_) => {
                        contenido.set(Some(content));
                        if let Some(ref t) = toasts {
                            t.dispatch(ToastAction::Push(
                                "Documento guardado".into(),
                                ToastKind::Success,
                            ));
                        }
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        })
    };

    // Template selection handler
    let on_select_template = {
        let contenido = contenido.clone();
        let mode = mode.clone();
        let error = error.clone();
        let et = entity_type.clone();
        let ei = entity_id.clone();
        Callback::from(move |plantilla_id: String| {
            let contenido = contenido.clone();
            let mode = mode.clone();
            let error = error.clone();
            let et = et.clone();
            let ei = ei.clone();
            spawn_local(async move {
                match fetch_plantilla_rellena(&plantilla_id, &et, &ei).await {
                    Ok(resp) => {
                        contenido.set(Some(resp.contenido));
                        mode.set(PageMode::Editor);
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        })
    };

    // Blank document handler
    let on_blank = {
        let contenido = contenido.clone();
        let mode = mode.clone();
        Callback::from(move |_: MouseEvent| {
            contenido.set(Some(Value::Array(Vec::new())));
            mode.set(PageMode::Editor);
        })
    };

    let tipo_documento = documento.as_ref().and_then(|d| d.tipo_documento.clone());
    let doc_id_attr = documento.as_ref().map(|d| AttrValue::from(d.id.clone()));

    html! {
        <div>
            <EditorPageHeader />
            if let Some(ref err) = *error {
                <ErrorBanner message={err.clone()} />
            }
            {match &*mode {
                PageMode::Loading => html! { <Loading /> },
                PageMode::TemplateSelector => html! {
                    <TemplateSelectorGrid
                        plantillas={(*plantillas).clone()}
                        on_select={on_select_template}
                        on_blank={on_blank}
                    />
                },
                PageMode::Editor => html! {
                    <DocumentEditor
                        contenido={(*contenido).clone()}
                        readonly={false}
                        entity_type={AttrValue::from(entity_type.clone())}
                        entity_id={AttrValue::from(entity_id.clone())}
                        tipo_documento={tipo_documento.map(AttrValue::from)}
                        documento_id={doc_id_attr}
                        on_save={on_save}
                    />
                },
            }}
        </div>
    }
}

// ── Header sub-component ───────────────────────────────────────────────

#[derive(Properties, PartialEq, Eq)]
struct EditorPageHeaderProps {}

#[component]
fn EditorPageHeader(_props: &EditorPageHeaderProps) -> Html {
    let navigator = use_navigator();
    let on_back = {
        let navigator = navigator;
        Callback::from(move |_: MouseEvent| {
            if let Some(ref nav) = navigator {
                nav.back();
            }
        })
    };

    html! {
        <div style="display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-4);">
            <button class="gi-btn gi-btn-ghost" onclick={on_back}>
                {"← Volver"}
            </button>
            <h1 class="text-display" style="font-size: var(--text-lg); font-weight: 600; color: var(--text-primary);">
                {"Editor de Documento"}
            </h1>
        </div>
    }
}

// ── Template selector grid ─────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct TemplateSelectorGridProps {
    plantillas: Vec<PlantillaResponse>,
    on_select: Callback<String>,
    on_blank: Callback<MouseEvent>,
}

#[component]
fn TemplateSelectorGrid(props: &TemplateSelectorGridProps) -> Html {
    html! {
        <div>
            <h2 class="text-display" style="font-size: var(--text-base); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">
                {"Seleccionar Plantilla"}
            </h2>
            <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: var(--space-4);">
                { for props.plantillas.iter().map(|p| {
                    let on_select = props.on_select.clone();
                    let id = p.id.clone();
                    html! {
                        <TemplateCard
                            nombre={p.nombre.clone()}
                            tipo_documento={p.tipo_documento.clone()}
                            on_click={Callback::from(move |_: MouseEvent| on_select.emit(id.clone()))}
                        />
                    }
                }) }
            </div>
            <div style="margin-top: var(--space-5); text-align: center;">
                <button class="gi-btn gi-btn-ghost" onclick={props.on_blank.clone()}>
                    {"Crear Documento en Blanco"}
                </button>
            </div>
        </div>
    }
}

// ── Template card sub-component ────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct TemplateCardProps {
    nombre: String,
    tipo_documento: String,
    on_click: Callback<MouseEvent>,
}

fn template_icon(tipo: &str) -> &'static str {
    match tipo {
        "contrato_arrendamiento" | "addendum" => "📋",
        "recibo_pago" => "🧾",
        "acta_notarial" => "📝",
        "carta_referencia" => "✉️",
        _ => "📄",
    }
}

fn template_description(tipo: &str) -> &'static str {
    match tipo {
        "contrato_arrendamiento" => "Contrato estándar de alquiler con cláusulas DR",
        "recibo_pago" => "Recibo para pagos de alquiler",
        "acta_notarial" => "Acta de entrega o devolución de propiedad",
        "carta_referencia" => "Carta de referencia para inquilino",
        "addendum" => "Modificación a contrato existente",
        _ => "Plantilla de documento",
    }
}

#[component]
fn TemplateCard(props: &TemplateCardProps) -> Html {
    let icon = template_icon(&props.tipo_documento);
    let desc = template_description(&props.tipo_documento);

    html! {
        <div
            class="gi-template-card"
            onclick={props.on_click.clone()}
            tabindex="0"
            role="button"
            aria-label={format!("Plantilla: {}", props.nombre)}
            onkeydown={
                Callback::from(move |e: KeyboardEvent| {
                    if e.key() == "Enter" || e.key() == " " {
                        e.prevent_default();
                        if let Some(target) = e.target() {
                            if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                                el.click();
                            }
                        }
                    }
                })
            }
        >
            <div class="gi-template-card-icon">{icon}</div>
            <div class="gi-template-card-name">{&props.nombre}</div>
            <div class="gi-template-card-desc">{desc}</div>
        </div>
    }
}

// ── API helpers ─────────────────────────────────────────────────────────

#[allow(clippy::future_not_send)]
async fn fetch_document(
    entity_type: &str,
    entity_id: &str,
    documento_id: &str,
) -> Result<DocumentoResponse, String> {
    let docs: Vec<DocumentoResponse> =
        api_get(&format!("/documentos/{entity_type}/{entity_id}")).await?;
    docs.into_iter()
        .find(|d| d.id == documento_id)
        .ok_or_else(|| "Documento no encontrado.".to_string())
}

#[allow(clippy::future_not_send)]
async fn fetch_plantillas(entity_type: &str) -> Result<Vec<PlantillaResponse>, String> {
    api_get(&format!("/documentos/plantillas?entity_type={entity_type}")).await
}

#[allow(clippy::future_not_send)]
async fn fetch_plantilla_rellena(
    plantilla_id: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<PlantillaResponse, String> {
    api_get(&format!(
        "/documentos/plantillas/{plantilla_id}/rellenar/{entity_type}/{entity_id}"
    ))
    .await
}
