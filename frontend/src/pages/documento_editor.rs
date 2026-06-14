use serde_json::Value;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::components::common::document_editor::DocumentEditor;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::loading::Loading;
use crate::components::common::signature_canvas::SignatureCanvas;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_get, api_post, api_put};
use crate::types::documento::{
    DocumentoResponse, FirmaResponse, FirmarRequest, PlantillaResponse, SolicitarFirmaRequest,
    SolicitarFirmaResponse,
};

#[derive(Properties, PartialEq, Eq)]
pub struct DocumentoEditorPageProps {
    pub entity_type: String,
    pub entity_id: String,
    #[prop_or_default]
    pub documento_id: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
enum PageMode {
    Loading,
    TemplateSelector,
    Editor,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GuardarEditorRequest {
    contenido_editable: Value,
}

#[allow(clippy::redundant_clone)]
#[component]
pub fn DocumentoEditorPage(props: &DocumentoEditorPageProps) -> Html {
    let mode = use_state(|| PageMode::Loading);
    let contenido = use_state(|| Option::<Value>::None);
    let documento = use_state(|| Option::<DocumentoResponse>::None);
    let plantillas = use_state(Vec::<PlantillaResponse>::new);
    let firmas = use_state(Vec::<FirmaResponse>::new);
    let error = use_state(|| Option::<String>::None);
    let _saving = use_state(|| false);
    let toasts = use_context::<ToastContext>();
    let show_signature_modal = use_state(|| false);
    let show_solicitar_modal = use_state(|| false);
    let reload_firmas = use_state(|| 0u32);

    let entity_type = props.entity_type.clone();
    let entity_id = props.entity_id.clone();
    let documento_id = props.documento_id.clone();

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

    {
        let firmas = firmas.clone();
        let documento = documento.clone();
        let reload_firmas_val = *reload_firmas;
        use_effect_with(
            (documento.as_ref().map(|d| d.id.clone()), reload_firmas_val),
            move |(doc_id, _)| {
                let doc_id = doc_id.clone();
                let firmas = firmas;
                if let Some(doc_id) = doc_id {
                    spawn_local(async move {
                        if let Ok(result) =
                            api_get::<Vec<FirmaResponse>>(&format!("/documentos/{doc_id}/firmas"))
                                .await
                        {
                            firmas.set(result);
                        }
                    });
                }
            },
        );
    }

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

    let on_blank = {
        let contenido = contenido.clone();
        let mode = mode.clone();
        Callback::from(move |_: MouseEvent| {
            contenido.set(Some(Value::Array(Vec::new())));
            mode.set(PageMode::Editor);
        })
    };

    let on_open_signature = {
        let show_signature_modal = show_signature_modal.clone();
        Callback::from(move |_: MouseEvent| show_signature_modal.set(true))
    };

    let on_cancel_signature = {
        let show_signature_modal = show_signature_modal.clone();
        Callback::from(move |()| show_signature_modal.set(false))
    };

    let on_submit_signature = {
        let documento = documento.clone();
        let show_signature_modal = show_signature_modal.clone();
        let toasts = toasts.clone();
        let error = error.clone();
        let reload_firmas = reload_firmas.clone();
        Callback::from(move |firma_imagen: String| {
            let doc = (*documento).clone();
            let Some(doc) = doc else { return };
            let show_signature_modal = show_signature_modal.clone();
            let toasts = toasts.clone();
            let error = error.clone();
            let reload_firmas = reload_firmas.clone();
            let doc_id = doc.id.clone();
            spawn_local(async move {
                let body = FirmarRequest { firma_imagen };
                match api_post::<FirmaResponse, _>(&format!("/documentos/{doc_id}/firmar"), &body)
                    .await
                {
                    Ok(_) => {
                        show_signature_modal.set(false);
                        reload_firmas.set(*reload_firmas + 1);
                        if let Some(ref t) = toasts {
                            t.dispatch(ToastAction::Push(
                                "Documento firmado exitosamente".into(),
                                ToastKind::Success,
                            ));
                        }
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        })
    };

    let on_open_solicitar = {
        let show_solicitar_modal = show_solicitar_modal.clone();
        Callback::from(move |_: MouseEvent| show_solicitar_modal.set(true))
    };

    let on_cancel_solicitar = {
        let show_solicitar_modal = show_solicitar_modal.clone();
        Callback::from(move |_: MouseEvent| show_solicitar_modal.set(false))
    };

    let on_submit_solicitar = {
        let documento = documento.clone();
        let show_solicitar_modal = show_solicitar_modal.clone();
        let toasts = toasts.clone();
        let error = error.clone();
        let reload_firmas = reload_firmas.clone();
        Callback::from(move |(nombre, email): (String, String)| {
            let doc = (*documento).clone();
            let Some(doc) = doc else { return };
            let show_solicitar_modal = show_solicitar_modal.clone();
            let toasts = toasts.clone();
            let error = error.clone();
            let reload_firmas = reload_firmas.clone();
            let doc_id = doc.id.clone();
            spawn_local(async move {
                let body = SolicitarFirmaRequest {
                    firmante_nombre: nombre,
                    email,
                };
                match api_post::<SolicitarFirmaResponse, _>(
                    &format!("/documentos/{doc_id}/solicitar-firma"),
                    &body,
                )
                .await
                {
                    Ok(_) => {
                        show_solicitar_modal.set(false);
                        reload_firmas.set(*reload_firmas + 1);
                        if let Some(ref t) = toasts {
                            t.dispatch(ToastAction::Push(
                                "Solicitud de firma enviada exitosamente".into(),
                                ToastKind::Success,
                            ));
                        }
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        })
    };

    let tipo_documento = documento.as_ref().and_then(|d| d.tipo_documento.clone());
    let doc_id_attr = documento.as_ref().map(|d| AttrValue::from(d.id.clone()));
    let is_sealed = documento.as_ref().is_some_and(|d| d.sellado);

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
                    <>
                        <DocumentEditor
                            contenido={(*contenido).clone()}
                            readonly={is_sealed}
                            entity_type={AttrValue::from(entity_type.clone())}
                            entity_id={AttrValue::from(entity_id.clone())}
                            tipo_documento={tipo_documento.map(AttrValue::from)}
                            documento_id={doc_id_attr.clone()}
                            on_save={on_save.clone()}
                        />
                        if doc_id_attr.is_some() {
                            <SignaturePanel
                                firmas={(*firmas).clone()}
                                sellado={is_sealed}
                                on_firmar={on_open_signature.clone()}
                                on_solicitar={on_open_solicitar.clone()}
                            />
                        }
                    </>
                },
            }}
            if *show_signature_modal {
                <SignatureModal
                    on_submit={on_submit_signature}
                    on_cancel={on_cancel_signature}
                />
            }
            if *show_solicitar_modal {
                <SolicitarFirmaModal
                    on_submit={on_submit_solicitar}
                    on_cancel={on_cancel_solicitar}
                />
            }
        </div>
    }
}

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
                {"â† Volver"}
            </button>
            <h1 class="text-display" style="font-size: var(--text-lg); font-weight: 600; color: var(--text-primary);">
                {"Editor de Documento"}
            </h1>
        </div>
    }
}

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

#[derive(Properties, PartialEq)]
struct TemplateCardProps {
    nombre: String,
    tipo_documento: String,
    on_click: Callback<MouseEvent>,
}

fn template_icon(tipo: &str) -> &'static str {
    match tipo {
        "contrato_arrendamiento" | "addendum" => "ðŸ“‹",
        "recibo_pago" => "ðŸ§¾",
        "acta_notarial" => "ðŸ“",
        "carta_referencia" => "âœ‰ï¸",
        _ => "ðŸ“„",
    }
}

fn template_description(tipo: &str) -> &'static str {
    match tipo {
        "contrato_arrendamiento" => "Contrato estÃ¡ndar de alquiler con clÃ¡usulas DR",
        "recibo_pago" => "Recibo para pagos de alquiler",
        "acta_notarial" => "Acta de entrega o devoluciÃ³n de propiedad",
        "carta_referencia" => "Carta de referencia para inquilino",
        "addendum" => "ModificaciÃ³n a contrato existente",
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

#[derive(Properties, PartialEq)]
struct SignaturePanelProps {
    firmas: Vec<FirmaResponse>,
    sellado: bool,
    on_firmar: Callback<MouseEvent>,
    on_solicitar: Callback<MouseEvent>,
}

#[component]
fn SignaturePanel(props: &SignaturePanelProps) -> Html {
    let propietario_firma = props
        .firmas
        .iter()
        .find(|f| f.firmante_tipo == "propietario");
    let inquilino_firma = props.firmas.iter().find(|f| f.firmante_tipo == "inquilino");

    html! {
        <div class="gi-signature-panel" style="margin-top: var(--space-4); padding: var(--space-4); border: 1px solid var(--border-primary); border-radius: var(--radius-md); background: var(--bg-secondary);">
            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--space-3);">
                <h3 style="font-size: var(--text-base); font-weight: 600; color: var(--text-primary); margin: 0;">
                    {"Estado de Firmas"}
                </h3>
                if props.sellado {
                    <span class="gi-badge gi-badge-success" style="font-size: var(--text-xs); padding: var(--space-1) var(--space-2); border-radius: var(--radius-sm); background: var(--color-success-bg); color: var(--color-success-text); font-weight: 600;">
                        {"Sellado"}
                    </span>
                }
            </div>
            <div style="display: flex; flex-direction: column; gap: var(--space-2); margin-bottom: var(--space-3);">
                <SignatureStatusRow
                    label="Propietario"
                    firma={propietario_firma.cloned()}
                />
                <SignatureStatusRow
                    label="Inquilino"
                    firma={inquilino_firma.cloned()}
                />
            </div>
            if !props.sellado {
                <div style="display: flex; gap: var(--space-2);">
                    if propietario_firma.is_none() {
                        <button class="gi-btn gi-btn-primary" onclick={props.on_firmar.clone()}>
                            {"Firmar Documento"}
                        </button>
                    }
                    if inquilino_firma.is_none() {
                        <button class="gi-btn gi-btn-ghost" onclick={props.on_solicitar.clone()}>
                            {"Solicitar Firma del Inquilino"}
                        </button>
                    }
                </div>
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct SignatureStatusRowProps {
    label: &'static str,
    firma: Option<FirmaResponse>,
}

#[component]
fn SignatureStatusRow(props: &SignatureStatusRowProps) -> Html {
    let (icon, status_text, color) = match &props.firma {
        Some(f) if f.estado == "firmado" => ("âœ“", "Firmado", "var(--color-success-text)"),
        Some(f) if f.estado == "pendiente" => ("â³", "Pendiente", "var(--color-warning-text)"),
        Some(_) | None => ("â€”", "Sin firma", "var(--text-tertiary)"),
    };

    let nombre = props
        .firma
        .as_ref()
        .map_or("â€”", |f| f.firmante_nombre.as_str());

    html! {
        <div style="display: flex; align-items: center; gap: var(--space-2); font-size: var(--text-sm);">
            <span style={format!("color: {color}; font-weight: 600;")}>{icon}</span>
            <span style="color: var(--text-secondary); min-width: 90px;">{props.label}{":"}</span>
            <span style="color: var(--text-primary);">{nombre}</span>
            <span style={format!("color: {color}; margin-left: auto;")}>{status_text}</span>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct SignatureModalProps {
    on_submit: Callback<String>,
    on_cancel: Callback<()>,
}

#[component]
fn SignatureModal(props: &SignatureModalProps) -> Html {
    html! {
        <div class="gi-modal-overlay" role="dialog" aria-modal="true" aria-label="Firmar documento">
            <div class="gi-modal" style="max-width: 480px;">
                <h3 style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-3); color: var(--text-primary);">
                    {"Firmar Documento"}
                </h3>
                <p style="font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-3);">
                    {"Dibuje su firma en el recuadro a continuaciÃ³n."}
                </p>
                <SignatureCanvas
                    on_submit={props.on_submit.clone()}
                    on_cancel={props.on_cancel.clone()}
                />
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct SolicitarFirmaModalProps {
    on_submit: Callback<(String, String)>,
    on_cancel: Callback<MouseEvent>,
}

#[component]
fn SolicitarFirmaModal(props: &SolicitarFirmaModalProps) -> Html {
    let nombre = use_state(String::new);
    let email = use_state(String::new);

    let on_nombre_change = {
        let nombre = nombre.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
            {
                nombre.set(input.value());
            }
        })
    };

    let on_email_change = {
        let email = email.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
            {
                email.set(input.value());
            }
        })
    };

    let on_submit_click = {
        let nombre = nombre.clone();
        let email = email.clone();
        let on_submit = props.on_submit.clone();
        Callback::from(move |_: MouseEvent| {
            let n = (*nombre).clone();
            let e = (*email).clone();
            if !n.trim().is_empty() && !e.trim().is_empty() {
                on_submit.emit((n, e));
            }
        })
    };

    let can_submit = !nombre.trim().is_empty() && !email.trim().is_empty();

    html! {
        <div class="gi-modal-overlay" role="dialog" aria-modal="true" aria-label="Solicitar firma del inquilino">
            <div class="gi-modal" style="max-width: 420px;">
                <h3 style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-3); color: var(--text-primary);">
                    {"Solicitar Firma del Inquilino"}
                </h3>
                <div style="display: flex; flex-direction: column; gap: var(--space-3); margin-bottom: var(--space-4);">
                    <div>
                        <label style="display: block; font-size: var(--text-sm); font-weight: 500; color: var(--text-secondary); margin-bottom: var(--space-1);">
                            {"Nombre del firmante"}
                        </label>
                        <input
                            type="text"
                            class="gi-input"
                            placeholder="Nombre completo"
                            value={(*nombre).clone()}
                            oninput={on_nombre_change}
                        />
                    </div>
                    <div>
                        <label style="display: block; font-size: var(--text-sm); font-weight: 500; color: var(--text-secondary); margin-bottom: var(--space-1);">
                            {"Correo electrÃ³nico"}
                        </label>
                        <input
                            type="email"
                            class="gi-input"
                            placeholder="correo@ejemplo.com"
                            value={(*email).clone()}
                            oninput={on_email_change}
                        />
                    </div>
                </div>
                <div style="display: flex; justify-content: flex-end; gap: var(--space-2);">
                    <button class="gi-btn gi-btn-ghost" onclick={props.on_cancel.clone()}>
                        {"Cancelar"}
                    </button>
                    <button class="gi-btn gi-btn-primary" onclick={on_submit_click} disabled={!can_submit}>
                        {"Enviar Solicitud"}
                    </button>
                </div>
            </div>
        </div>
    }
}
