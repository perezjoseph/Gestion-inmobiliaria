use std::collections::HashMap;

use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthContext, Route};
use crate::components::common::confidence_input::ConfidenceInput;
use crate::components::common::currency_display::CurrencyDisplay;
use crate::components::common::data_table::DataTable;
use crate::components::common::delete_confirm_modal::DeleteConfirmModal;
use crate::components::common::document_gallery::DocumentGallery;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::ocr_scan_button::OcrScanButton;
use crate::components::common::skeleton::TableSkeleton;
use crate::components::common::pagination::Pagination;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{BASE_URL, api_delete, api_get, api_post, api_put};
use crate::types::PaginatedResponse;
use crate::types::contrato::Contrato;
use crate::types::inquilino::Inquilino;
use crate::types::ocr::OcrExtractField;
use crate::types::pago::{CreatePago, Pago, UpdatePago};
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

#[derive(Properties, PartialEq)]
struct PagoFilterBarProps {
    filter_contrato: UseStateHandle<String>,
    filter_estado: UseStateHandle<String>,
    contratos: Vec<Contrato>,
    contrato_label: Callback<String, String>,
    on_apply: Callback<MouseEvent>,
    on_clear: Callback<MouseEvent>,
}

#[component]
fn PagoFilterBar(props: &PagoFilterBarProps) -> Html {
    let fc = props.filter_contrato.clone();
    let on_contrato_change = Callback::from(move |e: Event| {
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
                    <label class="gi-label">{"Contrato"}</label>
                    <select onchange={on_contrato_change} class="gi-input">
                        <option value="" selected={props.filter_contrato.is_empty()}>{"Todos"}</option>
                        { for props.contratos.iter().map(|c| {
                            let sel = *props.filter_contrato == c.id;
                            let label = props.contrato_label.emit(c.id.clone());
                            html! { <option value={c.id.clone()} selected={sel}>{label}</option> }
                        })}
                    </select>
                </div>
                <div>
                    <label class="gi-label">{"Estado"}</label>
                    <select onchange={on_estado_change} class="gi-input">
                        <option value="" selected={props.filter_estado.is_empty()}>{"Todos"}</option>
                        <option value="pendiente" selected={*props.filter_estado == "pendiente"}>{"Pendiente"}</option>
                        <option value="pagado" selected={*props.filter_estado == "pagado"}>{"Pagado"}</option>
                        <option value="atrasado" selected={*props.filter_estado == "atrasado"}>{"Atrasado"}</option>
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

fn estado_badge(estado: &str) -> (&'static str, &'static str) {
    match estado {
        "pagado" => ("gi-badge gi-badge-success", "Pagado"),
        "pendiente" => ("gi-badge gi-badge-warning", "Pendiente"),
        "atrasado" => ("gi-badge gi-badge-error", "Atrasado"),
        _ => ("gi-badge gi-badge-neutral", "Desconocido"),
    }
}

fn metodo_label(metodo: &str) -> &str {
    match metodo {
        "efectivo" => "Efectivo",
        "transferencia" => "Transferencia",
        "cheque" => "Cheque",
        _ => metodo,
    }
}

#[derive(Clone, Default, PartialEq)]
struct FormErrors {
    contrato_id: Option<String>,
    monto: Option<String>,
    fecha_vencimiento: Option<String>,
}

impl FormErrors {
    const fn has_errors(&self) -> bool {
        self.contrato_id.is_some() || self.monto.is_some() || self.fecha_vencimiento.is_some()
    }
}

#[derive(Properties, PartialEq)]
struct PagoFormProps {
    is_editing: bool,
    contrato_id: UseStateHandle<String>,
    monto: UseStateHandle<String>,
    moneda: UseStateHandle<String>,
    fecha_pago: UseStateHandle<String>,
    fecha_vencimiento: UseStateHandle<String>,
    metodo_pago: UseStateHandle<String>,
    estado_form: UseStateHandle<String>,
    notas: UseStateHandle<String>,
    contratos: Vec<Contrato>,
    contrato_label: Callback<String, String>,
    form_errors: FormErrors,
    submitting: bool,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
    confidences: HashMap<String, f64>,
    on_ocr_result: Callback<Vec<OcrExtractField>>,
    on_confidence_clear: Callback<String>,
    #[prop_or_default]
    editing_id: Option<String>,
    #[prop_or_default]
    token: String,
}

#[component]
fn PagoForm(props: &PagoFormProps) -> Html {
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

    let confidence_for = |name: &str| -> Option<f64> {
        props.confidences.get(name).copied()
    };

    let input_cb_conf = |state: &UseStateHandle<String>, field_name: &str| -> Callback<InputEvent> {
        let s = state.clone();
        let clear = props.on_confidence_clear.clone();
        let name = field_name.to_string();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            s.set(input.value());
            clear.emit(name.clone());
        })
    };

    let scan_button = if props.is_editing {
        html! {}
    } else {
        html! {
            <OcrScanButton
                document_type="deposito_bancario"
                on_result={props.on_ocr_result.clone()}
                label={AttrValue::from("📷 Escanear Recibo")}
            />
        }
    };

    html! {
        <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
            <div style="display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-4);">
                <h2 class="text-display" style="font-size: var(--text-lg); font-weight: 600; color: var(--text-primary);">
                    {if props.is_editing { "Editar Pago" } else { "Nuevo Pago" }}</h2>
                {scan_button}
            </div>
            <form onsubmit={props.on_submit.clone()} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                <div>
                    <label class="gi-label">{"Contrato *"}</label>
                    <select onchange={select_cb!(props.contrato_id)} disabled={props.is_editing}
                        class={input_class(fe.contrato_id.is_some())}>
                        <option value="" selected={props.contrato_id.is_empty()}>{"— Seleccionar contrato —"}</option>
                        { for props.contratos.iter().map(|c| {
                            let sel = *props.contrato_id == c.id;
                            let label = props.contrato_label.emit(c.id.clone());
                            html! { <option value={c.id.clone()} selected={sel}>{label}</option> }
                        })}
                    </select>
                    {field_error(&fe.contrato_id)}
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
                    <label class="gi-label">{"Fecha de Vencimiento *"}</label>
                    <input type="date" value={(*props.fecha_vencimiento).clone()} oninput={input_cb!(props.fecha_vencimiento)} disabled={props.is_editing}
                        class={input_class(fe.fecha_vencimiento.is_some())} />
                    {field_error(&fe.fecha_vencimiento)}
                </div>
                <div>
                    <label class="gi-label">{"Fecha de Pago"}</label>
                    <ConfidenceInput
                        value={AttrValue::from((*props.fecha_pago).clone())}
                        confidence={confidence_for("fecha_pago")}
                        oninput={input_cb_conf(&props.fecha_pago, "fecha_pago")}
                        input_type="date"
                    />
                </div>
                <div>
                    <label class="gi-label">{"Método de Pago"}</label>
                    <select onchange={select_cb!(props.metodo_pago)} class="gi-input">
                        <option value="" selected={props.metodo_pago.is_empty()}>{"— Sin especificar —"}</option>
                        <option value="efectivo" selected={*props.metodo_pago == "efectivo"}>{"Efectivo"}</option>
                        <option value="transferencia" selected={*props.metodo_pago == "transferencia"}>{"Transferencia"}</option>
                        <option value="cheque" selected={*props.metodo_pago == "cheque"}>{"Cheque"}</option>
                    </select>
                </div>
                if props.is_editing {
                    <div>
                        <label class="gi-label">{"Estado"}</label>
                        <select onchange={select_cb!(props.estado_form)} class="gi-input">
                            <option value="pendiente" selected={*props.estado_form == "pendiente"}>{"Pendiente"}</option>
                            <option value="pagado" selected={*props.estado_form == "pagado"}>{"Pagado"}</option>
                            <option value="atrasado" selected={*props.estado_form == "atrasado"}>{"Atrasado"}</option>
                        </select>
                    </div>
                }
                <div style="grid-column: 1 / -1;">
                    <label class="gi-label">{"Notas"}</label>
                    <ConfidenceInput
                        value={AttrValue::from((*props.notas).clone())}
                        confidence={confidence_for("notas")}
                        oninput={input_cb_conf(&props.notas, "notas")}
                        placeholder={AttrValue::from("Notas opcionales")}
                    />
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
                        entity_type={"pago".to_string()}
                        entity_id={id.clone()}
                        token={props.token.clone()}
                    />
                </div>
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct PagoListProps {
    items: Vec<Pago>,
    user_rol: String,
    headers: Vec<String>,
    total: u64,
    page: u64,
    per_page: u64,
    contrato_label: Callback<String, String>,
    on_edit: Callback<Pago>,
    on_delete: Callback<Pago>,
    on_new: Callback<MouseEvent>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
}

fn render_pago_row(
    p: &Pago,
    user_rol: &str,
    contrato_label: &Callback<String, String>,
    on_edit: &Callback<Pago>,
    on_delete: &Callback<Pago>,
) -> Html {
    let c_label = contrato_label.emit(p.contrato_id.clone());
    let (badge_cls, badge_label) = estado_badge(&p.estado);
    let pc = p.clone();
    let pd = p.clone();
    let on_edit = on_edit.clone();
    let on_delete_click = on_delete.clone();
    let actions = render_pago_actions(user_rol, &p.estado, &p.id, pc, pd, on_edit, on_delete_click);
    html! {
        <tr>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{c_label}</td>
            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);"><CurrencyDisplay monto={p.monto} moneda={p.moneda.clone()} /></td>
            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">
                {p.fecha_pago.as_deref().map_or_else(|| "—".into(), format_date_display)}</td>
            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{format_date_display(&p.fecha_vencimiento)}</td>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">
                {p.metodo_pago.as_deref().map_or("—", metodo_label)}</td>
            <td style="padding: var(--space-3) var(--space-5);"><span class={badge_cls}>{badge_label}</span></td>
            {actions}
        </tr>
    }
}

fn render_pago_actions(
    user_rol: &str,
    estado: &str,
    pago_id: &str,
    pc: Pago,
    pd: Pago,
    on_edit: Callback<Pago>,
    on_delete_click: Callback<Pago>,
) -> Html {
    if !can_write(user_rol) {
        return html! {};
    }
    let recibo_btn = if estado == "pagado" {
        let recibo_id = pago_id.to_string();
        html! {
            <button onclick={Callback::from(move |_: MouseEvent| {
                let url = format!("{BASE_URL}/pagos/{recibo_id}/recibo");
                let _ = web_sys::window().and_then(|w| w.open_with_url(&url).ok());
            })} class="gi-btn-text" style="color: var(--color-primary-500);">{"Recibo"}</button>
        }
    } else {
        html! {}
    };
    let delete_btn = if can_delete(user_rol) {
        html! { <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(pd.clone()))} class="gi-btn-text" style="color: var(--color-error);">{"Eliminar"}</button> }
    } else {
        html! {}
    };
    html! {
        <td style="padding: var(--space-3) var(--space-5); display: flex; gap: var(--space-2);">
            <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(pc.clone()))} class="gi-btn-text">{"Editar"}</button>
            {recibo_btn}
            {delete_btn}
        </td>
    }
}

fn render_pago_empty_state(user_rol: &str, on_new: &Callback<MouseEvent>) -> Html {
    html! {
        <div class="gi-empty-state">
            <div class="gi-empty-state-icon">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4">
                    <line x1="12" y1="1" x2="12" y2="23"/>
                    <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>
                </svg>
            </div>
            <div class="gi-empty-state-title">{"Sin pagos registrados"}</div>
            <p class="gi-empty-state-text">{"Los pagos se registran contra contratos activos. Necesita al menos un contrato vigente para comenzar a registrar cobros."}</p>
            if can_write(user_rol) {
                <button onclick={on_new.clone()} class="gi-btn gi-btn-primary" style="margin-top: var(--space-3);">{"+ Nuevo Pago"}</button>
            }
            <div class="gi-empty-state-hint">
                {"¿No tiene contratos? "}
                <Link<Route> to={Route::Contratos} classes="gi-btn-text">
                    {"Crear contrato primero"}
                </Link<Route>>
            </div>
        </div>
    }
}

#[component]
fn PagoList(props: &PagoListProps) -> Html {
    if props.items.is_empty() {
        return render_pago_empty_state(&props.user_rol, &props.on_new);
    }

    html! {
        <>
            <DataTable headers={props.headers.clone()}>
                { for props.items.iter().map(|p| render_pago_row(p, &props.user_rol, &props.contrato_label, &props.on_edit, &props.on_delete)) }
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

#[allow(clippy::too_many_arguments)]
fn make_pago_edit_cb(
    contrato_id: &UseStateHandle<String>,
    monto: &UseStateHandle<String>,
    moneda: &UseStateHandle<String>,
    fecha_pago: &UseStateHandle<String>,
    fecha_vencimiento: &UseStateHandle<String>,
    metodo_pago: &UseStateHandle<String>,
    estado_form: &UseStateHandle<String>,
    notas: &UseStateHandle<String>,
    editing: &UseStateHandle<Option<Pago>>,
    show_form: &UseStateHandle<bool>,
    form_errors: &UseStateHandle<FormErrors>,
) -> Callback<Pago> {
    let (contrato_id, monto, moneda) = (contrato_id.clone(), monto.clone(), moneda.clone());
    let (fecha_pago, fecha_vencimiento, metodo_pago) = (
        fecha_pago.clone(),
        fecha_vencimiento.clone(),
        metodo_pago.clone(),
    );
    let (estado_form, notas) = (estado_form.clone(), notas.clone());
    let (editing, show_form, form_errors) =
        (editing.clone(), show_form.clone(), form_errors.clone());
    Callback::from(move |p: Pago| {
        contrato_id.set(p.contrato_id.clone());
        monto.set(p.monto.to_string());
        moneda.set(p.moneda.clone());
        fecha_pago.set(p.fecha_pago.clone().unwrap_or_default());
        fecha_vencimiento.set(p.fecha_vencimiento.clone());
        metodo_pago.set(p.metodo_pago.clone().unwrap_or_default());
        estado_form.set(p.estado.clone());
        notas.set(p.notas.clone().unwrap_or_default());
        editing.set(Some(p));
        show_form.set(true);
        form_errors.set(FormErrors::default());
    })
}

fn build_pagos_url(pg: u64, pp: u64, f_contrato: &str, f_estado: &str) -> String {
    let mut params = vec![format!("page={pg}"), format!("perPage={pp}")];
    if !f_contrato.is_empty() {
        params.push(format!("contratoId={f_contrato}"));
    }
    if !f_estado.is_empty() {
        params.push(format!("estado={f_estado}"));
    }
    format!("/pagos?{}", params.join("&"))
}

fn load_pago_refs(
    contratos: UseStateHandle<Vec<Contrato>>,
    propiedades: UseStateHandle<Vec<Propiedad>>,
    inquilinos_list: UseStateHandle<Vec<Inquilino>>,
) {
    spawn_local(async move {
        if let Ok(resp) = api_get::<PaginatedResponse<Contrato>>("/contratos?perPage=100").await {
            contratos.set(resp.data);
        }
        if let Ok(resp) = api_get::<PaginatedResponse<Propiedad>>("/propiedades?perPage=200").await
        {
            propiedades.set(resp.data);
        }
        if let Ok(resp) = api_get::<PaginatedResponse<Inquilino>>("/inquilinos?perPage=200").await {
            inquilinos_list.set(resp.data);
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

fn validate_pago_fields(contrato_id: &str, monto: &str, fecha_vencimiento: &str) -> FormErrors {
    let mut errs = FormErrors::default();
    if contrato_id.is_empty() {
        errs.contrato_id = Some("Debe seleccionar un contrato".into());
    }
    match monto.parse::<f64>() {
        Ok(v) if v <= 0.0 => errs.monto = Some("El monto debe ser mayor a 0".into()),
        Err(_) => errs.monto = Some("Monto inválido".into()),
        _ => {}
    }
    if fecha_vencimiento.is_empty() {
        errs.fecha_vencimiento = Some("La fecha de vencimiento es obligatoria".into());
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

fn format_contrato_label(
    id: &str,
    contratos: &[Contrato],
    propiedades: &[Propiedad],
    inquilinos: &[Inquilino],
) -> String {
    contratos
        .iter()
        .find(|c| c.id == id)
        .map_or_else(
            || id.to_string(),
            |c| {
                let prop_name = propiedades
                    .iter()
                    .find(|p| p.id == c.propiedad_id)
                    .map_or("—", |p| p.titulo.as_str());
                let tenant_name = inquilinos
                    .iter()
                    .find(|i| i.id == c.inquilino_id)
                    .map_or_else(String::new, |i| format!("{} {}", i.nombre, i.apellido));
                if tenant_name.is_empty() {
                    format!("{} — {} {}", prop_name, c.moneda, c.monto_mensual)
                } else {
                    format!(
                        "{} ({}) — {} {}",
                        prop_name, tenant_name, c.moneda, c.monto_mensual
                    )
                }
            },
        )
}

#[allow(clippy::too_many_arguments)]
fn do_save_pago(
    editing_id: Option<String>,
    update: UpdatePago,
    create: CreatePago,
    reset_form: impl Fn() + 'static,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
    submitting: UseStateHandle<bool>,
) {
    spawn_local(async move {
        let res = match editing_id {
            Some(id) => api_put::<Pago, _>(&format!("/pagos/{id}"), &update)
                .await
                .map(|_| ()),
            None => api_post::<Pago, _>("/pagos", &create).await.map(|_| ()),
        };
        match res {
            Ok(()) => {
                reset_form();
                reload.set(*reload + 1);
                push_toast(toasts.as_ref(), "Pago guardado", ToastKind::Success);
            }
            Err(err) => error.set(Some(err)),
        }
        submitting.set(false);
    });
}

fn do_delete_pago(
    id: String,
    label: String,
    delete_target: UseStateHandle<Option<Pago>>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
    reload_for_undo: UseStateHandle<u32>,
) {
    spawn_local(async move {
        match api_delete(&format!("/pagos/{id}")).await {
            Ok(()) => {
                delete_target.set(None);
                reload.set(*reload + 1);
                let undo_reload = reload_for_undo;
                if let Some(t) = &toasts {
                    t.dispatch(ToastAction::PushWithUndo(
                        format!("Pago de {label} eliminado"),
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

fn load_pagos_data(
    items: UseStateHandle<Vec<Pago>>,
    total: UseStateHandle<u64>,
    error: UseStateHandle<Option<String>>,
    loading: UseStateHandle<bool>,
    url: String,
) {
    spawn_local(async move {
        loading.set(true);
        match api_get::<PaginatedResponse<Pago>>(&url).await {
            Ok(resp) => {
                total.set(resp.total);
                items.set(resp.data);
            }
            Err(err) => error.set(Some(err)),
        }
        loading.set(false);
    });
}

#[allow(clippy::too_many_arguments)]
fn handle_pago_submit(
    submitting: &UseStateHandle<bool>,
    validate_form: &dyn Fn() -> bool,
    monto: &UseStateHandle<String>,
    contrato_id: &UseStateHandle<String>,
    moneda: &UseStateHandle<String>,
    fecha_pago: &UseStateHandle<String>,
    fecha_vencimiento: &UseStateHandle<String>,
    metodo_pago: &UseStateHandle<String>,
    estado_form: &UseStateHandle<String>,
    notas: &UseStateHandle<String>,
    editing: &UseStateHandle<Option<Pago>>,
    reset_form: impl Fn() + 'static,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
) {
    if **submitting || !validate_form() {
        return;
    }
    let Ok(monto_val) = monto.parse::<f64>() else {
        return;
    };
    submitting.set(true);
    let editing_id = editing.as_ref().map(|e| e.id.clone());
    let update = UpdatePago {
        monto: Some(monto_val),
        fecha_pago: non_empty(fecha_pago),
        metodo_pago: non_empty(metodo_pago),
        estado: Some((**estado_form).clone()),
        notas: non_empty(notas),
    };
    let create = CreatePago {
        contrato_id: (**contrato_id).clone(),
        monto: monto_val,
        moneda: Some((**moneda).clone()),
        fecha_pago: non_empty(fecha_pago),
        fecha_vencimiento: (**fecha_vencimiento).clone(),
        metodo_pago: non_empty(metodo_pago),
        notas: non_empty(notas),
    };
    do_save_pago(
        editing_id,
        update,
        create,
        reset_form,
        reload,
        error,
        toasts,
        submitting.clone(),
    );
}

fn handle_pago_delete_confirm(
    delete_target: &UseStateHandle<Option<Pago>>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
) {
    if let Some(ref p) = **delete_target {
        let label = format_currency(&p.moneda, p.monto);
        do_delete_pago(
            p.id.clone(),
            label,
            delete_target.clone(),
            reload.clone(),
            error,
            toasts,
            reload,
        );
    }
}

fn handle_escape_pagos(
    delete_target: &UseStateHandle<Option<Pago>>,
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
pub fn Pagos() -> Html {
    let auth = use_context::<AuthContext>();
    let toasts = use_context::<ToastContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();

    let items = use_state(Vec::<Pago>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let contratos = use_state(Vec::<Contrato>::new);
    let propiedades = use_state(Vec::<Propiedad>::new);
    let inquilinos_list = use_state(Vec::<Inquilino>::new);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let show_form = use_state(|| false);
    let editing = use_state(|| Option::<Pago>::None);
    let delete_target = use_state(|| Option::<Pago>::None);
    let submitting = use_state(|| false);
    let form_errors = use_state(FormErrors::default);
    let reload = use_state(|| 0u32);

    let contrato_id = use_state(String::new);
    let monto = use_state(String::new);
    let moneda = use_state(|| "DOP".to_string());
    let fecha_pago = use_state(String::new);
    let fecha_vencimiento = use_state(String::new);
    let metodo_pago = use_state(String::new);
    let estado_form = use_state(|| "pendiente".to_string());
    let notas = use_state(String::new);

    let filter_contrato = use_state(String::new);
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
        let f_contrato = (*filter_contrato).clone();
        let f_estado = (*filter_estado).clone();
        use_effect_with(
            (reload_val, pg, f_contrato, f_estado),
            move |(_, _, f_contrato, f_estado)| {
                let url = build_pagos_url(pg, pp, f_contrato, f_estado);
                load_pagos_data(items, total, error, loading, url);
            },
        );
    }

    {
        let contratos = contratos.clone();
        let propiedades = propiedades.clone();
        let inquilinos_list = inquilinos_list.clone();
        use_effect_with((), move |()| {
            load_pago_refs(contratos, propiedades, inquilinos_list);
        });
    }

    let contrato_label = {
        let contratos = contratos.clone();
        Callback::from(move |id: String| -> String {
            format_contrato_label(&id, &contratos, &propiedades, &inquilinos_list)
        })
    };

    let reset_form = {
        let contrato_id = contrato_id.clone();
        let monto = monto.clone();
        let moneda = moneda.clone();
        let fecha_pago = fecha_pago.clone();
        let fecha_vencimiento = fecha_vencimiento.clone();
        let metodo_pago = metodo_pago.clone();
        let estado_form = estado_form.clone();
        let notas = notas.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        let confidences = confidences.clone();
        move || {
            contrato_id.set(String::new());
            monto.set(String::new());
            moneda.set("DOP".into());
            fecha_pago.set(String::new());
            fecha_vencimiento.set(String::new());
            metodo_pago.set(String::new());
            estado_form.set("pendiente".into());
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
            handle_escape_pagos(&delete_target, &show_form, &reset_form);
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
        let fecha_pago = fecha_pago.clone();
        let metodo_pago = metodo_pago.clone();
        let notas = notas.clone();
        let confidences = confidences.clone();
        Callback::from(move |fields: Vec<OcrExtractField>| {
            let mut conf_map = HashMap::new();
            let mut referencia = String::new();
            let mut cuenta = String::new();
            for field in &fields {
                match field.name.as_str() {
                    "monto" => monto.set(field.value.clone()),
                    "moneda" => moneda.set(field.value.clone()),
                    "fecha" => fecha_pago.set(field.value.clone()),
                    "referencia" => referencia.clone_from(&field.value),
                    "cuenta" => cuenta.clone_from(&field.value),
                    _ => {}
                }
                conf_map.insert(field.name.clone(), field.confidence);
            }
            metodo_pago.set("transferencia".into());
            let notas_parts: Vec<&str> = [referencia.as_str(), cuenta.as_str()]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
            if !notas_parts.is_empty() {
                notas.set(notas_parts.join(" "));
                let max_conf = [
                    conf_map.get("referencia").copied(),
                    conf_map.get("cuenta").copied(),
                ]
                .into_iter()
                .flatten()
                .fold(0.0_f64, f64::max);
                conf_map.insert("notas".to_string(), max_conf);
            }
            conf_map.insert("fecha_pago".to_string(),
                conf_map.get("fecha").copied().unwrap_or(0.0));
            conf_map.insert("metodo_pago".to_string(), 1.0);
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

    let on_new = super::page_helpers::new_cb(reset_form.clone(), &show_form, true);

    let on_edit = make_pago_edit_cb(
        &contrato_id,
        &monto,
        &moneda,
        &fecha_pago,
        &fecha_vencimiento,
        &metodo_pago,
        &estado_form,
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
            handle_pago_delete_confirm(
                &delete_target,
                reload.clone(),
                error.clone(),
                toasts.clone(),
            );
        })
    };

    let on_delete_cancel = super::page_helpers::delete_cancel_cb(&delete_target);

    let validate_form = {
        let contrato_id = contrato_id.clone();
        let monto = monto.clone();
        let fecha_vencimiento = fecha_vencimiento.clone();
        let form_errors = form_errors.clone();
        move || -> bool {
            let errs = validate_pago_fields(&contrato_id, &monto, &fecha_vencimiento);
            let valid = !errs.has_errors();
            form_errors.set(errs);
            valid
        }
    };

    let on_submit = {
        let contrato_id = contrato_id.clone();
        let monto = monto.clone();
        let moneda = moneda.clone();
        let fecha_pago = fecha_pago.clone();
        let fecha_vencimiento = fecha_vencimiento.clone();
        let metodo_pago = metodo_pago.clone();
        let estado_form = estado_form.clone();
        let notas = notas.clone();
        let editing = editing.clone();
        let error = error.clone();
        let reload = reload.clone();
        let reset_form = reset_form.clone();
        let submitting = submitting.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            handle_pago_submit(
                &submitting,
                &validate_form,
                &monto,
                &contrato_id,
                &moneda,
                &fecha_pago,
                &fecha_vencimiento,
                &metodo_pago,
                &estado_form,
                &notas,
                &editing,
                reset_form.clone(),
                reload.clone(),
                error.clone(),
                toasts.clone(),
            );
        })
    };

    let on_cancel = super::page_helpers::cancel_cb(reset_form);

    let on_filter_apply = super::page_helpers::filter_apply_cb(&page, &reload);
    let on_filter_clear = {
        let fc = filter_contrato.clone();
        let fe = filter_estado.clone();
        super::page_helpers::filter_clear_cb(
            move || {
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

    render_pagos_view(
        &loading,
        &user_rol,
        &error,
        &delete_target,
        on_delete_confirm,
        on_delete_cancel,
        filter_contrato,
        filter_estado,
        &contratos,
        &contrato_label,
        on_filter_apply,
        on_filter_clear,
        &show_form,
        &editing,
        contrato_id,
        monto,
        moneda,
        fecha_pago,
        fecha_vencimiento,
        metodo_pago,
        estado_form,
        notas,
        &form_errors,
        &submitting,
        on_submit,
        on_cancel,
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
    )
}

#[allow(clippy::too_many_arguments)]
fn render_pagos_view(
    loading: &UseStateHandle<bool>,
    user_rol: &str,
    error: &UseStateHandle<Option<String>>,
    delete_target: &UseStateHandle<Option<Pago>>,
    on_delete_confirm: Callback<MouseEvent>,
    on_delete_cancel: Callback<MouseEvent>,
    filter_contrato: UseStateHandle<String>,
    filter_estado: UseStateHandle<String>,
    contratos: &UseStateHandle<Vec<Contrato>>,
    contrato_label: &Callback<String, String>,
    on_filter_apply: Callback<MouseEvent>,
    on_filter_clear: Callback<MouseEvent>,
    show_form: &UseStateHandle<bool>,
    editing: &UseStateHandle<Option<Pago>>,
    contrato_id: UseStateHandle<String>,
    monto: UseStateHandle<String>,
    moneda: UseStateHandle<String>,
    fecha_pago: UseStateHandle<String>,
    fecha_vencimiento: UseStateHandle<String>,
    metodo_pago: UseStateHandle<String>,
    estado_form: UseStateHandle<String>,
    notas: UseStateHandle<String>,
    form_errors: &UseStateHandle<FormErrors>,
    submitting: &UseStateHandle<bool>,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
    confidences: &UseStateHandle<HashMap<String, f64>>,
    on_ocr_result: Callback<Vec<OcrExtractField>>,
    on_confidence_clear: Callback<String>,
    editing_id: Option<String>,
    token: String,
    items: &UseStateHandle<Vec<Pago>>,
    total: &UseStateHandle<u64>,
    page: &UseStateHandle<u64>,
    per_page: &UseStateHandle<u64>,
    on_edit: Callback<Pago>,
    on_delete_click: Callback<Pago>,
    on_new: Callback<MouseEvent>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
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
        "Contrato".into(),
        "Monto".into(),
        "Fecha Pago".into(),
        "Vencimiento".into(),
        "Método".into(),
        "Estado".into(),
        last_header,
    ];
    let new_btn = render_new_btn_pago(user_rol, &on_new);
    let error_html = render_opt_error_pago(error, &on_delete_cancel);
    let delete_html =
        render_delete_confirm_pago(delete_target, &on_delete_confirm, &on_delete_cancel);
    let form_html = render_pago_form_section(
        show_form,
        editing,
        contrato_id,
        monto,
        moneda,
        fecha_pago,
        fecha_vencimiento,
        metodo_pago,
        estado_form,
        notas,
        contratos,
        contrato_label,
        form_errors,
        submitting,
        on_submit,
        on_cancel,
        confidences,
        on_ocr_result,
        on_confidence_clear,
        editing_id,
        token,
    );

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Pagos"}</h1>
                {new_btn}
            </div>
            {error_html}
            {delete_html}
            <PagoFilterBar
                filter_contrato={filter_contrato} filter_estado={filter_estado}
                contratos={(**contratos).clone()} contrato_label={contrato_label.clone()}
                on_apply={on_filter_apply} on_clear={on_filter_clear}
            />
            {form_html}
            <PagoList
                items={(**items).clone()} user_rol={user_rol.to_string()} headers={headers}
                total={**total} page={**page} per_page={**per_page}
                contrato_label={contrato_label.clone()} on_edit={on_edit}
                on_delete={on_delete_click} on_new={on_new}
                on_page_change={on_page_change} on_per_page_change={on_per_page_change}
            />
        </div>
    }
}

fn render_new_btn_pago(user_rol: &str, on_new: &Callback<MouseEvent>) -> Html {
    if can_write(user_rol) {
        html! { <button onclick={on_new.clone()} class="gi-btn gi-btn-primary">{"+ Nuevo Pago"}</button> }
    } else {
        html! {}
    }
}

fn render_opt_error_pago(
    error: &UseStateHandle<Option<String>>,
    _on_cancel: &Callback<MouseEvent>,
) -> Html {
    (**error).as_ref().map_or_else(
        || html! {},
        |err| {
            let error = error.clone();
            html! { <ErrorBanner message={err.clone()} onclose={Callback::from(move |_: MouseEvent| error.set(None))} /> }
        },
    )
}

fn render_delete_confirm_pago(
    target: &UseStateHandle<Option<Pago>>,
    on_confirm: &Callback<MouseEvent>,
    on_cancel: &Callback<MouseEvent>,
) -> Html {
    (**target).as_ref().map_or_else(
        || html! {},
        |p| html! {
            <DeleteConfirmModal
                message={format!("¿Está seguro de que desea eliminar el pago de {}? Esta acción no se puede deshacer.", format_currency(&p.moneda, p.monto))}
                on_confirm={on_confirm.clone()} on_cancel={on_cancel.clone()}
            />
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn render_pago_form_section(
    show_form: &UseStateHandle<bool>,
    editing: &UseStateHandle<Option<Pago>>,
    contrato_id: UseStateHandle<String>,
    monto: UseStateHandle<String>,
    moneda: UseStateHandle<String>,
    fecha_pago: UseStateHandle<String>,
    fecha_vencimiento: UseStateHandle<String>,
    metodo_pago: UseStateHandle<String>,
    estado_form: UseStateHandle<String>,
    notas: UseStateHandle<String>,
    contratos: &UseStateHandle<Vec<Contrato>>,
    contrato_label: &Callback<String, String>,
    form_errors: &UseStateHandle<FormErrors>,
    submitting: &UseStateHandle<bool>,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
    confidences: &UseStateHandle<HashMap<String, f64>>,
    on_ocr_result: Callback<Vec<OcrExtractField>>,
    on_confidence_clear: Callback<String>,
    editing_id: Option<String>,
    token: String,
) -> Html {
    if !**show_form {
        return html! {};
    }
    html! {
        <PagoForm
            is_editing={editing.is_some()}
            contrato_id={contrato_id} monto={monto} moneda={moneda}
            fecha_pago={fecha_pago} fecha_vencimiento={fecha_vencimiento}
            metodo_pago={metodo_pago} estado_form={estado_form} notas={notas}
            contratos={(**contratos).clone()} contrato_label={contrato_label.clone()}
            form_errors={(**form_errors).clone()} submitting={**submitting}
            on_submit={on_submit} on_cancel={on_cancel}
            confidences={(**confidences).clone()} on_ocr_result={on_ocr_result}
            on_confidence_clear={on_confidence_clear}
            editing_id={editing_id}
            token={token}
        />
    }
}
