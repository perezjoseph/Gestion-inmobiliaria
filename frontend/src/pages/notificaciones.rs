use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::data_table::DataTable;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::pagination::Pagination;
use crate::components::common::skeleton::TableSkeleton;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_get, api_put};
use crate::types::PaginatedResponse;
use crate::types::notificacion::{MarcarTodasResponse, Notificacion};
use crate::utils::format_date_display;

fn push_toast(toasts: Option<&ToastContext>, msg: &str, kind: ToastKind) {
    if let Some(t) = toasts {
        t.dispatch(ToastAction::Push(msg.into(), kind));
    }
}

fn tipo_badge(tipo: &str) -> (&'static str, &'static str) {
    match tipo {
        "pago_vencido" => ("gi-badge gi-badge-error", "Pago Vencido"),
        "contrato_por_vencer" => ("gi-badge gi-badge-warning", "Contrato por Vencer"),
        "documento_vencido" => ("gi-badge gi-badge-caution", "Documento Vencido"),
        "mantenimiento_actualizado" => ("gi-badge gi-badge-info", "Mantenimiento"),
        _ => ("gi-badge gi-badge-neutral", "Otro"),
    }
}

fn tipo_color(tipo: &str) -> &'static str {
    match tipo {
        "pago_vencido" => "var(--color-error)",
        "contrato_por_vencer" => "var(--color-warning, orange)",
        "documento_vencido" => "var(--color-caution, #ca8a04)",
        "mantenimiento_actualizado" => "var(--color-info, #2563eb)",
        _ => "var(--text-secondary)",
    }
}

fn tipo_icon(tipo: &str) -> &'static str {
    match tipo {
        "pago_vencido" => "💰",
        "contrato_por_vencer" => "📄",
        "documento_vencido" => "📁",
        "mantenimiento_actualizado" => "🔧",
        _ => "🔔",
    }
}

#[derive(Properties, PartialEq)]
struct NotificacionFilterBarProps {
    filter_tipo: UseStateHandle<String>,
    filter_leida: UseStateHandle<String>,
    on_apply: Callback<MouseEvent>,
    on_clear: Callback<MouseEvent>,
}

#[component]
fn NotificacionFilterBar(props: &NotificacionFilterBarProps) -> Html {
    let ft = props.filter_tipo.clone();
    let on_tipo_change = Callback::from(move |e: Event| {
        let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
        ft.set(el.value());
    });
    let fl = props.filter_leida.clone();
    let on_leida_change = Callback::from(move |e: Event| {
        let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
        fl.set(el.value());
    });
    html! {
        <div class="gi-filter-bar">
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--space-3); align-items: end;">
                <div>
                    <label class="gi-label">{"Tipo"}</label>
                    <select onchange={on_tipo_change} class="gi-input">
                        <option value="" selected={props.filter_tipo.is_empty()}>{"Todos"}</option>
                        <option value="pago_vencido" selected={*props.filter_tipo == "pago_vencido"}>{"Pago Vencido"}</option>
                        <option value="contrato_por_vencer" selected={*props.filter_tipo == "contrato_por_vencer"}>{"Contrato por Vencer"}</option>
                        <option value="documento_vencido" selected={*props.filter_tipo == "documento_vencido"}>{"Documento Vencido"}</option>
                        <option value="mantenimiento_actualizado" selected={*props.filter_tipo == "mantenimiento_actualizado"}>{"Mantenimiento"}</option>
                    </select>
                </div>
                <div>
                    <label class="gi-label">{"Estado"}</label>
                    <select onchange={on_leida_change} class="gi-input">
                        <option value="" selected={props.filter_leida.is_empty()}>{"Todas"}</option>
                        <option value="false" selected={*props.filter_leida == "false"}>{"No leídas"}</option>
                        <option value="true" selected={*props.filter_leida == "true"}>{"Leídas"}</option>
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

fn render_notificacion_row(n: &Notificacion, on_mark_read: &Callback<String>) -> Html {
    let (badge_cls, badge_label) = tipo_badge(&n.tipo);
    let color = tipo_color(&n.tipo);
    let icon = tipo_icon(&n.tipo);
    let is_unread = !n.leida;
    let row_style = if is_unread {
        "background-color: var(--surface-raised, var(--color-primary-50, #eff6ff));"
    } else {
        ""
    };
    let font_weight = if is_unread { "600" } else { "400" };

    let mark_btn = if is_unread {
        let id = n.id.clone();
        let cb = on_mark_read.clone();
        html! {
            <button onclick={Callback::from(move |_: MouseEvent| cb.emit(id.clone()))}
                class="gi-btn-text" style="font-size: var(--text-xs);">
                {"Marcar leída"}
            </button>
        }
    } else {
        html! { <span style="font-size: var(--text-xs); color: var(--text-tertiary);">{"✓ Leída"}</span> }
    };

    html! {
        <tr style={row_style}>
            <td style="padding: var(--space-3) var(--space-5);">
                <span style={format!("color: {color}; margin-right: var(--space-1);")}>{icon}</span>
                <span class={badge_cls}>{badge_label}</span>
            </td>
            <td style={format!("padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: {font_weight};")}>{&n.titulo}</td>
            <td style={format!("padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: {font_weight}; color: var(--text-secondary);")}>{&n.mensaje}</td>
            <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                {mark_btn}
            </td>
            <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">
                {format_date_display(&n.created_at)}
            </td>
        </tr>
    }
}

#[derive(Properties, PartialEq)]
struct NotificacionListProps {
    items: Vec<Notificacion>,
    headers: Vec<String>,
    total: u64,
    page: u64,
    per_page: u64,
    on_mark_read: Callback<String>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
}

#[component]
fn NotificacionList(props: &NotificacionListProps) -> Html {
    if props.items.is_empty() {
        return html! {
            <div class="gi-empty-state">
                <div class="gi-empty-state-icon">
                    <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4">
                        <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/>
                        <path d="M13.73 21a2 2 0 0 1-3.46 0"/>
                    </svg>
                </div>
                <div class="gi-empty-state-title">{"Sin notificaciones"}</div>
                <p class="gi-empty-state-text">{"No tiene notificaciones en este momento. Las notificaciones aparecerán cuando haya pagos vencidos, contratos por vencer u otros eventos del sistema."}</p>
            </div>
        };
    }

    html! {
        <>
            <DataTable headers={props.headers.clone()}>
                { for props.items.iter().map(|n| render_notificacion_row(n, &props.on_mark_read)) }
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

fn build_notificaciones_url(pg: u64, pp: u64, f_tipo: &str, f_leida: &str) -> String {
    let mut params = vec![format!("page={pg}"), format!("perPage={pp}")];
    if !f_tipo.is_empty() {
        params.push(format!("tipo={f_tipo}"));
    }
    if !f_leida.is_empty() {
        params.push(format!("leida={f_leida}"));
    }
    format!("/notificaciones?{}", params.join("&"))
}

fn load_notificaciones(
    items: UseStateHandle<Vec<Notificacion>>,
    total: UseStateHandle<u64>,
    error: UseStateHandle<Option<String>>,
    loading: UseStateHandle<bool>,
    url: String,
) {
    spawn_local(async move {
        loading.set(true);
        match api_get::<PaginatedResponse<Notificacion>>(&url).await {
            Ok(resp) => {
                total.set(resp.total);
                items.set(resp.data);
            }
            Err(err) => error.set(Some(err)),
        }
        loading.set(false);
    });
}

#[component]
#[allow(clippy::redundant_clone)]
pub fn Notificaciones() -> Html {
    let toasts = use_context::<ToastContext>();

    let items = use_state(Vec::<Notificacion>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let reload = use_state(|| 0u32);

    let filter_tipo = use_state(String::new);
    let filter_leida = use_state(String::new);
    let applied_tipo = use_state(String::new);
    let applied_leida = use_state(String::new);

    // Load data on mount and when reload/page changes
    {
        let items = items.clone();
        let total = total.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        let pg = *page;
        let pp = *per_page;
        let tipo_val = (*applied_tipo).clone();
        let leida_val = (*applied_leida).clone();
        use_effect_with((reload_val, pg), move |_| {
            let url = build_notificaciones_url(pg, pp, &tipo_val, &leida_val);
            load_notificaciones(items, total, error, loading, url);
        });
    }

    // Filter apply
    let on_filter_apply = {
        let filter_tipo = filter_tipo.clone();
        let filter_leida = filter_leida.clone();
        let applied_tipo = applied_tipo.clone();
        let applied_leida = applied_leida.clone();
        let page = page.clone();
        let reload = reload.clone();
        Callback::from(move |_: MouseEvent| {
            applied_tipo.set((*filter_tipo).clone());
            applied_leida.set((*filter_leida).clone());
            page.set(1);
            reload.set(*reload + 1);
        })
    };

    // Filter clear
    let on_filter_clear = {
        let filter_tipo = filter_tipo.clone();
        let filter_leida = filter_leida.clone();
        let applied_tipo = applied_tipo.clone();
        let applied_leida = applied_leida.clone();
        let page = page.clone();
        let reload = reload.clone();
        Callback::from(move |_: MouseEvent| {
            filter_tipo.set(String::new());
            filter_leida.set(String::new());
            applied_tipo.set(String::new());
            applied_leida.set(String::new());
            page.set(1);
            reload.set(*reload + 1);
        })
    };

    // Mark single as read
    let on_mark_read = {
        let reload = reload.clone();
        let error = error.clone();
        let toasts = toasts.clone();
        Callback::from(move |id: String| {
            let reload = reload.clone();
            let error = error.clone();
            let toasts = toasts.clone();
            spawn_local(async move {
                let url = format!("/notificaciones/{id}/leer");
                match api_put::<Notificacion, _>(&url, &()).await {
                    Ok(_) => {
                        reload.set(*reload + 1);
                        push_toast(
                            toasts.as_ref(),
                            "Notificación marcada como leída",
                            ToastKind::Success,
                        );
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        })
    };

    // Mark all as read
    let on_mark_all_read = {
        let reload = reload.clone();
        let error = error.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            let reload = reload.clone();
            let error = error.clone();
            let toasts = toasts.clone();
            spawn_local(async move {
                match api_put::<MarcarTodasResponse, _>("/notificaciones/leer-todas", &()).await {
                    Ok(resp) => {
                        reload.set(*reload + 1);
                        let msg =
                            format!("{} notificaciones marcadas como leídas", resp.actualizadas);
                        push_toast(toasts.as_ref(), &msg, ToastKind::Success);
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
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

    if *loading {
        return html! { <TableSkeleton title_width="200px" columns={5} has_filter={true} /> };
    }

    let headers = vec![
        "Tipo".into(),
        "Título".into(),
        "Mensaje".into(),
        "Estado".into(),
        "Fecha".into(),
    ];

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Notificaciones"}</h1>
                <button onclick={on_mark_all_read} class="gi-btn gi-btn-primary">
                    {"Marcar todas como leídas"}
                </button>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone();
                    move |_: MouseEvent| error.set(None)
                })} />
            }

            <NotificacionFilterBar
                filter_tipo={filter_tipo}
                filter_leida={filter_leida}
                on_apply={on_filter_apply}
                on_clear={on_filter_clear}
            />

            <NotificacionList
                items={(*items).clone()}
                headers={headers}
                total={*total}
                page={*page}
                per_page={*per_page}
                on_mark_read={on_mark_read}
                on_page_change={on_page_change}
                on_per_page_change={on_per_page_change}
            />
        </div>
    }
}
