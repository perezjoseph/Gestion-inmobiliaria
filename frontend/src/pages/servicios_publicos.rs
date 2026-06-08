use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::components::common::data_table::DataTable;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::TableSkeleton;
use crate::services::api::api_get;
use crate::types::propiedad::Propiedad;
use crate::types::servicio_publico::ResponsabilidadEfectiva;
use crate::types::unidad::Unidad;

#[component]
pub fn ServiciosPublicos() -> Html {
    let propiedades = use_state(Vec::<Propiedad>::new);
    let unidades = use_state(Vec::<Unidad>::new);
    let items = use_state(Vec::<ResponsabilidadEfectiva>::new);
    let selected_prop = use_state(|| Option::<String>::None);
    let selected_unit = use_state(|| Option::<String>::None);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);
    let loading_props = use_state(|| true);
    let reload = use_state(|| 0u32);

    let auth = use_context::<AuthContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map_or("", |u| u.rol.as_str());
    let can_write = user_rol == "admin" || user_rol == "gerente";

    if !can_write {
        return html! {
            <div class="gi-empty-state">
                <div class="gi-empty-state-title">{"No tiene permisos para acceder a esta sección"}</div>
            </div>
        };
    }

    // Load properties on mount
    {
        let propiedades = propiedades.clone();
        let loading_props = loading_props.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                if let Ok(resp) = api_get::<Vec<Propiedad>>("/propiedades/todas").await {
                    propiedades.set(resp);
                }
                loading_props.set(false);
            });
        });
    }

    // Load units when property selected
    {
        let unidades = unidades.clone();
        let selected_prop_val = (*selected_prop).clone();
        let selected_unit = selected_unit.clone();
        use_effect_with(selected_prop_val.clone(), move |_deps| {
            if let Some(prop_id) = selected_prop_val {
                let unidades = unidades.clone();
                let selected_unit = selected_unit.clone();
                spawn_local(async move {
                    selected_unit.set(None);
                    let url = format!("/propiedades/{prop_id}/unidades");
                    if let Ok(resp) = api_get::<Vec<Unidad>>(&url).await {
                        unidades.set(resp);
                    } else {
                        unidades.set(vec![]);
                    }
                });
            } else {
                unidades.set(vec![]);
                selected_unit.set(None);
            }
        });
    }

    // Load servicios when unit selected
    {
        let items = items.clone();
        let error = error.clone();
        let loading = loading.clone();
        let selected_prop_val = (*selected_prop).clone();
        let selected_unit_val = (*selected_unit).clone();
        let reload_val = *reload;
        let closure_unit = selected_unit_val.clone();
        use_effect_with((selected_unit_val, reload_val), move |_deps| {
            if let (Some(prop_id), Some(unit_id)) = (selected_prop_val, closure_unit) {
                let items = items.clone();
                let error = error.clone();
                spawn_local(async move {
                    loading.set(true);
                    let url = format!("/propiedades/{prop_id}/unidades/{unit_id}/servicios");
                    match api_get::<Vec<ResponsabilidadEfectiva>>(&url).await {
                        Ok(resp) => items.set(resp),
                        Err(err) => error.set(Some(err)),
                    }
                    loading.set(false);
                });
            } else {
                items.set(vec![]);
            }
        });
    }

    let on_prop_change = {
        let selected_prop = selected_prop.clone();
        Callback::from(move |e: Event| {
            let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let val = el.value();
            if val.is_empty() {
                selected_prop.set(None);
            } else {
                selected_prop.set(Some(val));
            }
        })
    };

    let on_unit_change = {
        let selected_unit = selected_unit.clone();
        Callback::from(move |e: Event| {
            let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let val = el.value();
            if val.is_empty() {
                selected_unit.set(None);
            } else {
                selected_unit.set(Some(val));
            }
        })
    };

    if *loading_props {
        return html! { <TableSkeleton title_width="260px" columns={3} has_filter=false /> };
    }

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Servicios Públicos"}</h1>
                if can_write && (*selected_unit).is_some() {
                    <button class="gi-btn gi-btn-primary">{"Asignar"}</button>
                }
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <div class="gi-filter-bar">
                <div style="display: flex; gap: var(--space-3); align-items: end;">
                    <div>
                        <label class="gi-label">{"Propiedad"}</label>
                        <select onchange={on_prop_change} class="gi-input">
                            <option value="">{"Seleccione propiedad"}</option>
                            { for (*propiedades).iter().map(|p| {
                                html! { <option value={p.id.clone()}>{&p.titulo}</option> }
                            })}
                        </select>
                    </div>
                    <div>
                        <label class="gi-label">{"Unidad"}</label>
                        <select onchange={on_unit_change} class="gi-input" disabled={(*selected_prop).is_none()}>
                            <option value="">{"Seleccione unidad"}</option>
                            { for (*unidades).iter().map(|u| {
                                html! { <option value={u.id.clone()}>{&u.numero_unidad}</option> }
                            })}
                        </select>
                    </div>
                </div>
            </div>

            if (*selected_unit).is_none() {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-title">{"Seleccione una propiedad y unidad"}</div>
                    <p class="gi-empty-state-text">{"Seleccione una propiedad y unidad para ver las responsabilidades de servicios públicos."}</p>
                </div>
            } else if *loading {
                <TableSkeleton title_width="200px" columns={3} has_filter=false />
            } else if (*items).is_empty() {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-title">{"Sin responsabilidades asignadas"}</div>
                    <p class="gi-empty-state-text">{"No hay servicios públicos configurados para esta unidad."}</p>
                </div>
            } else {
                {render_servicios_table(&items)}
            }
        </div>
    }
}

#[allow(dead_code)]
fn render_servicios_table(items: &[ResponsabilidadEfectiva]) -> Html {
    let headers = vec![
        "Proveedor".into(),
        "Responsable".into(),
        "Override de Contrato".into(),
    ];
    html! {
        <DataTable headers={headers}>
            { for items.iter().map(|item| {
                let override_badge = if item.es_override_contrato {
                    html! { <span class="gi-badge gi-badge-warning">{"Sí"}</span> }
                } else {
                    html! { <span class="gi-badge gi-badge-neutral">{"No"}</span> }
                };
                html! {
                    <tr>
                        <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                            {&item.proveedor_servicio}
                        </td>
                        <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                            {&item.responsable}
                        </td>
                        <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                            {override_badge}
                        </td>
                    </tr>
                }
            })}
        </DataTable>
    }
}
