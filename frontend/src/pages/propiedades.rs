use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::components::common::currency_display::CurrencyDisplay;
use crate::components::common::data_table::DataTable;
use crate::components::common::delete_confirm_modal::DeleteConfirmModal;
use crate::components::common::document_gallery::DocumentGallery;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::loading::Loading;
use crate::components::common::pagination::Pagination;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_delete, api_get, api_post, api_put};
use crate::types::PaginatedResponse;
use crate::types::propiedad::{CreatePropiedad, Propiedad, UpdatePropiedad};
use crate::utils::{can_delete, can_write};

fn push_toast(toasts: &Option<ToastContext>, msg: &str, kind: ToastKind) {
    if let Some(t) = toasts {
        t.dispatch(ToastAction::Push(msg.into(), kind));
    }
}

#[derive(Properties, PartialEq)]
struct PropiedadFilterBarProps {
    filter_ciudad: UseStateHandle<String>,
    filter_tipo: UseStateHandle<String>,
    filter_estado: UseStateHandle<String>,
    on_apply: Callback<MouseEvent>,
    on_clear: Callback<MouseEvent>,
}

#[function_component]
fn PropiedadFilterBar(props: &PropiedadFilterBarProps) -> Html {
    let fc = props.filter_ciudad.clone();
    let on_ciudad_input = Callback::from(move |e: InputEvent| {
        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
        fc.set(input.value());
    });
    let ft = props.filter_tipo.clone();
    let on_tipo_change = Callback::from(move |e: Event| {
        let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
        ft.set(el.value());
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
                    <label class="gi-label">{"Ciudad"}</label>
                    <input type="text" value={(*props.filter_ciudad).clone()} oninput={on_ciudad_input}
                        class="gi-input" placeholder="Filtrar por ciudad" />
                </div>
                <div>
                    <label class="gi-label">{"Tipo"}</label>
                    <select onchange={on_tipo_change} class="gi-input">
                        <option value="" selected={props.filter_tipo.is_empty()}>{"Todos"}</option>
                        <option value="casa" selected={*props.filter_tipo == "casa"}>{"Casa"}</option>
                        <option value="apartamento" selected={*props.filter_tipo == "apartamento"}>{"Apartamento"}</option>
                        <option value="local" selected={*props.filter_tipo == "local"}>{"Local"}</option>
                        <option value="terreno" selected={*props.filter_tipo == "terreno"}>{"Terreno"}</option>
                        <option value="oficina" selected={*props.filter_tipo == "oficina"}>{"Oficina"}</option>
                    </select>
                </div>
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

fn estado_badge(estado: &str) -> (&'static str, &'static str) {
    match estado {
        "disponible" => ("gi-badge gi-badge-success", "Disponible"),
        "ocupada" => ("gi-badge gi-badge-info", "Ocupada"),
        "mantenimiento" => ("gi-badge gi-badge-warning", "Mantenimiento"),
        _ => ("gi-badge gi-badge-neutral", "Desconocido"),
    }
}

#[derive(Clone, Default, PartialEq)]
struct FormErrors {
    titulo: Option<String>,
    direccion: Option<String>,
    ciudad: Option<String>,
    provincia: Option<String>,
    precio: Option<String>,
}

impl FormErrors {
    fn has_errors(&self) -> bool {
        self.titulo.is_some()
            || self.direccion.is_some()
            || self.ciudad.is_some()
            || self.provincia.is_some()
            || self.precio.is_some()
    }
}

#[derive(Properties, PartialEq)]
struct PropiedadFormProps {
    is_editing: bool,
    titulo: UseStateHandle<String>,
    descripcion: UseStateHandle<String>,
    direccion: UseStateHandle<String>,
    ciudad: UseStateHandle<String>,
    provincia: UseStateHandle<String>,
    tipo_propiedad: UseStateHandle<String>,
    habitaciones: UseStateHandle<String>,
    banos: UseStateHandle<String>,
    area_m2: UseStateHandle<String>,
    precio: UseStateHandle<String>,
    moneda: UseStateHandle<String>,
    estado: UseStateHandle<String>,
    form_errors: FormErrors,
    submitting: bool,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
    #[prop_or_default]
    editing_id: Option<String>,
    #[prop_or_default]
    token: String,
}

#[function_component]
fn PropiedadForm(props: &PropiedadFormProps) -> Html {
    let show_optional = use_state(|| false);
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

    let toggle_optional = {
        let show_optional = show_optional.clone();
        Callback::from(move |_: MouseEvent| {
            show_optional.set(!*show_optional);
        })
    };
    let opt_open = *show_optional;

    html! {
        <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
            <h2 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">
                {if props.is_editing { "Editar Propiedad" } else { "Nueva Propiedad" }}</h2>
            <form onsubmit={props.on_submit.clone()} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                <div>
                    <label class="gi-label">{"Título *"}</label>
                    <input type="text" value={(*props.titulo).clone()} oninput={input_cb!(props.titulo)}
                        class={if fe.titulo.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                    if let Some(ref msg) = fe.titulo { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label">{"Dirección *"}</label>
                    <input type="text" value={(*props.direccion).clone()} oninput={input_cb!(props.direccion)}
                        class={if fe.direccion.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                    if let Some(ref msg) = fe.direccion { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label">{"Ciudad *"}</label>
                    <input type="text" value={(*props.ciudad).clone()} oninput={input_cb!(props.ciudad)}
                        class={if fe.ciudad.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                    if let Some(ref msg) = fe.ciudad { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label">{"Provincia *"}</label>
                    <input type="text" value={(*props.provincia).clone()} oninput={input_cb!(props.provincia)}
                        class={if fe.provincia.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                    if let Some(ref msg) = fe.provincia { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label">{"Precio *"}</label>
                    <input type="number" step="0.01" min="0" value={(*props.precio).clone()} oninput={input_cb!(props.precio)}
                        class={if fe.precio.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                    if let Some(ref msg) = fe.precio { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label">{"Moneda"}</label>
                    <select onchange={select_cb!(props.moneda)} class="gi-input">
                        <option value="DOP" selected={*props.moneda == "DOP"}>{"DOP"}</option>
                        <option value="USD" selected={*props.moneda == "USD"}>{"USD"}</option>
                    </select>
                </div>
                <div>
                    <label class="gi-label">{"Tipo"}</label>
                    <select onchange={select_cb!(props.tipo_propiedad)} class="gi-input">
                        <option value="casa" selected={*props.tipo_propiedad == "casa"}>{"Casa"}</option>
                        <option value="apartamento" selected={*props.tipo_propiedad == "apartamento"}>{"Apartamento"}</option>
                        <option value="local" selected={*props.tipo_propiedad == "local"}>{"Local"}</option>
                        <option value="terreno" selected={*props.tipo_propiedad == "terreno"}>{"Terreno"}</option>
                        <option value="oficina" selected={*props.tipo_propiedad == "oficina"}>{"Oficina"}</option>
                    </select>
                </div>
                <div>
                    <label class="gi-label">{"Estado"}</label>
                    <select onchange={select_cb!(props.estado)} class="gi-input">
                        <option value="disponible" selected={*props.estado == "disponible"}>{"Disponible"}</option>
                        <option value="ocupada" selected={*props.estado == "ocupada"}>{"Ocupada"}</option>
                        <option value="mantenimiento" selected={*props.estado == "mantenimiento"}>{"Mantenimiento"}</option>
                    </select>
                </div>
                <div style="grid-column: 1 / -1;">
                    <button type="button" class="gi-collapsible-trigger" onclick={toggle_optional}>
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                            style={if opt_open { "transform: rotate(90deg); transition: transform 0.2s;" } else { "transition: transform 0.2s;" }}>
                            <path d="M9 18l6-6-6-6"/>
                        </svg>
                        {" Campos opcionales"}
                    </button>
                    <div class={if opt_open { "gi-collapsible-content open" } else { "gi-collapsible-content" }}>
                        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4); padding-top: var(--space-3);">
                            <div style="grid-column: 1 / -1;">
                                <label class="gi-label">{"Descripción"}</label>
                                <input type="text" value={(*props.descripcion).clone()} oninput={input_cb!(props.descripcion)} class="gi-input" />
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
                        </div>
                    </div>
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
                        entity_type={"propiedad".to_string()}
                        entity_id={id.clone()}
                        token={props.token.clone()}
                    />
                </div>
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct PropiedadListProps {
    items: Vec<Propiedad>,
    user_rol: String,
    headers: Vec<String>,
    sortable_fields: Vec<String>,
    current_sort: Option<String>,
    current_order: Option<String>,
    total: u64,
    page: u64,
    per_page: u64,
    on_sort: Callback<(String, String)>,
    on_edit: Callback<Propiedad>,
    on_delete: Callback<Propiedad>,
    on_new: Callback<MouseEvent>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
}

#[function_component]
fn PropiedadList(props: &PropiedadListProps) -> Html {
    if props.items.is_empty() {
        return html! {
            <div class="gi-empty-state">
                <div class="gi-empty-state-icon">
                    <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4">
                        <path d="M3 21V9l9-6 9 6v12"/><path d="M9 21V12h6v9"/>
                    </svg>
                </div>
                <div class="gi-empty-state-title">{"Sin propiedades registradas"}</div>
                <p class="gi-empty-state-text">{"Las propiedades son la base de su portafolio. Agregue la primera para luego asignar inquilinos y contratos."}</p>
                if can_write(&props.user_rol) {
                    <button onclick={props.on_new.clone()} class="gi-btn gi-btn-primary" style="margin-top: var(--space-4);">
                        {"+ Nueva Propiedad"}
                    </button>
                }
            </div>
        };
    }

    html! {
        <>
            <DataTable headers={props.headers.clone()} sortable_fields={props.sortable_fields.clone()} current_sort={props.current_sort.clone()} current_order={props.current_order.clone()} on_sort={props.on_sort.clone()}>
                { for props.items.iter().map(|p| {
                    let on_edit = props.on_edit.clone();
                    let on_delete_click = props.on_delete.clone();
                    let pc = p.clone();
                    let pd = p.clone();
                    let user_rol = props.user_rol.clone();
                    let (badge_cls, badge_label) = estado_badge(&p.estado);
                    let tipo_label = match p.tipo_propiedad.as_str() {
                        "casa" => "Casa",
                        "apartamento" => "Apartamento",
                        "local" => "Local",
                        "terreno" => "Terreno",
                        "oficina" => "Oficina",
                        other => other,
                    };
                    html! {
                        <tr>
                            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{&p.titulo}</td>
                            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{&p.direccion}</td>
                            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">{&p.ciudad}</td>
                            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{tipo_label}</td>
                            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);"><CurrencyDisplay monto={p.precio} moneda={p.moneda.clone()} /></td>
                            <td style="padding: var(--space-3) var(--space-5);"><span class={badge_cls}>{badge_label}</span></td>
                            if can_write(&user_rol) {
                                <td style="padding: var(--space-3) var(--space-5); display: flex; gap: var(--space-2);">
                                    <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(pc.clone()))}
                                        class="gi-btn-text">{"Editar"}</button>
                                    if can_delete(&user_rol) {
                                        <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(pd.clone()))}
                                            class="gi-btn-text" style="color: var(--color-error);">{"Eliminar"}</button>
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

fn validate_propiedad_fields(
    titulo: &str,
    direccion: &str,
    ciudad: &str,
    provincia: &str,
    precio: &str,
) -> FormErrors {
    let mut errs = FormErrors::default();
    if titulo.trim().is_empty() {
        errs.titulo = Some("El título es obligatorio".into());
    }
    if direccion.trim().is_empty() {
        errs.direccion = Some("La dirección es obligatoria".into());
    }
    if ciudad.trim().is_empty() {
        errs.ciudad = Some("La ciudad es obligatoria".into());
    }
    if provincia.trim().is_empty() {
        errs.provincia = Some("La provincia es obligatoria".into());
    }
    match precio.parse::<f64>() {
        Ok(v) if v <= 0.0 => errs.precio = Some("El precio debe ser mayor a 0".into()),
        Err(_) => errs.precio = Some("Precio inválido".into()),
        _ => {}
    }
    errs
}

fn non_empty_prop(s: &str) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn do_save_propiedad(
    editing_id: Option<String>,
    update: UpdatePropiedad,
    create: CreatePropiedad,
    reset_form: impl Fn() + 'static,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
    submitting: UseStateHandle<bool>,
) {
    spawn_local(async move {
        let res = match editing_id {
            Some(id) => api_put::<Propiedad, _>(&format!("/propiedades/{id}"), &update)
                .await
                .map(|_| ()),
            None => api_post::<Propiedad, _>("/propiedades", &create)
                .await
                .map(|_| ()),
        };
        match res {
            Ok(()) => {
                reset_form();
                reload.set(*reload + 1);
                push_toast(&toasts, "Propiedad guardada", ToastKind::Success);
            }
            Err(err) => error.set(Some(err)),
        }
        submitting.set(false);
    });
}

fn do_delete_propiedad(
    id: String,
    label: String,
    delete_target: UseStateHandle<Option<Propiedad>>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
    reload_for_undo: UseStateHandle<u32>,
) {
    spawn_local(async move {
        match api_delete(&format!("/propiedades/{id}")).await {
            Ok(()) => {
                delete_target.set(None);
                reload.set(*reload + 1);
                let undo_reload = reload_for_undo;
                if let Some(t) = &toasts {
                    t.dispatch(ToastAction::PushWithUndo(
                        format!("\"{label}\" eliminada"),
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

fn register_escape_listener_p(
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

fn build_propiedades_url(
    pg: u64,
    pp: u64,
    fc: &str,
    ft: &str,
    fe: &str,
    sf: &Option<String>,
    so: &Option<String>,
) -> String {
    let mut params = vec![format!("page={pg}"), format!("perPage={pp}")];
    if !fc.is_empty() {
        params.push(format!("ciudad={fc}"));
    }
    if !ft.is_empty() {
        params.push(format!("tipoPropiedad={ft}"));
    }
    if !fe.is_empty() {
        params.push(format!("estado={fe}"));
    }
    if let Some(field) = sf {
        params.push(format!("sortBy={field}"));
    }
    if let Some(order) = so {
        params.push(format!("sortOrder={order}"));
    }
    format!("/propiedades?{}", params.join("&"))
}

#[function_component]
pub fn Propiedades() -> Html {
    let auth = use_context::<AuthContext>();
    let toasts = use_context::<ToastContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();

    let items = use_state(Vec::<Propiedad>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let show_form = use_state(|| false);
    let editing = use_state(|| Option::<Propiedad>::None);
    let delete_target = use_state(|| Option::<Propiedad>::None);
    let form_errors = use_state(FormErrors::default);
    let reload = use_state(|| 0u32);
    let submitting = use_state(|| false);

    let titulo = use_state(String::new);
    let descripcion = use_state(String::new);
    let direccion = use_state(String::new);
    let ciudad = use_state(String::new);
    let provincia = use_state(String::new);
    let tipo_propiedad = use_state(|| "casa".to_string());
    let habitaciones = use_state(String::new);
    let banos = use_state(String::new);
    let area_m2 = use_state(String::new);
    let precio = use_state(String::new);
    let moneda = use_state(|| "DOP".to_string());
    let estado = use_state(|| "disponible".to_string());

    let filter_ciudad = use_state(String::new);
    let filter_tipo = use_state(String::new);
    let filter_estado = use_state(String::new);
    let sort_field = use_state(|| Option::<String>::None);
    let sort_order = use_state(|| Option::<String>::None);

    {
        let items = items.clone();
        let total = total.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        let pg = *page;
        let pp = *per_page;
        let fc = (*filter_ciudad).clone();
        let ft = (*filter_tipo).clone();
        let fe = (*filter_estado).clone();
        let sf = (*sort_field).clone();
        let so = (*sort_order).clone();
        use_effect_with(
            (
                reload_val,
                pg,
                fc.clone(),
                ft.clone(),
                fe.clone(),
                sf.clone(),
                so.clone(),
            ),
            move |_| {
                spawn_local(async move {
                    loading.set(true);
                    let url = build_propiedades_url(pg, pp, &fc, &ft, &fe, &sf, &so);
                    match api_get::<PaginatedResponse<Propiedad>>(&url).await {
                        Ok(resp) => {
                            items.set(resp.data);
                            total.set(resp.total);
                        }
                        Err(err) => error.set(Some(err)),
                    }
                    loading.set(false);
                });
            },
        );
    }

    let reset_form = {
        let titulo = titulo.clone();
        let descripcion = descripcion.clone();
        let direccion = direccion.clone();
        let ciudad = ciudad.clone();
        let provincia = provincia.clone();
        let tipo_propiedad = tipo_propiedad.clone();
        let habitaciones = habitaciones.clone();
        let banos = banos.clone();
        let area_m2 = area_m2.clone();
        let precio = precio.clone();
        let moneda = moneda.clone();
        let estado = estado.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        move || {
            titulo.set(String::new());
            descripcion.set(String::new());
            direccion.set(String::new());
            ciudad.set(String::new());
            provincia.set(String::new());
            tipo_propiedad.set("casa".into());
            habitaciones.set(String::new());
            banos.set(String::new());
            area_m2.set(String::new());
            precio.set(String::new());
            moneda.set("DOP".into());
            estado.set("disponible".into());
            editing.set(None);
            show_form.set(false);
            form_errors.set(FormErrors::default());
        }
    };

    let escape_handler = use_mut_ref(|| Option::<Box<dyn Fn()>>::None);
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
        use_effect_with((), move |_| {
            let listener = register_escape_listener_p(escape_handler);
            move || drop(listener)
        });
    }

    let on_new = {
        let reset_form = reset_form.clone();
        let show_form = show_form.clone();
        Callback::from(move |_: MouseEvent| {
            reset_form();
            show_form.set(true);
        })
    };

    let on_edit = {
        let titulo = titulo.clone();
        let descripcion = descripcion.clone();
        let direccion = direccion.clone();
        let ciudad = ciudad.clone();
        let provincia = provincia.clone();
        let tipo_propiedad = tipo_propiedad.clone();
        let habitaciones = habitaciones.clone();
        let banos = banos.clone();
        let area_m2 = area_m2.clone();
        let precio = precio.clone();
        let moneda = moneda.clone();
        let estado = estado.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        Callback::from(move |p: Propiedad| {
            titulo.set(p.titulo.clone());
            descripcion.set(p.descripcion.clone().unwrap_or_default());
            direccion.set(p.direccion.clone());
            ciudad.set(p.ciudad.clone());
            provincia.set(p.provincia.clone());
            tipo_propiedad.set(p.tipo_propiedad.clone());
            habitaciones.set(p.habitaciones.map(|v| v.to_string()).unwrap_or_default());
            banos.set(p.banos.map(|v| v.to_string()).unwrap_or_default());
            area_m2.set(p.area_m2.map(|v| v.to_string()).unwrap_or_default());
            precio.set(p.precio.to_string());
            moneda.set(p.moneda.clone());
            estado.set(p.estado.clone());
            editing.set(Some(p));
            show_form.set(true);
            form_errors.set(FormErrors::default());
        })
    };

    let on_delete_click = {
        let delete_target = delete_target.clone();
        Callback::from(move |p: Propiedad| {
            delete_target.set(Some(p));
        })
    };

    let on_delete_confirm = {
        let error = error.clone();
        let reload = reload.clone();
        let delete_target = delete_target.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(ref p) = *delete_target {
                do_delete_propiedad(
                    p.id.clone(),
                    p.titulo.clone(),
                    delete_target.clone(),
                    reload.clone(),
                    error.clone(),
                    toasts.clone(),
                    reload.clone(),
                );
            }
        })
    };

    let on_delete_cancel = {
        let delete_target = delete_target.clone();
        Callback::from(move |_: MouseEvent| {
            delete_target.set(None);
        })
    };

    let validate_form = {
        let titulo = titulo.clone();
        let direccion = direccion.clone();
        let ciudad = ciudad.clone();
        let provincia = provincia.clone();
        let precio = precio.clone();
        let form_errors = form_errors.clone();
        move || -> bool {
            let errs = validate_propiedad_fields(&titulo, &direccion, &ciudad, &provincia, &precio);
            let valid = !errs.has_errors();
            form_errors.set(errs);
            valid
        }
    };

    let on_submit = {
        let titulo = titulo.clone();
        let descripcion = descripcion.clone();
        let direccion = direccion.clone();
        let ciudad = ciudad.clone();
        let provincia = provincia.clone();
        let tipo_propiedad = tipo_propiedad.clone();
        let habitaciones = habitaciones.clone();
        let banos = banos.clone();
        let area_m2 = area_m2.clone();
        let precio = precio.clone();
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
            if *submitting || !validate_form() {
                return;
            }
            let Ok(precio_val) = precio.parse::<f64>() else {
                return;
            };
            submitting.set(true);
            let desc = non_empty_prop(&descripcion);
            let hab = habitaciones.parse::<i32>().ok();
            let ban = banos.parse::<i32>().ok();
            let area = area_m2.parse::<f64>().ok();
            let editing_id = editing.as_ref().map(|e| e.id.clone());
            let update = UpdatePropiedad {
                titulo: Some((*titulo).clone()),
                descripcion: desc.clone(),
                direccion: Some((*direccion).clone()),
                ciudad: Some((*ciudad).clone()),
                provincia: Some((*provincia).clone()),
                tipo_propiedad: Some((*tipo_propiedad).clone()),
                habitaciones: hab,
                banos: ban,
                area_m2: area,
                precio: Some(precio_val),
                moneda: Some((*moneda).clone()),
                estado: Some((*estado).clone()),
                imagenes: None,
            };
            let create = CreatePropiedad {
                titulo: (*titulo).clone(),
                descripcion: desc,
                direccion: (*direccion).clone(),
                ciudad: (*ciudad).clone(),
                provincia: (*provincia).clone(),
                tipo_propiedad: (*tipo_propiedad).clone(),
                habitaciones: hab,
                banos: ban,
                area_m2: area,
                precio: precio_val,
                moneda: Some((*moneda).clone()),
                estado: Some((*estado).clone()),
                imagenes: None,
            };
            do_save_propiedad(
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
        let reset_form = reset_form.clone();
        Callback::from(move |_: MouseEvent| reset_form())
    };

    let on_filter_apply = {
        let reload = reload.clone();
        let page = page.clone();
        Callback::from(move |_: MouseEvent| {
            page.set(1);
            reload.set(*reload + 1);
        })
    };
    let on_filter_clear = {
        let filter_ciudad = filter_ciudad.clone();
        let filter_tipo = filter_tipo.clone();
        let filter_estado = filter_estado.clone();
        let reload = reload.clone();
        let page = page.clone();
        Callback::from(move |_: MouseEvent| {
            filter_ciudad.set(String::new());
            filter_tipo.set(String::new());
            filter_estado.set(String::new());
            page.set(1);
            reload.set(*reload + 1);
        })
    };

    let on_page_change = {
        let page = page.clone();
        let reload = reload.clone();
        Callback::from(move |p: u64| {
            page.set(p);
            reload.set(*reload + 1);
        })
    };
    let on_per_page_change = {
        let per_page = per_page.clone();
        let page = page.clone();
        let reload = reload.clone();
        Callback::from(move |pp: u64| {
            per_page.set(pp);
            page.set(1);
            reload.set(*reload + 1);
        })
    };

    let on_sort = {
        let sort_field = sort_field.clone();
        let sort_order = sort_order.clone();
        let reload = reload.clone();
        let page = page.clone();
        Callback::from(move |(field, order): (String, String)| {
            sort_field.set(Some(field));
            sort_order.set(Some(order));
            page.set(1);
            reload.set(*reload + 1);
        })
    };

    if *loading {
        return html! { <Loading /> };
    }

    let headers = vec![
        "Título".into(),
        "Dirección".into(),
        "Ciudad".into(),
        "Tipo".into(),
        "Precio".into(),
        "Estado".into(),
        if can_write(&user_rol) {
            "Acciones".into()
        } else {
            String::new()
        },
    ];

    let sortable_fields: Vec<String> = vec![
        "titulo".into(),
        "direccion".into(),
        "ciudad".into(),
        "".into(),
        "precio".into(),
        "estado".into(),
        "".into(),
    ];

    let editing_id = editing.as_ref().map(|e| e.id.clone());
    let token = auth
        .as_ref()
        .and_then(|a| a.token.clone())
        .unwrap_or_default();

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Propiedades"}</h1>
                if can_write(&user_rol) {
                    <button onclick={on_new.clone()} class="gi-btn gi-btn-primary">{"+ Nueva Propiedad"}</button>
                }
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone();
                    move |_: MouseEvent| error.set(None)
                })} />
            }

            if let Some(ref target) = *delete_target {
                <DeleteConfirmModal
                    message={format!("¿Está seguro de que desea eliminar la propiedad \"{}\"? Esta acción no se puede deshacer.", target.titulo)}
                    on_confirm={on_delete_confirm.clone()}
                    on_cancel={on_delete_cancel.clone()}
                />
            }

            <PropiedadFilterBar
                filter_ciudad={filter_ciudad.clone()}
                filter_tipo={filter_tipo.clone()}
                filter_estado={filter_estado.clone()}
                on_apply={on_filter_apply}
                on_clear={on_filter_clear}
            />

            if *show_form {
                <PropiedadForm
                    is_editing={editing.is_some()}
                    titulo={titulo.clone()}
                    descripcion={descripcion.clone()}
                    direccion={direccion.clone()}
                    ciudad={ciudad.clone()}
                    provincia={provincia.clone()}
                    tipo_propiedad={tipo_propiedad.clone()}
                    habitaciones={habitaciones.clone()}
                    banos={banos.clone()}
                    area_m2={area_m2.clone()}
                    precio={precio.clone()}
                    moneda={moneda.clone()}
                    estado={estado.clone()}
                    form_errors={(*form_errors).clone()}
                    submitting={*submitting}
                    on_submit={on_submit}
                    on_cancel={on_cancel}
                    editing_id={editing_id}
                    token={token}
                />
            }

            <PropiedadList
                items={(*items).clone()}
                user_rol={user_rol.clone()}
                headers={headers}
                sortable_fields={sortable_fields}
                current_sort={(*sort_field).clone()}
                current_order={(*sort_order).clone()}
                total={*total}
                page={*page}
                per_page={*per_page}
                on_sort={on_sort}
                on_edit={on_edit}
                on_delete={on_delete_click}
                on_new={on_new}
                on_page_change={on_page_change}
                on_per_page_change={on_per_page_change}
            />
        </div>
    }
}
