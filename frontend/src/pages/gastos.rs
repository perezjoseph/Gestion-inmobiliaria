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
use crate::components::common::document_gallery::DocumentGallery;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::ocr_scan_button::OcrScanButton;
use crate::components::common::pagination::Pagination;
use crate::components::common::skeleton::TableSkeleton;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_delete, api_get, api_post, api_put};
use crate::types::PaginatedResponse;
use crate::types::gasto::{CreateGasto, Gasto, UpdateGasto};
use crate::types::ocr::OcrExtractField;
use crate::types::propiedad::Propiedad;
use crate::utils::{
    EscapeHandler, can_delete, can_write, field_error, format_currency, format_date_display,
    input_class,
};

fn push_toast(toasts: Option<&ToastContext>, msg: &str, kind: ToastKind) {
    if let Some(t) = toasts {
        t.dispatch(ToastAction::Push(msg.into(), kind));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)]
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
    const fn has_errors(&self) -> bool {
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

#[component]
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

#[derive(Properties, PartialEq)]
struct GastoFormProps {
    is_editing: bool,
    propiedad_id: UseStateHandle<String>,
    unidad_id: UseStateHandle<String>,
    categoria: UseStateHandle<String>,
    descripcion: UseStateHandle<String>,
    monto: UseStateHandle<String>,
    moneda: UseStateHandle<String>,
    fecha_gasto: UseStateHandle<String>,
    estado_form: UseStateHandle<String>,
    proveedor: UseStateHandle<String>,
    numero_factura: UseStateHandle<String>,
    notas: UseStateHandle<String>,
    propiedades: Vec<Propiedad>,
    unidades: Vec<Unidad>,
    form_errors: FormErrors,
    submitting: bool,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
    on_propiedad_change: Callback<Event>,
    confidences: HashMap<String, f64>,
    on_ocr_result: Callback<Vec<OcrExtractField>>,
    on_confidence_clear: Callback<String>,
    #[prop_or_default]
    editing_id: Option<String>,
    #[prop_or_default]
    token: String,
}

#[component]
fn GastoForm(props: &GastoFormProps) -> Html {
    let fe = props.form_errors.clone();

    macro_rules! input_cb {
        ($state:expr) => {{
            let s = $state.clone();
            Callback::from(move |e: InputEvent| {
                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                s.set(input.value());
            })
        }};
    }
    macro_rules! select_cb {
        ($state:expr) => {{
            let s = $state.clone();
            Callback::from(move |e: Event| {
                let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
                s.set(el.value());
            })
        }};
    }

    let confidence_for = |name: &str| -> Option<f64> { props.confidences.get(name).copied() };

    let input_cb_conf =
        |state: &UseStateHandle<String>, field_name: &str| -> Callback<InputEvent> {
            let s = state.clone();
            let clear = props.on_confidence_clear.clone();
            let name = field_name.to_string();
            Callback::from(move |e: InputEvent| {
                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                s.set(input.value());
                clear.emit(name.clone());
            })
        };

    let filtered_unidades: Vec<&Unidad> = props
        .unidades
        .iter()
        .filter(|u| u.propiedad_id == *props.propiedad_id)
        .collect();

    let scan_button = if props.is_editing {
        html! {}
    } else {
        html! {
            <OcrScanButton
                document_type="recibo_gasto"
                on_result={props.on_ocr_result.clone()}
                label={AttrValue::from("📷 Escanear Factura")}
            />
        }
    };

    html! {
        <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
            <div style="display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-4);">
                <h2 class="text-display" style="font-size: var(--text-lg); font-weight: 600; color: var(--text-primary);">
                    {if props.is_editing { "Editar Gasto" } else { "Nuevo Gasto" }}</h2>
                {scan_button}
            </div>
            <form onsubmit={props.on_submit.clone()} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                <div>
                    <label class="gi-label">{"Propiedad *"}</label>
                    <select onchange={props.on_propiedad_change.clone()} disabled={props.is_editing}
                        class={input_class(fe.propiedad_id.is_some())}>
                        <option value="" selected={props.propiedad_id.is_empty()}>{"— Seleccionar propiedad —"}</option>
                        { for props.propiedades.iter().map(|p| {
                            let sel = *props.propiedad_id == p.id;
                            html! { <option value={p.id.clone()} selected={sel}>{&p.titulo}</option> }
                        })}
                    </select>
                    {field_error(&fe.propiedad_id)}
                </div>
                <div>
                    <label class="gi-label">{"Unidad"}</label>
                    <select onchange={select_cb!(props.unidad_id)} class="gi-input">
                        <option value="" selected={props.unidad_id.is_empty()}>{"— Sin unidad —"}</option>
                        { for filtered_unidades.iter().map(|u| {
                            let sel = *props.unidad_id == u.id;
                            html! { <option value={u.id.clone()} selected={sel}>{&u.numero_unidad}</option> }
                        })}
                    </select>
                </div>
                <div>
                    <label class="gi-label">{"Categoría *"}</label>
                    <select onchange={select_cb!(props.categoria)}
                        class={input_class(fe.categoria.is_some())}>
                        <option value="" selected={props.categoria.is_empty()}>{"— Seleccionar categoría —"}</option>
                        { for CATEGORIAS.iter().map(|(val, label)| {
                            let sel = *props.categoria == *val;
                            html! { <option value={*val} selected={sel}>{*label}</option> }
                        })}
                    </select>
                    {field_error(&fe.categoria)}
                </div>
                <div>
                    <label class="gi-label">{"Descripción *"}</label>
                    <input type="text" value={(*props.descripcion).clone()} oninput={input_cb!(props.descripcion)}
                        placeholder="Descripción del gasto"
                        class={input_class(fe.descripcion.is_some())} />
                    {field_error(&fe.descripcion)}
                </div>
                <div>
                    <label class="gi-label">{"Monto *"}</label>
                    <ConfidenceInput
                        value={AttrValue::from((*props.monto).clone())}
                        confidence={confidence_for("monto")}
                        oninput={input_cb_conf(&props.monto, "monto")}
                        input_type="number"
                        class={AttrValue::from(if fe.monto.is_some() { "gi-input-error" } else { "" })}
                    />
                    {field_error(&fe.monto)}
                </div>
                <div>
                    <label class="gi-label">{"Moneda"}</label>
                    <select onchange={select_cb!(props.moneda)} class="gi-input">
                        <option value="DOP" selected={*props.moneda == "DOP"}>{"DOP"}</option>
                        <option value="USD" selected={*props.moneda == "USD"}>{"USD"}</option>
                    </select>
                </div>
                <div>
                    <label class="gi-label">{"Fecha de Gasto *"}</label>
                    <ConfidenceInput
                        value={AttrValue::from((*props.fecha_gasto).clone())}
                        confidence={confidence_for("fecha_gasto")}
                        oninput={input_cb_conf(&props.fecha_gasto, "fecha_gasto")}
                        input_type="date"
                        class={AttrValue::from(if fe.fecha_gasto.is_some() { "gi-input-error" } else { "" })}
                    />
                    {field_error(&fe.fecha_gasto)}
                </div>
                <div>
                    <label class="gi-label">{"Proveedor"}</label>
                    <ConfidenceInput
                        value={AttrValue::from((*props.proveedor).clone())}
                        confidence={confidence_for("proveedor")}
                        oninput={input_cb_conf(&props.proveedor, "proveedor")}
                        placeholder={AttrValue::from("Nombre del proveedor")}
                    />
                </div>
                <div>
                    <label class="gi-label">{"Número de Factura"}</label>
                    <ConfidenceInput
                        value={AttrValue::from((*props.numero_factura).clone())}
                        confidence={confidence_for("numero_factura")}
                        oninput={input_cb_conf(&props.numero_factura, "numero_factura")}
                        placeholder={AttrValue::from("Ej: FAC-001")}
                    />
                </div>
                if props.is_editing {
                    <div>
                        <label class="gi-label">{"Estado"}</label>
                        <select onchange={select_cb!(props.estado_form)} class="gi-input">
                            <option value="pendiente" selected={*props.estado_form == "pendiente"}>{"Pendiente"}</option>
                            <option value="pagado" selected={*props.estado_form == "pagado"}>{"Pagado"}</option>
                            <option value="cancelado" selected={*props.estado_form == "cancelado"}>{"Cancelado"}</option>
                        </select>
                    </div>
                }
                <div style="grid-column: 1 / -1;">
                    <label class="gi-label">{"Notas"}</label>
                    <input type="text" value={(*props.notas).clone()} oninput={input_cb!(props.notas)}
                        placeholder="Notas opcionales" class="gi-input" />
                </div>
                <div style="grid-column: 1 / -1; display: flex; gap: var(--space-2); justify-content: flex-end;">
                    <button type="button" onclick={props.on_cancel.clone()} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                    <button type="submit" disabled={props.submitting} class="gi-btn gi-btn-primary">
                        {if props.submitting { "Guardando..." } else { "Guardar" }}
                    </button>
                </div>
            </form>
            if let Some(ref id) = props.editing_id {
                <div style="margin-top: var(--space-5); border-top: 1px solid var(--border-subtle); padding-top: var(--space-5);">
                    <DocumentGallery
                        entity_type={"gasto".to_string()}
                        entity_id={id.clone()}
                        token={props.token.clone()}
                    />
                </div>
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct GastoListProps {
    items: Vec<Gasto>,
    user_rol: String,
    headers: Vec<String>,
    total: u64,
    page: u64,
    per_page: u64,
    prop_label: Callback<String, String>,
    on_edit: Callback<Gasto>,
    on_delete: Callback<Gasto>,
    on_new: Callback<MouseEvent>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
}

fn render_gasto_row(
    g: &Gasto,
    user_rol: &str,
    prop_label: &Callback<String, String>,
    on_edit: &Callback<Gasto>,
    on_delete: &Callback<Gasto>,
) -> Html {
    let p_label = prop_label.emit(g.propiedad_id.clone());
    let (badge_cls, badge_label) = estado_badge(&g.estado);
    let cat_label = CATEGORIAS
        .iter()
        .find(|(v, _)| *v == g.categoria)
        .map_or(g.categoria.as_str(), |(_, l)| l);
    let gc = g.clone();
    let gd = g.clone();
    let on_edit = on_edit.clone();
    let on_delete_click = on_delete.clone();
    let actions = if can_write(user_rol) {
        let delete_btn = if can_delete(user_rol) {
            html! { <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(gd.clone()))} class="gi-btn-text" style="color: var(--color-error);">{"Eliminar"}</button> }
        } else {
            html! {}
        };
        html! {
            <td style="padding: var(--space-3) var(--space-5); display: flex; gap: var(--space-2);">
                <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(gc.clone()))} class="gi-btn-text">{"Editar"}</button>
                {delete_btn}
            </td>
        }
    } else {
        html! {}
    };
    html! {
        <tr>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{p_label}</td>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{cat_label}</td>
            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);"><CurrencyDisplay monto={g.monto} moneda={g.moneda.clone()} /></td>
            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{format_date_display(&g.fecha_gasto)}</td>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">
                {g.proveedor.as_deref().unwrap_or("—")}</td>
            <td style="padding: var(--space-3) var(--space-5);"><span class={badge_cls}>{badge_label}</span></td>
            {actions}
        </tr>
    }
}

fn render_gasto_empty_state(user_rol: &str, on_new: &Callback<MouseEvent>) -> Html {
    html! {
        <div class="gi-empty-state">
            <div class="gi-empty-state-icon">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4">
                    <path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/>
                    <path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>
                </svg>
            </div>
            <div class="gi-empty-state-title">{"Sin gastos registrados"}</div>
            <p class="gi-empty-state-text">{"Registre los gastos de sus propiedades para llevar un control detallado de sus egresos."}</p>
            if can_write(user_rol) {
                <button onclick={on_new.clone()} class="gi-btn gi-btn-primary" style="margin-top: var(--space-3);">{"+ Nuevo Gasto"}</button>
            }
        </div>
    }
}

#[component]
fn GastoList(props: &GastoListProps) -> Html {
    if props.items.is_empty() {
        return render_gasto_empty_state(&props.user_rol, &props.on_new);
    }

    html! {
        <>
            <DataTable headers={props.headers.clone()}>
                { for props.items.iter().map(|g| render_gasto_row(g, &props.user_rol, &props.prop_label, &props.on_edit, &props.on_delete)) }
            </DataTable>
            <Pagination
                total={props.total}
                page={props.page}
                per_page={props.per_page}
                on_page_change={props.on_page_change.clone()}
                on_per_page_change={props.on_per_page_change.clone()}
            />
        </>
    }
}

fn validate_gasto_fields(
    propiedad_id: &str,
    categoria: &str,
    descripcion: &str,
    monto: &str,
    fecha_gasto: &str,
) -> FormErrors {
    let mut errs = FormErrors::default();
    if propiedad_id.is_empty() {
        errs.propiedad_id = Some("Debe seleccionar una propiedad".into());
    }
    if categoria.is_empty() {
        errs.categoria = Some("Debe seleccionar una categoría".into());
    }
    if descripcion.trim().is_empty() {
        errs.descripcion = Some("La descripción es obligatoria".into());
    }
    match monto.parse::<f64>() {
        Ok(v) if v <= 0.0 => errs.monto = Some("El monto debe ser mayor a 0".into()),
        Err(_) => errs.monto = Some("Monto inválido".into()),
        _ => {}
    }
    if fecha_gasto.is_empty() {
        errs.fecha_gasto = Some("La fecha de gasto es obligatoria".into());
    }
    errs
}

fn non_empty(s: &str) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn build_gastos_url(
    pg: u64,
    pp: u64,
    f_propiedad: &str,
    f_categoria: &str,
    f_estado: &str,
) -> String {
    let mut params = vec![format!("page={pg}"), format!("perPage={pp}")];
    if !f_propiedad.is_empty() {
        params.push(format!("propiedadId={f_propiedad}"));
    }
    if !f_categoria.is_empty() {
        params.push(format!("categoria={f_categoria}"));
    }
    if !f_estado.is_empty() {
        params.push(format!("estado={f_estado}"));
    }
    format!("/gastos?{}", params.join("&"))
}

fn load_gastos_data(
    items: UseStateHandle<Vec<Gasto>>,
    total: UseStateHandle<u64>,
    error: UseStateHandle<Option<String>>,
    loading: UseStateHandle<bool>,
    url: String,
) {
    spawn_local(async move {
        loading.set(true);
        match api_get::<PaginatedResponse<Gasto>>(&url).await {
            Ok(resp) => {
                total.set(resp.total);
                items.set(resp.data);
            }
            Err(err) => error.set(Some(err)),
        }
        loading.set(false);
    });
}

fn load_gasto_refs(propiedades: UseStateHandle<Vec<Propiedad>>) {
    spawn_local(async move {
        if let Ok(resp) = api_get::<PaginatedResponse<Propiedad>>("/propiedades?perPage=200").await
        {
            propiedades.set(resp.data);
        }
    });
}

fn register_escape_listener(escape_handler: EscapeHandler) -> Option<EventListener> {
    web_sys::window().and_then(|w| w.document()).map(|doc| {
        EventListener::new(&doc, "keydown", move |event| {
            if let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() {
                if event.key() == "Escape"
                    && let Some(ref cb) = *escape_handler.borrow()
                {
                    cb();
                }
            }
        })
    })
}

#[allow(clippy::too_many_arguments)]
fn make_gasto_edit_cb(
    propiedad_id: &UseStateHandle<String>,
    unidad_id: &UseStateHandle<String>,
    categoria: &UseStateHandle<String>,
    descripcion: &UseStateHandle<String>,
    monto: &UseStateHandle<String>,
    moneda: &UseStateHandle<String>,
    fecha_gasto: &UseStateHandle<String>,
    estado_form: &UseStateHandle<String>,
    proveedor: &UseStateHandle<String>,
    numero_factura: &UseStateHandle<String>,
    notas: &UseStateHandle<String>,
    editing: &UseStateHandle<Option<Gasto>>,
    show_form: &UseStateHandle<bool>,
    form_errors: &UseStateHandle<FormErrors>,
) -> Callback<Gasto> {
    let (propiedad_id, unidad_id, categoria) =
        (propiedad_id.clone(), unidad_id.clone(), categoria.clone());
    let (descripcion, monto, moneda) = (descripcion.clone(), monto.clone(), moneda.clone());
    let (fecha_gasto, estado_form, proveedor) =
        (fecha_gasto.clone(), estado_form.clone(), proveedor.clone());
    let (numero_factura, notas) = (numero_factura.clone(), notas.clone());
    let (editing, show_form, form_errors) =
        (editing.clone(), show_form.clone(), form_errors.clone());
    Callback::from(move |g: Gasto| {
        propiedad_id.set(g.propiedad_id.clone());
        unidad_id.set(g.unidad_id.clone().unwrap_or_default());
        categoria.set(g.categoria.clone());
        descripcion.set(g.descripcion.clone());
        monto.set(g.monto.to_string());
        moneda.set(g.moneda.clone());
        fecha_gasto.set(g.fecha_gasto.clone());
        estado_form.set(g.estado.clone());
        proveedor.set(g.proveedor.clone().unwrap_or_default());
        numero_factura.set(g.numero_factura.clone().unwrap_or_default());
        notas.set(g.notas.clone().unwrap_or_default());
        editing.set(Some(g));
        show_form.set(true);
        form_errors.set(FormErrors::default());
    })
}

#[allow(clippy::too_many_arguments)]
fn do_save_gasto(
    editing_id: Option<String>,
    update: UpdateGasto,
    create: CreateGasto,
    reset_form: impl Fn() + 'static,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
    submitting: UseStateHandle<bool>,
) {
    spawn_local(async move {
        let res = match editing_id {
            Some(id) => api_put::<Gasto, _>(&format!("/gastos/{id}"), &update)
                .await
                .map(|_| ()),
            None => api_post::<Gasto, _>("/gastos", &create).await.map(|_| ()),
        };
        match res {
            Ok(()) => {
                reset_form();
                reload.set(*reload + 1);
                push_toast(toasts.as_ref(), "Gasto guardado", ToastKind::Success);
            }
            Err(err) => error.set(Some(err)),
        }
        submitting.set(false);
    });
}

fn do_delete_gasto(
    id: String,
    label: String,
    delete_target: UseStateHandle<Option<Gasto>>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
    reload_for_undo: UseStateHandle<u32>,
) {
    spawn_local(async move {
        match api_delete(&format!("/gastos/{id}")).await {
            Ok(()) => {
                delete_target.set(None);
                reload.set(*reload + 1);
                let undo_reload = reload_for_undo;
                if let Some(t) = &toasts {
                    t.dispatch(ToastAction::PushWithUndo(
                        format!("Gasto de {label} eliminado"),
                        ToastKind::Info,
                        "Deshacer".into(),
                        std::rc::Rc::new(move || {
                            undo_reload.set(*undo_reload + 1);
                        }),
                    ));
                }
            }
            Err(err) => {
                delete_target.set(None);
                error.set(Some(err));
            }
        }
    });
}

fn handle_escape_gastos(
    delete_target: &UseStateHandle<Option<Gasto>>,
    show_form: &UseStateHandle<bool>,
    reset_form: &dyn Fn(),
) {
    if delete_target.is_some() {
        delete_target.set(None);
    } else if **show_form {
        reset_form();
    }
}

#[component]
pub fn Gastos() -> Html {
    let auth = use_context::<AuthContext>();
    let toasts = use_context::<ToastContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();

    let items = use_state(Vec::<Gasto>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let propiedades = use_state(Vec::<Propiedad>::new);
    let unidades = use_state(Vec::<Unidad>::new);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let show_form = use_state(|| false);
    let editing = use_state(|| Option::<Gasto>::None);
    let delete_target = use_state(|| Option::<Gasto>::None);
    let submitting = use_state(|| false);
    let form_errors = use_state(FormErrors::default);
    let reload = use_state(|| 0u32);

    let propiedad_id = use_state(String::new);
    let unidad_id = use_state(String::new);
    let categoria = use_state(String::new);
    let descripcion = use_state(String::new);
    let monto = use_state(String::new);
    let moneda = use_state(|| "DOP".to_string());
    let fecha_gasto = use_state(String::new);
    let estado_form = use_state(|| "pendiente".to_string());
    let proveedor = use_state(String::new);
    let numero_factura = use_state(String::new);
    let notas = use_state(String::new);

    let filter_propiedad = use_state(String::new);
    let filter_categoria = use_state(String::new);
    let filter_estado = use_state(String::new);

    let confidences = use_state(HashMap::<String, f64>::new);

    {
        let items = items.clone();
        let total = total.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        let pg = *page;
        let pp = *per_page;
        let f_propiedad = (*filter_propiedad).clone();
        let f_categoria = (*filter_categoria).clone();
        let f_estado = (*filter_estado).clone();
        use_effect_with(
            (reload_val, pg, f_propiedad, f_categoria, f_estado),
            move |(_, _, f_propiedad, f_categoria, f_estado)| {
                let url = build_gastos_url(pg, pp, f_propiedad, f_categoria, f_estado);
                load_gastos_data(items, total, error, loading, url);
            },
        );
    }

    {
        let propiedades = propiedades.clone();
        use_effect_with((), move |()| {
            load_gasto_refs(propiedades);
        });
    }

    let prop_label = {
        let propiedades = propiedades.clone();
        Callback::from(move |id: String| -> String {
            propiedades
                .iter()
                .find(|p| p.id == id)
                .map_or_else(|| id, |p| p.titulo.clone())
        })
    };

    let reset_form = {
        let propiedad_id = propiedad_id.clone();
        let unidad_id = unidad_id.clone();
        let categoria = categoria.clone();
        let descripcion = descripcion.clone();
        let monto = monto.clone();
        let moneda = moneda.clone();
        let fecha_gasto = fecha_gasto.clone();
        let estado_form = estado_form.clone();
        let proveedor = proveedor.clone();
        let numero_factura = numero_factura.clone();
        let notas = notas.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        let confidences = confidences.clone();
        move || {
            propiedad_id.set(String::new());
            unidad_id.set(String::new());
            categoria.set(String::new());
            descripcion.set(String::new());
            monto.set(String::new());
            moneda.set("DOP".into());
            fecha_gasto.set(String::new());
            estado_form.set("pendiente".into());
            proveedor.set(String::new());
            numero_factura.set(String::new());
            notas.set(String::new());
            editing.set(None);
            show_form.set(false);
            form_errors.set(FormErrors::default());
            confidences.set(HashMap::new());
        }
    };

    let escape_handler = use_mut_ref(|| None::<Box<dyn Fn()>>);
    {
        let delete_target = delete_target.clone();
        let show_form = show_form.clone();
        let reset_form = reset_form.clone();
        let handler = escape_handler.clone();
        *handler.borrow_mut() = Some(Box::new(move || {
            handle_escape_gastos(&delete_target, &show_form, &reset_form);
        }) as Box<dyn Fn()>);
    }
    {
        let escape_handler = escape_handler.clone();
        use_effect_with((), move |()| {
            let listener = register_escape_listener(escape_handler);
            move || drop(listener)
        });
    }

    let on_ocr_result = {
        let monto = monto.clone();
        let moneda = moneda.clone();
        let fecha_gasto = fecha_gasto.clone();
        let proveedor = proveedor.clone();
        let numero_factura = numero_factura.clone();
        let confidences = confidences.clone();
        Callback::from(move |fields: Vec<OcrExtractField>| {
            let mut conf_map = HashMap::new();
            for field in &fields {
                match field.name.as_str() {
                    "monto" => monto.set(field.value.clone()),
                    "moneda" => moneda.set(field.value.clone()),
                    "fecha" => fecha_gasto.set(field.value.clone()),
                    "proveedor" => proveedor.set(field.value.clone()),
                    "numero_factura" => numero_factura.set(field.value.clone()),
                    _ => {}
                }
                conf_map.insert(field.name.clone(), field.confidence);
            }
            conf_map.insert(
                "fecha_gasto".to_string(),
                conf_map.get("fecha").copied().unwrap_or(0.0),
            );
            confidences.set(conf_map);
        })
    };

    let on_confidence_clear = {
        let confidences = confidences.clone();
        Callback::from(move |name: String| {
            let mut c = (*confidences).clone();
            c.remove(&name);
            confidences.set(c);
        })
    };

    let on_propiedad_change = {
        let propiedad_id = propiedad_id.clone();
        let unidad_id = unidad_id.clone();
        Callback::from(move |e: Event| {
            let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
            propiedad_id.set(el.value());
            unidad_id.set(String::new());
        })
    };

    let on_new = super::page_helpers::new_cb(reset_form.clone(), &show_form, true);

    let on_edit = make_gasto_edit_cb(
        &propiedad_id,
        &unidad_id,
        &categoria,
        &descripcion,
        &monto,
        &moneda,
        &fecha_gasto,
        &estado_form,
        &proveedor,
        &numero_factura,
        &notas,
        &editing,
        &show_form,
        &form_errors,
    );

    let on_delete_click = super::page_helpers::delete_click_cb(&delete_target);

    let on_delete_confirm = {
        let error = error.clone();
        let reload = reload.clone();
        let delete_target = delete_target.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(ref g) = *delete_target {
                let label = format_currency(&g.moneda, g.monto);
                do_delete_gasto(
                    g.id.clone(),
                    label,
                    delete_target.clone(),
                    reload.clone(),
                    error.clone(),
                    toasts.clone(),
                    reload.clone(),
                );
            }
        })
    };

    let on_delete_cancel = super::page_helpers::delete_cancel_cb(&delete_target);

    let validate_form = {
        let propiedad_id = propiedad_id.clone();
        let categoria = categoria.clone();
        let descripcion = descripcion.clone();
        let monto = monto.clone();
        let fecha_gasto = fecha_gasto.clone();
        let form_errors = form_errors.clone();
        move || -> bool {
            let errs = validate_gasto_fields(
                &propiedad_id,
                &categoria,
                &descripcion,
                &monto,
                &fecha_gasto,
            );
            let valid = !errs.has_errors();
            form_errors.set(errs);
            valid
        }
    };

    let on_submit = {
        let propiedad_id = propiedad_id.clone();
        let unidad_id = unidad_id.clone();
        let categoria = categoria.clone();
        let descripcion = descripcion.clone();
        let monto = monto.clone();
        let moneda = moneda.clone();
        let fecha_gasto = fecha_gasto.clone();
        let estado_form = estado_form.clone();
        let proveedor = proveedor.clone();
        let numero_factura = numero_factura.clone();
        let notas = notas.clone();
        let editing = editing.clone();
        let error = error.clone();
        let reload = reload.clone();
        let reset_form = reset_form.clone();
        let submitting = submitting.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if *submitting || !validate_form() {
                return;
            }
            let Ok(monto_val) = monto.parse::<f64>() else {
                return;
            };
            submitting.set(true);
            let editing_id = editing.as_ref().map(|e| e.id.clone());
            let update = UpdateGasto {
                categoria: Some((*categoria).clone()),
                descripcion: Some((*descripcion).clone()),
                monto: Some(monto_val),
                moneda: Some((*moneda).clone()),
                fecha_gasto: Some((*fecha_gasto).clone()),
                unidad_id: non_empty(&unidad_id),
                proveedor: non_empty(&proveedor),
                numero_factura: non_empty(&numero_factura),
                estado: Some((*estado_form).clone()),
                notas: non_empty(&notas),
            };
            let create = CreateGasto {
                propiedad_id: (*propiedad_id).clone(),
                unidad_id: non_empty(&unidad_id),
                categoria: (*categoria).clone(),
                descripcion: (*descripcion).clone(),
                monto: monto_val,
                moneda: (*moneda).clone(),
                fecha_gasto: (*fecha_gasto).clone(),
                proveedor: non_empty(&proveedor),
                numero_factura: non_empty(&numero_factura),
                notas: non_empty(&notas),
            };
            do_save_gasto(
                editing_id,
                update,
                create,
                reset_form.clone(),
                reload.clone(),
                error.clone(),
                toasts.clone(),
                submitting.clone(),
            );
        })
    };

    let on_cancel = super::page_helpers::cancel_cb(reset_form);

    let on_filter_apply = super::page_helpers::filter_apply_cb(&page, &reload);
    let on_filter_clear = {
        let fp = filter_propiedad.clone();
        let fc = filter_categoria.clone();
        let fe = filter_estado.clone();
        super::page_helpers::filter_clear_cb(
            move || {
                fp.set(String::new());
                fc.set(String::new());
                fe.set(String::new());
            },
            &page,
            &reload,
        )
    };

    let (on_page_change, on_per_page_change) =
        super::page_helpers::pagination_cbs(&page, &per_page, &reload);

    let editing_id = editing.as_ref().map(|e| e.id.clone());
    let token = auth
        .as_ref()
        .and_then(|a| a.token.clone())
        .unwrap_or_default();

    render_gastos_view(
        &loading,
        &user_rol,
        &error,
        &delete_target,
        on_delete_confirm,
        on_delete_cancel,
        filter_propiedad,
        filter_categoria,
        filter_estado,
        &propiedades,
        on_filter_apply,
        on_filter_clear,
        &show_form,
        &editing,
        propiedad_id,
        unidad_id,
        categoria,
        descripcion,
        monto,
        moneda,
        fecha_gasto,
        estado_form,
        proveedor,
        numero_factura,
        notas,
        &form_errors,
        &submitting,
        on_submit,
        on_cancel,
        on_propiedad_change,
        &unidades,
        &confidences,
        on_ocr_result,
        on_confidence_clear,
        editing_id,
        token,
        &items,
        &total,
        &page,
        &per_page,
        on_edit,
        on_delete_click,
        on_new,
        on_page_change,
        on_per_page_change,
        &prop_label,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_gastos_view(
    loading: &UseStateHandle<bool>,
    user_rol: &str,
    error: &UseStateHandle<Option<String>>,
    delete_target: &UseStateHandle<Option<Gasto>>,
    on_delete_confirm: Callback<MouseEvent>,
    on_delete_cancel: Callback<MouseEvent>,
    filter_propiedad: UseStateHandle<String>,
    filter_categoria: UseStateHandle<String>,
    filter_estado: UseStateHandle<String>,
    propiedades: &UseStateHandle<Vec<Propiedad>>,
    on_filter_apply: Callback<MouseEvent>,
    on_filter_clear: Callback<MouseEvent>,
    show_form: &UseStateHandle<bool>,
    editing: &UseStateHandle<Option<Gasto>>,
    propiedad_id: UseStateHandle<String>,
    unidad_id: UseStateHandle<String>,
    categoria: UseStateHandle<String>,
    descripcion: UseStateHandle<String>,
    monto: UseStateHandle<String>,
    moneda: UseStateHandle<String>,
    fecha_gasto: UseStateHandle<String>,
    estado_form: UseStateHandle<String>,
    proveedor: UseStateHandle<String>,
    numero_factura: UseStateHandle<String>,
    notas: UseStateHandle<String>,
    form_errors: &UseStateHandle<FormErrors>,
    submitting: &UseStateHandle<bool>,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
    on_propiedad_change: Callback<Event>,
    unidades: &UseStateHandle<Vec<Unidad>>,
    confidences: &UseStateHandle<HashMap<String, f64>>,
    on_ocr_result: Callback<Vec<OcrExtractField>>,
    on_confidence_clear: Callback<String>,
    editing_id: Option<String>,
    token: String,
    items: &UseStateHandle<Vec<Gasto>>,
    total: &UseStateHandle<u64>,
    page: &UseStateHandle<u64>,
    per_page: &UseStateHandle<u64>,
    on_edit: Callback<Gasto>,
    on_delete_click: Callback<Gasto>,
    on_new: Callback<MouseEvent>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
    prop_label: &Callback<String, String>,
) -> Html {
    if **loading {
        return html! { <TableSkeleton title_width="160px" columns={7} has_filter=true /> };
    }

    let last_header: String = if can_write(user_rol) {
        "Acciones".into()
    } else {
        String::new()
    };
    let headers: Vec<String> = vec![
        "Propiedad".into(),
        "Categoría".into(),
        "Monto".into(),
        "Fecha".into(),
        "Proveedor".into(),
        "Estado".into(),
        last_header,
    ];

    let new_btn = if can_write(user_rol) {
        html! { <button onclick={on_new.clone()} class="gi-btn gi-btn-primary">{"+ Nuevo Gasto"}</button> }
    } else {
        html! {}
    };

    let error_html = (**error).as_ref().map_or_else(
        || html! {},
        |err| {
            let error = error.clone();
            html! { <ErrorBanner message={err.clone()} onclose={Callback::from(move |_: MouseEvent| error.set(None))} /> }
        },
    );

    let delete_html = (**delete_target).as_ref().map_or_else(
        || html! {},
        |g| html! {
            <DeleteConfirmModal
                message={format!("¿Está seguro de que desea eliminar el gasto de {}? Esta acción no se puede deshacer.", format_currency(&g.moneda, g.monto))}
                on_confirm={on_delete_confirm} on_cancel={on_delete_cancel}
            />
        },
    );

    let form_html = if **show_form {
        html! {
            <GastoForm
                is_editing={editing.is_some()}
                propiedad_id={propiedad_id} unidad_id={unidad_id}
                categoria={categoria} descripcion={descripcion}
                monto={monto} moneda={moneda} fecha_gasto={fecha_gasto}
                estado_form={estado_form} proveedor={proveedor}
                numero_factura={numero_factura} notas={notas}
                propiedades={(**propiedades).clone()} unidades={(**unidades).clone()}
                form_errors={(**form_errors).clone()} submitting={**submitting}
                on_submit={on_submit} on_cancel={on_cancel}
                on_propiedad_change={on_propiedad_change}
                confidences={(**confidences).clone()} on_ocr_result={on_ocr_result}
                on_confidence_clear={on_confidence_clear}
                editing_id={editing_id}
                token={token}
            />
        }
    } else {
        html! {}
    };

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Gastos"}</h1>
                {new_btn}
            </div>
            {error_html}
            {delete_html}
            <GastoFilterBar
                filter_propiedad={filter_propiedad} filter_categoria={filter_categoria}
                filter_estado={filter_estado}
                propiedades={(**propiedades).clone()}
                on_apply={on_filter_apply} on_clear={on_filter_clear}
            />
            {form_html}
            <GastoList
                items={(**items).clone()} user_rol={user_rol.to_string()} headers={headers}
                total={**total} page={**page} per_page={**per_page}
                prop_label={prop_label.clone()} on_edit={on_edit}
                on_delete={on_delete_click} on_new={on_new}
                on_page_change={on_page_change} on_per_page_change={on_per_page_change}
            />
        </div>
    }
}
