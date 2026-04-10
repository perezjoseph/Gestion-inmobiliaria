use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::data_table::DataTable;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::pagination::Pagination;
use crate::components::common::skeleton::TableSkeleton;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_get, api_put};
use crate::types::PaginatedResponse;
use crate::types::usuario::User;

fn push_toast(toasts: Option<&ToastContext>, msg: &str) {
    if let Some(t) = toasts {
        t.dispatch(ToastAction::Push(msg.into(), ToastKind::Success));
    }
}

#[derive(Properties, PartialEq)]
struct UsuarioRowProps {
    user: User,
    on_role_change: Callback<(String, String)>,
    on_toggle_active: Callback<(String, bool)>,
}

#[component]
fn UsuarioRow(props: &UsuarioRowProps) -> Html {
    let u = &props.user;
    let uid = u.id.clone();
    let uid2 = u.id.clone();
    let activo = u.activo;
    let on_role_change = props.on_role_change.clone();
    let on_toggle_active = props.on_toggle_active.clone();

    html! {
        <tr>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{&u.nombre}</td>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{&u.email}</td>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                <select
                    onchange={Callback::from(move |e: Event| {
                        let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
                        on_role_change.emit((uid.clone(), el.value()));
                    })}
                    class="gi-input" style="width: auto; padding: var(--space-1) var(--space-2);"
                >
                    <option value="admin" selected={u.rol == "admin"}>{"Admin"}</option>
                    <option value="gerente" selected={u.rol == "gerente"}>{"Gerente"}</option>
                    <option value="visualizador" selected={u.rol == "visualizador"}>{"Visualizador"}</option>
                </select>
            </td>
            <td style="padding: var(--space-3) var(--space-5);">
                <span class={if u.activo { "gi-badge gi-badge-success" } else { "gi-badge gi-badge-error" }}>
                    { if u.activo { "Activo" } else { "Inactivo" } }
                </span>
            </td>
            <td style="padding: var(--space-3) var(--space-5);">
                <button
                    onclick={Callback::from(move |_: MouseEvent| on_toggle_active.emit((uid2.clone(), activo)))}
                    class="gi-btn-text"
                >
                    { if activo { "Desactivar" } else { "Activar" } }
                </button>
            </td>
        </tr>
    }
}

#[component]
pub fn Usuarios() -> Html {
    let toasts = use_context::<ToastContext>();
    let items = use_state(Vec::<User>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let reload = use_state(|| 0u32);

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
                let url = format!("/usuarios?page={pg}&perPage={pp}");
                match api_get::<PaginatedResponse<User>>(&url).await {
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

    let on_role_change = {
        let error = error.clone();
        let reload = reload.clone();
        let toasts = toasts.clone();
        Callback::from(move |(id, rol): (String, String)| {
            let error = error.clone();
            let reload = reload.clone();
            let toasts = toasts.clone();
            spawn_local(async move {
                let body = serde_json::json!({ "nuevoRol": rol });
                match api_put::<User, _>(&format!("/usuarios/{id}/rol"), &body).await {
                    Ok(_) => {
                        reload.set(*reload + 1);
                        push_toast(toasts.as_ref(), "Rol actualizado");
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        })
    };

    let on_toggle_active = {
        let error = error.clone();
        let reload = reload;
        let toasts = toasts;
        Callback::from(move |(id, activo): (String, bool)| {
            let error = error.clone();
            let reload = reload.clone();
            let toasts = toasts.clone();
            let action = if activo { "desactivar" } else { "activar" };
            spawn_local(async move {
                let url = format!("/usuarios/{id}/{action}");
                match api_put::<User, _>(&url, &serde_json::json!({})).await {
                    Ok(_) => {
                        reload.set(*reload + 1);
                        let msg = if activo {
                            "Usuario desactivado"
                        } else {
                            "Usuario activado"
                        };
                        push_toast(toasts.as_ref(), msg);
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        })
    };

    if *loading {
        return html! { <TableSkeleton title_width="220px" columns={5} /> };
    }

    let headers = vec![
        "Nombre".into(),
        "Email".into(),
        "Rol".into(),
        "Estado".into(),
        "Acciones".into(),
    ];

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Gestión de Usuarios"}</h1>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            if (*items).is_empty() {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-title">{"Sin usuarios registrados"}</div>
                </div>
            } else {
                <DataTable headers={headers}>
                    { for (*items).iter().map(|u| {
                        html! {
                            <UsuarioRow
                                user={u.clone()}
                                on_role_change={on_role_change.clone()}
                                on_toggle_active={on_toggle_active.clone()}
                            />
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
