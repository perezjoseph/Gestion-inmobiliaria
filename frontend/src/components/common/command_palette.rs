use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthContext, Route};

#[derive(Clone, PartialEq)]
struct PaletteItem {
    label: &'static str,
    route: Route,
    icon: &'static str,
    is_action: bool,
}

fn all_items(can_write: bool) -> Vec<PaletteItem> {
    let mut items = vec![
        PaletteItem {
            label: "Panel de Control",
            route: Route::Dashboard,
            icon: "⌂",
            is_action: false,
        },
        PaletteItem {
            label: "Propiedades",
            route: Route::Propiedades,
            icon: "\u{1f3e0}",
            is_action: false,
        },
        PaletteItem {
            label: "Inquilinos",
            route: Route::Inquilinos,
            icon: "\u{1f465}",
            is_action: false,
        },
        PaletteItem {
            label: "Contratos",
            route: Route::Contratos,
            icon: "\u{1f4c4}",
            is_action: false,
        },
        PaletteItem {
            label: "Pagos",
            route: Route::Pagos,
            icon: "\u{1f4b0}",
            is_action: false,
        },
        PaletteItem {
            label: "Mantenimiento",
            route: Route::Mantenimiento,
            icon: "\u{1f527}",
            is_action: false,
        },
        PaletteItem {
            label: "Reportes",
            route: Route::Reportes,
            icon: "\u{1f4ca}",
            is_action: false,
        },
        PaletteItem {
            label: "Mi Perfil",
            route: Route::Perfil,
            icon: "\u{1f464}",
            is_action: false,
        },
    ];
    if can_write {
        items.push(PaletteItem {
            label: "Nueva Propiedad",
            route: Route::Propiedades,
            icon: "+",
            is_action: true,
        });
        items.push(PaletteItem {
            label: "Nuevo Pago",
            route: Route::Pagos,
            icon: "+",
            is_action: true,
        });
        items.push(PaletteItem {
            label: "Nuevo Contrato",
            route: Route::Contratos,
            icon: "+",
            is_action: true,
        });
    }
    items
}

fn filter_items(items: &[PaletteItem], query: &str) -> Vec<PaletteItem> {
    let q = query.to_lowercase();
    if q.is_empty() {
        return items.to_vec();
    }
    items
        .iter()
        .filter(|i| i.label.to_lowercase().contains(&q))
        .cloned()
        .collect()
}

#[function_component]
pub fn CommandPalette() -> Html {
    let open = use_state(|| false);
    let query = use_state(String::new);
    let selected = use_state(|| 0usize);
    let input_ref = use_node_ref();
    let navigator = use_navigator().expect("CommandPalette must be inside BrowserRouter");
    let auth = use_context::<AuthContext>();
    let user_can_write = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol == "admin" || u.rol == "gerente")
        .unwrap_or(false);

    let items = all_items(user_can_write);
    let filtered = filter_items(&items, &query);

    {
        let open = open.clone();
        use_effect_with((), move |_| {
            let listener = web_sys::window().and_then(|w| w.document()).map(|doc| {
                EventListener::new(&doc, "keydown", move |event| {
                    if let Some(ke) = event.dyn_ref::<web_sys::KeyboardEvent>()
                        && (ke.ctrl_key() || ke.meta_key())
                        && ke.key() == "k"
                    {
                        ke.prevent_default();
                        open.set(!*open);
                    }
                })
            });
            move || drop(listener)
        });
    }

    {
        let input_ref = input_ref.clone();
        let is_open = *open;
        use_effect_with(is_open, move |is_open| {
            if *is_open && let Some(el) = input_ref.cast::<web_sys::HtmlElement>() {
                let _ = el.focus();
            }
        });
    }

    let on_input = {
        let query = query.clone();
        let selected = selected.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            query.set(input.value());
            selected.set(0);
        })
    };

    let on_keydown = {
        let selected = selected.clone();
        let open = open.clone();
        let query = query.clone();
        let filtered = filtered.clone();
        let navigator = navigator.clone();
        Callback::from(move |e: KeyboardEvent| match e.key().as_str() {
            "ArrowDown" => {
                e.prevent_default();
                if !filtered.is_empty() {
                    selected.set((*selected + 1) % filtered.len());
                }
            }
            "ArrowUp" => {
                e.prevent_default();
                if !filtered.is_empty() {
                    selected.set(if *selected == 0 {
                        filtered.len() - 1
                    } else {
                        *selected - 1
                    });
                }
            }
            "Enter" => {
                if let Some(item) = filtered.get(*selected) {
                    navigator.push(&item.route);
                    open.set(false);
                    query.set(String::new());
                    selected.set(0);
                }
            }
            "Escape" => {
                open.set(false);
                query.set(String::new());
                selected.set(0);
            }
            _ => {}
        })
    };

    let on_overlay_click = {
        let open = open.clone();
        let query = query.clone();
        let selected = selected.clone();
        Callback::from(move |e: MouseEvent| {
            if let Some(target) = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let is_overlay = target
                    .get_attribute("class")
                    .map(|c| c.contains("gi-palette-overlay"))
                    .unwrap_or(false);
                if is_overlay {
                    open.set(false);
                    query.set(String::new());
                    selected.set(0);
                }
            }
        })
    };

    if !*open {
        return html! {};
    }

    html! {
        <div class="gi-palette-overlay" onclick={on_overlay_click}>
            <div class="gi-palette">
                <input
                    ref={input_ref}
                    type="text"
                    class="gi-palette-input"
                    placeholder="Buscar sección o acción..."
                    value={(*query).clone()}
                    oninput={on_input}
                    onkeydown={on_keydown}
                />
                <ul class="gi-palette-list">
                    { for filtered.iter().enumerate().map(|(i, item)| {
                        let is_selected = i == *selected;
                        let navigator = navigator.clone();
                        let route = item.route.clone();
                        let open = open.clone();
                        let query_state = query.clone();
                        let selected_state = selected.clone();
                        let on_click = Callback::from(move |_: MouseEvent| {
                            navigator.push(&route);
                            open.set(false);
                            query_state.set(String::new());
                            selected_state.set(0);
                        });
                        let item_class = if is_selected {
                            "gi-palette-item gi-palette-item-active"
                        } else {
                            "gi-palette-item"
                        };
                        let icon_class = if item.is_action {
                            "gi-palette-icon gi-palette-icon-action"
                        } else {
                            "gi-palette-icon"
                        };
                        html! {
                            <li class={item_class} onclick={on_click}>
                                <span class={icon_class}>{item.icon}</span>
                                <span>{item.label}</span>
                            </li>
                        }
                    })}
                    if filtered.is_empty() {
                        <li class="gi-palette-empty">{"Sin resultados"}</li>
                    }
                </ul>
                <PaletteFooter />
            </div>
        </div>
    }
}

#[function_component]
fn PaletteFooter() -> Html {
    html! {
        <div class="gi-palette-footer">
            <span>{"\u{2191}\u{2193} navegar"}</span>
            <span>{"\u{21b5} seleccionar"}</span>
            <span>{"esc cerrar"}</span>
        </div>
    }
}
