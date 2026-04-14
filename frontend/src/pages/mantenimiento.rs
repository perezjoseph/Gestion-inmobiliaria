use crate::app::AuthContext;
use crate::components::common::data_table::DataTable;
use crate::components::common::delete_confirm_modal::DeleteConfirmModal;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::loading::Loading;
use crate::components::common::pagination::Pagination;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::components::mantenimiento::{estado_badge, prioridad_badge};
use crate::services::api::{api_delete, api_get, api_post, api_put};
use crate::types::PaginatedResponse;
use crate::types::inquilino::Inquilino;
use crate::types::mantenimiento::{
    CambiarEstado, CreateNota, CreateSolicitud, Solicitud, UpdateSolicitud,
};
use crate::types::propiedad::Propiedad;
use crate::utils::{
    EscapeHandler, can_delete, can_write, field_error, format_currency, format_date_display,
    input_class,
};
use gloo_events::EventListener;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Unidad {
    id: String,
    propiedad_id: String,
    numero_unidad: String,
}

#[derive(Clone, PartialEq)]
enum View {
    List,
    Form,
    Detail(String),
}

#[derive(Clone, Default, PartialEq)]
struct FormErrors {
    titulo: Option<String>,
    propiedad_id: Option<String>,
}

impl FormErrors {
    fn has_errors(&self) -> bool {
        self.titulo.is_some() || self.propiedad_id.is_some()
    }
}

#[derive(Properties, PartialEq)]
struct MantenimientoFormProps {
    is_editing: bool,
    f_propiedad_id: UseStateHandle<String>,
    f_unidad_id: UseStateHandle<String>,
    f_inquilino_id: UseStateHandle<String>,
    f_titulo: UseStateHandle<String>,
    f_descripcion: UseStateHandle<String>,
    f_prioridad: UseStateHandle<String>,
    f_nombre_proveedor: UseStateHandle<String>,
    f_telefono_proveedor: UseStateHandle<String>,
    f_email_proveedor: UseStateHandle<String>,
    f_costo_monto: UseStateHandle<String>,
    f_costo_moneda: UseStateHandle<String>,
    propiedades: Vec<Propiedad>,
    unidades: Vec<Unidad>,
    inquilinos_list: Vec<Inquilino>,
    form_errors: FormErrors,
    submitting: bool,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
    error: Option<String>,
    on_error_close: Callback<MouseEvent>,
}

#[function_component]
fn MantenimientoForm(props: &MantenimientoFormProps) -> Html {
    let selected_prop = (*props.f_propiedad_id).clone();
    let filtered_unidades: Vec<&Unidad> = props
        .unidades
        .iter()
        .filter(|u| u.propiedad_id == selected_prop)
        .collect();
    let fe = props.form_errors.clone();
    let title = if props.is_editing {
        "Editar Solicitud"
    } else {
        "Nueva Solicitud"
    };
    let prop_class = input_class(fe.propiedad_id.is_some());
    let titulo_class = input_class(fe.titulo.is_some());
    let submit_text = if props.submitting {
        "Guardando..."
    } else {
        "Guardar"
    };
    let error_html = render_form_error(&props.error, &props.on_error_close);
    let prop_error_html = field_error(&fe.propiedad_id);
    let titulo_error_html = field_error(&fe.titulo);

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
    macro_rules! textarea_cb {
        ($state:expr) => {{
            let s = $state.clone();
            Callback::from(move |e: InputEvent| {
                let el: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                s.set(el.value());
            })
        }};
    }

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{title}</h1>
            </div>
            {error_html}
            <div class="gi-card" style="padding: var(--space-6);">
                <form onsubmit={props.on_submit.clone()} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                    <div>
                        <label class="gi-label">{"Propiedad *"}</label>
                        <select onchange={select_cb!(props.f_propiedad_id)} disabled={props.is_editing} class={prop_class}>
                            <option value="" selected={props.f_propiedad_id.is_empty()}>{"— Seleccionar propiedad —"}</option>
                            { for props.propiedades.iter().map(|p| {
                                let sel = *props.f_propiedad_id == p.id;
                                html! { <option value={p.id.clone()} selected={sel}>{&p.titulo}</option> }
                            })}
                        </select>
                        {prop_error_html}
                    </div>
                    <div>
                        <label class="gi-label">{"Unidad"}</label>
                        <select onchange={select_cb!(props.f_unidad_id)} class="gi-input">
                            <option value="" selected={props.f_unidad_id.is_empty()}>{"— Sin unidad —"}</option>
                            { for filtered_unidades.iter().map(|u| {
                                let sel = *props.f_unidad_id == u.id;
                                html! { <option value={u.id.clone()} selected={sel}>{&u.numero_unidad}</option> }
                            })}
                        </select>
                    </div>
                    <div>
                        <label class="gi-label">{"Inquilino"}</label>
                        <select onchange={select_cb!(props.f_inquilino_id)} class="gi-input">
                            <option value="" selected={props.f_inquilino_id.is_empty()}>{"— Sin inquilino —"}</option>
                            { for props.inquilinos_list.iter().map(|i| {
                                let sel = *props.f_inquilino_id == i.id;
                                let label = format!("{} {}", i.nombre, i.apellido);
                                html! { <option value={i.id.clone()} selected={sel}>{label}</option> }
                            })}
                        </select>
                    </div>
                    <div>
                        <label class="gi-label">{"Título *"}</label>
                        <input type="text" value={(*props.f_titulo).clone()} oninput={input_cb!(props.f_titulo)}
                            class={titulo_class} placeholder="Descripción breve del problema" />
                        {titulo_error_html}
                    </div>
                    <div style="grid-column: 1 / -1;">
                        <label class="gi-label">{"Descripción"}</label>
                        <textarea value={(*props.f_descripcion).clone()} oninput={textarea_cb!(props.f_descripcion)}
                            class="gi-input" style="min-height: 80px;" placeholder="Detalles adicionales del problema" />
                    </div>
                    <div>
                        <label class="gi-label">{"Prioridad"}</label>
                        <select onchange={select_cb!(props.f_prioridad)} class="gi-input">
                            <option value="baja" selected={*props.f_prioridad == "baja"}>{"Baja"}</option>
                            <option value="media" selected={*props.f_prioridad == "media"}>{"Media"}</option>
                            <option value="alta" selected={*props.f_prioridad == "alta"}>{"Alta"}</option>
                            <option value="urgente" selected={*props.f_prioridad == "urgente"}>{"Urgente"}</option>
                        </select>
                    </div>
                    <div>
                        <label class="gi-label">{"Nombre Proveedor"}</label>
                        <input type="text" value={(*props.f_nombre_proveedor).clone()} oninput={input_cb!(props.f_nombre_proveedor)}
                            class="gi-input" placeholder="Nombre del proveedor" />
                    </div>
                    <div>
                        <label class="gi-label">{"Teléfono Proveedor"}</label>
                        <input type="text" value={(*props.f_telefono_proveedor).clone()} oninput={input_cb!(props.f_telefono_proveedor)}
                            class="gi-input" placeholder="809-555-1234" />
                    </div>
                    <div>
                        <label class="gi-label">{"Email Proveedor"}</label>
                        <input type="email" value={(*props.f_email_proveedor).clone()} oninput={input_cb!(props.f_email_proveedor)}
                            class="gi-input" placeholder="proveedor@ejemplo.com" />
                    </div>
                    <div>
                        <label class="gi-label">{"Costo"}</label>
                        <input type="number" step="0.01" min="0" value={(*props.f_costo_monto).clone()} oninput={input_cb!(props.f_costo_monto)}
                            class="gi-input" placeholder="0.00" />
                    </div>
                    <div>
                        <label class="gi-label">{"Moneda"}</label>
                        <select onchange={select_cb!(props.f_costo_moneda)} class="gi-input">
                            <option value="DOP" selected={*props.f_costo_moneda == "DOP"}>{"DOP"}</option>
                            <option value="USD" selected={*props.f_costo_moneda == "USD"}>{"USD"}</option>
                        </select>
                    </div>
                    <div style="grid-column: 1 / -1; display: flex; gap: var(--space-2); justify-content: flex-end;">
                        <button type="button" onclick={props.on_cancel.clone()} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                        <button type="submit" disabled={props.submitting} class="gi-btn gi-btn-primary">{submit_text}</button>
                    </div>
                </form>
            </div>
        </div>
    }
}

fn render_form_error(error: &Option<String>, on_close: &Callback<MouseEvent>) -> Html {
    match error {
        Some(err) => html! { <ErrorBanner message={err.clone()} onclose={on_close.clone()} /> },
        None => html! {},
    }
}

#[derive(Properties, PartialEq)]
struct MantenimientoDetailProps {
    sol: Solicitud,
    user_rol: String,
    error: Option<String>,
    on_error_close: Callback<MouseEvent>,
    on_back: Callback<MouseEvent>,
    on_cambiar_estado: Callback<(String, String)>,
    on_add_nota: Callback<SubmitEvent>,
    nota_contenido: UseStateHandle<String>,
    nota_submitting: bool,
    prop_label: Callback<String, String>,
}

fn render_detail_actions(
    sol_id: &str,
    sol_estado: &str,
    user_rol: &str,
    on_cambiar_estado: &Callback<(String, String)>,
) -> Html {
    if !can_write(user_rol) {
        return html! {};
    }
    let action = match sol_estado {
        "pendiente" => Some(("en_progreso", "Iniciar Trabajo")),
        "en_progreso" => Some(("completado", "Completar")),
        _ => None,
    };
    let Some((next_estado, label)) = action else {
        return html! {};
    };
    let id = sol_id.to_string();
    let next = next_estado.to_string();
    let on_cambiar = on_cambiar_estado.clone();
    html! {
        <div style="display: flex; gap: var(--space-2);">
            <button onclick={Callback::from(move |_: MouseEvent| on_cambiar.emit((id.clone(), next.clone())))}
                class="gi-btn gi-btn-primary">{label}</button>
        </div>
    }
}

fn render_detail_field(label: &str, value: &str, style: &str) -> Html {
    html! {
        <div style={style.to_string()}>
            <span class="gi-label">{label}</span>
            <p style="font-size: var(--text-sm); color: var(--text-primary);">{value.to_string()}</p>
        </div>
    }
}

fn render_optional_field(label: &str, value: &Option<String>, style: &str) -> Html {
    match value {
        Some(v) => render_detail_field(label, v, style),
        None => html! {},
    }
}

fn render_detail_fields(sol: &Solicitud, prop_label: &Callback<String, String>) -> Html {
    let costo_html = match (&sol.costo_monto, &sol.costo_moneda) {
        (Some(monto), Some(moneda)) => {
            render_detail_field("Costo", &format_currency(moneda, *monto), "")
        }
        _ => html! {},
    };
    let fecha_inicio_html = render_optional_field(
        "Fecha Inicio",
        &sol.fecha_inicio.as_ref().map(|f| format_date_display(f)),
        "",
    );
    let fecha_fin_html = render_optional_field(
        "Fecha Fin",
        &sol.fecha_fin.as_ref().map(|f| format_date_display(f)),
        "",
    );

    html! {
        <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                {render_detail_field("Propiedad", &prop_label.emit(sol.propiedad_id.clone()), "")}
                <div>
                    <span class="gi-label">{"Estado"}</span>
                    <div>{estado_badge(&sol.estado)}</div>
                </div>
                <div>
                    <span class="gi-label">{"Prioridad"}</span>
                    <div>{prioridad_badge(&sol.prioridad)}</div>
                </div>
                {render_optional_field("Descripción", &sol.descripcion, "grid-column: 1 / -1;")}
                {render_optional_field("Proveedor", &sol.nombre_proveedor, "")}
                {render_optional_field("Teléfono Proveedor", &sol.telefono_proveedor, "")}
                {render_optional_field("Email Proveedor", &sol.email_proveedor, "")}
                {costo_html}
                {fecha_inicio_html}
                {fecha_fin_html}
                <div>
                    <span class="gi-label">{"Creado"}</span>
                    <p style="font-size: var(--text-sm); color: var(--text-secondary);">{format_date_display(&sol.created_at)}</p>
                </div>
                <div>
                    <span class="gi-label">{"Actualizado"}</span>
                    <p style="font-size: var(--text-sm); color: var(--text-secondary);">{format_date_display(&sol.updated_at)}</p>
                </div>
            </div>
        </div>
    }
}

fn render_detail_notas(
    sol: &Solicitud,
    user_rol: &str,
    on_add_nota: &Callback<SubmitEvent>,
    nota_contenido: &UseStateHandle<String>,
    nota_submitting: bool,
) -> Html {
    let nota_form = if can_write(user_rol) {
        let s = nota_contenido.clone();
        let textarea_cb = Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            s.set(el.value());
        });
        let btn_text = if nota_submitting {
            "Enviando..."
        } else {
            "Agregar"
        };
        html! {
            <form onsubmit={on_add_nota.clone()} style="display: flex; gap: var(--space-3); margin-bottom: var(--space-4);">
                <textarea value={(**nota_contenido).clone()} oninput={textarea_cb}
                    class="gi-input" style="flex: 1; min-height: 60px;" placeholder="Agregar una nota..." />
                <button type="submit" disabled={nota_submitting} class="gi-btn gi-btn-primary" style="align-self: flex-end;">{btn_text}</button>
            </form>
        }
    } else {
        html! {}
    };

    let notas_list = sol.notas.as_deref().unwrap_or_default();
    let notas_html = if notas_list.is_empty() {
        html! { <p style="font-size: var(--text-sm); color: var(--text-tertiary);">{"Sin notas aún."}</p> }
    } else {
        html! {
            <div style="display: flex; flex-direction: column; gap: var(--space-3);">
                { for notas_list.iter().map(|n| html! {
                    <div style="padding: var(--space-3); border: 1px solid var(--border-subtle); border-radius: 8px;">
                        <p style="font-size: var(--text-sm); color: var(--text-primary); margin-bottom: var(--space-1);">{&n.contenido}</p>
                        <p style="font-size: var(--text-xs); color: var(--text-tertiary);">{format_date_display(&n.created_at)}</p>
                    </div>
                })}
            </div>
        }
    };

    html! {
        <div class="gi-card" style="padding: var(--space-6);">
            <h2 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">{"Notas"}</h2>
            {nota_form}
            {notas_html}
        </div>
    }
}

#[function_component]
fn MantenimientoDetail(props: &MantenimientoDetailProps) -> Html {
    let sol = &props.sol;

    html! {
        <div>
            <div class="gi-page-header">
                <div style="display: flex; align-items: center; gap: var(--space-3);">
                    <button onclick={props.on_back.clone()} class="gi-btn gi-btn-ghost">{"← Volver"}</button>
                    <h1 class="gi-page-title">{&sol.titulo}</h1>
                </div>
                {render_detail_actions(&sol.id, &sol.estado, &props.user_rol, &props.on_cambiar_estado)}
            </div>

            if let Some(ref err) = props.error {
                <ErrorBanner message={err.clone()} onclose={props.on_error_close.clone()} />
            }

            {render_detail_fields(sol, &props.prop_label)}
            {render_detail_notas(sol, &props.user_rol, &props.on_add_nota, &props.nota_contenido, props.nota_submitting)}
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct MantenimientoListProps {
    items: Vec<Solicitud>,
    user_rol: String,
    headers: Vec<String>,
    total: u64,
    page: u64,
    per_page: u64,
    prop_label: Callback<String, String>,
    on_edit: Callback<Solicitud>,
    on_delete: Callback<Solicitud>,
    on_view_detail: Callback<String>,
    on_new: Callback<MouseEvent>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
}

fn render_mantenimiento_row(
    s: &Solicitud,
    user_rol: &str,
    prop_label: &Callback<String, String>,
    on_edit: &Callback<Solicitud>,
    on_delete: &Callback<Solicitud>,
    on_view_detail: &Callback<String>,
) -> Html {
    let p_label = prop_label.emit(s.propiedad_id.clone());
    let costo_display = match (&s.costo_monto, &s.costo_moneda) {
        (Some(monto), Some(moneda)) => format_currency(moneda, *monto),
        _ => "—".into(),
    };
    let sc = s.clone();
    let sd = s.clone();
    let sv = s.id.clone();
    let on_edit = on_edit.clone();
    let on_delete_click = on_delete.clone();
    let on_view_detail = on_view_detail.clone();
    let user_rol = user_rol.to_string();
    let actions_html = render_row_actions(&user_rol, sc, sd, on_edit, on_delete_click);
    html! {
        <tr>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{p_label}</td>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                <button onclick={Callback::from(move |_: MouseEvent| on_view_detail.emit(sv.clone()))}
                    class="gi-btn-text" style="font-weight: 500;">{&s.titulo}</button>
            </td>
            <td style="padding: var(--space-3) var(--space-5);">{prioridad_badge(&s.prioridad)}</td>
            <td style="padding: var(--space-3) var(--space-5);">{estado_badge(&s.estado)}</td>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">
                {s.nombre_proveedor.as_deref().unwrap_or("—")}</td>
            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{costo_display}</td>
            {actions_html}
        </tr>
    }
}

fn render_row_actions(
    user_rol: &str,
    sc: Solicitud,
    sd: Solicitud,
    on_edit: Callback<Solicitud>,
    on_delete_click: Callback<Solicitud>,
) -> Html {
    if !can_write(user_rol) {
        return html! {};
    }
    let delete_btn = if can_delete(user_rol) {
        html! { <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(sd.clone()))} class="gi-btn-text" style="color: var(--color-error);">{"Eliminar"}</button> }
    } else {
        html! {}
    };
    html! {
        <td style="padding: var(--space-3) var(--space-5); display: flex; gap: var(--space-2);">
            <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(sc.clone()))} class="gi-btn-text">{"Editar"}</button>
            {delete_btn}
        </td>
    }
}

fn render_mantenimiento_empty_state(user_rol: &str, on_new: &Callback<MouseEvent>) -> Html {
    html! {
        <div class="gi-empty-state">
            <div class="gi-empty-state-icon">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4">
                    <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>
                </svg>
            </div>
            <div class="gi-empty-state-title">{"Sin solicitudes de mantenimiento"}</div>
            <p class="gi-empty-state-text">{"Registre solicitudes de mantenimiento para dar seguimiento a reparaciones y trabajos en sus propiedades."}</p>
            if can_write(user_rol) {
                <button onclick={on_new.clone()} class="gi-btn gi-btn-primary" style="margin-top: var(--space-3);">{"+ Nueva Solicitud"}</button>
            }
        </div>
    }
}

#[function_component]
fn MantenimientoList(props: &MantenimientoListProps) -> Html {
    if props.items.is_empty() {
        return render_mantenimiento_empty_state(&props.user_rol, &props.on_new);
    }

    html! {
        <>
            <DataTable headers={props.headers.clone()}>
                { for props.items.iter().map(|s| render_mantenimiento_row(s, &props.user_rol, &props.prop_label, &props.on_edit, &props.on_delete, &props.on_view_detail)) }
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
fn make_mantenimiento_edit_cb(
    f_propiedad_id: &UseStateHandle<String>,
    f_unidad_id: &UseStateHandle<String>,
    f_inquilino_id: &UseStateHandle<String>,
    f_titulo: &UseStateHandle<String>,
    f_descripcion: &UseStateHandle<String>,
    f_prioridad: &UseStateHandle<String>,
    f_nombre_proveedor: &UseStateHandle<String>,
    f_telefono_proveedor: &UseStateHandle<String>,
    f_email_proveedor: &UseStateHandle<String>,
    f_costo_monto: &UseStateHandle<String>,
    f_costo_moneda: &UseStateHandle<String>,
    editing: &UseStateHandle<Option<Solicitud>>,
    view: &UseStateHandle<View>,
    form_errors: &UseStateHandle<FormErrors>,
) -> Callback<Solicitud> {
    let (f_propiedad_id, f_unidad_id, f_inquilino_id) = (
        f_propiedad_id.clone(),
        f_unidad_id.clone(),
        f_inquilino_id.clone(),
    );
    let (f_titulo, f_descripcion, f_prioridad) =
        (f_titulo.clone(), f_descripcion.clone(), f_prioridad.clone());
    let (f_nombre_proveedor, f_telefono_proveedor, f_email_proveedor) = (
        f_nombre_proveedor.clone(),
        f_telefono_proveedor.clone(),
        f_email_proveedor.clone(),
    );
    let (f_costo_monto, f_costo_moneda) = (f_costo_monto.clone(), f_costo_moneda.clone());
    let (editing, view, form_errors) = (editing.clone(), view.clone(), form_errors.clone());
    Callback::from(move |s: Solicitud| {
        f_propiedad_id.set(s.propiedad_id.clone());
        f_unidad_id.set(s.unidad_id.clone().unwrap_or_default());
        f_inquilino_id.set(s.inquilino_id.clone().unwrap_or_default());
        f_titulo.set(s.titulo.clone());
        f_descripcion.set(s.descripcion.clone().unwrap_or_default());
        f_prioridad.set(s.prioridad.clone());
        f_nombre_proveedor.set(s.nombre_proveedor.clone().unwrap_or_default());
        f_telefono_proveedor.set(s.telefono_proveedor.clone().unwrap_or_default());
        f_email_proveedor.set(s.email_proveedor.clone().unwrap_or_default());
        f_costo_monto.set(s.costo_monto.map(|v| v.to_string()).unwrap_or_default());
        f_costo_moneda.set(s.costo_moneda.clone().unwrap_or_else(|| "DOP".into()));
        editing.set(Some(s));
        view.set(View::Form);
        form_errors.set(FormErrors::default());
    })
}

fn validate_mantenimiento_fields(titulo: &str, propiedad_id: &str) -> FormErrors {
    let mut errs = FormErrors::default();
    if titulo.trim().is_empty() {
        errs.titulo = Some("El título es obligatorio".into());
    }
    if propiedad_id.is_empty() {
        errs.propiedad_id = Some("Debe seleccionar una propiedad".into());
    }
    errs
}

fn non_empty_mant(s: &str) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

#[allow(clippy::too_many_arguments)]
fn do_save_solicitud(
    editing_id: Option<String>,
    update: UpdateSolicitud,
    create: CreateSolicitud,
    reset_form: impl Fn() + 'static,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
    submitting: UseStateHandle<bool>,
) {
    spawn_local(async move {
        let res = match editing_id {
            Some(id) => api_put::<Solicitud, _>(&format!("/mantenimiento/{id}"), &update)
                .await
                .map(|_| ()),
            None => api_post::<Solicitud, _>("/mantenimiento", &create)
                .await
                .map(|_| ()),
        };
        match res {
            Ok(()) => {
                reset_form();
                reload.set(*reload + 1);
                push_toast(&toasts, "Solicitud guardada", ToastKind::Success);
            }
            Err(err) => error.set(Some(err)),
        }
        submitting.set(false);
    });
}

fn do_delete_solicitud(
    id: String,
    label: String,
    delete_target: UseStateHandle<Option<Solicitud>>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
) {
    spawn_local(async move {
        match api_delete(&format!("/mantenimiento/{id}")).await {
            Ok(()) => {
                delete_target.set(None);
                reload.set(*reload + 1);
                push_toast(
                    &toasts,
                    &format!("\"{}\" eliminada", label),
                    ToastKind::Info,
                );
            }
            Err(err) => {
                delete_target.set(None);
                error.set(Some(err));
            }
        }
    });
}

fn do_cambiar_estado(
    id: String,
    nuevo_estado: String,
    detail_item: UseStateHandle<Option<Solicitud>>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
) {
    let estado_label = nuevo_estado.clone();
    spawn_local(async move {
        let body = CambiarEstado {
            estado: nuevo_estado,
        };
        match api_put::<Solicitud, _>(&format!("/mantenimiento/{id}/estado"), &body).await {
            Ok(updated) => {
                detail_item.set(Some(updated));
                reload.set(*reload + 1);
                push_toast(
                    &toasts,
                    &format!("Estado cambiado a {estado_label}"),
                    ToastKind::Success,
                );
            }
            Err(err) => error.set(Some(err)),
        }
    });
}

fn do_add_nota(
    sol_id: String,
    contenido: String,
    nota_contenido: UseStateHandle<String>,
    nota_submitting: UseStateHandle<bool>,
    detail_item: UseStateHandle<Option<Solicitud>>,
    error: UseStateHandle<Option<String>>,
) {
    spawn_local(async move {
        let body = CreateNota { contenido };
        match api_post::<Solicitud, _>(&format!("/mantenimiento/{sol_id}/notas"), &body).await {
            Ok(updated) => {
                detail_item.set(Some(updated));
                nota_contenido.set(String::new());
            }
            Err(err) => error.set(Some(err)),
        }
        nota_submitting.set(false);
    });
}

fn build_mantenimiento_url(pg: u64, pp: u64, fe: &str, fp: &str) -> String {
    let mut params = vec![format!("page={pg}"), format!("perPage={pp}")];
    if !fe.is_empty() {
        params.push(format!("estado={fe}"));
    }
    if !fp.is_empty() {
        params.push(format!("prioridad={fp}"));
    }
    format!("/mantenimiento?{}", params.join("&"))
}

fn push_toast(toasts: &Option<ToastContext>, msg: &str, kind: ToastKind) {
    if let Some(t) = toasts {
        t.dispatch(ToastAction::Push(msg.into(), kind));
    }
}

fn load_mant_refs(
    propiedades: UseStateHandle<Vec<Propiedad>>,
    inquilinos_list: UseStateHandle<Vec<Inquilino>>,
) {
    spawn_local(async move {
        if let Ok(resp) =
            api_get::<PaginatedResponse<Propiedad>>("/propiedades?page=1&perPage=100").await
        {
            propiedades.set(resp.data);
        }
        if let Ok(data) = api_get::<PaginatedResponse<Inquilino>>("/inquilinos?perPage=100").await {
            inquilinos_list.set(data.data);
        }
    });
}

fn register_escape_listener_m(escape_handler: EscapeHandler) -> Option<EventListener> {
    web_sys::window().and_then(|w| w.document()).map(|doc| {
        EventListener::new(&doc, "keydown", move |event| {
            let event = event.dyn_ref::<web_sys::KeyboardEvent>().unwrap();
            if event.key() == "Escape"
                && let Some(ref cb) = *escape_handler.borrow()
            {
                cb();
            }
        })
    })
}

fn load_mantenimiento_data(
    items: UseStateHandle<Vec<Solicitud>>,
    total: UseStateHandle<u64>,
    error: UseStateHandle<Option<String>>,
    loading: UseStateHandle<bool>,
    url: String,
) {
    spawn_local(async move {
        loading.set(true);
        match api_get::<PaginatedResponse<Solicitud>>(&url).await {
            Ok(resp) => {
                items.set(resp.data);
                total.set(resp.total);
            }
            Err(err) => error.set(Some(err)),
        }
        loading.set(false);
    });
}

fn handle_escape_mantenimiento(
    delete_target: &UseStateHandle<Option<Solicitud>>,
    view_state: &UseStateHandle<View>,
    reset_form: &dyn Fn(),
) {
    if delete_target.is_some() {
        delete_target.set(None);
    } else if matches!(**view_state, View::Form) {
        reset_form();
    }
}

fn handle_view_detail(
    id: String,
    view: UseStateHandle<View>,
    detail_item: UseStateHandle<Option<Solicitud>>,
    error: UseStateHandle<Option<String>>,
    nota_contenido: UseStateHandle<String>,
) {
    spawn_local(async move {
        match api_get::<Solicitud>(&format!("/mantenimiento/{id}")).await {
            Ok(sol) => {
                detail_item.set(Some(sol));
                nota_contenido.set(String::new());
                view.set(View::Detail(id));
            }
            Err(err) => error.set(Some(err)),
        }
    });
}

fn handle_mantenimiento_delete_confirm(
    delete_target: &UseStateHandle<Option<Solicitud>>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
) {
    if let Some(ref s) = **delete_target {
        do_delete_solicitud(
            s.id.clone(),
            s.titulo.clone(),
            delete_target.clone(),
            reload,
            error,
            toasts,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_mantenimiento_submit(
    submitting: &UseStateHandle<bool>,
    validate_form: &dyn Fn() -> bool,
    f_propiedad_id: &UseStateHandle<String>,
    f_unidad_id: &UseStateHandle<String>,
    f_inquilino_id: &UseStateHandle<String>,
    f_titulo: &UseStateHandle<String>,
    f_descripcion: &UseStateHandle<String>,
    f_prioridad: &UseStateHandle<String>,
    f_nombre_proveedor: &UseStateHandle<String>,
    f_telefono_proveedor: &UseStateHandle<String>,
    f_email_proveedor: &UseStateHandle<String>,
    f_costo_monto: &UseStateHandle<String>,
    f_costo_moneda: &UseStateHandle<String>,
    editing: &UseStateHandle<Option<Solicitud>>,
    reset_form: impl Fn() + 'static,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
) {
    if **submitting || !validate_form() {
        return;
    }
    submitting.set(true);
    let costo = f_costo_monto.parse::<f64>().ok();
    let editing_id = editing.as_ref().map(|e| e.id.clone());
    let update = UpdateSolicitud {
        titulo: Some((**f_titulo).clone()),
        descripcion: non_empty_mant(f_descripcion),
        prioridad: Some((**f_prioridad).clone()),
        nombre_proveedor: non_empty_mant(f_nombre_proveedor),
        telefono_proveedor: non_empty_mant(f_telefono_proveedor),
        email_proveedor: non_empty_mant(f_email_proveedor),
        costo_monto: costo,
        costo_moneda: non_empty_mant(f_costo_moneda),
        unidad_id: non_empty_mant(f_unidad_id),
        inquilino_id: non_empty_mant(f_inquilino_id),
    };
    let create = CreateSolicitud {
        propiedad_id: (**f_propiedad_id).clone(),
        unidad_id: non_empty_mant(f_unidad_id),
        inquilino_id: non_empty_mant(f_inquilino_id),
        titulo: (**f_titulo).clone(),
        descripcion: non_empty_mant(f_descripcion),
        prioridad: Some((**f_prioridad).clone()),
        nombre_proveedor: non_empty_mant(f_nombre_proveedor),
        telefono_proveedor: non_empty_mant(f_telefono_proveedor),
        email_proveedor: non_empty_mant(f_email_proveedor),
        costo_monto: costo,
        costo_moneda: non_empty_mant(f_costo_moneda),
    };
    do_save_solicitud(
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

fn handle_add_nota(
    nota_contenido: &UseStateHandle<String>,
    nota_submitting: UseStateHandle<bool>,
    detail_item: &UseStateHandle<Option<Solicitud>>,
    error: UseStateHandle<Option<String>>,
) {
    let contenido = (**nota_contenido).clone();
    if contenido.trim().is_empty() || *nota_submitting {
        return;
    }
    nota_submitting.set(true);
    if let Some(ref sol) = **detail_item {
        do_add_nota(
            sol.id.clone(),
            contenido,
            nota_contenido.clone(),
            nota_submitting,
            detail_item.clone(),
            error,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_mantenimiento_view(
    loading: bool,
    view: &UseStateHandle<View>,
    detail_item: &UseStateHandle<Option<Solicitud>>,
    user_rol: &str,
    error: &UseStateHandle<Option<String>>,
    on_error_close: Callback<MouseEvent>,
    on_back_to_list: Callback<MouseEvent>,
    on_cambiar_estado: Callback<(String, String)>,
    on_add_nota: Callback<SubmitEvent>,
    nota_contenido: &UseStateHandle<String>,
    nota_submitting: bool,
    propiedades: &UseStateHandle<Vec<Propiedad>>,
    editing: &UseStateHandle<Option<Solicitud>>,
    f_propiedad_id: &UseStateHandle<String>,
    f_unidad_id: &UseStateHandle<String>,
    f_inquilino_id: &UseStateHandle<String>,
    f_titulo: &UseStateHandle<String>,
    f_descripcion: &UseStateHandle<String>,
    f_prioridad: &UseStateHandle<String>,
    f_nombre_proveedor: &UseStateHandle<String>,
    f_telefono_proveedor: &UseStateHandle<String>,
    f_email_proveedor: &UseStateHandle<String>,
    f_costo_monto: &UseStateHandle<String>,
    f_costo_moneda: &UseStateHandle<String>,
    unidades: &UseStateHandle<Vec<Unidad>>,
    inquilinos_list: &UseStateHandle<Vec<Inquilino>>,
    form_errors: &UseStateHandle<FormErrors>,
    submitting: bool,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
    delete_target: &UseStateHandle<Option<Solicitud>>,
    filter_estado: &UseStateHandle<String>,
    filter_prioridad: &UseStateHandle<String>,
    on_new: Callback<MouseEvent>,
    on_delete_cancel: Callback<MouseEvent>,
    on_delete_confirm: Callback<MouseEvent>,
    on_filter_apply: Callback<MouseEvent>,
    on_filter_clear: Callback<MouseEvent>,
    on_edit: Callback<Solicitud>,
    on_delete_click: Callback<Solicitud>,
    on_view_detail: Callback<String>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
    items: &UseStateHandle<Vec<Solicitud>>,
    total: u64,
    page: u64,
    per_page: u64,
) -> Html {
    if loading {
        return html! { <Loading /> };
    }

    let prop_label_cb = {
        let propiedades = (**propiedades).clone();
        Callback::from(move |id: String| -> String {
            propiedades
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.titulo.clone())
                .unwrap_or_else(|| "—".into())
        })
    };

    if let View::Detail(ref _id) = **view
        && let Some(ref sol) = **detail_item
    {
        return html! {
            <MantenimientoDetail
                sol={sol.clone()}
                user_rol={user_rol.to_string()}
                error={(**error).clone()}
                on_error_close={on_error_close}
                on_back={on_back_to_list}
                on_cambiar_estado={on_cambiar_estado}
                on_add_nota={on_add_nota}
                nota_contenido={nota_contenido.clone()}
                nota_submitting={nota_submitting}
                prop_label={prop_label_cb}
            />
        };
    }

    if matches!(**view, View::Form) {
        return html! {
            <MantenimientoForm
                is_editing={editing.is_some()}
                f_propiedad_id={f_propiedad_id.clone()}
                f_unidad_id={f_unidad_id.clone()}
                f_inquilino_id={f_inquilino_id.clone()}
                f_titulo={f_titulo.clone()}
                f_descripcion={f_descripcion.clone()}
                f_prioridad={f_prioridad.clone()}
                f_nombre_proveedor={f_nombre_proveedor.clone()}
                f_telefono_proveedor={f_telefono_proveedor.clone()}
                f_email_proveedor={f_email_proveedor.clone()}
                f_costo_monto={f_costo_monto.clone()}
                f_costo_moneda={f_costo_moneda.clone()}
                propiedades={(**propiedades).clone()}
                unidades={(**unidades).clone()}
                inquilinos_list={(**inquilinos_list).clone()}
                form_errors={(**form_errors).clone()}
                submitting={submitting}
                on_submit={on_submit}
                on_cancel={on_cancel}
                error={(**error).clone()}
                on_error_close={on_error_close}
            />
        };
    }

    render_mantenimiento_list_view(
        user_rol,
        error,
        delete_target,
        filter_estado,
        filter_prioridad,
        on_new,
        on_error_close,
        on_delete_cancel,
        on_delete_confirm,
        on_filter_apply,
        on_filter_clear,
        on_edit,
        on_delete_click,
        on_view_detail,
        on_page_change,
        on_per_page_change,
        items,
        total,
        page,
        per_page,
        prop_label_cb,
    )
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

#[allow(clippy::too_many_arguments)]
fn render_mantenimiento_list_view(
    user_rol: &str,
    error: &UseStateHandle<Option<String>>,
    delete_target: &UseStateHandle<Option<Solicitud>>,
    filter_estado: &UseStateHandle<String>,
    filter_prioridad: &UseStateHandle<String>,
    on_new: Callback<MouseEvent>,
    on_error_close: Callback<MouseEvent>,
    on_delete_cancel: Callback<MouseEvent>,
    on_delete_confirm: Callback<MouseEvent>,
    on_filter_apply: Callback<MouseEvent>,
    on_filter_clear: Callback<MouseEvent>,
    on_edit: Callback<Solicitud>,
    on_delete_click: Callback<Solicitud>,
    on_view_detail: Callback<String>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
    items: &UseStateHandle<Vec<Solicitud>>,
    total: u64,
    page: u64,
    per_page: u64,
    prop_label_cb: Callback<String, String>,
) -> Html {
    let last_header: String = if can_write(user_rol) {
        "Acciones".into()
    } else {
        String::new()
    };
    let headers = vec![
        "Propiedad".into(),
        "Título".into(),
        "Prioridad".into(),
        "Estado".into(),
        "Proveedor".into(),
        "Costo".into(),
        last_header,
    ];
    let new_btn = render_new_button(user_rol, &on_new, "+ Nueva Solicitud");
    let error_html = render_opt_error(error, on_error_close);
    let delete_html =
        render_delete_confirm_mant(delete_target, on_delete_confirm, on_delete_cancel);

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Mantenimiento"}</h1>
                {new_btn}
            </div>
            {error_html}
            {delete_html}
            <MantFilterBar filter_estado={filter_estado.clone()} filter_prioridad={filter_prioridad.clone()} {on_filter_apply} {on_filter_clear} />
            <MantenimientoList
                items={(**items).clone()} user_rol={user_rol.to_string()} headers={headers}
                total={total} page={page} per_page={per_page} prop_label={prop_label_cb}
                on_edit={on_edit} on_delete={on_delete_click} on_view_detail={on_view_detail}
                on_new={on_new} on_page_change={on_page_change} on_per_page_change={on_per_page_change}
            />
        </div>
    }
}

fn render_new_button(user_rol: &str, on_new: &Callback<MouseEvent>, label: &str) -> Html {
    if can_write(user_rol) {
        html! { <button onclick={on_new.clone()} class="gi-btn gi-btn-primary">{label}</button> }
    } else {
        html! {}
    }
}

fn render_opt_error(
    error: &UseStateHandle<Option<String>>,
    on_close: Callback<MouseEvent>,
) -> Html {
    match (*error).as_ref() {
        Some(err) => html! { <ErrorBanner message={err.clone()} onclose={on_close} /> },
        None => html! {},
    }
}

fn render_delete_confirm_mant(
    target: &UseStateHandle<Option<Solicitud>>,
    on_confirm: Callback<MouseEvent>,
    on_cancel: Callback<MouseEvent>,
) -> Html {
    match (**target).as_ref() {
        Some(t) => html! {
            <DeleteConfirmModal
                message={format!("¿Está seguro de que desea eliminar la solicitud \"{}\"? Esta acción no se puede deshacer.", t.titulo)}
                on_confirm={on_confirm} on_cancel={on_cancel}
            />
        },
        None => html! {},
    }
}

#[derive(Properties, PartialEq)]
struct MantFilterBarProps {
    filter_estado: UseStateHandle<String>,
    filter_prioridad: UseStateHandle<String>,
    on_filter_apply: Callback<MouseEvent>,
    on_filter_clear: Callback<MouseEvent>,
}

#[function_component]
fn MantFilterBar(props: &MantFilterBarProps) -> Html {
    html! {
        <div class="gi-filter-bar">
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--space-3); align-items: end;">
                <div>
                    <label class="gi-label">{"Estado"}</label>
                    <select onchange={select_cb!(props.filter_estado)} class="gi-input">
                        <option value="" selected={props.filter_estado.is_empty()}>{"Todos"}</option>
                        <option value="pendiente" selected={*props.filter_estado == "pendiente"}>{"Pendiente"}</option>
                        <option value="en_progreso" selected={*props.filter_estado == "en_progreso"}>{"En Progreso"}</option>
                        <option value="completado" selected={*props.filter_estado == "completado"}>{"Completado"}</option>
                        <option value="cancelado" selected={*props.filter_estado == "cancelado"}>{"Cancelado"}</option>
                    </select>
                </div>
                <div>
                    <label class="gi-label">{"Prioridad"}</label>
                    <select onchange={select_cb!(props.filter_prioridad)} class="gi-input">
                        <option value="" selected={props.filter_prioridad.is_empty()}>{"Todas"}</option>
                        <option value="baja" selected={*props.filter_prioridad == "baja"}>{"Baja"}</option>
                        <option value="media" selected={*props.filter_prioridad == "media"}>{"Media"}</option>
                        <option value="alta" selected={*props.filter_prioridad == "alta"}>{"Alta"}</option>
                        <option value="urgente" selected={*props.filter_prioridad == "urgente"}>{"Urgente"}</option>
                    </select>
                </div>
                <div style="display: flex; gap: var(--space-2);">
                    <button onclick={props.on_filter_apply.clone()} class="gi-btn gi-btn-primary">{"Filtrar"}</button>
                    <button onclick={props.on_filter_clear.clone()} class="gi-btn gi-btn-ghost">{"Limpiar"}</button>
                </div>
            </div>
        </div>
    }
}

#[function_component]
pub fn Mantenimiento() -> Html {
    let auth = use_context::<AuthContext>();
    let toasts = use_context::<ToastContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();

    let items = use_state(Vec::<Solicitud>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let propiedades = use_state(Vec::<Propiedad>::new);
    let unidades = use_state(Vec::<Unidad>::new);
    let inquilinos_list = use_state(Vec::<Inquilino>::new);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let view = use_state(|| View::List);
    let editing = use_state(|| Option::<Solicitud>::None);
    let detail_item = use_state(|| Option::<Solicitud>::None);
    let delete_target = use_state(|| Option::<Solicitud>::None);
    let submitting = use_state(|| false);
    let form_errors = use_state(FormErrors::default);
    let reload = use_state(|| 0u32);

    let f_propiedad_id = use_state(String::new);
    let f_unidad_id = use_state(String::new);
    let f_inquilino_id = use_state(String::new);
    let f_titulo = use_state(String::new);
    let f_descripcion = use_state(String::new);
    let f_prioridad = use_state(|| "media".to_string());
    let f_nombre_proveedor = use_state(String::new);
    let f_telefono_proveedor = use_state(String::new);
    let f_email_proveedor = use_state(String::new);
    let f_costo_monto = use_state(String::new);
    let f_costo_moneda = use_state(|| "DOP".to_string());

    let filter_estado = use_state(String::new);
    let filter_prioridad = use_state(String::new);

    let nota_contenido = use_state(String::new);
    let nota_submitting = use_state(|| false);

    {
        let items = items.clone();
        let total = total.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        let pg = *page;
        let pp = *per_page;
        let fe = (*filter_estado).clone();
        let fp = (*filter_prioridad).clone();
        use_effect_with((reload_val, pg, fe.clone(), fp.clone()), move |_| {
            let url = build_mantenimiento_url(pg, pp, &fe, &fp);
            load_mantenimiento_data(items, total, error, loading, url);
        });
    }

    {
        let propiedades = propiedades.clone();
        let inquilinos_list = inquilinos_list.clone();
        use_effect_with((), move |_| {
            load_mant_refs(propiedades, inquilinos_list);
        });
    }

    let reset_form = {
        let f_propiedad_id = f_propiedad_id.clone();
        let f_unidad_id = f_unidad_id.clone();
        let f_inquilino_id = f_inquilino_id.clone();
        let f_titulo = f_titulo.clone();
        let f_descripcion = f_descripcion.clone();
        let f_prioridad = f_prioridad.clone();
        let f_nombre_proveedor = f_nombre_proveedor.clone();
        let f_telefono_proveedor = f_telefono_proveedor.clone();
        let f_email_proveedor = f_email_proveedor.clone();
        let f_costo_monto = f_costo_monto.clone();
        let f_costo_moneda = f_costo_moneda.clone();
        let editing = editing.clone();
        let view = view.clone();
        let form_errors = form_errors.clone();
        move || {
            f_propiedad_id.set(String::new());
            f_unidad_id.set(String::new());
            f_inquilino_id.set(String::new());
            f_titulo.set(String::new());
            f_descripcion.set(String::new());
            f_prioridad.set("media".into());
            f_nombre_proveedor.set(String::new());
            f_telefono_proveedor.set(String::new());
            f_email_proveedor.set(String::new());
            f_costo_monto.set(String::new());
            f_costo_moneda.set("DOP".into());
            editing.set(None);
            view.set(View::List);
            form_errors.set(FormErrors::default());
        }
    };

    let escape_handler = use_mut_ref(|| Option::<Box<dyn Fn()>>::None);
    {
        let delete_target = delete_target.clone();
        let view_state = view.clone();
        let reset_form = reset_form.clone();
        let handler = escape_handler.clone();
        *handler.borrow_mut() = Some(Box::new(move || {
            handle_escape_mantenimiento(&delete_target, &view_state, &reset_form);
        }) as Box<dyn Fn()>);
    }
    {
        let escape_handler = escape_handler.clone();
        use_effect_with((), move |_| {
            let listener = register_escape_listener_m(escape_handler);
            move || drop(listener)
        });
    }

    let on_new = super::page_helpers::new_cb(reset_form.clone(), &view, View::Form);

    let on_edit = make_mantenimiento_edit_cb(
        &f_propiedad_id,
        &f_unidad_id,
        &f_inquilino_id,
        &f_titulo,
        &f_descripcion,
        &f_prioridad,
        &f_nombre_proveedor,
        &f_telefono_proveedor,
        &f_email_proveedor,
        &f_costo_monto,
        &f_costo_moneda,
        &editing,
        &view,
        &form_errors,
    );

    let on_view_detail = {
        let view = view.clone();
        let detail_item = detail_item.clone();
        let error = error.clone();
        let nota_contenido = nota_contenido.clone();
        Callback::from(move |id: String| {
            handle_view_detail(
                id,
                view.clone(),
                detail_item.clone(),
                error.clone(),
                nota_contenido.clone(),
            );
        })
    };

    let on_back_to_list = {
        let view = view.clone();
        let detail_item = detail_item.clone();
        Callback::from(move |_: MouseEvent| {
            detail_item.set(None);
            view.set(View::List);
        })
    };

    let on_delete_click = super::page_helpers::delete_click_cb(&delete_target);

    let on_delete_confirm = {
        let error = error.clone();
        let reload = reload.clone();
        let delete_target = delete_target.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            handle_mantenimiento_delete_confirm(
                &delete_target,
                reload.clone(),
                error.clone(),
                toasts.clone(),
            );
        })
    };

    let on_delete_cancel = super::page_helpers::delete_cancel_cb(&delete_target);

    let validate_form = {
        let f_titulo = f_titulo.clone();
        let f_propiedad_id = f_propiedad_id.clone();
        let form_errors = form_errors.clone();
        move || -> bool {
            let errs = validate_mantenimiento_fields(&f_titulo, &f_propiedad_id);
            let valid = !errs.has_errors();
            form_errors.set(errs);
            valid
        }
    };

    let on_submit = {
        let f_propiedad_id = f_propiedad_id.clone();
        let f_unidad_id = f_unidad_id.clone();
        let f_inquilino_id = f_inquilino_id.clone();
        let f_titulo = f_titulo.clone();
        let f_descripcion = f_descripcion.clone();
        let f_prioridad = f_prioridad.clone();
        let f_nombre_proveedor = f_nombre_proveedor.clone();
        let f_telefono_proveedor = f_telefono_proveedor.clone();
        let f_email_proveedor = f_email_proveedor.clone();
        let f_costo_monto = f_costo_monto.clone();
        let f_costo_moneda = f_costo_moneda.clone();
        let editing = editing.clone();
        let error = error.clone();
        let reload = reload.clone();
        let reset_form = reset_form.clone();
        let validate_form = validate_form.clone();
        let toasts = toasts.clone();
        let submitting = submitting.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            handle_mantenimiento_submit(
                &submitting,
                &validate_form,
                &f_propiedad_id,
                &f_unidad_id,
                &f_inquilino_id,
                &f_titulo,
                &f_descripcion,
                &f_prioridad,
                &f_nombre_proveedor,
                &f_telefono_proveedor,
                &f_email_proveedor,
                &f_costo_monto,
                &f_costo_moneda,
                &editing,
                reset_form.clone(),
                reload.clone(),
                error.clone(),
                toasts.clone(),
            );
        })
    };

    let on_cancel = super::page_helpers::cancel_cb(reset_form.clone());

    let on_cambiar_estado = {
        let error = error.clone();
        let detail_item = detail_item.clone();
        let reload = reload.clone();
        let toasts = toasts.clone();
        Callback::from(move |(id, nuevo_estado): (String, String)| {
            do_cambiar_estado(
                id,
                nuevo_estado,
                detail_item.clone(),
                reload.clone(),
                error.clone(),
                toasts.clone(),
            );
        })
    };

    let on_add_nota = {
        let nota_contenido = nota_contenido.clone();
        let nota_submitting = nota_submitting.clone();
        let detail_item = detail_item.clone();
        let error = error.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            handle_add_nota(
                &nota_contenido,
                nota_submitting.clone(),
                &detail_item,
                error.clone(),
            );
        })
    };

    let on_filter_apply = super::page_helpers::filter_apply_cb(&page, &reload);
    let on_filter_clear = {
        let fe = filter_estado.clone();
        let fp = filter_prioridad.clone();
        super::page_helpers::filter_clear_cb(
            move || {
                fe.set(String::new());
                fp.set(String::new());
            },
            &page,
            &reload,
        )
    };

    let (on_page_change, on_per_page_change) =
        super::page_helpers::pagination_cbs(&page, &per_page, &reload);

    let on_error_close = super::page_helpers::error_close_cb(&error);

    render_mantenimiento_view(
        *loading,
        &view,
        &detail_item,
        &user_rol,
        &error,
        on_error_close,
        on_back_to_list,
        on_cambiar_estado,
        on_add_nota,
        &nota_contenido,
        *nota_submitting,
        &propiedades,
        &editing,
        &f_propiedad_id,
        &f_unidad_id,
        &f_inquilino_id,
        &f_titulo,
        &f_descripcion,
        &f_prioridad,
        &f_nombre_proveedor,
        &f_telefono_proveedor,
        &f_email_proveedor,
        &f_costo_monto,
        &f_costo_moneda,
        &unidades,
        &inquilinos_list,
        &form_errors,
        *submitting,
        on_submit,
        on_cancel,
        &delete_target,
        &filter_estado,
        &filter_prioridad,
        on_new,
        on_delete_cancel,
        on_delete_confirm,
        on_filter_apply,
        on_filter_clear,
        on_edit,
        on_delete_click,
        on_view_detail,
        on_page_change,
        on_per_page_change,
        &items,
        *total,
        *page,
        *per_page,
    )
}
