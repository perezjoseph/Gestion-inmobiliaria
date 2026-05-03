use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::components::common::currency_display::CurrencyDisplay;
use crate::components::common::data_table::DataTable;
use crate::components::common::delete_confirm_modal::DeleteConfirmModal;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::pagination::Pagination;
use crate::components::common::skeleton::TableSkeleton;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_delete, api_get, api_post, api_put};
use crate::types::PaginatedResponse;
use crate::types::unidad::{CreateUnidad, Unidad, UpdateUnidad};
use crate::utils::{can_delete, can_write, field_error, input_class};

fn push_toast(toasts: Option<&ToastContext>, msg: &str, kind: ToastKind) {
    if let Some(t) = toasts {
        t.dispatch(ToastAction::Push(msg.into(), kind));
    }
}

// ---------------------------------------------------------------------------
// Estado badge
// ---------------------------------------------------------------------------

fn estado_badge(estado: &str) -> (&'static str, &'static str) {
    match estado {
        "disponible" => ("gi-badge gi-badge-success", "Disponible"),
        "ocupada" => ("gi-badge gi-badge-info", "Ocupada"),
        "mantenimiento" => ("gi-badge gi-badge-warning", "Mantenimiento"),
        _ => ("gi-badge gi-badge-neutral", "Desconocido"),
    }
}

// ---------------------------------------------------------------------------
// Form errors
// ---------------------------------------------------------------------------

#[derive(Clone, Default, PartialEq)]
struct FormErrors {
    numero_unidad: Option<String>,
}

impl FormErrors {
    const fn has_errors(&self) -> bool {
        self.numero_unidad.is_some()
    }
}

// ---------------------------------------------------------------------------
// Filter bar sub-component
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct UnidadFilterBarProps {
    filter_estado: UseStateHandle<String>,
    on_apply: Callback<MouseEvent>,
    on_clear: Callback<MouseEvent>,
}

#[component]
fn UnidadFilterBar(props: &UnidadFilterBarProps) -> Html {
    let fe = props.filter_estado.clone();
    let on_estado_change = Callback::from(move |e: Event| {
        let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
        fe.set(el.value());
    });
    html! {
        <div class="gi-filter-bar">
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--space-3); align-items: end;">
                <div>
                    <label class="gi-label">{"Estado"}</label>
                    <select onchange={on_estado_change} class="gi-input">
                        <option value="" selected={props.filter_estado.is_empty()}>{"Todos"}</option>
                        <option value="disponible" selected={*props.filter_estado == "disponible"}>{"Disponible"}</option>
                        <option value="ocupada" selected={*props.filter_estado == "ocupada"}>{"Ocupada"}</option>
                        <option value="mantenimiento" selected={*props.filter_estado == "mantenimiento"}>{"Mantenimiento"}</option>
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

// ---------------------------------------------------------------------------
// Form sub-component
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct UnidadFormProps {
    is_editing: bool,
    numero_unidad: UseStateHandle<String>,
    piso: UseStateHandle<String>,
    habitaciones: UseStateHandle<String>,
    banos: UseStateHandle<String>,
    area_m2: UseStateHandle<String>,
    descripcion: UseStateHandle<String>,
    precio: UseStateHandle<String>,
    moneda: UseStateHandle<String>,
    estado_form: UseStateHandle<String>,
    form_errors: FormErrors,
    submitting: bool,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
}

#[component]
fn UnidadForm(props: &UnidadFormProps) -> Html {
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
                {if props.is_editing { "Editar Unidad" } else { "Nueva Unidad" }}
            </h2>
            <form onsubmit={props.on_submit.clone()} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                <div>
                    <label class="gi-label">{"Número de Unidad *"}</label>
                    <input type="text" value={(*props.numero_unidad).clone()} oninput={input_cb!(props.numero_unidad)}
                        placeholder="Ej: 101, A-3" class={input_class(fe.numero_unidad.is_some())} />
                    {field_error(&fe.numero_unidad)}
                </div>
                <div>
                    <label class="gi-label">{"Piso"}</label>
                    <input type="number" value={(*props.piso).clone()} oninput={input_cb!(props.piso)} class="gi-input" />
                </div>
                <div>
                    <label class="gi-label">{"Habitaciones"}</label>
                    <input type="number" min="0" value={(*props.habitaciones).clone()} oninput={input_cb!(props.habitaciones)} class="gi-input" />
                </div>
                <div>
                    <label class="gi-label">{"Baños"}</label>
                    <input type="number" min="0" value={(*props.banos).clone()} oninput={input_cb!(props.banos)} class="gi-input" />
                </div>
                <div>
                    <label class="gi-label">{"Área (m²)"}</label>
                    <input type="number" step="0.01" min="0" value={(*props.area_m2).clone()} oninput={input_cb!(props.area_m2)} class="gi-input" />
                </div>
                <div>
                    <label class="gi-label">{"Precio *"}</label>
                    <input type="number" step="0.01" min="0" value={(*props.precio).clone()} oninput={input_cb!(props.precio)}
                        class="gi-input" />
                </div>
                <div>
                    <label class="gi-label">{"Moneda"}</label>
                    <select onchange={select_cb!(props.moneda)} class="gi-input">
                        <option value="DOP" selected={*props.moneda == "DOP"}>{"DOP"}</option>
                        <option value="USD" selected={*props.moneda == "USD"}>{"USD"}</option>
                    </select>
                </div>
                <div>
                    <label class="gi-label">{"Estado"}</label>
                    <select onchange={select_cb!(props.estado_form)} class="gi-input">
                        <option value="disponible" selected={*props.estado_form == "disponible"}>{"Disponible"}</option>
                        <option value="ocupada" selected={*props.estado_form == "ocupada"}>{"Ocupada"}</option>
                        <option value="mantenimiento" selected={*props.estado_form == "mantenimiento"}>{"Mantenimiento"}</option>
                    </select>
                </div>
                <div style="grid-column: 1 / -1;">
                    <label class="gi-label">{"Descripción"}</label>
                    <input type="text" value={(*props.descripcion).clone()} oninput={input_cb!(props.descripcion)}
                        placeholder="Descripción opcional" class="gi-input" />
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

// ---------------------------------------------------------------------------
// List sub-component
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct UnidadListProps {
    items: Vec<Unidad>,
    user_rol: String,
    headers: Vec<String>,
    total: u64,
    page: u64,
    per_page: u64,
    on_edit: Callback<Unidad>,
    on_delete: Callback<Unidad>,
    on_new: Callback<MouseEvent>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
}

fn render_unidad_row(
    u: &Unidad,
    user_rol: &str,
    on_edit: &Callback<Unidad>,
    on_delete: &Callback<Unidad>,
) -> Html {
    let (badge_cls, badge_label) = estado_badge(&u.estado);
    let area_display = u
        .area_m2
        .map_or_else(|| "—".to_string(), |a| format!("{a:.2} m²"));
    let uc = u.clone();
    let ud = u.clone();
    let on_edit = on_edit.clone();
    let on_delete_click = on_delete.clone();
    let actions = if can_write(user_rol) {
        let delete_btn = if can_delete(user_rol) {
            html! { <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(ud.clone()))} class="gi-btn-text" style="color: var(--color-error);">{"Eliminar"}</button> }
        } else {
            html! {}
        };
        html! {
            <td style="padding: var(--space-3) var(--space-5);">
                <div class="gi-row-actions">
                    <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(uc.clone()))} class="gi-btn-text">{"Editar"}</button>
                    {delete_btn}
                </div>
            </td>
        }
    } else {
        html! {}
    };
    html! {
        <tr>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{&u.numero_unidad}</td>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{u.piso.map_or_else(|| "—".to_string(), |v| v.to_string())}</td>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{u.habitaciones.map_or_else(|| "—".to_string(), |v| v.to_string())}</td>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{u.banos.map_or_else(|| "—".to_string(), |v| v.to_string())}</td>
            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{area_display}</td>
            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);"><CurrencyDisplay monto={u.precio} moneda={u.moneda.clone()} /></td>
            <td style="padding: var(--space-3) var(--space-5);"><span class={badge_cls}>{badge_label}</span></td>
            {actions}
        </tr>
    }
}

fn render_unidad_empty_state(user_rol: &str, on_new: &Callback<MouseEvent>) -> Html {
    html! {
        <div class="gi-empty-state">
            <div class="gi-empty-state-icon">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4">
                    <rect x="3" y="3" width="7" height="7" rx="1"/>
                    <rect x="14" y="3" width="7" height="7" rx="1"/>
                    <rect x="3" y="14" width="7" height="7" rx="1"/>
                    <rect x="14" y="14" width="7" height="7" rx="1"/>
                </svg>
            </div>
            <div class="gi-empty-state-title">{"Sin unidades registradas"}</div>
            <p class="gi-empty-state-text">{"Agregue las unidades de esta propiedad (apartamentos, locales, oficinas)."}</p>
            if can_write(user_rol) {
                <button onclick={on_new.clone()} class="gi-btn gi-btn-primary" style="margin-top: var(--space-3);">{"+ Nueva Unidad"}</button>
            }
        </div>
    }
}

#[component]
fn UnidadList(props: &UnidadListProps) -> Html {
    if props.items.is_empty() {
        return render_unidad_empty_state(&props.user_rol, &props.on_new);
    }

    html! {
        <>
            <DataTable headers={props.headers.clone()}>
                { for props.items.iter().map(|u| render_unidad_row(u, &props.user_rol, &props.on_edit, &props.on_delete)) }
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

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn validate_unidad_fields(numero_unidad: &str) -> FormErrors {
    let mut errs = FormErrors::default();
    if numero_unidad.trim().is_empty() {
        errs.numero_unidad = Some("El número de unidad es requerido".into());
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

fn parse_opt_i32(s: &str) -> Option<i32> {
    if s.trim().is_empty() {
        None
    } else {
        s.parse().ok()
    }
}

fn parse_opt_f64(s: &str) -> Option<f64> {
    if s.trim().is_empty() {
        None
    } else {
        s.parse().ok()
    }
}

fn build_unidades_url(propiedad_id: &str, pg: u64, pp: u64, f_estado: &str) -> String {
    let mut params = vec![format!("page={pg}"), format!("perPage={pp}")];
    if !f_estado.is_empty() {
        params.push(format!("estado={f_estado}"));
    }
    format!("/propiedades/{propiedad_id}/unidades?{}", params.join("&"))
}

fn load_unidades_data(
    items: UseStateHandle<Vec<Unidad>>,
    total: UseStateHandle<u64>,
    error: UseStateHandle<Option<String>>,
    loading: UseStateHandle<bool>,
    url: String,
) {
    spawn_local(async move {
        loading.set(true);
        match api_get::<PaginatedResponse<Unidad>>(&url).await {
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
fn do_save_unidad(
    propiedad_id: String,
    editing_id: Option<String>,
    update: UpdateUnidad,
    create: CreateUnidad,
    reset_form: impl Fn() + 'static,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
    submitting: UseStateHandle<bool>,
) {
    spawn_local(async move {
        let res = match editing_id {
            Some(id) => api_put::<Unidad, _>(
                &format!("/propiedades/{propiedad_id}/unidades/{id}"),
                &update,
            )
            .await
            .map(|_| ()),
            None => {
                api_post::<Unidad, _>(&format!("/propiedades/{propiedad_id}/unidades"), &create)
                    .await
                    .map(|_| ())
            }
        };
        match res {
            Ok(()) => {
                reset_form();
                reload.set(*reload + 1);
                push_toast(toasts.as_ref(), "Unidad guardada", ToastKind::Success);
            }
            Err(err) => error.set(Some(err)),
        }
        submitting.set(false);
    });
}

fn do_delete_unidad(
    propiedad_id: String,
    id: String,
    label: String,
    delete_target: UseStateHandle<Option<Unidad>>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
) {
    spawn_local(async move {
        match api_delete(&format!("/propiedades/{propiedad_id}/unidades/{id}")).await {
            Ok(()) => {
                delete_target.set(None);
                reload.set(*reload + 1);
                if let Some(t) = &toasts {
                    t.dispatch(ToastAction::Push(
                        format!("Unidad \"{label}\" eliminada"),
                        ToastKind::Info,
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

fn register_escape_listener(escape_handler: crate::utils::EscapeHandler) -> Option<EventListener> {
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
fn make_unidad_edit_cb(
    numero_unidad: &UseStateHandle<String>,
    piso: &UseStateHandle<String>,
    habitaciones: &UseStateHandle<String>,
    banos: &UseStateHandle<String>,
    area_m2: &UseStateHandle<String>,
    descripcion: &UseStateHandle<String>,
    precio: &UseStateHandle<String>,
    moneda: &UseStateHandle<String>,
    estado_form: &UseStateHandle<String>,
    editing: &UseStateHandle<Option<Unidad>>,
    show_form: &UseStateHandle<bool>,
    form_errors: &UseStateHandle<FormErrors>,
) -> Callback<Unidad> {
    let (numero_unidad, piso, habitaciones) =
        (numero_unidad.clone(), piso.clone(), habitaciones.clone());
    let (banos, area_m2, descripcion) = (banos.clone(), area_m2.clone(), descripcion.clone());
    let (precio, moneda, estado_form) = (precio.clone(), moneda.clone(), estado_form.clone());
    let (editing, show_form, form_errors) =
        (editing.clone(), show_form.clone(), form_errors.clone());
    Callback::from(move |u: Unidad| {
        numero_unidad.set(u.numero_unidad.clone());
        piso.set(u.piso.map_or_else(String::new, |v| v.to_string()));
        habitaciones.set(u.habitaciones.map_or_else(String::new, |v| v.to_string()));
        banos.set(u.banos.map_or_else(String::new, |v| v.to_string()));
        area_m2.set(u.area_m2.map_or_else(String::new, |v| v.to_string()));
        descripcion.set(u.descripcion.clone().unwrap_or_default());
        precio.set(u.precio.to_string());
        moneda.set(u.moneda.clone());
        estado_form.set(u.estado.clone());
        editing.set(Some(u));
        show_form.set(true);
        form_errors.set(FormErrors::default());
    })
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq, Eq)]
pub struct UnidadesTabProps {
    pub propiedad_id: AttrValue,
}

#[component]
#[allow(clippy::redundant_clone)]
pub fn UnidadesTab(props: &UnidadesTabProps) -> Html {
    let auth = use_context::<AuthContext>();
    let toasts = use_context::<ToastContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();

    let items = use_state(Vec::<Unidad>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let show_form = use_state(|| false);
    let editing = use_state(|| Option::<Unidad>::None);
    let delete_target = use_state(|| Option::<Unidad>::None);
    let submitting = use_state(|| false);
    let form_errors = use_state(FormErrors::default);
    let reload = use_state(|| 0u32);

    // Form fields
    let numero_unidad = use_state(String::new);
    let piso = use_state(String::new);
    let habitaciones = use_state(String::new);
    let banos = use_state(String::new);
    let area_m2 = use_state(String::new);
    let descripcion = use_state(String::new);
    let precio = use_state(|| "0".to_string());
    let moneda = use_state(|| "DOP".to_string());
    let estado_form = use_state(|| "disponible".to_string());

    let filter_estado = use_state(String::new);

    // Data fetching — depends on (reload_val, page) only
    {
        let items = items.clone();
        let total = total.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        let pg = *page;
        let pp = *per_page;
        let prop_id = props.propiedad_id.to_string();
        let f_estado = (*filter_estado).clone();
        use_effect_with((reload_val, pg), move |_| {
            let url = build_unidades_url(&prop_id, pg, pp, &f_estado);
            load_unidades_data(items, total, error, loading, url);
        });
    }

    // Reset form closure
    let reset_form = {
        let numero_unidad = numero_unidad.clone();
        let piso = piso.clone();
        let habitaciones = habitaciones.clone();
        let banos = banos.clone();
        let area_m2 = area_m2.clone();
        let descripcion = descripcion.clone();
        let precio = precio.clone();
        let moneda = moneda.clone();
        let estado_form = estado_form.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        move || {
            numero_unidad.set(String::new());
            piso.set(String::new());
            habitaciones.set(String::new());
            banos.set(String::new());
            area_m2.set(String::new());
            descripcion.set(String::new());
            precio.set("0".into());
            moneda.set("DOP".into());
            estado_form.set("disponible".into());
            editing.set(None);
            show_form.set(false);
            form_errors.set(FormErrors::default());
        }
    };

    // Escape key handler
    let escape_handler = use_mut_ref(|| None::<Box<dyn Fn()>>);
    {
        let delete_target = delete_target.clone();
        let show_form = show_form.clone();
        let reset_form = reset_form.clone();
        let handler = escape_handler.clone();
        *handler.borrow_mut() = Some(Box::new(move || {
            if delete_target.is_some() {
                delete_target.set(None);
            } else if *show_form {
                reset_form();
            }
        }) as Box<dyn Fn()>);
    }
    {
        let escape_handler = escape_handler.clone();
        use_effect_with((), move |()| {
            let listener = register_escape_listener(escape_handler);
            move || drop(listener)
        });
    }

    // Callbacks
    let on_new = {
        let reset_form = reset_form.clone();
        let show_form = show_form.clone();
        Callback::from(move |_: MouseEvent| {
            reset_form();
            show_form.set(true);
        })
    };

    let on_edit = make_unidad_edit_cb(
        &numero_unidad,
        &piso,
        &habitaciones,
        &banos,
        &area_m2,
        &descripcion,
        &precio,
        &moneda,
        &estado_form,
        &editing,
        &show_form,
        &form_errors,
    );

    let on_delete_click = {
        let dt = delete_target.clone();
        Callback::from(move |u: Unidad| dt.set(Some(u)))
    };

    let on_delete_confirm = {
        let error = error.clone();
        let reload = reload.clone();
        let delete_target = delete_target.clone();
        let toasts = toasts.clone();
        let prop_id = props.propiedad_id.to_string();
        Callback::from(move |_: MouseEvent| {
            if let Some(ref u) = *delete_target {
                do_delete_unidad(
                    prop_id.clone(),
                    u.id.clone(),
                    u.numero_unidad.clone(),
                    delete_target.clone(),
                    reload.clone(),
                    error.clone(),
                    toasts.clone(),
                );
            }
        })
    };

    let on_delete_cancel = {
        let dt = delete_target.clone();
        Callback::from(move |_: MouseEvent| dt.set(None))
    };

    let validate_form = {
        let numero_unidad = numero_unidad.clone();
        let form_errors = form_errors.clone();
        move || -> bool {
            let errs = validate_unidad_fields(&numero_unidad);
            let valid = !errs.has_errors();
            form_errors.set(errs);
            valid
        }
    };

    let on_submit = {
        let numero_unidad = numero_unidad.clone();
        let piso = piso.clone();
        let habitaciones = habitaciones.clone();
        let banos = banos.clone();
        let area_m2 = area_m2.clone();
        let descripcion = descripcion.clone();
        let precio = precio.clone();
        let moneda = moneda.clone();
        let estado_form = estado_form.clone();
        let editing = editing.clone();
        let error = error.clone();
        let reload = reload.clone();
        let reset_form = reset_form.clone();
        let submitting = submitting.clone();
        let toasts = toasts.clone();
        let prop_id = props.propiedad_id.to_string();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if *submitting || !validate_form() {
                return;
            }
            let precio_val = precio.parse::<f64>().unwrap_or(0.0);
            submitting.set(true);
            let editing_id = editing.as_ref().map(|u| u.id.clone());
            let update = UpdateUnidad {
                numero_unidad: Some((*numero_unidad).clone()),
                piso: parse_opt_i32(&piso),
                habitaciones: parse_opt_i32(&habitaciones),
                banos: parse_opt_i32(&banos),
                area_m2: parse_opt_f64(&area_m2),
                precio: Some(precio_val),
                moneda: Some((*moneda).clone()),
                estado: Some((*estado_form).clone()),
                descripcion: non_empty(&descripcion),
            };
            let create = CreateUnidad {
                numero_unidad: (*numero_unidad).clone(),
                piso: parse_opt_i32(&piso),
                habitaciones: parse_opt_i32(&habitaciones),
                banos: parse_opt_i32(&banos),
                area_m2: parse_opt_f64(&area_m2),
                precio: precio_val,
                moneda: Some((*moneda).clone()),
                estado: Some((*estado_form).clone()),
                descripcion: non_empty(&descripcion),
            };
            do_save_unidad(
                prop_id.clone(),
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

    let on_cancel = {
        let reset_form = reset_form;
        Callback::from(move |_: MouseEvent| reset_form())
    };

    let on_filter_apply = {
        let page = page.clone();
        let reload = reload.clone();
        Callback::from(move |_: MouseEvent| {
            page.set(1);
            reload.set(*reload + 1);
        })
    };

    let on_filter_clear = {
        let fe = filter_estado.clone();
        let page = page.clone();
        let reload = reload.clone();
        Callback::from(move |_: MouseEvent| {
            fe.set(String::new());
            page.set(1);
            reload.set(*reload + 1);
        })
    };

    let on_page_change = {
        let p = page.clone();
        let r = reload.clone();
        Callback::from(move |v: u64| {
            p.set(v);
            r.set(*r + 1);
        })
    };
    let on_per_page_change = {
        let pp = per_page.clone();
        let p = page.clone();
        let r = reload.clone();
        Callback::from(move |v: u64| {
            pp.set(v);
            p.set(1);
            r.set(*r + 1);
        })
    };

    render_unidades_view(
        &loading,
        &user_rol,
        &error,
        &delete_target,
        on_delete_confirm,
        on_delete_cancel,
        filter_estado,
        on_filter_apply,
        on_filter_clear,
        &show_form,
        &editing,
        numero_unidad,
        piso,
        habitaciones,
        banos,
        area_m2,
        descripcion,
        precio,
        moneda,
        estado_form,
        &form_errors,
        &submitting,
        on_submit,
        on_cancel,
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

// ---------------------------------------------------------------------------
// Render view (split from main component to keep html! blocks small)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn render_unidades_view(
    loading: &UseStateHandle<bool>,
    user_rol: &str,
    error: &UseStateHandle<Option<String>>,
    delete_target: &UseStateHandle<Option<Unidad>>,
    on_delete_confirm: Callback<MouseEvent>,
    on_delete_cancel: Callback<MouseEvent>,
    filter_estado: UseStateHandle<String>,
    on_filter_apply: Callback<MouseEvent>,
    on_filter_clear: Callback<MouseEvent>,
    show_form: &UseStateHandle<bool>,
    editing: &UseStateHandle<Option<Unidad>>,
    numero_unidad: UseStateHandle<String>,
    piso: UseStateHandle<String>,
    habitaciones: UseStateHandle<String>,
    banos: UseStateHandle<String>,
    area_m2: UseStateHandle<String>,
    descripcion: UseStateHandle<String>,
    precio: UseStateHandle<String>,
    moneda: UseStateHandle<String>,
    estado_form: UseStateHandle<String>,
    form_errors: &UseStateHandle<FormErrors>,
    submitting: &UseStateHandle<bool>,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
    items: &UseStateHandle<Vec<Unidad>>,
    total: &UseStateHandle<u64>,
    page: &UseStateHandle<u64>,
    per_page: &UseStateHandle<u64>,
    on_edit: Callback<Unidad>,
    on_delete_click: Callback<Unidad>,
    on_new: Callback<MouseEvent>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
) -> Html {
    if **loading {
        return html! { <TableSkeleton title_width="160px" columns={8} has_filter=true /> };
    }

    let last_header: String = if can_write(user_rol) {
        "Acciones".into()
    } else {
        String::new()
    };
    let headers: Vec<String> = vec![
        "Número".into(),
        "Piso".into(),
        "Hab.".into(),
        "Baños".into(),
        "Área".into(),
        "Precio".into(),
        "Estado".into(),
        last_header,
    ];

    let new_btn = if can_write(user_rol) {
        html! { <button onclick={on_new.clone()} class="gi-btn gi-btn-primary">{"+ Nueva Unidad"}</button> }
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
        |u| html! {
            <DeleteConfirmModal
                message={format!("¿Está seguro de que desea eliminar la unidad \"{}\"? Esta acción no se puede deshacer.", u.numero_unidad)}
                on_confirm={on_delete_confirm} on_cancel={on_delete_cancel}
            />
        },
    );

    let form_html = if **show_form {
        html! {
            <UnidadForm
                is_editing={editing.is_some()}
                numero_unidad={numero_unidad}
                piso={piso} habitaciones={habitaciones} banos={banos}
                area_m2={area_m2} descripcion={descripcion}
                precio={precio} moneda={moneda} estado_form={estado_form}
                form_errors={(**form_errors).clone()} submitting={**submitting}
                on_submit={on_submit} on_cancel={on_cancel}
            />
        }
    } else {
        html! {}
    };

    html! {
        <div>
            <div class="gi-page-header">
                <h2 class="gi-page-title" style="font-size: var(--text-lg);">{"Unidades"}</h2>
                {new_btn}
            </div>
            {error_html}
            {delete_html}
            <UnidadFilterBar
                filter_estado={filter_estado}
                on_apply={on_filter_apply} on_clear={on_filter_clear}
            />
            {form_html}
            <UnidadList
                items={(**items).clone()} user_rol={user_rol.to_string()} headers={headers}
                total={**total} page={**page} per_page={**per_page}
                on_edit={on_edit} on_delete={on_delete_click} on_new={on_new}
                on_page_change={on_page_change} on_per_page_change={on_per_page_change}
            />
        </div>
    }
}
