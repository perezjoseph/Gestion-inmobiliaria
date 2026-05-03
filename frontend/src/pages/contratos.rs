use std::collections::HashMap;
use std::rc::Rc;

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
use crate::components::common::pagination::Pagination;
use crate::components::common::skeleton::TableSkeleton;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_delete, api_get, api_post, api_put};
use crate::types::PaginatedResponse;
use crate::types::contrato::{CambiarEstadoDeposito, Contrato, CreateContrato, UpdateContrato};
use crate::types::inquilino::Inquilino;
use crate::types::ocr::OcrExtractField;
use crate::types::pago_generacion::{GenerarPagosResponse, PreviewPagos};
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

struct ContratoActions {
    edit: Callback<Contrato>,
    delete: Callback<Contrato>,
    renew: Callback<Contrato>,
    terminate: Callback<Contrato>,
    generar_pagos: Callback<Contrato>,
}

#[allow(clippy::missing_const_for_fn)]
fn estado_badge(estado: &str) -> (&'static str, &'static str) {
    match estado {
        "activo" => ("gi-badge gi-badge-success", "Activo"),
        "vencido" => ("gi-badge gi-badge-warning", "Vencido"),
        "terminado" => ("gi-badge gi-badge-error", "Terminado"),
        _ => ("gi-badge gi-badge-neutral", "Desconocido"),
    }
}

#[derive(Clone, Default, PartialEq)]
struct FormErrors {
    propiedad_id: Option<String>,
    inquilino_id: Option<String>,
    fecha_inicio: Option<String>,
    fecha_fin: Option<String>,
    monto_mensual: Option<String>,
}
impl FormErrors {
    const fn has_errors(&self) -> bool {
        self.propiedad_id.is_some()
            || self.inquilino_id.is_some()
            || self.fecha_inicio.is_some()
            || self.fecha_fin.is_some()
            || self.monto_mensual.is_some()
    }
}

#[derive(Properties, PartialEq)]
struct ContratoFormProps {
    is_editing: bool,
    propiedad_id: UseStateHandle<String>,
    inquilino_id: UseStateHandle<String>,
    fecha_inicio: UseStateHandle<String>,
    fecha_fin: UseStateHandle<String>,
    monto_mensual: UseStateHandle<String>,
    deposito: UseStateHandle<String>,
    moneda: UseStateHandle<String>,
    estado: UseStateHandle<String>,
    recargo_porcentaje: UseStateHandle<String>,
    dias_gracia: UseStateHandle<String>,
    propiedades: Vec<Propiedad>,
    inquilinos: Vec<Inquilino>,
    form_errors: FormErrors,
    submitting: bool,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
    confidences: Rc<HashMap<String, f64>>,
    on_ocr_result: Callback<Vec<OcrExtractField>>,
    on_confidence_clear: Callback<String>,
    #[prop_or_default]
    editing_id: Option<String>,
    #[prop_or_default]
    token: String,
}

#[component]
fn ContratoForm(props: &ContratoFormProps) -> Html {
    let fe = props.form_errors.clone();

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

    let scan_button = if props.is_editing {
        html! {}
    } else {
        html! {
            <OcrScanButton
                document_type="contrato"
                on_result={props.on_ocr_result.clone()}
                label={AttrValue::from("📷 Escanear Contrato")}
            />
        }
    };

    html! {
        <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
            <div style="display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-4);">
                <h2 class="text-display" style="font-size: var(--text-lg); font-weight: 600; color: var(--text-primary);">
                    {if props.is_editing { "Editar Contrato" } else { "Nuevo Contrato" }}</h2>
                {scan_button}
            </div>
            <form onsubmit={props.on_submit.clone()} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                <div>
                    <label class="gi-label">{"Propiedad *"}</label>
                    <select onchange={select_cb!(props.propiedad_id)} disabled={props.is_editing}
                        class={input_class(fe.propiedad_id.is_some())}>
                        <option value="" selected={props.propiedad_id.is_empty()}>{"— Seleccionar propiedad —"}</option>
                        { for props.propiedades.iter().map(|p| {
                            let sel = *props.propiedad_id == p.id;
                            html! { <option value={p.id.clone()} selected={sel}>{format!("{} — {}", p.titulo, p.direccion)}</option> }
                        })}
                    </select>
                    {field_error(&fe.propiedad_id)}
                </div>
                <div>
                    <label class="gi-label">{"Inquilino *"}</label>
                    <select onchange={select_cb!(props.inquilino_id)} disabled={props.is_editing}
                        class={input_class(fe.inquilino_id.is_some())}>
                        <option value="" selected={props.inquilino_id.is_empty()}>{"— Seleccionar inquilino —"}</option>
                        { for props.inquilinos.iter().map(|i| {
                            let sel = *props.inquilino_id == i.id;
                            html! { <option value={i.id.clone()} selected={sel}>{format!("{} {} ({})", i.nombre, i.apellido, i.cedula)}</option> }
                        })}
                    </select>
                    {field_error(&fe.inquilino_id)}
                </div>
                <div>
                    <label class="gi-label" title="No puede solaparse con otro contrato activo de la misma propiedad">{"Fecha Inicio *"}</label>
                    <ConfidenceInput
                        value={AttrValue::from((*props.fecha_inicio).clone())}
                        confidence={confidence_for("fecha_inicio")}
                        oninput={input_cb_conf(&props.fecha_inicio, "fecha_inicio")}
                        input_type="date"
                        class={AttrValue::from(if fe.fecha_inicio.is_some() { "gi-input-error" } else { "" })}
                    />
                    {field_error(&fe.fecha_inicio)}
                </div>
                <div>
                    <label class="gi-label" title="Debe ser posterior a la fecha de inicio. No puede solaparse con otro contrato activo">{"Fecha Fin *"}</label>
                    <ConfidenceInput
                        value={AttrValue::from((*props.fecha_fin).clone())}
                        confidence={confidence_for("fecha_fin")}
                        oninput={input_cb_conf(&props.fecha_fin, "fecha_fin")}
                        input_type="date"
                        class={AttrValue::from(if fe.fecha_fin.is_some() { "gi-input-error" } else { "" })}
                    />
                    {field_error(&fe.fecha_fin)}
                </div>
                <div>
                    <label class="gi-label">{"Monto Mensual *"}</label>
                    <ConfidenceInput
                        value={AttrValue::from((*props.monto_mensual).clone())}
                        confidence={confidence_for("monto_mensual")}
                        oninput={input_cb_conf(&props.monto_mensual, "monto_mensual")}
                        input_type="number"
                        class={AttrValue::from(if fe.monto_mensual.is_some() { "gi-input-error" } else { "" })}
                    />
                    {field_error(&fe.monto_mensual)}
                </div>
                <div>
                    <label class="gi-label">{"Depósito"}</label>
                    <ConfidenceInput
                        value={AttrValue::from((*props.deposito).clone())}
                        confidence={confidence_for("deposito")}
                        oninput={input_cb_conf(&props.deposito, "deposito")}
                        input_type="number"
                    />
                </div>
                <div>
                    <label class="gi-label">{"Moneda"}</label>
                    <select onchange={select_cb!(props.moneda)} disabled={props.is_editing} class="gi-input">
                        <option value="DOP" selected={*props.moneda == "DOP"}>{"DOP"}</option>
                        <option value="USD" selected={*props.moneda == "USD"}>{"USD"}</option>
                    </select>
                </div>
                <div>
                    <label class="gi-label">{"Porcentaje de Recargo (%)"}</label>
                    <input type="number" step="0.01" min="0" max="100"
                        value={(*props.recargo_porcentaje).clone()}
                        oninput={input_cb_conf(&props.recargo_porcentaje, "recargo_porcentaje")}
                        class="gi-input"
                        placeholder="Usar valor por defecto" />
                    if props.recargo_porcentaje.is_empty() {
                        <span style="font-size: var(--text-xs); color: var(--text-secondary); margin-top: var(--space-1); display: block;">
                            {"(usa valor por defecto de la organización)"}
                        </span>
                    }
                </div>
                <div>
                    <label class="gi-label">{"Días de Gracia"}</label>
                    <input type="number" min="0" step="1"
                        value={(*props.dias_gracia).clone()}
                        oninput={input_cb_conf(&props.dias_gracia, "dias_gracia")}
                        class="gi-input"
                        placeholder="Sin período de gracia" />
                </div>
                if props.is_editing {
                    <div>
                        <label class="gi-label">{"Estado"}</label>
                        <select onchange={select_cb!(props.estado)} class="gi-input">
                            <option value="activo" selected={*props.estado == "activo"}>{"Activo"}</option>
                            <option value="vencido" selected={*props.estado == "vencido"}>{"Vencido"}</option>
                            <option value="terminado" selected={*props.estado == "terminado"}>{"Terminado"}</option>
                        </select>
                    </div>
                }
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
                        entity_type={"contrato".to_string()}
                        entity_id={id.clone()}
                        token={props.token.clone()}
                    />
                </div>
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct RenewModalProps {
    renew_fecha_fin: UseStateHandle<String>,
    renew_monto: UseStateHandle<String>,
    on_confirm: Callback<MouseEvent>,
    on_cancel: Callback<MouseEvent>,
}

#[component]
fn RenewModal(props: &RenewModalProps) -> Html {
    html! {
        <div class="gi-modal-overlay">
            <div class="gi-modal">
                <h3 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">{"Renovar Contrato"}</h3>
                <div style="display: flex; flex-direction: column; gap: var(--space-3); margin-bottom: var(--space-4);">
                    <div>
                        <label class="gi-label">{"Nueva Fecha de Fin *"}</label>
                        <input type="date" value={(*props.renew_fecha_fin).clone()}
                            oninput={Callback::from({
                                let renew_fecha_fin = props.renew_fecha_fin.clone();
                                move |e: InputEvent| {
                                    let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                    renew_fecha_fin.set(input.value());
                                }
                            })}
                            class="gi-input" />
                    </div>
                    <div>
                        <label class="gi-label">{"Nuevo Monto Mensual *"}</label>
                        <input type="number" step="0.01" min="0" value={(*props.renew_monto).clone()}
                            oninput={Callback::from({
                                let renew_monto = props.renew_monto.clone();
                                move |e: InputEvent| {
                                    let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                    renew_monto.set(input.value());
                                }
                            })}
                            class="gi-input" />
                    </div>
                </div>
                <div style="display: flex; justify-content: flex-end; gap: var(--space-2);">
                    <button onclick={props.on_cancel.clone()} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                    <button onclick={props.on_confirm.clone()} class="gi-btn gi-btn-primary">{"Renovar"}</button>
                </div>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct TerminateModalProps {
    terminate_fecha: UseStateHandle<String>,
    on_confirm: Callback<MouseEvent>,
    on_cancel: Callback<MouseEvent>,
}

#[component]
fn TerminateModal(props: &TerminateModalProps) -> Html {
    html! {
        <div class="gi-modal-overlay">
            <div class="gi-modal">
                <h3 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">{"Terminar Contrato Anticipadamente"}</h3>
                <div style="margin-bottom: var(--space-4);">
                    <label class="gi-label">{"Fecha de Terminación *"}</label>
                    <input type="date" value={(*props.terminate_fecha).clone()}
                        oninput={Callback::from({
                            let terminate_fecha = props.terminate_fecha.clone();
                            move |e: InputEvent| {
                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                terminate_fecha.set(input.value());
                            }
                        })}
                        class="gi-input" />
                </div>
                <div style="display: flex; justify-content: flex-end; gap: var(--space-2);">
                    <button onclick={props.on_cancel.clone()} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                    <button onclick={props.on_confirm.clone()} class="gi-btn gi-btn-danger">{"Terminar"}</button>
                </div>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct GenerarPagosPreviewModalProps {
    preview: PreviewPagos,
    confirming: bool,
    on_confirm: Callback<MouseEvent>,
    on_cancel: Callback<MouseEvent>,
}

#[component]
fn GenerarPagosPreviewModal(props: &GenerarPagosPreviewModalProps) -> Html {
    let summary = render_preview_summary(&props.preview);
    let table = render_preview_table(&props.preview.pagos);
    html! {
        <div class="gi-modal-overlay">
            <div class="gi-modal" style="max-width: 640px; max-height: 80vh; display: flex; flex-direction: column;">
                <h3 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">
                    {"Generar Pagos — Vista Previa"}
                </h3>
                {summary}
                <div style="overflow-y: auto; flex: 1; margin-bottom: var(--space-4);">
                    {table}
                </div>
                <div style="display: flex; justify-content: flex-end; gap: var(--space-2);">
                    <button onclick={props.on_cancel.clone()} class="gi-btn gi-btn-ghost" disabled={props.confirming}>{"Cancelar"}</button>
                    <button onclick={props.on_confirm.clone()} class="gi-btn gi-btn-primary" disabled={props.confirming}>
                        {if props.confirming { "Generando..." } else { "Confirmar" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

fn render_preview_summary(preview: &PreviewPagos) -> Html {
    let moneda = preview.pagos.first().map_or("DOP", |p| p.moneda.as_str());
    html! {
        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-2); margin-bottom: var(--space-4); padding: var(--space-3); background: var(--bg-subtle); border-radius: var(--radius-md);">
            <div><span style="font-weight: 600;">{"Total pagos: "}</span>{preview.total_pagos}</div>
            <div><span style="font-weight: 600;">{"Monto total: "}</span>{format_currency(moneda, preview.monto_total)}</div>
            <div><span style="font-weight: 600;">{"Existentes: "}</span>{preview.pagos_existentes}</div>
            <div><span style="font-weight: 600;">{"Nuevos: "}</span>{preview.pagos_nuevos}</div>
        </div>
    }
}

fn render_preview_table(pagos: &[crate::types::pago_generacion::PagoPreviewItem]) -> Html {
    if pagos.is_empty() {
        return html! { <p style="color: var(--text-secondary); text-align: center;">{"No hay pagos nuevos por generar."}</p> };
    }
    html! {
        <table class="gi-table" style="width: 100%;">
            <thead>
                <tr>
                    <th style="padding: var(--space-2) var(--space-3); text-align: left; font-size: var(--text-sm);">{"Fecha Vencimiento"}</th>
                    <th style="padding: var(--space-2) var(--space-3); text-align: right; font-size: var(--text-sm);">{"Monto"}</th>
                    <th style="padding: var(--space-2) var(--space-3); text-align: left; font-size: var(--text-sm);">{"Moneda"}</th>
                </tr>
            </thead>
            <tbody>
                { for pagos.iter().map(|p| html! {
                    <tr>
                        <td style="padding: var(--space-2) var(--space-3); font-size: var(--text-sm);">{format_date_display(&p.fecha_vencimiento)}</td>
                        <td class="tabular-nums" style="padding: var(--space-2) var(--space-3); font-size: var(--text-sm); text-align: right;">{format_currency(&p.moneda, p.monto)}</td>
                        <td style="padding: var(--space-2) var(--space-3); font-size: var(--text-sm);">{&p.moneda}</td>
                    </tr>
                })}
            </tbody>
        </table>
    }
}

// --- Deposit Section ---

fn deposito_estado_badge(estado: &str) -> (&'static str, &'static str) {
    match estado {
        "pendiente" => ("bg-yellow-100 text-yellow-800", "Pendiente"),
        "cobrado" => ("bg-blue-100 text-blue-800", "Cobrado"),
        "devuelto" => ("bg-green-100 text-green-800", "Devuelto"),
        "retenido" => ("bg-red-100 text-red-800", "Retenido"),
        _ => ("bg-gray-100 text-gray-800", "Desconocido"),
    }
}

#[derive(Properties, PartialEq)]
struct DepositoSectionProps {
    pub contrato: Contrato,
    pub user_rol: AttrValue,
    #[prop_or_default]
    pub on_cobrar: Option<Callback<MouseEvent>>,
    #[prop_or_default]
    pub on_devolver: Option<Callback<MouseEvent>>,
    #[prop_or_default]
    pub on_retener: Option<Callback<MouseEvent>>,
}

#[component]
fn DepositoSection(props: &DepositoSectionProps) -> Html {
    let c = &props.contrato;

    let has_deposito = c.deposito.is_some_and(|d| d > 0.0);
    if !has_deposito {
        return html! {};
    }

    let deposito_val = c.deposito.unwrap_or(0.0);
    let estado_dep = c.estado_deposito.as_deref().unwrap_or("pendiente");
    let (badge_cls, badge_label) = deposito_estado_badge(estado_dep);

    let info_html = render_deposito_info(c, deposito_val, badge_cls, badge_label);
    let retention_html = render_deposito_retention(c, deposito_val, estado_dep);
    let buttons_html = render_deposito_buttons(
        estado_dep,
        &props.user_rol,
        &props.on_cobrar,
        &props.on_devolver,
        &props.on_retener,
    );

    html! {
        <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
            <h3 class="text-display" style="font-size: var(--text-lg); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-4);">
                {"Depósito de Garantía"}
            </h3>
            {info_html}
            {retention_html}
            {buttons_html}
        </div>
    }
}

fn render_deposito_info(
    c: &Contrato,
    deposito_val: f64,
    badge_cls: &str,
    badge_label: &str,
) -> Html {
    let fecha_cobro = c.fecha_cobro_deposito.as_deref().map(format_date_display);
    let fecha_devolucion = c
        .fecha_devolucion_deposito
        .as_deref()
        .map(format_date_display);

    html! {
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--space-3); margin-bottom: var(--space-4);">
            <div>
                <span class="gi-label" style="display: block; margin-bottom: var(--space-1);">{"Monto del Depósito"}</span>
                <span class="tabular-nums" style="font-weight: 500;">
                    <CurrencyDisplay monto={deposito_val} moneda={c.moneda.clone()} />
                </span>
            </div>
            <div>
                <span class="gi-label" style="display: block; margin-bottom: var(--space-1);">{"Estado"}</span>
                <span class={format!("inline-block px-2 py-0.5 rounded text-xs font-medium {badge_cls}")}>
                    {badge_label}
                </span>
            </div>
            if let Some(fecha) = fecha_cobro {
                <div>
                    <span class="gi-label" style="display: block; margin-bottom: var(--space-1);">{"Fecha de Cobro"}</span>
                    <span class="tabular-nums">{fecha}</span>
                </div>
            }
            if let Some(fecha) = fecha_devolucion {
                <div>
                    <span class="gi-label" style="display: block; margin-bottom: var(--space-1);">{"Fecha de Devolución"}</span>
                    <span class="tabular-nums">{fecha}</span>
                </div>
            }
        </div>
    }
}

fn render_deposito_retention(c: &Contrato, deposito_val: f64, estado_dep: &str) -> Html {
    if estado_dep != "retenido" {
        return html! {};
    }

    let monto_retenido = c.monto_retenido.unwrap_or(0.0);
    let monto_devuelto = deposito_val - monto_retenido;
    let motivo = c.motivo_retencion.as_deref().unwrap_or("");

    html! {
        <div style="padding: var(--space-3); background: var(--bg-subtle); border-radius: var(--radius-md); margin-bottom: var(--space-4);">
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--space-3); margin-bottom: var(--space-3);">
                <div>
                    <span class="gi-label" style="display: block; margin-bottom: var(--space-1);">{"Monto Retenido"}</span>
                    <span class="tabular-nums" style="font-weight: 500; color: var(--color-error);">
                        <CurrencyDisplay monto={monto_retenido} moneda={c.moneda.clone()} />
                    </span>
                </div>
                <div>
                    <span class="gi-label" style="display: block; margin-bottom: var(--space-1);">{"Monto Devuelto"}</span>
                    <span class="tabular-nums" style="font-weight: 500; color: var(--color-success);">
                        <CurrencyDisplay monto={monto_devuelto} moneda={c.moneda.clone()} />
                    </span>
                </div>
            </div>
            <div>
                <span class="gi-label" style="display: block; margin-bottom: var(--space-1);">{"Motivo de Retención"}</span>
                <p style="margin: 0; color: var(--text-primary);">{motivo}</p>
            </div>
        </div>
    }
}

#[allow(clippy::ref_option)]
fn render_deposito_buttons(
    estado_dep: &str,
    user_rol: &str,
    on_cobrar: &Option<Callback<MouseEvent>>,
    on_devolver: &Option<Callback<MouseEvent>>,
    on_retener: &Option<Callback<MouseEvent>>,
) -> Html {
    if !can_write(user_rol) {
        return html! {};
    }

    match estado_dep {
        "pendiente" => on_cobrar.as_ref().map_or_else(
            || html! {},
            |cb| {
                html! {
                    <div style="display: flex; gap: var(--space-2);">
                        <button onclick={cb.clone()} class="gi-btn gi-btn-primary">
                            {"Marcar como Cobrado"}
                        </button>
                    </div>
                }
            },
        ),
        "cobrado" => {
            let devolver_btn = on_devolver.as_ref().map_or_else(
                || html! {},
                |cb| {
                    html! {
                        <button onclick={cb.clone()} class="gi-btn gi-btn-primary">
                            {"Devolver Depósito"}
                        </button>
                    }
                },
            );
            let retener_btn = on_retener.as_ref().map_or_else(
                || html! {},
                |cb| {
                    html! {
                        <button onclick={cb.clone()} class="gi-btn gi-btn-danger">
                            {"Retener Depósito"}
                        </button>
                    }
                },
            );
            html! {
                <div style="display: flex; gap: var(--space-2);">
                    {devolver_btn}
                    {retener_btn}
                </div>
            }
        }
        // devuelto / retenido → no buttons (final states)
        _ => html! {},
    }
}

// --- Retention Modal ---

#[derive(Properties, PartialEq)]
struct RetenerDepositoModalProps {
    pub visible: bool,
    pub contrato_id: AttrValue,
    pub on_close: Callback<()>,
    pub on_success: Callback<()>,
}

#[derive(Clone, Default, PartialEq)]
struct RetenerFormErrors {
    monto_retenido: Option<String>,
    motivo_retencion: Option<String>,
}

#[component]
fn RetenerDepositoModal(props: &RetenerDepositoModalProps) -> Html {
    let monto_retenido = use_state(String::new);
    let motivo_retencion = use_state(String::new);
    let form_errors = use_state(RetenerFormErrors::default);
    let submitting = use_state(|| false);
    let toasts = use_context::<ToastContext>();

    if !props.visible {
        return html! {};
    }

    let validate = {
        let monto_retenido = monto_retenido.clone();
        let motivo_retencion = motivo_retencion.clone();
        let form_errors = form_errors.clone();
        move || -> bool {
            let mut errs = RetenerFormErrors::default();
            match monto_retenido.parse::<f64>() {
                Ok(v) if v <= 0.0 => {
                    errs.monto_retenido = Some("El monto retenido debe ser mayor a cero".into());
                }
                Err(_) | Ok(_) if monto_retenido.is_empty() => {
                    errs.monto_retenido = Some("El monto retenido es requerido".into());
                }
                _ => {}
            }
            if motivo_retencion.trim().is_empty() {
                errs.motivo_retencion = Some("El motivo de retención es requerido".into());
            }
            let valid = errs.monto_retenido.is_none() && errs.motivo_retencion.is_none();
            form_errors.set(errs);
            valid
        }
    };

    let on_confirm = {
        let monto_retenido = monto_retenido.clone();
        let motivo_retencion = motivo_retencion.clone();
        let submitting = submitting.clone();
        let contrato_id = props.contrato_id.clone();
        let on_close_confirm = props.on_close.clone();
        let on_success = props.on_success.clone();
        Callback::from(move |_: MouseEvent| {
            if *submitting || !validate() {
                return;
            }
            submitting.set(true);
            let monto: f64 = if let Ok(v) = monto_retenido.parse() {
                v
            } else {
                submitting.set(false);
                return;
            };
            let motivo = (*motivo_retencion).clone();
            let contrato_id = contrato_id.clone();
            let on_success = on_success.clone();
            let on_close = on_close_confirm.clone();
            let toasts = toasts.clone();
            let submitting = submitting.clone();
            spawn_local(async move {
                let body = CambiarEstadoDeposito {
                    estado: "retenido".to_string(),
                    monto_retenido: Some(monto),
                    motivo_retencion: Some(motivo),
                };
                match api_put::<Contrato, _>(&format!("/contratos/{contrato_id}/deposito"), &body)
                    .await
                {
                    Ok(_) => {
                        push_toast(
                            toasts.as_ref(),
                            "Depósito retenido exitosamente",
                            ToastKind::Success,
                        );
                        on_success.emit(());
                        on_close.emit(());
                    }
                    Err(err) => {
                        push_toast(toasts.as_ref(), &err, ToastKind::Error);
                    }
                }
                submitting.set(false);
            });
        })
    };

    let on_cancel = {
        let on_close = props.on_close.clone();
        Callback::from(move |_: MouseEvent| {
            on_close.emit(());
        })
    };

    let form_body = render_retener_form_body(&monto_retenido, &motivo_retencion, &form_errors);

    html! {
        <div class="gi-modal-overlay">
            <div class="gi-modal">
                <h3 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">
                    {"Retener Depósito"}
                </h3>
                {form_body}
                <div style="display: flex; justify-content: flex-end; gap: var(--space-2);">
                    <button onclick={on_cancel} class="gi-btn gi-btn-ghost" disabled={*submitting}>{"Cancelar"}</button>
                    <button onclick={on_confirm} class="gi-btn gi-btn-danger" disabled={*submitting}>
                        {if *submitting { "Reteniendo..." } else { "Confirmar Retención" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

fn render_retener_form_body(
    monto_retenido: &UseStateHandle<String>,
    motivo_retencion: &UseStateHandle<String>,
    form_errors: &UseStateHandle<RetenerFormErrors>,
) -> Html {
    let fe = (**form_errors).clone();

    let on_monto_input = {
        let monto_retenido = monto_retenido.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            monto_retenido.set(input.value());
        })
    };

    let on_motivo_input = {
        let motivo_retencion = motivo_retencion.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            motivo_retencion.set(input.value());
        })
    };

    let monto_val: String = (**monto_retenido).clone();
    let motivo_val: String = (**motivo_retencion).clone();

    html! {
        <div style="display: flex; flex-direction: column; gap: var(--space-3); margin-bottom: var(--space-4);">
            <div>
                <label class="gi-label">{"Monto a Retener *"}</label>
                <input type="number" step="0.01" min="0"
                    value={AttrValue::from(monto_val)}
                    oninput={on_monto_input}
                    class={input_class(fe.monto_retenido.is_some())} />
                {field_error(&fe.monto_retenido)}
            </div>
            <div>
                <label class="gi-label">{"Motivo de Retención *"}</label>
                <textarea
                    rows="3"
                    value={AttrValue::from(motivo_val)}
                    oninput={on_motivo_input}
                    class={input_class(fe.motivo_retencion.is_some())}
                    placeholder="Describa el motivo de la retención..."
                />
                {field_error(&fe.motivo_retencion)}
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ContratoListProps {
    items: Vec<Contrato>,
    user_rol: String,
    headers: Vec<String>,
    total: u64,
    page: u64,
    per_page: u64,
    prop_label: Callback<String, String>,
    inq_label: Callback<String, String>,
    on_edit: Callback<Contrato>,
    on_delete: Callback<Contrato>,
    on_renew: Callback<Contrato>,
    on_terminate: Callback<Contrato>,
    on_generar_pagos: Callback<Contrato>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
}

#[component]
fn ContratoList(props: &ContratoListProps) -> Html {
    if props.items.is_empty() {
        return render_contrato_empty_state();
    }

    html! {
        <>
            <DataTable headers={props.headers.clone()}>
                { for {
                    let actions = ContratoActions {
                        edit: props.on_edit.clone(),
                        delete: props.on_delete.clone(),
                        renew: props.on_renew.clone(),
                        terminate: props.on_terminate.clone(),
                        generar_pagos: props.on_generar_pagos.clone(),
                    };
                    props.items.iter().map(move |c| render_contrato_row(c, &props.user_rol, &props.prop_label, &props.inq_label, &actions))
                } }
            </DataTable>
            <Pagination
                total={props.total} page={props.page} per_page={props.per_page}
                on_page_change={props.on_page_change.clone()} on_per_page_change={props.on_per_page_change.clone()}
            />
        </>
    }
}

fn render_contrato_empty_state() -> Html {
    html! {
        <div class="gi-empty-state">
            <div class="gi-empty-state-icon">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                    <polyline points="14 2 14 8 20 8"/>
                    <line x1="16" y1="13" x2="8" y2="13"/>
                    <line x1="16" y1="17" x2="8" y2="17"/>
                </svg>
            </div>
            <div class="gi-empty-state-title">{"Sin contratos registrados"}</div>
            <p class="gi-empty-state-text">{"Un contrato vincula una propiedad con un inquilino. Necesita tener al menos una propiedad y un inquilino registrados antes de crear un contrato."}</p>
            <div class="gi-empty-state-hint">
                <Link<Route> to={Route::Propiedades} classes="gi-btn-text">{"Propiedades"}</Link<Route>>
                {" · "}
                <Link<Route> to={Route::Inquilinos} classes="gi-btn-text">{"Inquilinos"}</Link<Route>>
            </div>
        </div>
    }
}

fn render_contrato_row(
    c: &Contrato,
    user_rol: &str,
    prop_label: &Callback<String, String>,
    inq_label: &Callback<String, String>,
    actions: &ContratoActions,
) -> Html {
    let p_label = prop_label.emit(c.propiedad_id.clone());
    let i_label = inq_label.emit(c.inquilino_id.clone());
    let (badge_cls, badge_label) = estado_badge(&c.estado);
    let action_html = render_contrato_actions(user_rol, c, actions);
    html! {
        <tr>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{p_label}</td>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{i_label}</td>
            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{format_date_display(&c.fecha_inicio)}</td>
            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{format_date_display(&c.fecha_fin)}</td>
            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);"><CurrencyDisplay monto={c.monto_mensual} moneda={c.moneda.clone()} /></td>
            <td style="padding: var(--space-3) var(--space-5);"><span class={badge_cls}>{badge_label}</span></td>
            {action_html}
        </tr>
    }
}

fn render_contrato_actions(user_rol: &str, c: &Contrato, actions: &ContratoActions) -> Html {
    if !can_write(user_rol) {
        return html! {};
    }
    let cc = c.clone();
    let cd = c.clone();
    let cr = c.clone();
    let ct = c.clone();
    let cg = c.clone();
    let on_edit = actions.edit.clone();
    let on_delete_click = actions.delete.clone();
    let on_renew_click = actions.renew.clone();
    let on_terminate_click = actions.terminate.clone();
    let on_generar_click = actions.generar_pagos.clone();
    let active_btns = if c.estado == "activo" {
        html! {
            <>
                <Link<Route> to={Route::Pagos} classes="gi-btn-text gi-text-success">{"Registrar Pago"}</Link<Route>>
                <button onclick={Callback::from(move |_: MouseEvent| on_generar_click.emit(cg.clone()))} class="gi-btn-text" style="color: var(--color-primary-500);">{"Generar Pagos"}</button>
                <button onclick={Callback::from(move |_: MouseEvent| on_renew_click.emit(cr.clone()))} class="gi-btn-text" style="color: var(--color-primary-500);">{"Renovar"}</button>
                <button onclick={Callback::from(move |_: MouseEvent| on_terminate_click.emit(ct.clone()))} class="gi-btn-text" style="color: var(--color-warning);">{"Terminar"}</button>
            </>
        }
    } else {
        html! {}
    };
    let delete_btn = if can_delete(user_rol) {
        html! { <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(cd.clone()))} class="gi-btn-text" style="color: var(--color-error);">{"Eliminar"}</button> }
    } else {
        html! {}
    };
    html! {
        <td style="padding: var(--space-3) var(--space-5); display: flex; gap: var(--space-2); flex-wrap: wrap;">
            <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(cc.clone()))} class="gi-btn-text">{"Editar"}</button>
            {active_btns}
            {delete_btn}
        </td>
    }
}

#[allow(clippy::too_many_arguments)]
fn make_contrato_edit_cb(
    propiedad_id: &UseStateHandle<String>,
    inquilino_id: &UseStateHandle<String>,
    fecha_inicio: &UseStateHandle<String>,
    fecha_fin: &UseStateHandle<String>,
    monto_mensual: &UseStateHandle<String>,
    deposito: &UseStateHandle<String>,
    moneda: &UseStateHandle<String>,
    estado: &UseStateHandle<String>,
    recargo_porcentaje: &UseStateHandle<String>,
    dias_gracia: &UseStateHandle<String>,
    editing: &UseStateHandle<Option<Contrato>>,
    show_form: &UseStateHandle<bool>,
    form_errors: &UseStateHandle<FormErrors>,
) -> Callback<Contrato> {
    let (propiedad_id, inquilino_id, fecha_inicio, fecha_fin) = (
        propiedad_id.clone(),
        inquilino_id.clone(),
        fecha_inicio.clone(),
        fecha_fin.clone(),
    );
    let (monto_mensual, deposito, moneda, estado) = (
        monto_mensual.clone(),
        deposito.clone(),
        moneda.clone(),
        estado.clone(),
    );
    let recargo_porcentaje = recargo_porcentaje.clone();
    let dias_gracia = dias_gracia.clone();
    let (editing, show_form, form_errors) =
        (editing.clone(), show_form.clone(), form_errors.clone());
    Callback::from(move |c: Contrato| {
        propiedad_id.set(c.propiedad_id.clone());
        inquilino_id.set(c.inquilino_id.clone());
        fecha_inicio.set(c.fecha_inicio.clone());
        fecha_fin.set(c.fecha_fin.clone());
        monto_mensual.set(c.monto_mensual.to_string());
        deposito.set(c.deposito.map(|v| v.to_string()).unwrap_or_default());
        moneda.set(c.moneda.clone());
        estado.set(c.estado.clone());
        recargo_porcentaje.set(
            c.recargo_porcentaje
                .map(|v| v.to_string())
                .unwrap_or_default(),
        );
        dias_gracia.set(c.dias_gracia.map(|v| v.to_string()).unwrap_or_default());
        editing.set(Some(c));
        show_form.set(true);
        form_errors.set(FormErrors::default());
    })
}

fn validate_contrato_fields(
    propiedad_id: &str,
    inquilino_id: &str,
    fecha_inicio: &str,
    fecha_fin: &str,
    monto_mensual: &str,
) -> FormErrors {
    let mut errs = FormErrors::default();
    if propiedad_id.is_empty() {
        errs.propiedad_id = Some("Debe seleccionar una propiedad".into());
    }
    if inquilino_id.is_empty() {
        errs.inquilino_id = Some("Debe seleccionar un inquilino".into());
    }
    if fecha_inicio.is_empty() {
        errs.fecha_inicio = Some("La fecha de inicio es obligatoria".into());
    }
    if fecha_fin.is_empty() {
        errs.fecha_fin = Some("La fecha de fin es obligatoria".into());
    }
    if !fecha_inicio.is_empty() && !fecha_fin.is_empty() && fecha_fin <= fecha_inicio {
        errs.fecha_fin = Some("La fecha de fin debe ser posterior a la fecha de inicio".into());
    }
    match monto_mensual.parse::<f64>() {
        Ok(v) if v <= 0.0 => errs.monto_mensual = Some("El monto debe ser mayor a 0".into()),
        Err(_) => errs.monto_mensual = Some("Monto inválido".into()),
        _ => {}
    }
    errs
}

#[allow(clippy::too_many_arguments)]
fn do_save_contrato(
    editing_id: Option<String>,
    update: UpdateContrato,
    create: CreateContrato,
    reset_form: impl Fn() + 'static,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
    submitting: UseStateHandle<bool>,
) {
    spawn_local(async move {
        let res = match editing_id {
            Some(id) => api_put::<Contrato, _>(&format!("/contratos/{id}"), &update)
                .await
                .map(|c| c.pagos_generados),
            None => api_post::<Contrato, _>("/contratos", &create)
                .await
                .map(|c| c.pagos_generados),
        };
        match res {
            Ok(pagos_gen) => {
                reset_form();
                reload.set(*reload + 1);
                let msg = match pagos_gen {
                    Some(n) if n > 0 => format!("Contrato guardado — {n} pagos generados"),
                    _ => "Contrato guardado".to_string(),
                };
                push_toast(toasts.as_ref(), &msg, ToastKind::Success);
            }
            Err(err) => error.set(Some(err)),
        }
        submitting.set(false);
    });
}

fn do_delete_contrato(
    id: String,
    label: String,
    delete_target: UseStateHandle<Option<Contrato>>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
    reload_for_undo: UseStateHandle<u32>,
) {
    spawn_local(async move {
        match api_delete(&format!("/contratos/{id}")).await {
            Ok(()) => {
                delete_target.set(None);
                reload.set(*reload + 1);
                let undo_reload = reload_for_undo;
                if let Some(t) = &toasts {
                    t.dispatch(ToastAction::PushWithUndo(
                        format!("\"{label}\" eliminado"),
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

#[allow(clippy::too_many_arguments)]
fn do_renew_contrato(
    id: String,
    fecha_fin: String,
    monto: f64,
    renew_target: UseStateHandle<Option<Contrato>>,
    renew_fecha_fin: UseStateHandle<String>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
) {
    spawn_local(async move {
        let body = serde_json::json!({ "fechaFin": fecha_fin, "montoMensual": monto });
        match api_post::<Contrato, _>(&format!("/contratos/{id}/renovar"), &body).await {
            Ok(_) => {
                renew_target.set(None);
                renew_fecha_fin.set(String::new());
                reload.set(*reload + 1);
                push_toast(toasts.as_ref(), "Contrato renovado", ToastKind::Success);
            }
            Err(err) => error.set(Some(err)),
        }
    });
}

fn do_terminate_contrato(
    id: String,
    fecha: String,
    terminate_target: UseStateHandle<Option<Contrato>>,
    terminate_fecha: UseStateHandle<String>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
) {
    spawn_local(async move {
        let body = serde_json::json!({ "fechaTerminacion": fecha });
        match api_post::<Contrato, _>(&format!("/contratos/{id}/terminar"), &body).await {
            Ok(_) => {
                terminate_target.set(None);
                terminate_fecha.set(String::new());
                reload.set(*reload + 1);
                push_toast(toasts.as_ref(), "Contrato terminado", ToastKind::Success);
            }
            Err(err) => error.set(Some(err)),
        }
    });
}

fn load_contrato_refs(
    propiedades: UseStateHandle<Vec<Propiedad>>,
    inquilinos: UseStateHandle<Vec<Inquilino>>,
) {
    spawn_local(async move {
        if let Ok(resp) =
            api_get::<PaginatedResponse<Propiedad>>("/propiedades?page=1&perPage=100").await
        {
            propiedades.set(resp.data);
        }
        if let Ok(data) = api_get::<PaginatedResponse<Inquilino>>("/inquilinos?perPage=100").await {
            inquilinos.set(data.data);
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

fn load_contratos_data(
    items: UseStateHandle<Vec<Contrato>>,
    total: UseStateHandle<u64>,
    error: UseStateHandle<Option<String>>,
    loading: UseStateHandle<bool>,
    url: String,
) {
    spawn_local(async move {
        loading.set(true);
        match api_get::<PaginatedResponse<Contrato>>(&url).await {
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
fn handle_contrato_submit(
    submitting: UseStateHandle<bool>,
    validate_form: impl Fn() -> bool,
    monto_mensual: &UseStateHandle<String>,
    deposito: &UseStateHandle<String>,
    fecha_fin: &UseStateHandle<String>,
    propiedad_id: &UseStateHandle<String>,
    inquilino_id: &UseStateHandle<String>,
    fecha_inicio: &UseStateHandle<String>,
    moneda: &UseStateHandle<String>,
    estado: &UseStateHandle<String>,
    recargo_porcentaje: &UseStateHandle<String>,
    dias_gracia: &UseStateHandle<String>,
    editing: &UseStateHandle<Option<Contrato>>,
    reset_form: impl Fn() + 'static,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
) {
    if *submitting || !validate_form() {
        return;
    }
    let Ok(monto_val) = monto_mensual.parse::<f64>() else {
        return;
    };
    submitting.set(true);
    let dep = deposito.parse::<f64>().ok();
    let editing_id = editing.as_ref().map(|e| e.id.clone());
    let recargo = if recargo_porcentaje.is_empty() {
        None
    } else {
        recargo_porcentaje.parse::<f64>().ok()
    };
    let gracia = if dias_gracia.is_empty() {
        None
    } else {
        dias_gracia.parse::<i32>().ok()
    };
    let update = UpdateContrato {
        fecha_fin: Some((**fecha_fin).clone()),
        monto_mensual: Some(monto_val),
        deposito: dep,
        estado: Some((**estado).clone()),
        recargo_porcentaje: recargo,
        dias_gracia: gracia,
    };
    let create = CreateContrato {
        propiedad_id: (**propiedad_id).clone(),
        inquilino_id: (**inquilino_id).clone(),
        fecha_inicio: (**fecha_inicio).clone(),
        fecha_fin: (**fecha_fin).clone(),
        monto_mensual: monto_val,
        deposito: dep,
        moneda: Some((**moneda).clone()),
        recargo_porcentaje: recargo,
        dias_gracia: gracia,
    };
    do_save_contrato(
        editing_id, update, create, reset_form, reload, error, toasts, submitting,
    );
}

fn handle_contrato_delete_confirm(
    delete_target: &UseStateHandle<Option<Contrato>>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
) {
    if let Some(ref c) = **delete_target {
        let label = format!("Contrato {}", &c.id[..8.min(c.id.len())]);
        do_delete_contrato(
            c.id.clone(),
            label,
            delete_target.clone(),
            reload.clone(),
            error,
            toasts,
            reload,
        );
    }
}

fn handle_contrato_renew_confirm(
    renew_target: &UseStateHandle<Option<Contrato>>,
    renew_fecha_fin: UseStateHandle<String>,
    renew_monto: &UseStateHandle<String>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
) {
    if let Some(ref c) = **renew_target {
        do_renew_contrato(
            c.id.clone(),
            (*renew_fecha_fin).clone(),
            renew_monto.parse::<f64>().unwrap_or(0.0),
            renew_target.clone(),
            renew_fecha_fin,
            reload,
            error,
            toasts,
        );
    }
}

fn handle_contrato_terminate_confirm(
    terminate_target: &UseStateHandle<Option<Contrato>>,
    terminate_fecha: UseStateHandle<String>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
) {
    if let Some(ref c) = **terminate_target {
        do_terminate_contrato(
            c.id.clone(),
            (*terminate_fecha).clone(),
            terminate_target.clone(),
            terminate_fecha,
            reload,
            error,
            toasts,
        );
    }
}

fn handle_escape_contratos(
    delete_target: &UseStateHandle<Option<Contrato>>,
    show_form: &UseStateHandle<bool>,
    reset_form: impl Fn(),
) {
    if delete_target.is_some() {
        delete_target.set(None);
    } else if **show_form {
        reset_form();
    }
}

struct ContratoViewState<'a> {
    loading: bool,
    user_rol: &'a str,
    error: &'a UseStateHandle<Option<String>>,
    items: &'a UseStateHandle<Vec<Contrato>>,
    total: u64,
    page: u64,
    per_page: u64,
}

struct ContratoFormState<'a> {
    show_form: bool,
    editing: &'a UseStateHandle<Option<Contrato>>,
    propiedad_id: &'a UseStateHandle<String>,
    inquilino_id: &'a UseStateHandle<String>,
    fecha_inicio: &'a UseStateHandle<String>,
    fecha_fin: &'a UseStateHandle<String>,
    monto_mensual: &'a UseStateHandle<String>,
    deposito: &'a UseStateHandle<String>,
    moneda: &'a UseStateHandle<String>,
    estado: &'a UseStateHandle<String>,
    recargo_porcentaje: &'a UseStateHandle<String>,
    dias_gracia: &'a UseStateHandle<String>,
    propiedades: &'a UseStateHandle<Vec<Propiedad>>,
    inquilinos: &'a UseStateHandle<Vec<Inquilino>>,
    form_errors: &'a UseStateHandle<FormErrors>,
    submitting: bool,
    confidences: &'a UseStateHandle<Rc<HashMap<String, f64>>>,
    editing_id: Option<String>,
    token: String,
}

struct ContratoViewCallbacks {
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
    on_new: Callback<MouseEvent>,
    on_edit: Callback<Contrato>,
    on_delete_click: Callback<Contrato>,
    on_delete_confirm: Callback<MouseEvent>,
    on_delete_cancel: Callback<MouseEvent>,
    on_renew_click: Callback<Contrato>,
    on_renew_confirm: Callback<MouseEvent>,
    on_renew_cancel: Callback<MouseEvent>,
    on_terminate_click: Callback<Contrato>,
    on_terminate_confirm: Callback<MouseEvent>,
    on_terminate_cancel: Callback<MouseEvent>,
    on_generar_pagos_click: Callback<Contrato>,
    on_preview_confirm: Callback<MouseEvent>,
    on_preview_cancel: Callback<MouseEvent>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
    prop_label: Callback<String, String>,
    inq_label: Callback<String, String>,
    on_ocr_result: Callback<Vec<OcrExtractField>>,
    on_confidence_clear: Callback<String>,
    on_cobrar: Callback<MouseEvent>,
    on_devolver: Callback<MouseEvent>,
    on_retener: Callback<MouseEvent>,
    on_retener_close: Callback<()>,
    on_retener_success: Callback<()>,
}

struct ContratoModalState<'a> {
    delete_target: &'a UseStateHandle<Option<Contrato>>,
    renew_target: &'a UseStateHandle<Option<Contrato>>,
    renew_fecha_fin: &'a UseStateHandle<String>,
    renew_monto: &'a UseStateHandle<String>,
    terminate_target: &'a UseStateHandle<Option<Contrato>>,
    terminate_fecha: &'a UseStateHandle<String>,
    preview_data: Option<&'a PreviewPagos>,
    preview_confirming: bool,
    show_retener_modal: bool,
    retener_contrato_id: AttrValue,
}

fn render_contratos_view(
    view: ContratoViewState<'_>,
    form: ContratoFormState<'_>,
    callbacks: ContratoViewCallbacks,
    modals: ContratoModalState<'_>,
) -> Html {
    if view.loading {
        return html! { <TableSkeleton title_width="180px" columns={7} /> };
    }

    let last_header: String = if can_write(view.user_rol) {
        "Acciones".into()
    } else {
        String::new()
    };
    let headers = vec![
        "Propiedad".into(),
        "Inquilino".into(),
        "Inicio".into(),
        "Fin".into(),
        "Monto".into(),
        "Estado".into(),
        last_header,
    ];
    let new_btn = render_new_btn_contrato(view.user_rol, &callbacks.on_new);
    let error_html = render_opt_error_contrato(view.error);
    let delete_html = render_delete_confirm_contrato(
        modals.delete_target,
        &callbacks.prop_label,
        &callbacks.on_delete_confirm,
        &callbacks.on_delete_cancel,
    );
    let renew_html = render_renew_modal(
        modals.renew_target,
        modals.renew_fecha_fin,
        modals.renew_monto,
        callbacks.on_renew_confirm,
        callbacks.on_renew_cancel,
    );
    let terminate_html = render_terminate_modal(
        modals.terminate_target,
        modals.terminate_fecha,
        callbacks.on_terminate_confirm,
        callbacks.on_terminate_cancel,
    );
    let form_html = render_contrato_form_section(
        form.show_form,
        form.editing,
        form.propiedad_id,
        form.inquilino_id,
        form.fecha_inicio,
        form.fecha_fin,
        form.monto_mensual,
        form.deposito,
        form.moneda,
        form.estado,
        form.recargo_porcentaje,
        form.dias_gracia,
        form.propiedades,
        form.inquilinos,
        form.form_errors,
        form.submitting,
        callbacks.on_submit,
        callbacks.on_cancel,
        form.confidences,
        callbacks.on_ocr_result,
        callbacks.on_confidence_clear,
        form.editing_id,
        form.token,
        view.user_rol,
        callbacks.on_cobrar,
        callbacks.on_devolver,
        callbacks.on_retener,
        modals.show_retener_modal,
        modals.retener_contrato_id,
        callbacks.on_retener_close,
        callbacks.on_retener_success,
    );

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Contratos"}</h1>
                {new_btn}
            </div>
            {error_html}
            {delete_html}
            {renew_html}
            {terminate_html}
            {modals.preview_data.map_or_else(|| html! {}, |preview| html! {
                    <GenerarPagosPreviewModal
                        preview={(*preview).clone()}
                        confirming={modals.preview_confirming}
                        on_confirm={callbacks.on_preview_confirm}
                        on_cancel={callbacks.on_preview_cancel}
                    />
                })}
            {form_html}
            <ContratoList
                items={(**view.items).clone()} user_rol={view.user_rol.to_string()} headers={headers}
                total={view.total} page={view.page} per_page={view.per_page}
                prop_label={callbacks.prop_label.clone()} inq_label={callbacks.inq_label.clone()}
                on_edit={callbacks.on_edit} on_delete={callbacks.on_delete_click}
                on_renew={callbacks.on_renew_click} on_terminate={callbacks.on_terminate_click}
                on_generar_pagos={callbacks.on_generar_pagos_click}
                on_page_change={callbacks.on_page_change} on_per_page_change={callbacks.on_per_page_change}
            />
        </div>
    }
}

fn render_new_btn_contrato(user_rol: &str, on_new: &Callback<MouseEvent>) -> Html {
    if can_write(user_rol) {
        html! { <button onclick={on_new.clone()} class="gi-btn gi-btn-primary">{"+ Nuevo Contrato"}</button> }
    } else {
        html! {}
    }
}

fn render_opt_error_contrato(error: &UseStateHandle<Option<String>>) -> Html {
    (*error).as_ref().map_or_else(
        || html! {},
        |err| {
            let error = error.clone();
            html! { <ErrorBanner message={err.clone()} onclose={Callback::from(move |_: MouseEvent| error.set(None))} /> }
        },
    )
}

fn render_delete_confirm_contrato(
    target: &UseStateHandle<Option<Contrato>>,
    prop_label: &Callback<String, String>,
    on_confirm: &Callback<MouseEvent>,
    on_cancel: &Callback<MouseEvent>,
) -> Html {
    (**target).as_ref().map_or_else(
        || html! {},
        |t| html! {
            <DeleteConfirmModal
                message={format!("¿Está seguro de que desea eliminar el contrato de la propiedad \"{}\"? Esta acción no se puede deshacer.", prop_label.emit(t.propiedad_id.clone()))}
                on_confirm={on_confirm.clone()} on_cancel={on_cancel.clone()}
            />
        },
    )
}

fn render_renew_modal(
    target: &UseStateHandle<Option<Contrato>>,
    fecha_fin: &UseStateHandle<String>,
    monto: &UseStateHandle<String>,
    on_confirm: Callback<MouseEvent>,
    on_cancel: Callback<MouseEvent>,
) -> Html {
    if target.is_some() {
        html! { <RenewModal renew_fecha_fin={fecha_fin.clone()} renew_monto={monto.clone()} on_confirm={on_confirm} on_cancel={on_cancel} /> }
    } else {
        html! {}
    }
}

fn render_terminate_modal(
    target: &UseStateHandle<Option<Contrato>>,
    fecha: &UseStateHandle<String>,
    on_confirm: Callback<MouseEvent>,
    on_cancel: Callback<MouseEvent>,
) -> Html {
    if target.is_some() {
        html! { <TerminateModal terminate_fecha={fecha.clone()} on_confirm={on_confirm} on_cancel={on_cancel} /> }
    } else {
        html! {}
    }
}

fn render_recargo_detail(c: &Contrato) -> Html {
    let recargo_text = c.recargo_porcentaje.map_or_else(
        || html! {
            <span style="color: var(--text-secondary); font-style: italic;">
                {"(usa valor por defecto de la organización)"}
            </span>
        },
        |v| html! { <span class="tabular-nums" style="font-weight: 500;">{format!("{v:.2}%")}</span> },
    );
    let gracia_text = c.dias_gracia.map_or_else(
        || html! { <span style="color: var(--text-secondary);">{"No definido"}</span> },
        |v| html! { <span class="tabular-nums" style="font-weight: 500;">{format!("{v} días")}</span> },
    );
    html! {
        <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
            <h3 class="text-display" style="font-size: var(--text-lg); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-4);">
                {"Recargo por Mora"}
            </h3>
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--space-3);">
                <div>
                    <span class="gi-label" style="display: block; margin-bottom: var(--space-1);">{"Porcentaje de Recargo"}</span>
                    {recargo_text}
                </div>
                <div>
                    <span class="gi-label" style="display: block; margin-bottom: var(--space-1);">{"Días de Gracia"}</span>
                    {gracia_text}
                </div>
            </div>
        </div>
    }
}

#[allow(clippy::too_many_arguments)]
fn render_contrato_form_section(
    show_form: bool,
    editing: &UseStateHandle<Option<Contrato>>,
    propiedad_id: &UseStateHandle<String>,
    inquilino_id: &UseStateHandle<String>,
    fecha_inicio: &UseStateHandle<String>,
    fecha_fin: &UseStateHandle<String>,
    monto_mensual: &UseStateHandle<String>,
    deposito: &UseStateHandle<String>,
    moneda: &UseStateHandle<String>,
    estado: &UseStateHandle<String>,
    recargo_porcentaje: &UseStateHandle<String>,
    dias_gracia: &UseStateHandle<String>,
    propiedades: &UseStateHandle<Vec<Propiedad>>,
    inquilinos: &UseStateHandle<Vec<Inquilino>>,
    form_errors: &UseStateHandle<FormErrors>,
    submitting: bool,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
    confidences: &UseStateHandle<Rc<HashMap<String, f64>>>,
    on_ocr_result: Callback<Vec<OcrExtractField>>,
    on_confidence_clear: Callback<String>,
    editing_id: Option<String>,
    token: String,
    user_rol: &str,
    on_cobrar: Callback<MouseEvent>,
    on_devolver: Callback<MouseEvent>,
    on_retener: Callback<MouseEvent>,
    show_retener_modal: bool,
    retener_contrato_id: AttrValue,
    on_retener_close: Callback<()>,
    on_retener_success: Callback<()>,
) -> Html {
    if !show_form {
        return html! {};
    }

    let deposito_html = (**editing).as_ref().map_or_else(
        || html! {},
        |c| {
            html! {
                <>
                    <DepositoSection
                        contrato={c.clone()}
                        user_rol={AttrValue::from(user_rol.to_string())}
                        on_cobrar={on_cobrar}
                        on_devolver={on_devolver}
                        on_retener={on_retener}
                    />
                    <RetenerDepositoModal
                        visible={show_retener_modal}
                        contrato_id={retener_contrato_id}
                        on_close={on_retener_close}
                        on_success={on_retener_success}
                    />
                </>
            }
        },
    );

    let recargo_detail_html = (**editing)
        .as_ref()
        .map_or_else(|| html! {}, render_recargo_detail);

    html! {
        <>
            <ContratoForm
                is_editing={editing.is_some()}
                propiedad_id={propiedad_id.clone()} inquilino_id={inquilino_id.clone()}
                fecha_inicio={fecha_inicio.clone()} fecha_fin={fecha_fin.clone()}
                monto_mensual={monto_mensual.clone()} deposito={deposito.clone()}
                moneda={moneda.clone()} estado={estado.clone()}
                recargo_porcentaje={recargo_porcentaje.clone()} dias_gracia={dias_gracia.clone()}
                propiedades={(**propiedades).clone()} inquilinos={(**inquilinos).clone()}
                form_errors={(**form_errors).clone()} submitting={submitting}
                on_submit={on_submit} on_cancel={on_cancel}
                confidences={(**confidences).clone()} on_ocr_result={on_ocr_result}
                on_confidence_clear={on_confidence_clear}
                editing_id={editing_id}
                token={token}
            />
            {recargo_detail_html}
            {deposito_html}
        </>
    }
}

#[component]
pub fn Contratos() -> Html {
    let auth = use_context::<AuthContext>();
    let toasts = use_context::<ToastContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();

    let items = use_state(Vec::<Contrato>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let propiedades = use_state(Vec::<Propiedad>::new);
    let inquilinos = use_state(Vec::<Inquilino>::new);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let show_form = use_state(|| false);
    let editing = use_state(|| Option::<Contrato>::None);
    let delete_target = use_state(|| Option::<Contrato>::None);
    let submitting = use_state(|| false);
    let form_errors = use_state(FormErrors::default);
    let reload = use_state(|| 0u32);

    let propiedad_id = use_state(String::new);
    let inquilino_id = use_state(String::new);
    let fecha_inicio = use_state(String::new);
    let fecha_fin = use_state(String::new);
    let monto_mensual = use_state(String::new);
    let deposito = use_state(String::new);
    let moneda = use_state(|| "DOP".to_string());
    let estado = use_state(|| "activo".to_string());
    let recargo_porcentaje = use_state(String::new);
    let dias_gracia = use_state(String::new);

    let renew_target = use_state(|| Option::<Contrato>::None);
    let renew_fecha_fin = use_state(String::new);
    let renew_monto = use_state(String::new);
    let terminate_target = use_state(|| Option::<Contrato>::None);
    let terminate_fecha = use_state(String::new);

    let preview_target = use_state(|| Option::<Contrato>::None);
    let preview_data = use_state(|| Option::<PreviewPagos>::None);
    let preview_confirming = use_state(|| false);

    let confidences = use_state(|| Rc::new(HashMap::<String, f64>::new()));

    let show_retener_modal = use_state(|| false);

    {
        let items = items.clone();
        let total = total.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        let pg = *page;
        let pp = *per_page;
        use_effect_with((reload_val, pg), move |_| {
            load_contratos_data(
                items,
                total,
                error,
                loading,
                format!("/contratos?page={pg}&perPage={pp}"),
            );
        });
    }

    {
        let propiedades = propiedades.clone();
        let inquilinos = inquilinos.clone();
        use_effect_with((), move |()| {
            load_contrato_refs(propiedades, inquilinos);
        });
    }

    let prop_label = {
        let propiedades = propiedades.clone();
        Callback::from(move |id: String| -> String {
            propiedades
                .iter()
                .find(|p| p.id == id)
                .map_or_else(|| id.clone(), |p| format!("{} — {}", p.titulo, p.direccion))
        })
    };

    let inq_label = {
        let inquilinos = inquilinos.clone();
        Callback::from(move |id: String| -> String {
            inquilinos.iter().find(|i| i.id == id).map_or_else(
                || id.clone(),
                |i| format!("{} {} ({})", i.nombre, i.apellido, i.cedula),
            )
        })
    };

    let reset_form = {
        let propiedad_id = propiedad_id.clone();
        let inquilino_id = inquilino_id.clone();
        let fecha_inicio = fecha_inicio.clone();
        let fecha_fin = fecha_fin.clone();
        let monto_mensual = monto_mensual.clone();
        let deposito = deposito.clone();
        let moneda = moneda.clone();
        let estado = estado.clone();
        let recargo_porcentaje = recargo_porcentaje.clone();
        let dias_gracia = dias_gracia.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        let confidences = confidences.clone();
        move || {
            propiedad_id.set(String::new());
            inquilino_id.set(String::new());
            fecha_inicio.set(String::new());
            fecha_fin.set(String::new());
            monto_mensual.set(String::new());
            deposito.set(String::new());
            moneda.set("DOP".into());
            estado.set("activo".into());
            recargo_porcentaje.set(String::new());
            dias_gracia.set(String::new());
            editing.set(None);
            show_form.set(false);
            form_errors.set(FormErrors::default());
            confidences.set(Rc::new(HashMap::new()));
        }
    };

    let escape_handler = use_mut_ref(|| None::<Box<dyn Fn()>>);
    {
        let delete_target = delete_target.clone();
        let show_form = show_form.clone();
        let reset_form = reset_form.clone();
        let handler = escape_handler.clone();
        *handler.borrow_mut() = Some(Box::new(move || {
            handle_escape_contratos(&delete_target, &show_form, &reset_form);
        }) as Box<dyn Fn()>);
    }
    {
        let escape_handler = escape_handler.clone();
        use_effect_with((), move |()| {
            let listener = register_escape_listener(escape_handler);
            move || drop(listener)
        });
    }

    let on_new = super::page_helpers::new_cb(reset_form.clone(), &show_form, true);

    let on_edit = make_contrato_edit_cb(
        &propiedad_id,
        &inquilino_id,
        &fecha_inicio,
        &fecha_fin,
        &monto_mensual,
        &deposito,
        &moneda,
        &estado,
        &recargo_porcentaje,
        &dias_gracia,
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
            handle_contrato_delete_confirm(
                &delete_target,
                reload.clone(),
                error.clone(),
                toasts.clone(),
            );
        })
    };

    let on_delete_cancel = super::page_helpers::delete_cancel_cb(&delete_target);

    let validate_form = {
        let propiedad_id = propiedad_id.clone();
        let inquilino_id = inquilino_id.clone();
        let fecha_inicio = fecha_inicio.clone();
        let fecha_fin = fecha_fin.clone();
        let monto_mensual = monto_mensual.clone();
        let form_errors = form_errors.clone();
        move || -> bool {
            let errs = validate_contrato_fields(
                &propiedad_id,
                &inquilino_id,
                &fecha_inicio,
                &fecha_fin,
                &monto_mensual,
            );
            let valid = !errs.has_errors();
            form_errors.set(errs);
            valid
        }
    };

    let on_submit = {
        let propiedad_id = propiedad_id.clone();
        let inquilino_id = inquilino_id.clone();
        let fecha_inicio = fecha_inicio.clone();
        let fecha_fin = fecha_fin.clone();
        let monto_mensual = monto_mensual.clone();
        let deposito = deposito.clone();
        let moneda = moneda.clone();
        let estado = estado.clone();
        let recargo_porcentaje = recargo_porcentaje.clone();
        let dias_gracia = dias_gracia.clone();
        let editing = editing.clone();
        let error = error.clone();
        let reload = reload.clone();
        let reset_form = reset_form.clone();
        let toasts = toasts.clone();
        let submitting = submitting.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            handle_contrato_submit(
                submitting.clone(),
                &validate_form,
                &monto_mensual,
                &deposito,
                &fecha_fin,
                &propiedad_id,
                &inquilino_id,
                &fecha_inicio,
                &moneda,
                &estado,
                &recargo_porcentaje,
                &dias_gracia,
                &editing,
                reset_form.clone(),
                reload.clone(),
                error.clone(),
                toasts.clone(),
            );
        })
    };

    let on_cancel = super::page_helpers::cancel_cb(reset_form);

    let on_ocr_result = {
        let monto_mensual = monto_mensual.clone();
        let moneda = moneda.clone();
        let fecha_inicio = fecha_inicio.clone();
        let fecha_fin = fecha_fin.clone();
        let deposito = deposito.clone();
        let confidences = confidences.clone();
        Callback::from(move |fields: Vec<OcrExtractField>| {
            let mut conf_map = HashMap::new();
            for field in &fields {
                match field.name.as_str() {
                    "monto_mensual" => monto_mensual.set(field.value.clone()),
                    "moneda" => moneda.set(field.value.clone()),
                    "fecha_inicio" => fecha_inicio.set(field.value.clone()),
                    "fecha_fin" => fecha_fin.set(field.value.clone()),
                    "deposito" => deposito.set(field.value.clone()),
                    _ => {}
                }
                conf_map.insert(field.name.clone(), field.confidence);
            }
            confidences.set(Rc::new(conf_map));
        })
    };

    let on_confidence_clear = {
        let confidences = confidences.clone();
        Callback::from(move |name: String| {
            let mut c = (*confidences).as_ref().clone();
            c.remove(&name);
            confidences.set(Rc::new(c));
        })
    };

    let on_renew_click = {
        let renew_target = renew_target.clone();
        let renew_monto = renew_monto.clone();
        Callback::from(move |c: Contrato| {
            renew_monto.set(c.monto_mensual.to_string());
            renew_target.set(Some(c));
        })
    };

    let on_renew_confirm = {
        let renew_target = renew_target.clone();
        let renew_fecha_fin = renew_fecha_fin.clone();
        let renew_monto = renew_monto.clone();
        let error = error.clone();
        let reload = reload.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            handle_contrato_renew_confirm(
                &renew_target,
                renew_fecha_fin.clone(),
                &renew_monto,
                reload.clone(),
                error.clone(),
                toasts.clone(),
            );
        })
    };

    let on_renew_cancel = super::page_helpers::delete_cancel_cb(&renew_target);

    let on_terminate_click = {
        let terminate_target = terminate_target.clone();
        Callback::from(move |c: Contrato| {
            terminate_target.set(Some(c));
        })
    };

    let on_terminate_confirm = {
        let terminate_target = terminate_target.clone();
        let terminate_fecha = terminate_fecha.clone();
        let error = error.clone();
        let reload = reload.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            handle_contrato_terminate_confirm(
                &terminate_target,
                terminate_fecha.clone(),
                reload.clone(),
                error.clone(),
                toasts.clone(),
            );
        })
    };

    let on_terminate_cancel = super::page_helpers::delete_cancel_cb(&terminate_target);

    let on_generar_pagos_click = {
        let preview_target = preview_target.clone();
        let preview_data = preview_data.clone();
        let error = error.clone();
        Callback::from(move |c: Contrato| {
            let id = c.id.clone();
            let preview_data = preview_data.clone();
            let error = error.clone();
            preview_target.set(Some(c));
            spawn_local(async move {
                match api_get::<PreviewPagos>(&format!("/contratos/{id}/pagos/preview")).await {
                    Ok(data) => preview_data.set(Some(data)),
                    Err(err) => error.set(Some(err)),
                }
            });
        })
    };

    let on_preview_confirm = {
        let preview_target = preview_target.clone();
        let preview_data = preview_data.clone();
        let preview_confirming = preview_confirming.clone();
        let reload = reload.clone();
        let error = error.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            let Some(ref c) = *preview_target else {
                return;
            };
            let id = c.id.clone();
            let preview_target = preview_target.clone();
            let preview_data = preview_data.clone();
            let preview_confirming = preview_confirming.clone();
            let reload = reload.clone();
            let error = error.clone();
            let toasts = toasts.clone();
            preview_confirming.set(true);
            spawn_local(async move {
                let body = serde_json::json!({});
                match api_post::<GenerarPagosResponse, _>(
                    &format!("/contratos/{id}/pagos/generar"),
                    &body,
                )
                .await
                {
                    Ok(resp) => {
                        preview_target.set(None);
                        preview_data.set(None);
                        reload.set(*reload + 1);
                        let msg = format!("{} pagos generados", resp.pagos_generados);
                        push_toast(toasts.as_ref(), &msg, ToastKind::Success);
                    }
                    Err(err) => error.set(Some(err)),
                }
                preview_confirming.set(false);
            });
        })
    };

    let on_preview_cancel = {
        let preview_data = preview_data.clone();
        Callback::from(move |_: MouseEvent| {
            preview_target.set(None);
            preview_data.set(None);
        })
    };

    let on_cobrar = {
        let editing = editing.clone();
        let reload = reload.clone();
        let error = error.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            let Some(ref c) = *editing else { return };
            let id = c.id.clone();
            let reload = reload.clone();
            let error = error.clone();
            let toasts = toasts.clone();
            spawn_local(async move {
                let body = CambiarEstadoDeposito {
                    estado: "cobrado".to_string(),
                    monto_retenido: None,
                    motivo_retencion: None,
                };
                match api_put::<Contrato, _>(&format!("/contratos/{id}/deposito"), &body).await {
                    Ok(_) => {
                        push_toast(
                            toasts.as_ref(),
                            "Depósito marcado como cobrado",
                            ToastKind::Success,
                        );
                        reload.set(*reload + 1);
                    }
                    Err(err) => {
                        push_toast(toasts.as_ref(), &err, ToastKind::Error);
                        error.set(Some(err));
                    }
                }
            });
        })
    };

    let on_devolver = {
        let editing = editing.clone();
        let reload = reload.clone();
        let error = error.clone();
        Callback::from(move |_: MouseEvent| {
            let Some(ref c) = *editing else { return };
            let id = c.id.clone();
            let reload = reload.clone();
            let error = error.clone();
            let toasts = toasts.clone();
            spawn_local(async move {
                let body = CambiarEstadoDeposito {
                    estado: "devuelto".to_string(),
                    monto_retenido: None,
                    motivo_retencion: None,
                };
                match api_put::<Contrato, _>(&format!("/contratos/{id}/deposito"), &body).await {
                    Ok(_) => {
                        push_toast(
                            toasts.as_ref(),
                            "Depósito devuelto exitosamente",
                            ToastKind::Success,
                        );
                        reload.set(*reload + 1);
                    }
                    Err(err) => {
                        push_toast(toasts.as_ref(), &err, ToastKind::Error);
                        error.set(Some(err));
                    }
                }
            });
        })
    };

    let on_retener = {
        let show_retener_modal = show_retener_modal.clone();
        Callback::from(move |_: MouseEvent| {
            show_retener_modal.set(true);
        })
    };

    let on_retener_close = {
        let show_retener_modal = show_retener_modal.clone();
        Callback::from(move |()| {
            show_retener_modal.set(false);
        })
    };

    let on_retener_success = {
        let reload = reload.clone();
        Callback::from(move |()| {
            reload.set(*reload + 1);
        })
    };

    let (on_page_change, on_per_page_change) =
        super::page_helpers::pagination_cbs(&page, &per_page, &reload);

    let editing_id = editing.as_ref().map(|e| e.id.clone());
    let retener_contrato_id =
        AttrValue::from(editing.as_ref().map(|e| e.id.clone()).unwrap_or_default());
    let token = auth
        .as_ref()
        .and_then(|a| a.token.clone())
        .unwrap_or_default();

    render_contratos_view(
        ContratoViewState {
            loading: *loading,
            user_rol: &user_rol,
            error: &error,
            items: &items,
            total: *total,
            page: *page,
            per_page: *per_page,
        },
        ContratoFormState {
            show_form: *show_form,
            editing: &editing,
            propiedad_id: &propiedad_id,
            inquilino_id: &inquilino_id,
            fecha_inicio: &fecha_inicio,
            fecha_fin: &fecha_fin,
            monto_mensual: &monto_mensual,
            deposito: &deposito,
            moneda: &moneda,
            estado: &estado,
            recargo_porcentaje: &recargo_porcentaje,
            dias_gracia: &dias_gracia,
            propiedades: &propiedades,
            inquilinos: &inquilinos,
            form_errors: &form_errors,
            submitting: *submitting,
            confidences: &confidences,
            editing_id,
            token,
        },
        ContratoViewCallbacks {
            on_submit,
            on_cancel,
            on_new,
            on_edit,
            on_delete_click,
            on_delete_confirm,
            on_delete_cancel,
            on_renew_click,
            on_renew_confirm,
            on_renew_cancel,
            on_terminate_click,
            on_terminate_confirm,
            on_terminate_cancel,
            on_generar_pagos_click,
            on_preview_confirm,
            on_preview_cancel,
            on_page_change,
            on_per_page_change,
            prop_label,
            inq_label,
            on_ocr_result,
            on_confidence_clear,
            on_cobrar,
            on_devolver,
            on_retener,
            on_retener_close,
            on_retener_success,
        },
        ContratoModalState {
            delete_target: &delete_target,
            renew_target: &renew_target,
            renew_fecha_fin: &renew_fecha_fin,
            renew_monto: &renew_monto,
            terminate_target: &terminate_target,
            terminate_fecha: &terminate_fecha,
            preview_data: preview_data.as_ref(),
            preview_confirming: *preview_confirming,
            show_retener_modal: *show_retener_modal,
            retener_contrato_id,
        },
    )
}
