use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthContext, Route};
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::pages::chatbot_config::ChatbotConfig;
use crate::services::api::{api_get, api_put};
use crate::types::configuracion::{RecargoDefectoConfig, UpdateRecargoDefectoRequest};
use wasm_bindgen_futures::spawn_local;

// ---------------------------------------------------------------------------
// Tab definitions
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConfigTab {
    General,
    Chatbot,
}

// ---------------------------------------------------------------------------
// ConfigTabNav — horizontal tab navigation
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct ConfigTabNavProps {
    pub active: ConfigTab,
    pub on_tab: Callback<ConfigTab>,
    pub can_write: bool,
}

#[component]
fn ConfigTabNav(props: &ConfigTabNavProps) -> Html {
    let tab_btn = |tab: ConfigTab, label: &'static str, icon: Html| {
        let is_active = props.active == tab;
        let on_click = {
            let on_tab = props.on_tab.clone();
            Callback::from(move |_: MouseEvent| on_tab.emit(tab))
        };

        let class = if is_active {
            "gi-config-tab gi-config-tab-active"
        } else {
            "gi-config-tab"
        };

        html! {
            <button
                type="button"
                role="tab"
                aria-selected={is_active.to_string()}
                class={class}
                onclick={on_click}
            >
                {icon}
                <span>{label}</span>
            </button>
        }
    };

    let general_icon = html! {
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3"/>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
        </svg>
    };

    let chatbot_icon = html! {
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
        </svg>
    };

    html! {
        <div role="tablist" aria-label="Secciones de configuración" class="gi-config-tabs">
            {tab_btn(ConfigTab::General, "General", general_icon)}
            if props.can_write {
                {tab_btn(ConfigTab::Chatbot, "Chatbot WhatsApp", chatbot_icon)}
            }
        </div>
    }
}

// ---------------------------------------------------------------------------
// RecargoSection — late fee configuration
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct RecargoSectionProps {
    pub is_admin: bool,
}

#[component]
fn RecargoSection(props: &RecargoSectionProps) -> Html {
    let toasts = use_context::<ToastContext>();
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let current_porcentaje = use_state(|| Option::<f64>::None);
    let input_value = use_state(String::new);
    let saving = use_state(|| false);

    {
        let loading = loading.clone();
        let error = error.clone();
        let current_porcentaje = current_porcentaje.clone();
        let input_value = input_value.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                match api_get::<RecargoDefectoConfig>("/configuracion/recargo").await {
                    Ok(config) => {
                        if let Some(p) = config.porcentaje {
                            input_value.set(format!("{p:.2}"));
                        }
                        current_porcentaje.set(config.porcentaje);
                    }
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        });
    }

    let on_save = {
        let input_value = input_value.clone();
        let error = error.clone();
        let current_porcentaje = current_porcentaje.clone();
        let saving = saving.clone();
        let toasts = toasts;
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let val = (*input_value).trim().to_string();
            let Ok(parsed) = val.parse::<f64>() else {
                error.set(Some("Ingrese un número válido".into()));
                return;
            };
            if !(0.0..=100.0).contains(&parsed) {
                error.set(Some("El porcentaje debe estar entre 0 y 100".into()));
                return;
            }
            let body = UpdateRecargoDefectoRequest { porcentaje: parsed };
            let error = error.clone();
            let current_porcentaje = current_porcentaje.clone();
            let saving = saving.clone();
            let toasts = toasts.clone();
            saving.set(true);
            spawn_local(async move {
                match api_put::<serde_json::Value, _>("/configuracion/recargo", &body).await {
                    Ok(_) => {
                        current_porcentaje.set(Some(parsed));
                        error.set(None);
                        if let Some(t) = &toasts {
                            t.dispatch(ToastAction::Push(
                                "Recargo por mora actualizado".into(),
                                ToastKind::Success,
                            ));
                        }
                    }
                    Err(err) => error.set(Some(err)),
                }
                saving.set(false);
            });
        })
    };

    let on_input = {
        let input_value = input_value.clone();
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlInputElement = e.target_unchecked_into();
            input_value.set(el.value());
        })
    };

    if *loading {
        return html! {
            <div class="gi-card" style="padding: var(--space-5);">
                <div class="gi-spinner" style="margin: var(--space-4) auto;"></div>
            </div>
        };
    }

    let display_value =
        current_porcentaje.map_or_else(|| "No configurado".to_string(), |p| format!("{p:.2}%"));

    html! {
        <div class="gi-card" style="padding: var(--space-5);">
            <h2 style="font-size: var(--text-base); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-4);">
                {"Recargo por Mora"}
            </h2>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <div style="margin-bottom: var(--space-4);">
                <span style="font-size: var(--text-sm); color: var(--text-secondary);">
                    {"Porcentaje actual: "}
                </span>
                <span style="font-size: var(--text-sm); font-weight: 600; color: var(--text-primary);">
                    {display_value}
                </span>
            </div>

            <form onsubmit={on_save} style="display: flex; flex-direction: column; gap: var(--space-3);">
                <div>
                    <label class="gi-label">{"Porcentaje de Recargo (%)"}</label>
                    <input
                        type="number"
                        step="0.01"
                        min="0"
                        max="100"
                        placeholder="Ej: 5.00"
                        value={(*input_value).clone()}
                        oninput={on_input}
                        class="gi-input"
                        disabled={!props.is_admin}
                        style={if props.is_admin { "" } else { "opacity: 0.6;" }}
                    />
                    if !props.is_admin {
                        <p style="font-size: var(--text-xs); color: var(--text-tertiary); margin-top: var(--space-1);">
                            {"Solo los administradores pueden modificar este valor."}
                        </p>
                    }
                </div>
                if props.is_admin {
                    <div style="display: flex; justify-content: flex-end;">
                        <button type="submit" class="gi-btn gi-btn-primary" disabled={*saving}>
                            {if *saving { "Guardando..." } else { "Guardar" }}
                        </button>
                    </div>
                }
            </form>
        </div>
    }
}

// ---------------------------------------------------------------------------
// GeneralTab — general settings content
// ---------------------------------------------------------------------------

#[component]
fn GeneralTab() -> Html {
    let auth = use_context::<AuthContext>();
    let is_admin = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .is_some_and(|u| u.rol == "admin");

    html! {
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr)); gap: var(--space-5);">
            <RecargoSection is_admin={is_admin} />
        </div>
    }
}

// ---------------------------------------------------------------------------
// Configuracion — main page with tabs
// ---------------------------------------------------------------------------

#[component]
pub fn Configuracion() -> Html {
    let route = use_route::<Route>();
    let navigator = use_navigator();
    let auth = use_context::<AuthContext>();

    let can_write = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .is_some_and(|u| u.rol == "admin" || u.rol == "gerente");

    // Determine active tab from route
    let active_tab = if route.as_ref() == Some(&Route::ConfiguracionChatbot) {
        ConfigTab::Chatbot
    } else {
        ConfigTab::General
    };

    let on_tab = {
        let navigator = navigator;
        Callback::from(move |tab: ConfigTab| {
            if let Some(nav) = &navigator {
                match tab {
                    ConfigTab::General => nav.push(&Route::Configuracion),
                    ConfigTab::Chatbot => nav.push(&Route::ConfiguracionChatbot),
                }
            }
        })
    };

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Configuración"}</h1>
            </div>

            <ConfigTabNav active={active_tab} on_tab={on_tab} can_write={can_write} />

            <div class="gi-config-content">
                {match active_tab {
                    ConfigTab::General => html! { <GeneralTab /> },
                    ConfigTab::Chatbot => html! { <ChatbotConfig /> },
                }}
            </div>
        </div>
    }
}
