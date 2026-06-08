use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use yew::prelude::*;

/// Slide-over drawer component.
///
/// On desktop (>768px): slides in from the right, 420px wide, with a dimmed backdrop.
/// On mobile (≤768px): slides up from the bottom as a full-width sheet.
///
/// Closes on: Escape key, backdrop click, or explicit `on_close` callback.
#[derive(Properties, PartialEq)]
pub struct DrawerProps {
    /// Whether the drawer is currently open.
    pub open: bool,
    /// Callback to close the drawer.
    pub on_close: Callback<()>,
    /// Title shown in the drawer header.
    #[prop_or_default]
    pub title: AttrValue,
    /// Content rendered inside the drawer body.
    #[prop_or_default]
    pub children: Html,
}

#[component]
pub fn Drawer(props: &DrawerProps) -> Html {
    let on_close = props.on_close.clone();

    // Escape key listener
    {
        let on_close = on_close.clone();
        let open = props.open;
        use_effect_with(open, move |is_open| {
            if !is_open {
                return;
            }
            let listener = web_sys::window().and_then(|w| w.document()).map(|doc| {
                let on_close = on_close.clone();
                EventListener::new(&doc, "keydown", move |event| {
                    if let Some(e) = event.dyn_ref::<web_sys::KeyboardEvent>() {
                        if e.key() == "Escape" {
                            on_close.emit(());
                        }
                    }
                })
            });
            // Keep listener alive until cleanup
            drop(listener);
        });
    }

    // Backdrop click
    let on_backdrop_click = {
        let on_close = on_close.clone();
        Callback::from(move |_: MouseEvent| {
            on_close.emit(());
        })
    };

    // Close button
    let on_close_btn = {
        let on_close = on_close.clone();
        Callback::from(move |_: MouseEvent| {
            on_close.emit(());
        })
    };

    let overlay_class = if props.open {
        "gi-drawer-overlay gi-drawer-overlay-open"
    } else {
        "gi-drawer-overlay"
    };

    let panel_class = if props.open {
        "gi-drawer-panel gi-drawer-panel-open"
    } else {
        "gi-drawer-panel"
    };

    html! {
        <div class={overlay_class} aria-hidden={(!props.open).to_string()}>
            <div class="gi-drawer-backdrop" onclick={on_backdrop_click}></div>
            <aside class={panel_class} role="dialog" aria-modal="true"
                aria-label={props.title.clone()}>
                <div class="gi-drawer-header">
                    <h2 class="gi-drawer-title">{&props.title}</h2>
                    <button class="gi-drawer-close" onclick={on_close_btn}
                        aria-label="Cerrar" type="button">
                        <svg width="20" height="20" viewBox="0 0 24 24" fill="none"
                            stroke="currentColor" stroke-width="2" stroke-linecap="round"
                            stroke-linejoin="round">
                            <line x1="18" y1="6" x2="6" y2="18"/>
                            <line x1="6" y1="6" x2="18" y2="18"/>
                        </svg>
                    </button>
                </div>
                <div class="gi-drawer-body">
                    {props.children.clone()}
                </div>
            </aside>
        </div>
    }
}
