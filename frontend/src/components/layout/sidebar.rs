use std::cmp::Reverse;
use std::collections::HashMap;

use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthContext, Route};
use crate::services::api::api_get;
use crate::types::DashboardStats;

const FRECUENTES_KEY: &str = "gi_frecuentes";
const SIDEBAR_COLLAPSED_KEY: &str = "gi_sidebar_collapsed";
const MAX_FRECUENTES: usize = 4;

#[derive(Properties, PartialEq)]
pub struct SidebarProps {
    pub is_open: bool,
    pub on_nav_click: Callback<()>,
}

fn storage_get(key: &str) -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()
        .flatten()?
        .get_item(key)
        .ok()
        .flatten()
}

fn storage_set(key: &str, value: &str) {
    if let Some(win) = web_sys::window()
        && let Ok(Some(storage)) = win.local_storage()
    {
        let _ = storage.set_item(key, value);
    }
}

const fn is_trackable_route(route: &Route) -> bool {
    matches!(
        route,
        Route::Dashboard
            | Route::Propiedades
            | Route::Inquilinos
            | Route::Contratos
            | Route::Pagos
            | Route::Gastos
            | Route::Mantenimiento
            | Route::Desahucios
            | Route::Reportes
    )
}

const fn route_to_key(route: &Route) -> &'static str {
    match route {
        Route::Dashboard => "dashboard",
        Route::Propiedades => "propiedades",
        Route::Inquilinos => "inquilinos",
        Route::Contratos => "contratos",
        Route::Pagos => "pagos",
        Route::Gastos => "gastos",
        Route::Mantenimiento => "mantenimiento",
        Route::Desahucios => "desahucios",
        Route::Reportes => "reportes",
        _ => "",
    }
}

fn key_to_route(key: &str) -> Option<Route> {
    match key {
        "dashboard" => Some(Route::Dashboard),
        "propiedades" => Some(Route::Propiedades),
        "inquilinos" => Some(Route::Inquilinos),
        "contratos" => Some(Route::Contratos),
        "pagos" => Some(Route::Pagos),
        "gastos" => Some(Route::Gastos),
        "mantenimiento" => Some(Route::Mantenimiento),
        "desahucios" => Some(Route::Desahucios),
        "reportes" => Some(Route::Reportes),
        _ => None,
    }
}

const fn route_label(route: &Route) -> &'static str {
    match route {
        Route::Dashboard => "Panel de Control",
        Route::Propiedades => "Propiedades",
        Route::Inquilinos => "Inquilinos",
        Route::Contratos => "Contratos",
        Route::Pagos => "Pagos",
        Route::Gastos => "Gastos",
        Route::Mantenimiento => "Mantenimiento",
        Route::Desahucios => "Desahucios",
        Route::Reportes => "Reportes",
        _ => "",
    }
}

fn route_icon(route: &Route) -> Html {
    match route {
        Route::Dashboard => icon_dashboard(),
        Route::Propiedades => icon_properties(),
        Route::Inquilinos => icon_tenants(),
        Route::Contratos => icon_contracts(),
        Route::Pagos => icon_payments(),
        Route::Gastos => icon_expenses(),
        Route::Mantenimiento => icon_maintenance(),
        Route::Desahucios => icon_desahucios(),
        Route::Reportes => icon_reports(),
        _ => html! {},
    }
}

fn load_visit_counts() -> HashMap<String, u32> {
    storage_get(FRECUENTES_KEY)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_visit_counts(counts: &HashMap<String, u32>) {
    if let Ok(json) = serde_json::to_string(counts) {
        storage_set(FRECUENTES_KEY, &json);
    }
}

fn increment_visit(route: &Route) {
    if !is_trackable_route(route) {
        return;
    }
    let key = route_to_key(route);
    if key.is_empty() {
        return;
    }
    let mut counts = load_visit_counts();
    *counts.entry(key.to_string()).or_insert(0) += 1;
    save_visit_counts(&counts);
}

fn top_frecuentes() -> Vec<Route> {
    let counts = load_visit_counts();
    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by_key(|b| Reverse(b.1));
    sorted
        .into_iter()
        .take(MAX_FRECUENTES)
        .filter(|(_, count)| *count >= 3)
        .filter_map(|(key, _)| key_to_route(&key))
        .collect()
}

fn load_collapsed_groups() -> Vec<String> {
    storage_get(SIDEBAR_COLLAPSED_KEY)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| vec!["herramientas".to_string(), "sistema".to_string()])
}

fn save_collapsed_groups(groups: &[String]) {
    if let Ok(json) = serde_json::to_string(groups) {
        storage_set(SIDEBAR_COLLAPSED_KEY, &json);
    }
}

fn icon_dashboard() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="7" height="7" rx="1"/>
            <rect x="14" y="3" width="7" height="7" rx="1"/>
            <rect x="3" y="14" width="7" height="7" rx="1"/>
            <rect x="14" y="14" width="7" height="7" rx="1"/>
        </svg>
    }
}

fn icon_properties() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 21V9l9-6 9 6v12"/>
            <path d="M9 21V12h6v9"/>
        </svg>
    }
}

fn icon_tenants() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/>
            <circle cx="9" cy="7" r="4"/>
            <path d="M22 21v-2a4 4 0 0 0-3-3.87"/>
            <path d="M16 3.13a4 4 0 0 1 0 7.75"/>
        </svg>
    }
}

fn icon_contracts() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
            <line x1="16" y1="13" x2="8" y2="13"/>
            <line x1="16" y1="17" x2="8" y2="17"/>
        </svg>
    }
}

fn icon_payments() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="1" x2="12" y2="23"/>
            <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>
        </svg>
    }
}

fn icon_expenses() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/>
            <path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>
        </svg>
    }
}

fn icon_maintenance() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>
        </svg>
    }
}

fn icon_reports() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
        </svg>
    }
}

fn icon_users() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
            <circle cx="9" cy="7" r="4"/>
            <path d="M23 21v-2a4 4 0 0 0-3-3.87"/>
            <path d="M16 3.13a4 4 0 0 1 0 7.75"/>
        </svg>
    }
}

fn icon_audit() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
        </svg>
    }
}

fn icon_import() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="17 8 12 3 7 8"/>
            <line x1="12" y1="3" x2="12" y2="15"/>
        </svg>
    }
}

fn icon_profile() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/>
            <circle cx="12" cy="7" r="4"/>
        </svg>
    }
}

fn icon_settings() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3"/>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
        </svg>
    }
}

fn icon_plantillas() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="18" height="18" rx="2"/>
            <path d="M3 9h18"/>
            <path d="M9 21V9"/>
        </svg>
    }
}

fn icon_desahucios() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 21V9l9-6 9 6v12"/>
            <line x1="3" y1="21" x2="21" y2="21"/>
            <line x1="9" y1="21" x2="9" y2="12"/>
            <line x1="15" y1="21" x2="15" y2="12"/>
            <line x1="2" y1="3" x2="22" y2="21"/>
        </svg>
    }
}

fn icon_dgii() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="2" y="3" width="20" height="18" rx="2"/>
            <path d="M8 7h8"/>
            <path d="M8 11h8"/>
            <path d="M8 15h4"/>
        </svg>
    }
}

fn icon_servicios_publicos() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
        </svg>
    }
}

fn icon_ncf() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 5H7a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-2"/>
            <rect x="9" y="3" width="6" height="4" rx="1"/>
            <path d="M9 14l2 2 4-4"/>
        </svg>
    }
}

fn icon_tareas() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>
        </svg>
    }
}

fn icon_invitaciones() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/>
            <polyline points="22,6 12,13 2,6"/>
        </svg>
    }
}

fn icon_organizacion() -> Html {
    html! {
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 21h18"/>
            <path d="M5 21V7l8-4v18"/>
            <path d="M19 21V11l-6-4"/>
            <path d="M9 9v.01"/>
            <path d="M9 12v.01"/>
            <path d="M9 15v.01"/>
            <path d="M9 18v.01"/>
        </svg>
    }
}

fn attention_badge(count: u64) -> Html {
    if count == 0 {
        return html! {};
    }
    let label = if count > 9 {
        "9+".to_string()
    } else {
        count.to_string()
    };
    html! {
        <span class="gi-sidebar-attention-badge">{label}</span>
    }
}

#[component]
pub fn Sidebar(props: &SidebarProps) -> Html {
    let current_route = use_route::<Route>();
    let on_nav_click = props.on_nav_click.clone();
    let auth = use_context::<AuthContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map_or("", |u| u.rol.as_str());
    let is_admin = user_rol == "admin";
    let can_write = user_rol == "admin" || user_rol == "gerente";

    let collapsed_groups = use_state(load_collapsed_groups);

    let pagos_atrasados = use_state(|| 0u64);
    {
        let pagos_atrasados = pagos_atrasados.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                if let Ok(stats) = api_get::<DashboardStats>("/dashboard/stats").await {
                    pagos_atrasados.set(stats.pagos_atrasados);
                }
            });
        });
    }

    let frecuentes = use_state(top_frecuentes);
    {
        let frecuentes = frecuentes.clone();
        use_effect_with(current_route.clone(), move |route| {
            if let Some(r) = route {
                increment_visit(r);
                frecuentes.set(top_frecuentes());
            }
        });
    }

    let link_class = |route: &Route| -> String {
        let is_active = match route {
            Route::Configuracion => matches!(
                current_route.as_ref(),
                Some(Route::Configuracion | Route::ConfiguracionChatbot)
            ),
            _ => current_route.as_ref() == Some(route),
        };
        if is_active {
            "gi-sidebar-link gi-sidebar-link-active".to_string()
        } else {
            "gi-sidebar-link".to_string()
        }
    };

    let open_class = if props.is_open { " open" } else { "" };

    let make_click = |on_nav: Callback<()>| -> Callback<MouseEvent> {
        Callback::from(move |_: MouseEvent| {
            on_nav.emit(());
        })
    };

    let make_toggle = |group_id: &'static str| -> Callback<MouseEvent> {
        let collapsed_groups = collapsed_groups.clone();
        Callback::from(move |_: MouseEvent| {
            let mut groups = (*collapsed_groups).clone();
            if let Some(pos) = groups.iter().position(|g| g == group_id) {
                groups.remove(pos);
            } else {
                groups.push(group_id.to_string());
            }
            save_collapsed_groups(&groups);
            collapsed_groups.set(groups);
        })
    };

    let ops_collapsed = collapsed_groups.iter().any(|g| g == "operaciones");
    let tools_collapsed = collapsed_groups.iter().any(|g| g == "herramientas");
    let sys_collapsed = collapsed_groups.iter().any(|g| g == "sistema");

    let ops_content_class = if ops_collapsed {
        "gi-sidebar-group-content"
    } else {
        "gi-sidebar-group-content gi-sidebar-group-content-open"
    };
    let tools_content_class = if tools_collapsed {
        "gi-sidebar-group-content"
    } else {
        "gi-sidebar-group-content gi-sidebar-group-content-open"
    };
    let sys_content_class = if sys_collapsed {
        "gi-sidebar-group-content"
    } else {
        "gi-sidebar-group-content gi-sidebar-group-content-open"
    };

    let ops_chevron = if ops_collapsed {
        "gi-group-chevron"
    } else {
        "gi-group-chevron gi-group-chevron-open"
    };
    let tools_chevron = if tools_collapsed {
        "gi-group-chevron"
    } else {
        "gi-group-chevron gi-group-chevron-open"
    };
    let sys_chevron = if sys_collapsed {
        "gi-group-chevron"
    } else {
        "gi-group-chevron gi-group-chevron-open"
    };

    let frecuentes_html = if frecuentes.is_empty() {
        html! {}
    } else {
        html! {
            <div class="gi-sidebar-group gi-sidebar-frecuentes">
                <div class="gi-sidebar-group-label">{"Frecuentes"}</div>
                <ul class="gi-sidebar-nav">
                    { for frecuentes.iter().map(|route| {
                        html! {
                            <li onclick={make_click(on_nav_click.clone())}>
                                <Link<Route> to={route.clone()}
                                    classes={classes!(link_class(route))}>
                                    {route_icon(route)}
                                    {route_label(route)}
                                </Link<Route>>
                            </li>
                        }
                    })}
                </ul>
                <div class="gi-sidebar-divider" />
            </div>
        }
    };

    html! {
        <aside class={format!("gi-sidebar w-64 min-h-screen flex flex-col{open_class}")}
               role="navigation" aria-label="Menú principal">
            <div class="gi-sidebar-brand">
                <div class="gi-logo-mark gi-logo-mark-sm">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <rect x="3" y="3" width="7" height="9" rx="1"/>
                        <rect x="14" y="3" width="7" height="5" rx="1"/>
                        <rect x="3" y="15" width="7" height="6" rx="1"/>
                        <rect x="14" y="11" width="7" height="10" rx="1"/>
                    </svg>
                </div>
                <div>
                    <span class="text-display gi-sidebar-brand-name">{"GI"}</span>
                    <p class="gi-sidebar-brand-sub">{"Rep. Dominicana"}</p>
                </div>
            </div>
            <nav class="flex-1 overflow-y-auto">
                {frecuentes_html}

                <div class="gi-sidebar-group">
                    <button class="gi-sidebar-group-toggle" onclick={make_toggle("operaciones")}
                        aria-expanded={(!ops_collapsed).to_string()}>
                        <span class="gi-sidebar-group-label">{"Operaciones"}</span>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                            stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class={ops_chevron}>
                            <polyline points="9 18 15 12 9 6"/>
                        </svg>
                    </button>
                    <div class={ops_content_class}>
                        <ul class="gi-sidebar-nav">
                            <li onclick={make_click(on_nav_click.clone())}>
                                <Link<Route> to={Route::Dashboard} classes={classes!(link_class(&Route::Dashboard))}>
                                    {icon_dashboard()}{"Panel de Control"}
                                </Link<Route>>
                            </li>
                            <li onclick={make_click(on_nav_click.clone())}>
                                <Link<Route> to={Route::Propiedades} classes={classes!(link_class(&Route::Propiedades))}>
                                    {icon_properties()}{"Propiedades"}
                                </Link<Route>>
                            </li>
                            <li onclick={make_click(on_nav_click.clone())}>
                                <Link<Route> to={Route::Inquilinos} classes={classes!(link_class(&Route::Inquilinos))}>
                                    {icon_tenants()}{"Inquilinos"}
                                </Link<Route>>
                            </li>
                            <li onclick={make_click(on_nav_click.clone())}>
                                <Link<Route> to={Route::Contratos} classes={classes!(link_class(&Route::Contratos))}>
                                    {icon_contracts()}{"Contratos"}
                                </Link<Route>>
                            </li>
                            <li onclick={make_click(on_nav_click.clone())}>
                                <Link<Route> to={Route::Pagos} classes={classes!(link_class(&Route::Pagos))}>
                                    {icon_payments()}{"Pagos"}
                                    {attention_badge(*pagos_atrasados)}
                                </Link<Route>>
                            </li>
                            <li onclick={make_click(on_nav_click.clone())}>
                                <Link<Route> to={Route::Gastos} classes={classes!(link_class(&Route::Gastos))}>
                                    {icon_expenses()}{"Gastos"}
                                </Link<Route>>
                            </li>
                            <li onclick={make_click(on_nav_click.clone())}>
                                <Link<Route> to={Route::Mantenimiento} classes={classes!(link_class(&Route::Mantenimiento))}>
                                    {icon_maintenance()}{"Mantenimiento"}
                                </Link<Route>>
                            </li>
                            <li onclick={make_click(on_nav_click.clone())} title="Proceso legal de desalojo — Seguimiento de procedimientos de desahucio">
                                <Link<Route> to={Route::Desahucios} classes={classes!(link_class(&Route::Desahucios))}>
                                    {icon_desahucios()}{"Desahucios"}
                                </Link<Route>>
                            </li>
                        </ul>
                    </div>
                </div>

                if can_write {
                    <div class="gi-sidebar-group">
                        <div class="gi-sidebar-divider" />
                        <button class="gi-sidebar-group-toggle" onclick={make_toggle("herramientas")}
                            aria-expanded={(!tools_collapsed).to_string()}>
                            <span class="gi-sidebar-group-label">{"Herramientas"}</span>
                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                                stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class={tools_chevron}>
                                <polyline points="9 18 15 12 9 6"/>
                            </svg>
                        </button>
                        <div class={tools_content_class}>
                            <ul class="gi-sidebar-nav">
                                <li onclick={make_click(on_nav_click.clone())}>
                                    <Link<Route> to={Route::Reportes} classes={classes!(link_class(&Route::Reportes))}>
                                        {icon_reports()}{"Reportes"}
                                    </Link<Route>>
                                </li>
                                <li onclick={make_click(on_nav_click.clone())}>
                                    <Link<Route> to={Route::Importar} classes={classes!(link_class(&Route::Importar))}>
                                        {icon_import()}{"Importar"}
                                    </Link<Route>>
                                </li>
                                <li onclick={make_click(on_nav_click.clone())}>
                                    <Link<Route> to={Route::Plantillas} classes={classes!(link_class(&Route::Plantillas))}>
                                        {icon_plantillas()}{"Plantillas"}
                                    </Link<Route>>
                                </li>
                                <li onclick={make_click(on_nav_click.clone())} title="Dirección General de Impuestos Internos — Reportes fiscales y formularios">
                                    <Link<Route> to={Route::Dgii} classes={classes!(link_class(&Route::Dgii))}>
                                        {icon_dgii()}{"DGII"}
                                    </Link<Route>>
                                </li>
                                <li onclick={make_click(on_nav_click.clone())} title="Agua, luz, gas — Control de responsabilidades de servicios por unidad">
                                    <Link<Route> to={Route::ServiciosPublicos} classes={classes!(link_class(&Route::ServiciosPublicos))}>
                                        {icon_servicios_publicos()}{"Servicios Públicos"}
                                    </Link<Route>>
                                </li>
                            </ul>
                        </div>
                    </div>
                }

                <div class="gi-sidebar-group">
                    <div class="gi-sidebar-divider" />
                    <button class="gi-sidebar-group-toggle" onclick={make_toggle("sistema")}
                        aria-expanded={(!sys_collapsed).to_string()}>
                        <span class="gi-sidebar-group-label">{"Sistema"}</span>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                            stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class={sys_chevron}>
                            <polyline points="9 18 15 12 9 6"/>
                        </svg>
                    </button>
                    <div class={sys_content_class}>
                        <ul class="gi-sidebar-nav">
                            if is_admin {
                                <li onclick={make_click(on_nav_click.clone())}>
                                    <Link<Route> to={Route::UsuariosPage} classes={classes!(link_class(&Route::UsuariosPage))}>
                                        {icon_users()}{"Usuarios"}
                                    </Link<Route>>
                                </li>
                                <li onclick={make_click(on_nav_click.clone())} title="Registro de cambios — Historial de todas las modificaciones del sistema">
                                    <Link<Route> to={Route::AuditoriaPage} classes={classes!(link_class(&Route::AuditoriaPage))}>
                                        {icon_audit()}{"Auditoría"}
                                    </Link<Route>>
                                </li>
                                <li onclick={make_click(on_nav_click.clone())} title="Números de Comprobantes Fiscales — Secuencias de facturación requeridas por la DGII">
                                    <Link<Route> to={Route::Ncf} classes={classes!(link_class(&Route::Ncf))}>
                                        {icon_ncf()}{"NCF"}
                                    </Link<Route>>
                                </li>
                                <li onclick={make_click(on_nav_click.clone())} title="Tareas programadas del sistema — Historial de ejecuciones automáticas">
                                    <Link<Route> to={Route::Tareas} classes={classes!(link_class(&Route::Tareas))}>
                                        {icon_tareas()}{"Tareas"}
                                    </Link<Route>>
                                </li>
                                <li onclick={make_click(on_nav_click.clone())} title="Invitar usuarios — Enviar accesos a nuevos miembros del equipo">
                                    <Link<Route> to={Route::Invitaciones} classes={classes!(link_class(&Route::Invitaciones))}>
                                        {icon_invitaciones()}{"Invitaciones"}
                                    </Link<Route>>
                                </li>
                                <li onclick={make_click(on_nav_click.clone())} title="Datos de la empresa — Información fiscal y de contacto de su organización">
                                    <Link<Route> to={Route::Organizacion} classes={classes!(link_class(&Route::Organizacion))}>
                                        {icon_organizacion()}{"Organización"}
                                    </Link<Route>>
                                </li>
                            }
                            <li onclick={make_click(on_nav_click.clone())}>
                                <Link<Route> to={Route::Configuracion} classes={classes!(link_class(&Route::Configuracion))}>
                                    {icon_settings()}{"Configuración"}
                                </Link<Route>>
                            </li>
                            <li onclick={make_click(on_nav_click.clone())}>
                                <Link<Route> to={Route::Perfil} classes={classes!(link_class(&Route::Perfil))}>
                                    {icon_profile()}{"Mi Perfil"}
                                </Link<Route>>
                            </li>
                        </ul>
                    </div>
                </div>
            </nav>
            <div class="gi-sidebar-footer">
                {"Gestión Inmobiliaria RD"}
            </div>
        </aside>
    }
}
