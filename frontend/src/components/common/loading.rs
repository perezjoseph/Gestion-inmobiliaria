use yew::prelude::*;

#[component]
pub fn Loading() -> Html {
    html! {
        <div style="display: flex; flex-direction: column; justify-content: center; align-items: center; padding: var(--space-8); gap: var(--space-3);">
            <div class="gi-spinner"></div>
            <span style="font-size: var(--text-sm); color: var(--text-tertiary);">{"Cargando..."}</span>
        </div>
    }
}
