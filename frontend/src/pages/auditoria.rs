use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::data_table::DataTable;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::TableSkeleton;
use crate::components::common::pagination::Pagination;
use crate::services::api::api_get;
use crate::types::PaginatedResponse;
use crate::types::auditoria::AuditoriaEntry;
use crate::utils::format_date_display;

#[function_component]
pub fn Auditoria() -> Html {
    let items = use_state(Vec::<AuditoriaEntry>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let reload = use_state(|| 0u32);
    let filter_entity_type = use_state(String::new);

    {
        let items = items.clone();
        let total = total.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        let pg = *page;
        let pp = *per_page;
        let fet = (*filter_entity_type).clone();
        use_effect_with((reload_val, pg, fet.clone()), move |_| {
            spawn_local(async move {
                loading.set(true);
                let mut params = vec![format!("page={pg}"), format!("perPage={pp}")];
                if !fet.is_empty() {
                    params.push(format!("entityType={fet}"));
                }
                let url = format!("/auditoria?{}", params.join("&"));
                match api_get::<PaginatedResponse<AuditoriaEntry>>(&url).await {
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

    let on_filter_change = {
        let filter_entity_type = filter_entity_type.clone();
        let page = page.clone();
        let reload = reload;
        Callback::from(move |e: Event| {
            let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
            filter_entity_type.set(el.value());
            page.set(1);
            reload.set(*reload + 1);
        })
    };

    if *loading {
        return html! { <TableSkeleton title_width="260px" columns={5} has_filter=true /> };
    }

    let headers = vec![
        "Fecha".into(),
        "Acción".into(),
        "Entidad".into(),
        "ID Entidad".into(),
        "Cambios".into(),
    ];

    fn accion_label(accion: &str) -> &str {
        match accion {
            "crear" => "Crear",
            "actualizar" => "Actualizar",
            "eliminar" => "Eliminar",
            other => other,
        }
    }

    fn entity_label(et: &str) -> &str {
        match et {
            "propiedad" => "Propiedad",
            "inquilino" => "Inquilino",
            "contrato" => "Contrato",
            "pago" => "Pago",
            other => other,
        }
    }

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Registro de Auditoría"}</h1>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <div class="gi-filter-bar">
                <div style="display: flex; gap: var(--space-3); align-items: end;">
                    <div>
                        <label class="gi-label">{"Tipo de Entidad"}</label>
                        <select onchange={on_filter_change} class="gi-input">
                            <option value="" selected={filter_entity_type.is_empty()}>{"Todos"}</option>
                            <option value="propiedad" selected={*filter_entity_type == "propiedad"}>{"Propiedad"}</option>
                            <option value="inquilino" selected={*filter_entity_type == "inquilino"}>{"Inquilino"}</option>
                            <option value="contrato" selected={*filter_entity_type == "contrato"}>{"Contrato"}</option>
                            <option value="pago" selected={*filter_entity_type == "pago"}>{"Pago"}</option>
                        </select>
                    </div>
                </div>
            </div>

            if (*items).is_empty() {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-title">{"Sin registros de auditoría"}</div>
                    <p class="gi-empty-state-text">{"Las acciones realizadas en el sistema se registrarán aquí."}</p>
                </div>
            } else {
                <DataTable headers={headers}>
                    { for (*items).iter().map(|entry| {
                        let changes_str = serde_json::to_string_pretty(&entry.cambios)
                            .unwrap_or_else(|_| "{}".into());
                        let short_changes = if changes_str.len() > 80 {
                            format!("{}…", &changes_str[..80])
                        } else {
                            changes_str.clone()
                        };
                        let short_id = if entry.entity_id.len() > 8 {
                            format!("{}…", &entry.entity_id[..8])
                        } else {
                            entry.entity_id.clone()
                        };
                        html! {
                            <tr>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {format_date_display(&entry.created_at)}
                                </td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {accion_label(&entry.accion)}
                                </td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {entity_label(&entry.entity_type)}
                                </td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);" title={entry.entity_id.clone()}>
                                    {short_id}
                                </td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-xs); font-family: monospace; color: var(--text-secondary);" title={changes_str}>
                                    {short_changes}
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
