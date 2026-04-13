use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthContext, Route};
use crate::components::common::currency_display::CurrencyDisplay;
use crate::components::common::data_table::DataTable;
use crate::components::common::delete_confirm_modal::DeleteConfirmModal;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::loading::Loading;
use crate::components::common::pagination::Pagination;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_delete, api_get, api_post, api_put};
use crate::types::PaginatedResponse;
use crate::types::contrato::{Contrato, CreateContrato, UpdateContrato};
use crate::types::inquilino::Inquilino;
use crate::types::propiedad::Propiedad;
use crate::utils::{can_delete, can_write, format_date_display};

fn push_toast(toasts: &Option<ToastContext>, msg: &str, kind: ToastKind) {
    if let Some(t) = toasts {
        t.dispatch(ToastAction::Push(msg.into(), kind));
    }
}

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
    fn has_errors(&self) -> bool {
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
    propiedades: Vec<Propiedad>,
    inquilinos: Vec<Inquilino>,
    form_errors: FormErrors,
    submitting: bool,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
}

#[function_component]
fn ContratoForm(props: &ContratoFormProps) -> Html {
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
                {if props.is_editing { "Editar Contrato" } else { "Nuevo Contrato" }}</h2>
            <form onsubmit={props.on_submit.clone()} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                <div>
                    <label class="gi-label">{"Propiedad *"}</label>
                    <select onchange={select_cb!(props.propiedad_id)} disabled={props.is_editing}
                        class={if fe.propiedad_id.is_some() { "gi-input gi-input-error" } else { "gi-input" }}>
                        <option value="" selected={props.propiedad_id.is_empty()}>{"— Seleccionar propiedad —"}</option>
                        { for props.propiedades.iter().map(|p| {
                            let sel = *props.propiedad_id == p.id;
                            html! { <option value={p.id.clone()} selected={sel}>{format!("{} — {}", p.titulo, p.direccion)}</option> }
                        })}
                    </select>
                    if let Some(ref msg) = fe.propiedad_id { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label">{"Inquilino *"}</label>
                    <select onchange={select_cb!(props.inquilino_id)} disabled={props.is_editing}
                        class={if fe.inquilino_id.is_some() { "gi-input gi-input-error" } else { "gi-input" }}>
                        <option value="" selected={props.inquilino_id.is_empty()}>{"— Seleccionar inquilino —"}</option>
                        { for props.inquilinos.iter().map(|i| {
                            let sel = *props.inquilino_id == i.id;
                            html! { <option value={i.id.clone()} selected={sel}>{format!("{} {} ({})", i.nombre, i.apellido, i.cedula)}</option> }
                        })}
                    </select>
                    if let Some(ref msg) = fe.inquilino_id { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label" title="No puede solaparse con otro contrato activo de la misma propiedad">{"Fecha Inicio *"}</label>
                    <input type="date" value={(*props.fecha_inicio).clone()} oninput={input_cb!(props.fecha_inicio)} disabled={props.is_editing}
                        class={if fe.fecha_inicio.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                    if let Some(ref msg) = fe.fecha_inicio { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label" title="Debe ser posterior a la fecha de inicio. No puede solaparse con otro contrato activo">{"Fecha Fin *"}</label>
                    <input type="date" value={(*props.fecha_fin).clone()} oninput={input_cb!(props.fecha_fin)}
                        class={if fe.fecha_fin.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                    if let Some(ref msg) = fe.fecha_fin { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label">{"Monto Mensual *"}</label>
                    <input type="number" step="0.01" min="0" value={(*props.monto_mensual).clone()} oninput={input_cb!(props.monto_mensual)}
                        class={if fe.monto_mensual.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                    if let Some(ref msg) = fe.monto_mensual { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label">{"Depósito"}</label>
                    <input type="number" step="0.01" min="0" value={(*props.deposito).clone()} oninput={input_cb!(props.deposito)} class="gi-input" />
                </div>
                <div>
                    <label class="gi-label">{"Moneda"}</label>
                    <select onchange={select_cb!(props.moneda)} disabled={props.is_editing} class="gi-input">
                        <option value="DOP" selected={*props.moneda == "DOP"}>{"DOP"}</option>
                        <option value="USD" selected={*props.moneda == "USD"}>{"USD"}</option>
                    </select>
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

#[function_component]
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

#[function_component]
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
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
}

#[function_component]
fn ContratoList(props: &ContratoListProps) -> Html {
    if props.items.is_empty() {
        return html! {
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
                    <Link<Route> to={Route::Propiedades} classes="gi-btn-text">
                        {"Propiedades"}
                    </Link<Route>>
                    {" · "}
                    <Link<Route> to={Route::Inquilinos} classes="gi-btn-text">
                        {"Inquilinos"}
                    </Link<Route>>
                </div>
            </div>
        };
    }

    html! {
        <>
            <DataTable headers={props.headers.clone()}>
                { for props.items.iter().map(|c| {
                    let on_edit = props.on_edit.clone();
                    let on_delete_click = props.on_delete.clone();
                    let on_renew_click = props.on_renew.clone();
                    let on_terminate_click = props.on_terminate.clone();
                    let cc = c.clone(); let cd = c.clone(); let cr = c.clone(); let ct = c.clone();
                    let user_rol = props.user_rol.clone();
                    let p_label = props.prop_label.emit(c.propiedad_id.clone());
                    let i_label = props.inq_label.emit(c.inquilino_id.clone());
                    let (badge_cls, badge_label) = estado_badge(&c.estado);
                    let is_active = c.estado == "activo";
                    html! {
                        <tr>
                            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{p_label}</td>
                            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{i_label}</td>
                            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{format_date_display(&c.fecha_inicio)}</td>
                            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{format_date_display(&c.fecha_fin)}</td>
                            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);"><CurrencyDisplay monto={c.monto_mensual} moneda={c.moneda.clone()} /></td>
                            <td style="padding: var(--space-3) var(--space-5);"><span class={badge_cls}>{badge_label}</span></td>
                            if can_write(&user_rol) {
                                <td style="padding: var(--space-3) var(--space-5); display: flex; gap: var(--space-2); flex-wrap: wrap;">
                                    <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(cc.clone()))} class="gi-btn-text">{"Editar"}</button>
                                    if is_active {
                                        <Link<Route> to={Route::Pagos} classes="gi-btn-text gi-text-success">{"Registrar Pago"}</Link<Route>>
                                        <button onclick={Callback::from(move |_: MouseEvent| on_renew_click.emit(cr.clone()))} class="gi-btn-text" style="color: var(--color-primary-500);">{"Renovar"}</button>
                                        <button onclick={Callback::from(move |_: MouseEvent| on_terminate_click.emit(ct.clone()))} class="gi-btn-text" style="color: var(--color-warning);">{"Terminar"}</button>
                                    }
                                    if can_delete(&user_rol) {
                                        <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(cd.clone()))} class="gi-btn-text" style="color: var(--color-error);">{"Eliminar"}</button>
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
                .map(|_| ()),
            None => api_post::<Contrato, _>("/contratos", &create)
                .await
                .map(|_| ()),
        };
        match res {
            Ok(()) => {
                reset_form();
                reload.set(*reload + 1);
                push_toast(&toasts, "Contrato guardado", ToastKind::Success);
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
                push_toast(&toasts, "Contrato renovado", ToastKind::Success);
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
                push_toast(&toasts, "Contrato terminado", ToastKind::Success);
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

fn register_escape_listener(
    escape_handler: std::rc::Rc<std::cell::RefCell<Option<Box<dyn Fn()>>>>,
) -> Option<EventListener> {
    web_sys::window().and_then(|w| w.document()).map(|doc| {
        EventListener::new(&doc, "keydown", move |event| {
            let event = event.dyn_ref::<web_sys::KeyboardEvent>().unwrap();
            if event.key() == "Escape" {
                if let Some(ref cb) = *escape_handler.borrow() {
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
    let update = UpdateContrato {
        fecha_fin: Some((**fecha_fin).clone()),
        monto_mensual: Some(monto_val),
        deposito: dep,
        estado: Some((**estado).clone()),
    };
    let create = CreateContrato {
        propiedad_id: (**propiedad_id).clone(),
        inquilino_id: (**inquilino_id).clone(),
        fecha_inicio: (**fecha_inicio).clone(),
        fecha_fin: (**fecha_fin).clone(),
        monto_mensual: monto_val,
        deposito: dep,
        moneda: Some((**moneda).clone()),
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

#[allow(clippy::too_many_arguments)]
fn render_contratos_view(
    loading: bool,
    user_rol: &str,
    error: &UseStateHandle<Option<String>>,
    delete_target: &UseStateHandle<Option<Contrato>>,
    renew_target: &UseStateHandle<Option<Contrato>>,
    renew_fecha_fin: &UseStateHandle<String>,
    renew_monto: &UseStateHandle<String>,
    terminate_target: &UseStateHandle<Option<Contrato>>,
    terminate_fecha: &UseStateHandle<String>,
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
    propiedades: &UseStateHandle<Vec<Propiedad>>,
    inquilinos: &UseStateHandle<Vec<Inquilino>>,
    form_errors: &UseStateHandle<FormErrors>,
    submitting: bool,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
    on_new: Callback<MouseEvent>,
    on_delete_confirm: Callback<MouseEvent>,
    on_delete_cancel: Callback<MouseEvent>,
    on_renew_confirm: Callback<MouseEvent>,
    on_renew_cancel: Callback<MouseEvent>,
    on_terminate_confirm: Callback<MouseEvent>,
    on_terminate_cancel: Callback<MouseEvent>,
    items: &UseStateHandle<Vec<Contrato>>,
    total: u64,
    page: u64,
    per_page: u64,
    prop_label: Callback<String, String>,
    inq_label: Callback<String, String>,
    on_edit: Callback<Contrato>,
    on_delete_click: Callback<Contrato>,
    on_renew_click: Callback<Contrato>,
    on_terminate_click: Callback<Contrato>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
) -> Html {
    if loading {
        return html! { <Loading /> };
    }

    let headers = vec![
        "Propiedad".into(),
        "Inquilino".into(),
        "Inicio".into(),
        "Fin".into(),
        "Monto".into(),
        "Estado".into(),
        if can_write(user_rol) {
            "Acciones".into()
        } else {
            String::new()
        },
    ];

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Contratos"}</h1>
                if can_write(user_rol) {
                    <button onclick={on_new} class="gi-btn gi-btn-primary">{"+ Nuevo Contrato"}</button>
                }
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            if let Some(ref target) = **delete_target {
                <DeleteConfirmModal
                    message={format!("¿Está seguro de que desea eliminar el contrato de la propiedad \"{}\"? Esta acción no se puede deshacer.", prop_label.emit(target.propiedad_id.clone()))}
                    on_confirm={on_delete_confirm.clone()}
                    on_cancel={on_delete_cancel.clone()}
                />
            }

            if renew_target.is_some() {
                <RenewModal
                    renew_fecha_fin={renew_fecha_fin.clone()}
                    renew_monto={renew_monto.clone()}
                    on_confirm={on_renew_confirm}
                    on_cancel={on_renew_cancel}
                />
            }

            if terminate_target.is_some() {
                <TerminateModal
                    terminate_fecha={terminate_fecha.clone()}
                    on_confirm={on_terminate_confirm}
                    on_cancel={on_terminate_cancel}
                />
            }

            if show_form {
                <ContratoForm
                    is_editing={editing.is_some()}
                    propiedad_id={propiedad_id.clone()}
                    inquilino_id={inquilino_id.clone()}
                    fecha_inicio={fecha_inicio.clone()}
                    fecha_fin={fecha_fin.clone()}
                    monto_mensual={monto_mensual.clone()}
                    deposito={deposito.clone()}
                    moneda={moneda.clone()}
                    estado={estado.clone()}
                    propiedades={(**propiedades).clone()}
                    inquilinos={(**inquilinos).clone()}
                    form_errors={(**form_errors).clone()}
                    submitting={submitting}
                    on_submit={on_submit}
                    on_cancel={on_cancel}
                />
            }

            <ContratoList
                items={(**items).clone()}
                user_rol={user_rol.to_string()}
                headers={headers}
                total={total}
                page={page}
                per_page={per_page}
                prop_label={prop_label.clone()}
                inq_label={inq_label.clone()}
                on_edit={on_edit}
                on_delete={on_delete_click}
                on_renew={on_renew_click}
                on_terminate={on_terminate_click}
                on_page_change={on_page_change}
                on_per_page_change={on_per_page_change}
            />
        </div>
    }
}

#[function_component]
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

    let renew_target = use_state(|| Option::<Contrato>::None);
    let renew_fecha_fin = use_state(String::new);
    let renew_monto = use_state(String::new);
    let terminate_target = use_state(|| Option::<Contrato>::None);
    let terminate_fecha = use_state(String::new);

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
        use_effect_with((), move |_| {
            load_contrato_refs(propiedades, inquilinos);
        });
    }

    let prop_label = {
        let propiedades = propiedades.clone();
        Callback::from(move |id: String| -> String {
            propiedades
                .iter()
                .find(|p| p.id == id)
                .map(|p| format!("{} — {}", p.titulo, p.direccion))
                .unwrap_or_else(|| id.to_string())
        })
    };

    let inq_label = {
        let inquilinos = inquilinos.clone();
        Callback::from(move |id: String| -> String {
            inquilinos
                .iter()
                .find(|i| i.id == id)
                .map(|i| format!("{} {} ({})", i.nombre, i.apellido, i.cedula))
                .unwrap_or_else(|| id.to_string())
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
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        move || {
            propiedad_id.set(String::new());
            inquilino_id.set(String::new());
            fecha_inicio.set(String::new());
            fecha_fin.set(String::new());
            monto_mensual.set(String::new());
            deposito.set(String::new());
            moneda.set("DOP".into());
            estado.set("activo".into());
            editing.set(None);
            show_form.set(false);
            form_errors.set(FormErrors::default());
        }
    };

    let escape_handler = use_mut_ref(|| None::<Box<dyn Fn()>>);
    {
        let delete_target = delete_target.clone();
        let show_form = show_form.clone();
        let reset_form = reset_form.clone();
        let handler = escape_handler.clone();
        *handler.borrow_mut() = Some(Box::new(move || {
            handle_escape_contratos(&delete_target, &show_form, || reset_form());
        }) as Box<dyn Fn()>);
    }
    {
        let escape_handler = escape_handler.clone();
        use_effect_with((), move |_| {
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
        let editing = editing.clone();
        let error = error.clone();
        let reload = reload.clone();
        let reset_form = reset_form.clone();
        let validate_form = validate_form.clone();
        let toasts = toasts.clone();
        let submitting = submitting.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            handle_contrato_submit(
                submitting.clone(),
                || validate_form(),
                &monto_mensual,
                &deposito,
                &fecha_fin,
                &propiedad_id,
                &inquilino_id,
                &fecha_inicio,
                &moneda,
                &estado,
                &editing,
                reset_form.clone(),
                reload.clone(),
                error.clone(),
                toasts.clone(),
            );
        })
    };

    let on_cancel = super::page_helpers::cancel_cb(reset_form.clone());

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

    let on_renew_cancel = {
        let renew_target = renew_target.clone();
        Callback::from(move |_: MouseEvent| renew_target.set(None))
    };

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

    let on_terminate_cancel = {
        let terminate_target = terminate_target.clone();
        Callback::from(move |_: MouseEvent| terminate_target.set(None))
    };

    let (on_page_change, on_per_page_change) =
        super::page_helpers::pagination_cbs(&page, &per_page, &reload);

    render_contratos_view(
        *loading,
        &user_rol,
        &error,
        &delete_target,
        &renew_target,
        &renew_fecha_fin,
        &renew_monto,
        &terminate_target,
        &terminate_fecha,
        *show_form,
        &editing,
        &propiedad_id,
        &inquilino_id,
        &fecha_inicio,
        &fecha_fin,
        &monto_mensual,
        &deposito,
        &moneda,
        &estado,
        &propiedades,
        &inquilinos,
        &form_errors,
        *submitting,
        on_submit,
        on_cancel,
        on_new,
        on_delete_confirm,
        on_delete_cancel,
        on_renew_confirm,
        on_renew_cancel,
        on_terminate_confirm,
        on_terminate_cancel,
        &items,
        *total,
        *page,
        *per_page,
        prop_label,
        inq_label,
        on_edit,
        on_delete_click,
        on_renew_click,
        on_terminate_click,
        on_page_change,
        on_per_page_change,
    )
}
