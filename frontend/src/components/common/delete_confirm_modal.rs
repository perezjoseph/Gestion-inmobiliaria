use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct DeleteConfirmModalProps {
    pub message: String,
    pub on_confirm: Callback<MouseEvent>,
    pub on_cancel: Callback<MouseEvent>,
    #[prop_or_default]
    pub confirm_label: Option<String>,
}

#[component]
pub fn DeleteConfirmModal(props: &DeleteConfirmModalProps) -> Html {
    let cancel_ref = use_node_ref();

    {
        let cancel_ref = cancel_ref.clone();
        use_effect_with((), move |()| {
            if let Some(el) = cancel_ref.cast::<web_sys::HtmlElement>() {
                let _ = el.focus();
            }
        });
    }

    {
        let on_cancel = props.on_cancel.clone();
        use_effect_with((), move |()| {
            let listener = web_sys::window().and_then(|w| w.document()).map(|doc| {
                EventListener::new(&doc, "keydown", move |event| {
                    if let Some(ke) = event.dyn_ref::<web_sys::KeyboardEvent>()
                        && ke.key() == "Escape"
                        && let Ok(synthetic) = MouseEvent::new("click")
                    {
                        on_cancel.emit(synthetic);
                    }
                })
            });
            move || drop(listener)
        });
    }

    let on_overlay_click = {
        let on_cancel = props.on_cancel.clone();
        Callback::from(move |e: MouseEvent| {
            if let Some(target) = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
                && target
                    .get_attribute("class")
                    .is_some_and(|c| c.contains("gi-modal-overlay"))
            {
                on_cancel.emit(e);
            }
        })
    };

    html! {
        <div class="gi-modal-overlay"
            role="dialog"
            aria-modal="true"
            aria-labelledby="gi-modal-title"
            onclick={on_overlay_click}
        >
            <div class="gi-modal">
                <h3 id="gi-modal-title" class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-2); color: var(--text-primary);">
                    {"Confirmar eliminación"}</h3>
                <p style="font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-5);">
                    {&props.message}</p>
                <div style="display: flex; justify-content: flex-end; gap: var(--space-2);">
                    <button ref={cancel_ref} onclick={props.on_cancel.clone()} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                    <button onclick={props.on_confirm.clone()} class="gi-btn gi-btn-danger">{props.confirm_label.as_deref().unwrap_or("Eliminar")}</button>
                </div>
            </div>
        </div>
    }
}
