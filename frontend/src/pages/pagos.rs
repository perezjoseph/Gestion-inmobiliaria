use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthContext, Route};
use crate::components::common::currency_display::CurrencyDisplay;
use crate::components::common::data_table::DataTable;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::loading::Loading;
use crate::components::common::pagination::Pagination;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{BASE_URL, api_delete, api_get, api_post, api_put};
use crate::types::PaginatedResponse;
use crate::types::contrato::Contrato;
use crate::types::inquilino::Inquilino;
use crate::types::pago::{CreatePago, Pago, UpdatePago};
use crate::types::propiedad::Propiedad;
use crate::utils::{can_delete, can_write, format_currency, format_date_display};

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
    fn has_errors(&self) -> bool {
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
}

#[function_component]
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

    html! {
        <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
            <h2 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">
                {if props.is_editing { "Editar Pago" } else { "Nuevo Pago" }}</h2>
            <form onsubmit={props.on_submit.clone()} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                <div>
                    <label class="gi-label">{"Contrato *"}</label>
                    <select onchange={select_cb!(props.contrato_id)} disabled={props.is_editing}
                        class={if fe.contrato_id.is_some() { "gi-input gi-input-error" } else { "gi-input" }}>
                        <option value="" selected={props.contrato_id.is_empty()}>{"— Seleccionar contrato —"}</option>
                        { for props.contratos.iter().map(|c| {
                            let sel = *props.contrato_id == c.id;
                            let label = props.contrato_label.emit(c.id.clone());
                            html! { <option value={c.id.clone()} selected={sel}>{label}</option> }
                        })}
                    </select>
                    if let Some(ref msg) = fe.contrato_id { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label">{"Monto *"}</label>
                    <input type="number" step="0.01" min="0" value={(*props.monto).clone()} oninput={input_cb!(props.monto)}
                        class={if fe.monto.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                    if let Some(ref msg) = fe.monto { <p class="gi-field-error">{msg}</p> }
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
                        class={if fe.fecha_vencimiento.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                    if let Some(ref msg) = fe.fecha_vencimiento { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label">{"Fecha de Pago"}</label>
                    <input type="date" value={(*props.fecha_pago).clone()} oninput={input_cb!(props.fecha_pago)} class="gi-input" />
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
                    <input type="text" value={(*props.notas).clone()} oninput={input_cb!(props.notas)} class="gi-input" placeholder="Notas opcionales" />
                </div>
                <div style="grid-column: 1 / -1; display: flex; gap: var(--space-2); justify-content: flex-end;">
                    <button type="button" onclick={props.on_cancel.clone()} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                    <button type="submit" disabled={props.submitting} class="gi-btn gi-btn-primary">
                        {if props.submitting { "Guardando..." } else { "Guardar" }}
                    </button>
                </div>
            </form>
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

#[function_component]
fn PagoList(props: &PagoListProps) -> Html {
    if props.items.is_empty() {
        return html! {
            <div class="gi-empty-state">
                <div class="gi-empty-state-icon">
                    <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4">
                        <line x1="12" y1="1" x2="12" y2="23"/>
                        <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>
                    </svg>
                </div>
                <div class="gi-empty-state-title">{"Sin pagos registrados"}</div>
                <p class="gi-empty-state-text">{"Los pagos se registran contra contratos activos. Necesita al menos un contrato vigente para comenzar a registrar cobros."}</p>
                if can_write(&props.user_rol) {
                    <button onclick={props.on_new.clone()} class="gi-btn gi-btn-primary" style="margin-top: var(--space-3);">{"+ Nuevo Pago"}</button>
                }
                <div class="gi-empty-state-hint">
                    {"¿No tiene contratos? "}
                    <Link<Route> to={Route::Contratos} classes="gi-btn-text">
                        {"Crear contrato primero"}
                    </Link<Route>>
                </div>
            </div>
        };
    }

    html! {
        <>
            <DataTable headers={props.headers.clone()}>
                { for props.items.iter().map(|p| {
                    let on_edit = props.on_edit.clone();
                    let on_delete_click = props.on_delete.clone();
                    let pc = p.clone();
                    let pd = p.clone();
                    let user_rol = props.user_rol.clone();
                    let c_label = props.contrato_label.emit(p.contrato_id.clone());
                    let (badge_cls, badge_label) = estado_badge(&p.estado);
                    let is_pagado = p.estado == "pagado";
                    let recibo_id = p.id.clone();
                    html! {
                        <tr>
                            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{c_label}</td>
                            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);"><CurrencyDisplay monto={p.monto} moneda={p.moneda.clone()} /></td>
                            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">
                                {p.fecha_pago.as_deref().map(format_date_display).unwrap_or_else(|| "—".into())}</td>
                            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{format_date_display(&p.fecha_vencimiento)}</td>
                            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">
                                {p.metodo_pago.as_deref().map(metodo_label).unwrap_or("—")}</td>
                            <td style="padding: var(--space-3) var(--space-5);"><span class={badge_cls}>{badge_label}</span></td>
                            if can_write(&user_rol) {
                                <td style="padding: var(--space-3) var(--space-5); display: flex; gap: var(--space-2);">
                                    <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(pc.clone()))} class="gi-btn-text">{"Editar"}</button>
                                    if is_pagado {
                                        <button onclick={Callback::from(move |_: MouseEvent| {
                                            let url = format!("{BASE_URL}/pagos/{recibo_id}/recibo");
                                            let _ = web_sys::window().and_then(|w| w.open_with_url(&url).ok());
                                        })} class="gi-btn-text" style="color: var(--color-primary-500);">{"Recibo"}</button>
                                    }
                                    if can_delete(&user_rol) {
                                        <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(pd.clone()))} class="gi-btn-text" style="color: var(--color-error);">{"Eliminar"}</button>
                                    }
                                </td>
                            }
                        </tr>
                    }
                })}
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
