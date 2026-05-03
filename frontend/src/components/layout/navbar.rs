use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthAction, AuthContext, Route, ThemeContext};

use super::notification_bell::NotificationBell;

#[component]
fn NavbarSearch() -> Html {
    let Some(navigator) = use_navigator() else {
        return html! {};
    };
    let search_query = use_state(String::new);
    let search_open = use_state(|| false);

    let on_toggle_search = {
        let search_open = search_open.clone();
        let search_query = search_query.clone();
        Callback::from(move |_: MouseEvent| {
            let opening = !*search_open;
            search_open.set(opening);
            if !opening {
                search_query.set(String::new());
            }
        })
    };

    let on_search_input = {
        let search_query = search_query.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            search_query.set(input.value());
        })
    };

    let on_search_keydown = {
        let search_query = search_query.clone();
        let search_open = search_open.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                let q = (*search_query).trim().to_lowercase();
                if !q.is_empty() {
                    let route = resolve_search_route(&q);
                    navigator.push(&route);
                    search_query.set(String::new());
                    search_open.set(false);
                }
            } else if e.key() == "Escape" {
                search_query.set(String::new());
                search_open.set(false);
            }
        })
    };

    html! {
        <>
            if *search_open {
                <input type="text"
                    class="gi-input gi-navbar-search"
                    placeholder="Ir a sección... (propiedades, pagos, contratos)"
                    value={(*search_query).clone()}
                    oninput={on_search_input}
                    onkeydown={on_search_keydown}
                    style="width: 260px; font-size: var(--text-sm);"
                />
            }
            <button class="gi-btn gi-btn-ghost gi-navbar-search-btn" onclick={on_toggle_search}
                aria-label="Ir a sección" title="Ir a sección"
                style="padding: var(--space-2); border: none; display: flex; align-items: center; gap: var(--space-1);">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                    <polyline points="9 18 15 12 9 6"/>
                </svg>
                <span style="font-size: var(--text-xs); font-weight: 500;">{"Ir a…"}</span>
            </button>
            <kbd class="gi-kbd" title="Abrir paleta de comandos">{"\u{2318}K"}</kbd>
        </>
    }
}

fn resolve_search_route(q: &str) -> Route {
    if q.contains("pago") || q.contains("cobr") {
        Route::Pagos
    } else if q.contains("contrat") {
        Route::Contratos
    } else if q.contains("inquilin") || q.contains("arrendat") {
        Route::Inquilinos
    } else {
        Route::Propiedades
    }
}

fn apply_theme(is_dark: bool) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let theme_str = if is_dark { "dark" } else { "light" };
    if let Some(el) = window.document().and_then(|d| d.document_element()) {
        let _ = el.set_attribute("data-theme", theme_str);
    }
    if let Ok(Some(storage)) = window.local_storage() {
        let _ = storage.set_item("theme", theme_str);
    }
}

#[component]
fn ThemeToggle() -> Html {
    let Some(theme) = use_context::<ThemeContext>() else {
        return html! {};
    };

    let on_toggle_theme = {
        let theme = theme.clone();
        Callback::from(move |_: MouseEvent| {
            let new_dark = !*theme;
            theme.set(new_dark);
            apply_theme(new_dark);
        })
    };

    html! {
        <button class="gi-theme-toggle" onclick={on_toggle_theme}
            aria-label={if *theme { "Cambiar a modo claro" } else { "Cambiar a modo oscuro" }}
            title={if *theme { "Modo claro" } else { "Modo oscuro" }}>
        </button>
    }
}

#[derive(Properties, PartialEq)]
struct UserMenuProps {
    user_name: String,
    user_role: String,
}

#[component]
fn UserMenu(props: &UserMenuProps) -> Html {
    let Some(auth) = use_context::<AuthContext>() else {
        return html! {};
    };
    let Some(navigator) = use_navigator() else {
        return html! {};
    };

    let on_logout = {
        Callback::from(move |_: MouseEvent| {
            auth.dispatch(AuthAction::Logout);
            navigator.push(&Route::Login);
        })
    };

    let role_label = match props.user_role.as_str() {
        "admin" => "Administrador",
        "gerente" => "Gerente",
        "visualizador" => "Visualizador",
        _ => &props.user_role,
    };

    html! {
        <div class="gi-navbar-user" style="display: flex; align-items: center; gap: var(--space-3);">
            <div class="gi-navbar-user-info" style="text-align: right;">
                <div class="gi-navbar-user-name gi-text-sm" style="font-weight: 600; color: var(--text-primary);">
                    {&props.user_name}
                </div>
                <div class="gi-badge gi-badge-info" style="font-size: 0.65rem;">
                    {role_label}
                </div>
            </div>
            <button onclick={on_logout} class="gi-btn gi-btn-ghost gi-navbar-logout"
                style="font-size: var(--text-xs);">
                {"Cerrar sesión"}
            </button>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct NavbarProps {
    pub user_name: String,
    pub user_role: String,
    pub on_toggle_sidebar: Callback<MouseEvent>,
}

#[component]
pub fn Navbar(props: &NavbarProps) -> Html {
    html! {
        <nav class="gi-navbar" style="padding: var(--space-3) var(--space-5); display: flex; justify-content: space-between; align-items: center; gap: var(--space-2);">
            <div style="display: flex; align-items: center; gap: var(--space-3); min-width: 0;">
                <button class="gi-hamburger" onclick={props.on_toggle_sidebar.clone()}
                    aria-label="Abrir menú">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                        <line x1="3" y1="6" x2="21" y2="6"/>
                        <line x1="3" y1="12" x2="21" y2="12"/>
                        <line x1="3" y1="18" x2="21" y2="18"/>
                    </svg>
                </button>
                <span class="gi-navbar-title text-display" style="font-size: var(--text-lg); font-weight: 600; color: var(--text-primary);">
                    {"Gestión Inmobiliaria"}
                </span>
            </div>
            <div style="display: flex; align-items: center; gap: var(--space-3); flex-shrink: 0;">
                <NavbarSearch />
                <NotificationBell />
                <ThemeToggle />
                <UserMenu user_name={props.user_name.clone()} user_role={props.user_role.clone()} />
            </div>
        </nav>
    }
}
