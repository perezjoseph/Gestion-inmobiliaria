use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::components::common::data_table::DataTable;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::pagination::Pagination;
use crate::components::common::skeleton::TableSkeleton;
use crate::services::api::api_get;
use crate::types::PaginatedResponse;
use crate::types::tarea::EjecucionTarea;
use crate::utils::format_date_display;

#[component]
pub fn Tareas() -> Html {
    let items = use_state(Vec::<EjecucionTarea>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let reload = use_state(|| 0u32);

    let auth = use_context::<AuthContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map_or("", |u| u.rol.as_str());
    let is_admin = user_rol == "admin";

    {
        let items = items.clone();
        let total = total.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        let pg = *page;
        let pp = *per_page;
        use_effect_with((reload_val, pg), move |_| {
            spawn_local(async move {
                loading.set(true);
                let url = format!("/tareas/historial?page={pg}&perPage={pp}");
                match api_get::<PaginatedResponse<EjecucionTarea>>(&url).await {
                    Ok(resp) => {
                        total.set(resp.total);
                        items.set(resp.data);
                    }
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        });
    }

    if !is_admin {
        return html! {
            <div class="gi-empty-state">
                <div class="gi-empty-state-title">{"No tiene permisos para acceder a esta secciÃ³n"}</div>
            </div>
        };
    }

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
        Callback::from(move |pp: u64| {
            per_page.set(pp);
            page.set(1);
            reload.set(*reload + 1);
        })
    };

    if *loading {
        return html! { <TableSkeleton title_width="260px" columns={5} has_filter=false /> };
    }

    let headers = vec![
        "Tarea".into(),
        "Iniciado".into(),
        "DuraciÃ³n".into(),
        "Exitosa".into(),
        "Registros Afectados".into(),
    ];

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Historial de Tareas"}</h1>
                <button class="gi-btn gi-btn-primary">{"Ejecutar"}</button>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            if (*items).is_empty() {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-title">{"Sin ejecuciones de tareas"}</div>
                    <p class="gi-empty-state-text">{"El historial de tareas programadas aparecerÃ¡ aquÃ."}</p>
                </div>
            } else {
                <DataTable headers={headers}>
                    { for (*items).iter().map(|item| {
                        let duracion = format!("{:.1}s", item.duracion_ms as f64 / 1000.0);
                        let exitosa_badge = if item.exitosa {
                            html! { <span class="gi-badge gi-badge-success">{"SÃ"}</span> }
                        } else {
                            html! { <span class="gi-badge gi-badge-danger">{"No"}</span> }
                        };
                        html! {
                            <tr>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {&item.nombre_tarea}
                                </td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {format_date_display(&item.iniciado_en)}
                                </td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {duracion}
                                </td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {exitosa_badge}
                                </td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {item.registros_afectados}
                                </td>
                            </tr>
                        }
                    })}
                </DataTable>
                <Pagination
                    total={*total}
                    page={*page}
                    per_page={*per_page}
                    on_page_change={on_page_change}
                    on_per_page_change={on_per_page_change}
                />
            }
        </div>
    }
}
