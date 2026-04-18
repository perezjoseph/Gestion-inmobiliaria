use std::collections::HashMap;

use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::components::common::confidence_input::ConfidenceInput;
use crate::components::common::currency_display::CurrencyDisplay;
use crate::components::common::data_table::DataTable;
use crate::components::common::delete_confirm_modal::DeleteConfirmModal;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::ocr_scan_button::OcrScanButton;
use crate::components::common::pagination::Pagination;
use crate::components::common::skeleton::TableSkeleton;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_delete, api_get, api_post, api_put};
use crate::types::gasto::{CreateGasto, Gasto, UpdateGasto};
use crate::types::ocr::OcrExtractField;
use crate::types::propiedad::Propiedad;
use crate::types::PaginatedResponse;
use crate::utils::{
    EscapeHandler, can_delete, can_write, field_error, format_currency, format_date_display,
    input_class,
};

fn push_toast(toasts: &Option<ToastContext>, msg: &str, kind: ToastKind) {
    if let Some(t) = toasts {
        t.dispatch(ToastAction::Push(msg.into(), kind));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Unidad {
    id: String,
    propiedad_id: String,
    numero_unidad: String,
}

use serde::{Deserialize, Serialize};

const CATEGORIAS: &[(&str, &str)] = &[
    ("mantenimiento", "Mantenimiento"),
    ("servicios", "Servicios"),
    ("impuestos", "Impuestos"),
    ("seguros", "Seguros"),
    ("administracion", "Administración"),
    ("mejoras", "Mejoras"),
    ("legal", "Legal"),
    ("otro", "Otro"),
];

fn estado_badge(estado: &str) -> (&'static str, &'static str) {
    match estado {
        "pagado" => ("gi-badge gi-badge-success", "Pagado"),
        "pendiente" => ("gi-badge gi-badge-warning", "Pendiente"),
        "cancelado" => ("gi-badge gi-badge-error", "Cancelado"),
        _ => ("gi-badge gi-badge-neutral", "Desconocido"),
    }
}

#[derive(Clone, Default, PartialEq)]
struct FormErrors {
    propiedad_id: Option<String>,
    categoria: Option<String>,
    descripcion: Option<String>,
    monto: Option<String>,
    fecha_gasto: Option<String>,
}

impl FormErrors {
    fn has_errors(&self) -> bool {
        self.propiedad_id.is_some()
            || self.categoria.is_some()
            || self.descripcion.is_some()
            || self.monto.is_some()
            || self.fecha_gasto.is_some()
    }
}

#[derive(Properties, PartialEq)]
struct GastoFilterBarProps {
    filter_propiedad: UseStateHandle<String>,
    filter_categoria: UseStateHandle<String>,
    filter_estado: UseStateHandle<String>,
    propiedades: Vec<Propiedad>,
    on_apply: Callback<MouseEvent>,
    on_clear: Callback<MouseEvent>,
}

#[function_component]
fn GastoFilterBar(props: &GastoFilterBarProps) -> Html {
    let fp = props.filter_propiedad.clone();
    let on_prop_change = Callback::from(move |e: Event| {
        let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
        fp.set(el.value());
    });
    let fc = props.filter_categoria.clone();
    let on_cat_change = Callback::from(move |e: Event| {
        let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
        fc.set(el.value());
    });
    let fe = props.filter_estado.clone();
    let on_estado_change = Callback::from(move |e: Event| {
        let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
        fe.set(el.value());
    });
    html! {
        <div class="gi-filter-bar">
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--space-3); align-items: end;">
                <div>
                    <label class="gi-label">{"Propiedad"}</label>
                    <select onchange={on_prop_change} class="gi-input">
                        <option value="" selected={props.filter_propiedad.is_empty()}>{"Todas"}</option>
                        { for props.propiedades.iter().map(|p| {
                            let sel = *props.filter_propiedad == p.id;
                            html! { <option value={p.id.clone()} selected={sel}>{&p.titulo}</option> }
                        })}
                    </select>
                </div>
                <div>
                    <label class="gi-label">{"Categoría"}</label>
                    <select onchange={on_cat_change} class="gi-input">
                        <option value="" selected={props.filter_categoria.is_empty()}>{"Todas"}</option>
                        { for CATEGORIAS.iter().map(|(val, label)| {
                            let sel = *props.filter_categoria == *val;
                            html! { <option value={*val} selected={sel}>{*label}</option> }
                        })}
                    </select>
                </div>
                <div>
                    <label class="gi-label">{"Estado"}</label>
                    <select onchange={on_estado_change} class="gi-input">
                        <option value="" selected={props.filter_estado.is_empty()}>{"Todos"}</option>
                        <option value="pendiente" selected={*props.filter_estado == "pendiente"}>{"Pendiente"}</option>
                        <option value="pagado" selected={*props.filter_estado == "pagado"}>{"Pagado"}</option>
                        <option value="cancelado" selected={*props.filter_estado == "cancelado"}>{"Cancelado"}</option>
                    </select>
                </div>
                <div style="display: flex; gap: var(--space-2);">
                    <button onclick={props.on_apply.clone()} class="gi-btn gi-btn-primary">{"Filtrar"}</button>
                    <button onclick={props.on_clear.clone()} class="gi-btn gi-btn-ghost">{"Limpiar"}</button>
                </div>
            </div>
        </div>
    }
}
