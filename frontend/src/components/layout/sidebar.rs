use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;

#[derive(Properties, PartialEq)]
pub struct SidebarProps {
    pub is_open: bool,
    pub on_nav_click: Callback<()>,
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

#[function_component]
pub fn Sidebar(props: &SidebarProps) -> Html {
    let current_route = use_route::<Route>();
    let on_nav_click = props.on_nav_click.clone();

    let link_class = |route: &Route| -> String {
        let is_active = current_route.as_ref() == Some(route);
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

    html! {
        <aside class={format!("gi-sidebar w-64 min-h-screen flex flex-col{open_class}")}
               style="padding: var(--space-4);" role="navigation" aria-label="Menú principal">
            <div style="padding: var(--space-3) var(--space-4); margin-bottom: var(--space-5);">
                <span class="text-display" style="font-size: var(--text-lg); font-weight: 700; color: var(--text-inverse);">
                    {"Inmobiliaria"}
                </span>
                <p style="font-size: var(--text-xs); color: oklch(65% 0.01 185); margin-top: var(--space-1);">
                    {"República Dominicana"}
                </p>
            </div>
            <nav class="flex-1">
                <ul style="display: flex; flex-direction: column; gap: var(--space-1); list-style: none; padding: 0; margin: 0;">
                    <li onclick={make_click(on_nav_click.clone())}>
                        <Link<Route> to={Route::Dashboard}
                            classes={classes!(link_class(&Route::Dashboard))}>
                            {icon_dashboard()}
                            {"Panel de Control"}
                        </Link<Route>>
                    </li>
                    <li onclick={make_click(on_nav_click.clone())}>
                        <Link<Route> to={Route::Propiedades}
                            classes={classes!(link_class(&Route::Propiedades))}>
                            {icon_properties()}
                            {"Propiedades"}
                        </Link<Route>>
                    </li>
                    <li onclick={make_click(on_nav_click.clone())}>
                        <Link<Route> to={Route::Inquilinos}
                            classes={classes!(link_class(&Route::Inquilinos))}>
                            {icon_tenants()}
                            {"Inquilinos"}
                        </Link<Route>>
                    </li>
                    <li onclick={make_click(on_nav_click.clone())}>
                        <Link<Route> to={Route::Contratos}
                            classes={classes!(link_class(&Route::Contratos))}>
                            {icon_contracts()}
                            {"Contratos"}
                        </Link<Route>>
                    </li>
                    <li onclick={make_click(on_nav_click.clone())}>
                        <Link<Route> to={Route::Pagos}
                            classes={classes!(link_class(&Route::Pagos))}>
                            {icon_payments()}
                            {"Pagos"}
                        </Link<Route>>
                    </li>
                </ul>
            </nav>
            <div style="padding: var(--space-3) var(--space-4); font-size: var(--text-xs); color: oklch(55% 0.01 185);">
                {"v1.0 — Gestión Inmobiliaria"}
            </div>
        </aside>
    }
}
