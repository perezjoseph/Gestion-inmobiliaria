use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::components::common::data_table::DataTable;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::TableSkeleton;
use crate::services::api::api_get;
use crate::types::ncf::SecuenciaNcf;

#[component]
pub fn Ncf() -> Html {
    let items = use_state(Vec::<SecuenciaNcf>::new);
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
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        use_effect_with(reload_val, move |_| {
            spawn_local(async move {
                loading.set(true);
                match api_get::<Vec<SecuenciaNcf>>("/ncf/secuencias").await {
                    Ok(resp) => {
                        items.set(resp);
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

    if *loading {
        return html! { <TableSkeleton title_width="260px" columns={6} has_filter=false /> };
    }

    let headers = vec![
        "Tipo NCF".into(),
        "Prefijo".into(),
        "Siguiente NÃºmero".into(),
        "Rango Desde".into(),
        "Rango Hasta".into(),
        "Activo".into(),
    ];

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Secuencias NCF"}</h1>
                <button class="gi-btn gi-btn-primary">{"Configurar Rango"}</button>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            if (*items).is_empty() {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-title">{"Sin secuencias NCF configuradas"}</div>
                    <p class="gi-empty-state-text">{"Configure los rangos de comprobantes fiscales aquÃ­."}</p>
                </div>
            } else {
                <DataTable headers={headers}>
                    { for (*items).iter().map(|item| {
                        let activo_badge = if item.is_active {
                            html! { <span class="gi-badge gi-badge-success">{"SÃ­"}</span> }
                        } else {
                            html! { <span class="gi-badge gi-badge-neutral">{"No"}</span> }
                        };
                        html! {
                            <tr>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {&item.tipo_ncf}
                                </td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {&item.prefijo}
                                </td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {item.siguiente_numero}
                                </td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {item.rango_desde}
                                </td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {item.rango_hasta}
                                </td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {activo_badge}
                                </td>
                            </tr>
                        }
                    })}
                </DataTable>
            }
        </div>
    }
}
