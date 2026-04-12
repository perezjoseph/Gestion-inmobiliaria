use crate::app::AuthContext;
use crate::components::common::data_table::DataTable;
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
use crate::utils::{can_delete, can_write, format_currency, format_date_display};
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
                <h1 class="gi-page-title">{if props.is_editing { "Editar Solicitud" } else { "Nueva Solicitud" }}</h1>
            </div>

            if let Some(ref err) = props.error {
                <ErrorBanner message={err.clone()} onclose={props.on_error_close.clone()} />
            }

            <div class="gi-card" style="padding: var(--space-6);">
                <form onsubmit={props.on_submit.clone()} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                    <div>
                        <label class="gi-label">{"Propiedad *"}</label>
                        <select onchange={select_cb!(props.f_propiedad_id)} disabled={props.is_editing}
                            class={if fe.propiedad_id.is_some() { "gi-input gi-input-error" } else { "gi-input" }}>
                            <option value="" selected={props.f_propiedad_id.is_empty()}>{"— Seleccionar propiedad —"}</option>
                            { for props.propiedades.iter().map(|p| {
                                let sel = *props.f_propiedad_id == p.id;
                                html! { <option value={p.id.clone()} selected={sel}>{&p.titulo}</option> }
                            })}
                        </select>
                        if let Some(ref msg) = fe.propiedad_id { <p class="gi-field-error">{msg}</p> }
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
                            class={if fe.titulo.is_some() { "gi-input gi-input-error" } else { "gi-input" }}
                            placeholder="Descripción breve del problema" />
                        if let Some(ref msg) = fe.titulo { <p class="gi-field-error">{msg}</p> }
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
                        <button type="submit" disabled={props.submitting} class="gi-btn gi-btn-primary">
                            {if props.submitting { "Guardando..." } else { "Guardar" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
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

#[function_component]
fn MantenimientoDetail(props: &MantenimientoDetailProps) -> Html {
    let sol = &props.sol;
    let sol_id = sol.id.clone();
    let sol_estado = sol.estado.clone();

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
                <div style="display: flex; align-items: center; gap: var(--space-3);">
                    <button onclick={props.on_back.clone()} class="gi-btn gi-btn-ghost">{"← Volver"}</button>
                    <h1 class="gi-page-title">{&sol.titulo}</h1>
                </div>
                if can_write(&props.user_rol) {
                    <div style="display: flex; gap: var(--space-2);">
                        if sol_estado == "pendiente" {
                            <button onclick={Callback::from({
                                let id = sol_id.clone(); let on_cambiar = props.on_cambiar_estado.clone();
                                move |_: MouseEvent| on_cambiar.emit((id.clone(), "en_progreso".into()))
                            })} class="gi-btn gi-btn-primary">{"Iniciar Trabajo"}</button>
                        }
                        if sol_estado == "en_progreso" {
                            <button onclick={Callback::from({
                                let id = sol_id.clone(); let on_cambiar = props.on_cambiar_estado.clone();
                                move |_: MouseEvent| on_cambiar.emit((id.clone(), "completado".into()))
                            })} class="gi-btn gi-btn-primary">{"Completar"}</button>
                        }
                    </div>
                }
            </div>

            if let Some(ref err) = props.error {
                <ErrorBanner message={err.clone()} onclose={props.on_error_close.clone()} />
            }

            <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                    <div>
                        <span class="gi-label">{"Propiedad"}</span>
                        <p style="font-size: var(--text-sm); color: var(--text-primary);">{props.prop_label.emit(sol.propiedad_id.clone())}</p>
                    </div>
                    <div>
                        <span class="gi-label">{"Estado"}</span>
                        <div>{estado_badge(&sol.estado)}</div>
                    </div>
                    <div>
                        <span class="gi-label">{"Prioridad"}</span>
                        <div>{prioridad_badge(&sol.prioridad)}</div>
                    </div>
                    if let Some(ref desc) = sol.descripcion {
                        <div style="grid-column: 1 / -1;">
                            <span class="gi-label">{"Descripción"}</span>
                            <p style="font-size: var(--text-sm); color: var(--text-primary);">{desc}</p>
                        </div>
                    }
                    if let Some(ref nombre) = sol.nombre_proveedor {
                        <div>
                            <span class="gi-label">{"Proveedor"}</span>
                            <p style="font-size: var(--text-sm); color: var(--text-primary);">{nombre}</p>
                        </div>
                    }
                    if let Some(ref tel) = sol.telefono_proveedor {
                        <div>
                            <span class="gi-label">{"Teléfono Proveedor"}</span>
                            <p style="font-size: var(--text-sm); color: var(--text-primary);">{tel}</p>
                        </div>
                    }
                    if let Some(ref email) = sol.email_proveedor {
                        <div>
                            <span class="gi-label">{"Email Proveedor"}</span>
                            <p style="font-size: var(--text-sm); color: var(--text-primary);">{email}</p>
                        </div>
                    }
                    if sol.costo_monto.is_some() && sol.costo_moneda.is_some() {
                        <div>
                            <span class="gi-label">{"Costo"}</span>
                            <p style="font-size: var(--text-sm); color: var(--text-primary);">
                                {format_currency(sol.costo_moneda.as_deref().unwrap_or("DOP"), sol.costo_monto.unwrap_or(0.0))}
                            </p>
                        </div>
                    }
                    if let Some(ref fi) = sol.fecha_inicio {
                        <div>
                            <span class="gi-label">{"Fecha Inicio"}</span>
                            <p style="font-size: var(--text-sm); color: var(--text-primary);">{format_date_display(fi)}</p>
                        </div>
                    }
                    if let Some(ref ff) = sol.fecha_fin {
                        <div>
                            <span class="gi-label">{"Fecha Fin"}</span>
                            <p style="font-size: var(--text-sm); color: var(--text-primary);">{format_date_display(ff)}</p>
                        </div>
                    }
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

            <div class="gi-card" style="padding: var(--space-6);">
                <h2 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">{"Notas"}</h2>
                if can_write(&props.user_rol) {
                    <form onsubmit={props.on_add_nota.clone()} style="display: flex; gap: var(--space-3); margin-bottom: var(--space-4);">
                        <textarea
                            value={(*props.nota_contenido).clone()}
                            oninput={textarea_cb!(props.nota_contenido)}
                            class="gi-input"
                            style="flex: 1; min-height: 60px;"
                            placeholder="Agregar una nota..."
                        />
                        <button type="submit" disabled={props.nota_submitting} class="gi-btn gi-btn-primary" style="align-self: flex-end;">
                            {if props.nota_submitting { "Enviando..." } else { "Agregar" }}
                        </button>
                    </form>
                }
                {
                    if sol.notas.as_deref().unwrap_or_default().is_empty() {
                        html! {
                            <p style="font-size: var(--text-sm); color: var(--text-tertiary);">{"Sin notas aún."}</p>
                        }
                    } else {
                        html! {
                            <div style="display: flex; flex-direction: column; gap: var(--space-3);">
                                { for sol.notas.as_deref().unwrap_or_default().iter().map(|n| {
                                    html! {
                                        <div style="padding: var(--space-3); border: 1px solid var(--border-subtle); border-radius: 8px;">
                                            <p style="font-size: var(--text-sm); color: var(--text-primary); margin-bottom: var(--space-1);">{&n.contenido}</p>
                                            <p style="font-size: var(--text-xs); color: var(--text-tertiary);">{format_date_display(&n.created_at)}</p>
                                        </div>
                                    }
                                })}
                            </div>
                        }
                    }
                }
            </div>
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

#[function_component]
fn MantenimientoList(props: &MantenimientoListProps) -> Html {
    if props.items.is_empty() {
        return html! {
            <div class="gi-empty-state">
                <div class="gi-empty-state-icon">
                    <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4">
                        <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>
                    </svg>
                </div>
                <div class="gi-empty-state-title">{"Sin solicitudes de mantenimiento"}</div>
                <p class="gi-empty-state-text">{"Registre solicitudes de mantenimiento para dar seguimiento a reparaciones y trabajos en sus propiedades."}</p>
                if can_write(&props.user_rol) {
                    <button onclick={props.on_new.clone()} class="gi-btn gi-btn-primary" style="margin-top: var(--space-3);">{"+ Nueva Solicitud"}</button>
                }
            </div>
        };
    }

    html! {
        <>
            <DataTable headers={props.headers.clone()}>
                { for props.items.iter().map(|s| {
                    let on_edit = props.on_edit.clone();
                    let on_delete_click = props.on_delete.clone();
                    let on_view_detail = props.on_view_detail.clone();
                    let sc = s.clone();
                    let sd = s.clone();
                    let sv = s.id.clone();
                    let user_rol = props.user_rol.clone();
                    let p_label = props.prop_label.emit(s.propiedad_id.clone());
                    let costo_display = match (&s.costo_monto, &s.costo_moneda) {
                        (Some(monto), Some(moneda)) => format_currency(moneda, *monto),
                        _ => "—".into(),
                    };
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
                            if can_write(&user_rol) {
                                <td style="padding: var(--space-3) var(--space-5); display: flex; gap: var(--space-2);">
                                    <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(sc.clone()))} class="gi-btn-text">{"Editar"}</button>
                                    if can_delete(&user_rol) {
                                        <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(sd.clone()))} class="gi-btn-text" style="color: var(--color-error);">{"Eliminar"}</button>
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
