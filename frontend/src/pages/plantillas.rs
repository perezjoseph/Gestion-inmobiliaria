use serde::Serialize;
use serde_json::Value;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::components::common::data_table::DataTable;
use crate::components::common::delete_confirm_modal::DeleteConfirmModal;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::TableSkeleton;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_delete, api_get, api_post, api_put};
use crate::types::documento::PlantillaResponse;
use crate::utils::can_write;

fn push_toast(toasts: Option<&ToastContext>, msg: &str, kind: ToastKind) {
    if let Some(t) = toasts {
        t.dispatch(ToastAction::Push(msg.into(), kind));
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CrearPlantillaBody {
    nombre: String,
    tipo_documento: String,
    entity_type: String,
    contenido: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActualizarPlantillaBody {
    nombre: Option<String>,
    tipo_documento: Option<String>,
    entity_type: Option<String>,
    contenido: Option<Value>,
}

const TIPOS_DOCUMENTO: &[(&str, &str)] = &[
    ("contrato_arrendamiento", "Contrato de Arrendamiento"),
    ("acuerdo", "Acuerdo"),
    ("recibo", "Recibo"),
    ("carta", "Carta"),
    ("notificacion", "Notificación"),
    ("otro", "Otro"),
];

const ENTITY_TYPES: &[(&str, &str)] = &[
    ("propiedad", "Propiedad"),
    ("inquilino", "Inquilino"),
    ("contrato", "Contrato"),
    ("pago", "Pago"),
    ("gasto", "Gasto"),
];

fn blocks_to_text(contenido: &Value) -> String {
    let blocks = contenido
        .get("blocks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut lines = Vec::new();
    for block in &blocks {
        let block_type = block
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("paragraph");
        match block_type {
            "heading" => {
                let level = block.get("level").and_then(Value::as_u64).unwrap_or(1);
                let prefix = "#".repeat(level as usize);
                let text = block.get("text").and_then(Value::as_str).unwrap_or("");
                lines.push(format!("{prefix} {text}"));
            }
            "list" => {
                let ordered = block
                    .get("ordered")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let items = block.get("items").and_then(Value::as_array);
                if let Some(items) = items {
                    for (i, item) in items.iter().enumerate() {
                        let text = item.as_str().unwrap_or("");
                        if ordered {
                            lines.push(format!("{}. {text}", i + 1));
                        } else {
                            lines.push(format!("- {text}"));
                        }
                    }
                }
            }
            "page_break" => {
                lines.push("---".to_string());
            }
            _ => {
                let text = block.get("text").and_then(Value::as_str).unwrap_or("");
                lines.push(text.to_string());
            }
        }
    }
    lines.join("\n")
}

fn text_to_blocks(text: &str) -> Value {
    let mut blocks = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "---" {
            blocks.push(serde_json::json!({"type": "page_break"}));
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            blocks.push(serde_json::json!({"type": "heading", "level": 3, "text": rest}));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            blocks.push(serde_json::json!({"type": "heading", "level": 2, "text": rest}));
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            blocks.push(serde_json::json!({"type": "heading", "level": 1, "text": rest}));
        } else if let Some(rest) = trimmed.strip_prefix("- ") {
            blocks.push(serde_json::json!({
                "type": "list",
                "ordered": false,
                "items": [rest]
            }));
        } else if trimmed.len() > 2
            && trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
            && trimmed.contains(". ")
        {
            let text_part = trimmed.split_once(". ").map_or("", |(_num, rest)| rest);
            blocks.push(serde_json::json!({
                "type": "list",
                "ordered": true,
                "items": [text_part]
            }));
        } else {
            blocks.push(serde_json::json!({"type": "paragraph", "text": trimmed}));
        }
    }

    let mut merged: Vec<Value> = Vec::new();
    for block in blocks {
        let is_list = block.get("type").and_then(Value::as_str) == Some("list");
        if is_list {
            let ordered = block
                .get("ordered")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let items = block
                .get("items")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if let Some(last) = merged.last_mut() {
                let last_is_list = last.get("type").and_then(Value::as_str) == Some("list");
                let last_ordered = last
                    .get("ordered")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if last_is_list && last_ordered == ordered {
                    if let Some(last_items) = last.get_mut("items").and_then(Value::as_array_mut) {
                        last_items.extend(items);
                        continue;
                    }
                }
            }
        }
        merged.push(block);
    }

    serde_json::json!({"version": 1, "blocks": merged})
}

#[allow(clippy::redundant_clone)]
#[component]
pub fn Plantillas() -> Html {
    let auth = use_context::<AuthContext>();
    let toasts = use_context::<ToastContext>();
    let plantillas = use_state(Vec::<PlantillaResponse>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let reload = use_state(|| 0u32);

    let editing_id = use_state(|| Option::<String>::None);
    let form_nombre = use_state(String::new);
    let form_tipo = use_state(|| "contrato_arrendamiento".to_string());
    let form_entity = use_state(|| "contrato".to_string());
    let form_contenido = use_state(String::new);
    let show_form = use_state(|| false);
    let show_delete = use_state(|| Option::<String>::None);

    let user_can_write = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .is_some_and(|u| can_write(&u.rol));

    {
        let plantillas = plantillas.clone();
        let loading = loading.clone();
        let error = error.clone();
        let reload_val = *reload;
        use_effect_with(reload_val, move |_| {
            loading.set(true);
            spawn_local(async move {
                match api_get::<Vec<PlantillaResponse>>("/documentos/plantillas").await {
                    Ok(data) => {
                        plantillas.set(data);
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e)),
                }
                loading.set(false);
            });
        });
    }

    let on_create = {
        let show_form = show_form.clone();
        let editing_id = editing_id.clone();
        let form_nombre = form_nombre.clone();
        let form_tipo = form_tipo.clone();
        let form_entity = form_entity.clone();
        let form_contenido = form_contenido.clone();
        Callback::from(move |_: MouseEvent| {
            editing_id.set(None);
            form_nombre.set(String::new());
            form_tipo.set("contrato_arrendamiento".to_string());
            form_entity.set("contrato".to_string());
            form_contenido.set(String::new());
            show_form.set(true);
        })
    };

    let on_edit = {
        let show_form = show_form.clone();
        let editing_id = editing_id.clone();
        let form_nombre = form_nombre.clone();
        let form_tipo = form_tipo.clone();
        let form_entity = form_entity.clone();
        let form_contenido = form_contenido.clone();
        let plantillas = plantillas.clone();
        Callback::from(move |id: AttrValue| {
            if let Some(p) = plantillas.iter().find(|t| t.id == id.as_str()) {
                editing_id.set(Some(p.id.clone()));
                form_nombre.set(p.nombre.clone());
                form_tipo.set(p.tipo_documento.clone());
                form_entity.set(p.entity_type.clone());
                form_contenido.set(blocks_to_text(&p.contenido));
                show_form.set(true);
            }
        })
    };

    let on_submit = {
        let editing_id = editing_id.clone();
        let form_nombre = form_nombre.clone();
        let form_tipo = form_tipo.clone();
        let form_entity = form_entity.clone();
        let form_contenido = form_contenido.clone();
        let show_form = show_form.clone();
        let reload = reload.clone();
        let toasts = toasts.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let nombre = (*form_nombre).clone();
            let tipo = (*form_tipo).clone();
            let entity = (*form_entity).clone();
            let contenido_text = (*form_contenido).clone();
            let edit_id = (*editing_id).clone();
            let show_form = show_form.clone();
            let reload = reload.clone();
            let toasts = toasts.clone();
            let contenido = text_to_blocks(&contenido_text);
            spawn_local(async move {
                let result = if let Some(id) = edit_id {
                    let body = ActualizarPlantillaBody {
                        nombre: Some(nombre),
                        tipo_documento: Some(tipo),
                        entity_type: Some(entity),
                        contenido: Some(contenido),
                    };
                    api_put::<PlantillaResponse, _>(&format!("/documentos/plantillas/{id}"), &body)
                        .await
                        .map(|_| "Plantilla actualizada")
                } else {
                    let body = CrearPlantillaBody {
                        nombre,
                        tipo_documento: tipo,
                        entity_type: entity,
                        contenido,
                    };
                    api_post::<PlantillaResponse, _>("/documentos/plantillas", &body)
                        .await
                        .map(|_| "Plantilla creada")
                };
                match result {
                    Ok(msg) => {
                        push_toast(toasts.as_ref(), msg, ToastKind::Success);
                        show_form.set(false);
                        reload.set(*reload + 1);
                    }
                    Err(e) => push_toast(toasts.as_ref(), &e, ToastKind::Error),
                }
            });
        })
    };

    let on_delete_confirm = {
        let show_delete = show_delete.clone();
        let reload = reload.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(id) = (*show_delete).clone() {
                let show_delete = show_delete.clone();
                let reload = reload.clone();
                let toasts = toasts.clone();
                spawn_local(async move {
                    match api_delete(&format!("/documentos/plantillas/{id}")).await {
                        Ok(()) => {
                            push_toast(toasts.as_ref(), "Plantilla eliminada", ToastKind::Success);
                            reload.set(*reload + 1);
                        }
                        Err(e) => {
                            push_toast(toasts.as_ref(), &e, ToastKind::Error);
                        }
                    }
                    show_delete.set(None);
                });
            }
        })
    };

    let on_delete_cancel = {
        let show_delete = show_delete.clone();
        Callback::from(move |_: MouseEvent| {
            show_delete.set(None);
        })
    };

    if *loading {
        return html! { <TableSkeleton title_width="220px" columns={4} /> };
    }

    html! {
        <div>
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-5);">
                <h1 class="gi-page-title">{"Plantillas de Documentos"}</h1>
                if user_can_write {
                    <button class="gi-btn gi-btn-primary" onclick={on_create}>
                        {"Crear Plantilla"}
                    </button>
                }
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} />
            }

            if plantillas.is_empty() {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-icon">{"📄"}</div>
                    <div class="gi-empty-state-title">{"No hay plantillas"}</div>
                    <p class="gi-empty-state-text">{"Cree una plantilla para comenzar a generar documentos."}</p>
                </div>
            } else {
                <PlantillasTable
                    plantillas={(*plantillas).clone()}
                    user_can_write={user_can_write}
                    on_edit={on_edit}
                    show_delete={show_delete.clone()}
                />
            }

            if *show_form {
                <PlantillaFormModal
                    editing_id={(*editing_id).clone()}
                    form_nombre={form_nombre.clone()}
                    form_tipo={form_tipo.clone()}
                    form_entity={form_entity.clone()}
                    form_contenido={form_contenido.clone()}
                    on_submit={on_submit}
                    on_close={let sf = show_form.clone(); Callback::from(move |_: MouseEvent| sf.set(false))}
                />
            }

            if show_delete.is_some() {
                <DeleteConfirmModal
                    message="¿Está seguro de que desea eliminar esta plantilla? Esta acción no se puede deshacer."
                    on_confirm={on_delete_confirm}
                    on_cancel={on_delete_cancel}
                />
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct PlantillasTableProps {
    plantillas: Vec<PlantillaResponse>,
    user_can_write: bool,
    on_edit: Callback<AttrValue>,
    show_delete: UseStateHandle<Option<String>>,
}

#[component]
fn PlantillasTable(props: &PlantillasTableProps) -> Html {
    html! {
        <DataTable headers={vec![
            "Nombre".to_string(),
            "Tipo Documento".to_string(),
            "Tipo Entidad".to_string(),
            "Acciones".to_string(),
        ]}>
            { for props.plantillas.iter().map(|p| {
                let id = p.id.clone();
                let on_edit = props.on_edit.clone();
                let show_delete = props.show_delete.clone();
                let edit_id = AttrValue::from(id.clone());
                let user_can_write = props.user_can_write;
                html! {
                    <tr>
                        <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                            {&p.nombre}
                        </td>
                        <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                            {&p.tipo_documento}
                        </td>
                        <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                            {&p.entity_type}
                        </td>
                        <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                            if user_can_write {
                                <div style="display: flex; gap: var(--space-2);">
                                    <button class="gi-btn gi-btn-sm gi-btn-ghost"
                                        onclick={Callback::from(move |_: MouseEvent| on_edit.emit(edit_id.clone()))}>
                                        {"Editar"}
                                    </button>
                                    <button class="gi-btn gi-btn-sm gi-btn-ghost gi-btn-danger"
                                        onclick={let sd = show_delete.clone(); let did = id.clone(); Callback::from(move |_: MouseEvent| sd.set(Some(did.clone())))}>
                                        {"Eliminar"}
                                    </button>
                                </div>
                            }
                        </td>
                    </tr>
                }
            })}
        </DataTable>
    }
}

#[derive(Properties, PartialEq)]
struct PlantillaFormModalProps {
    editing_id: Option<String>,
    form_nombre: UseStateHandle<String>,
    form_tipo: UseStateHandle<String>,
    form_entity: UseStateHandle<String>,
    form_contenido: UseStateHandle<String>,
    on_submit: Callback<SubmitEvent>,
    on_close: Callback<MouseEvent>,
}

#[component]
fn PlantillaFormModal(props: &PlantillaFormModalProps) -> Html {
    let form_nombre = props.form_nombre.clone();
    let form_tipo = props.form_tipo.clone();
    let form_entity = props.form_entity.clone();
    let form_contenido = props.form_contenido.clone();

    html! {
        <div class="gi-modal-overlay" onclick={props.on_close.clone()}>
            <div class="gi-modal" style="max-width: 640px;" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                <h2 class="gi-modal-title">
                    {if props.editing_id.is_some() { "Editar Plantilla" } else { "Crear Plantilla" }}
                </h2>
                <form onsubmit={props.on_submit.clone()}>
                    <div class="gi-form-group">
                        <label class="gi-label">{"Nombre"}</label>
                        <input class="gi-input" type="text" required=true
                            value={(*form_nombre).clone()}
                            oninput={let fn_ = form_nombre.clone(); Callback::from(move |e: InputEvent| {
                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                fn_.set(input.value());
                            })} />
                    </div>
                    <div class="gi-form-group">
                        <label class="gi-label">{"Tipo de Documento"}</label>
                        <select class="gi-select" required=true
                            onchange={let ft = form_tipo.clone(); Callback::from(move |e: Event| {
                                let sel: web_sys::HtmlSelectElement = e.target_unchecked_into();
                                ft.set(sel.value());
                            })}>
                            {for TIPOS_DOCUMENTO.iter().map(|(val, label)| {
                                html! { <option value={*val} selected={*val == form_tipo.as_str()}>{label}</option> }
                            })}
                        </select>
                    </div>
                    <div class="gi-form-group">
                        <label class="gi-label">{"Tipo de Entidad"}</label>
                        <select class="gi-select" required=true
                            onchange={let fe = form_entity.clone(); Callback::from(move |e: Event| {
                                let sel: web_sys::HtmlSelectElement = e.target_unchecked_into();
                                fe.set(sel.value());
                            })}>
                            {for ENTITY_TYPES.iter().map(|(val, label)| {
                                html! { <option value={*val} selected={*val == form_entity.as_str()}>{label}</option> }
                            })}
                        </select>
                    </div>
                    <div class="gi-form-group">
                        <label class="gi-label">{"Contenido"}</label>
                        <p style="font-size: var(--text-xs); color: var(--text-tertiary); margin-bottom: var(--space-1);">
                            {"Use # para títulos, - para listas, --- para saltos de página. Placeholders: {{entidad.campo}}"}
                        </p>
                        <textarea
                            class="gi-input"
                            rows="10"
                            style="font-family: monospace; resize: vertical; min-height: 160px;"
                            placeholder={"# Título del documento\n\nContenido del documento con {{contrato.fecha_inicio}}..."}
                            value={(*form_contenido).clone()}
                            oninput={let fc = form_contenido.clone(); Callback::from(move |e: InputEvent| {
                                let input: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                                fc.set(input.value());
                            })}
                        />
                    </div>
                    <div style="display: flex; gap: var(--space-3); justify-content: flex-end; margin-top: var(--space-4);">
                        <button type="button" class="gi-btn gi-btn-ghost" onclick={props.on_close.clone()}>
                            {"Cancelar"}
                        </button>
                        <button type="submit" class="gi-btn gi-btn-primary">
                            {if props.editing_id.is_some() { "Guardar" } else { "Crear" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
