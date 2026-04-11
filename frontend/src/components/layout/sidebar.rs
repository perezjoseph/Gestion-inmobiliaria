use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthContext, Route};

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

#[function_component]
pub fn Sidebar(props: &SidebarProps) -> Html {
    let current_route = use_route::<Route>();
    let on_nav_click = props.on_nav_click.clone();
    let auth = use_context::<AuthContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.as_str())
        .unwrap_or("");
    let is_admin = user_rol == "admin";
    let can_write = user_rol == "admin" || user_rol == "gerente";

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
            <div style="padding: var(--space-3) var(--space-4); margin-bottom: var(--space-5); display: flex; align-items: center; gap: var(--space-3);">
                <div class="gi-logo-mark gi-logo-mark-sm">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M3 21V9l9-7 9 7v12"/>
                        <path d="M9 21V12h6v9"/>
                    </svg>
                </div>
                <div>
                    <span class="text-display" style="font-size: var(--text-base); font-weight: 700; color: var(--text-inverse); line-height: 1.2;">
                        {"GI"}
                    </span>
                    <p style="font-size: 0.65rem; color: var(--text-tertiary); line-height: 1.2;">
                        {"Rep. Dominicana"}
                    </p>
                </div>
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
                    <li onclick={make_click(on_nav_click.clone())}>
                        <Link<Route> to={Route::Reportes}
                            classes={classes!(link_class(&Route::Reportes))}>
                            {icon_reports()}
                            {"Reportes"}
                        </Link<Route>>
                    </li>
                    if can_write {
                        <li onclick={make_click(on_nav_click.clone())}>
                            <Link<Route> to={Route::Importar}
                                classes={classes!(link_class(&Route::Importar))}>
                                {icon_import()}
                                {"Importar"}
                            </Link<Route>>
                        </li>
                    }
                    if is_admin {
                        <li onclick={make_click(on_nav_click.clone())}>
                            <Link<Route> to={Route::UsuariosPage}
                                classes={classes!(link_class(&Route::UsuariosPage))}>
                                {icon_users()}
                                {"Usuarios"}
                            </Link<Route>>
                        </li>
                        <li onclick={make_click(on_nav_click.clone())}>
                            <Link<Route> to={Route::AuditoriaPage}
                                classes={classes!(link_class(&Route::AuditoriaPage))}>
                                {icon_audit()}
                                {"Auditoría"}
                            </Link<Route>>
                        </li>
                    }
                    <li onclick={make_click(on_nav_click.clone())}>
                        <Link<Route> to={Route::Perfil}
                            classes={classes!(link_class(&Route::Perfil))}>
                            {icon_profile()}
                            {"Mi Perfil"}
                        </Link<Route>>
                    </li>
                </ul>
            </nav>
            <div style="padding: var(--space-3) var(--space-4); font-size: var(--text-xs); color: var(--text-tertiary);">
                {"Gestión Inmobiliaria RD"}
            </div>
        </aside>
    }
}
