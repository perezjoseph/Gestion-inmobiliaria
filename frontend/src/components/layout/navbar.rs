use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthAction, AuthContext, Route, ThemeContext};

#[derive(Properties, PartialEq)]
pub struct NavbarProps {
    pub user_name: String,
    pub user_role: String,
    pub on_toggle_sidebar: Callback<MouseEvent>,
}

#[function_component]
pub fn Navbar(props: &NavbarProps) -> Html {
    let auth = use_context::<AuthContext>().unwrap();
    let navigator = use_navigator().unwrap();
    let theme = use_context::<ThemeContext>().unwrap();

    let on_logout = {
        let auth = auth.clone();
        let navigator = navigator.clone();
        Callback::from(move |_: MouseEvent| {
            auth.dispatch(AuthAction::Logout);
            navigator.push(&Route::Login);
        })
    };

    let on_toggle_theme = {
        let theme = theme.clone();
        Callback::from(move |_: MouseEvent| {
            let new_dark = !*theme;
            theme.set(new_dark);
            if let Some(window) = web_sys::window() {
                if let Some(doc) = window.document()
                    && let Some(el) = doc.document_element()
                {
                    let _ = el.set_attribute("data-theme", if new_dark { "dark" } else { "light" });
                }
                if let Ok(Some(storage)) = window.local_storage() {
                    let _ = storage.set_item("theme", if new_dark { "dark" } else { "light" });
                }
            }
        })
    };

    let role_label = match props.user_role.as_str() {
        "admin" => "Administrador",
        "gerente" => "Gerente",
        "visualizador" => "Visualizador",
        _ => &props.user_role,
    };

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
                <button class="gi-theme-toggle" onclick={on_toggle_theme}
                    aria-label={if *theme { "Cambiar a modo claro" } else { "Cambiar a modo oscuro" }}
                    title={if *theme { "Modo claro" } else { "Modo oscuro" }}>
                </button>
                <div class="gi-navbar-user" style="display: flex; align-items: center; gap: var(--space-3);">
                    <div class="gi-navbar-user-info" style="text-align: right;">
                        <div class="gi-navbar-user-name" style="font-size: var(--text-sm); font-weight: 600; color: var(--text-primary);">
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
            </div>
        </nav>
    }
}
