use yew::prelude::*;

#[component]
pub fn Footer() -> Html {
    html! {
        <footer class="gi-footer">
            <div style="display: flex; justify-content: center; align-items: center; gap: var(--space-4); flex-wrap: wrap;">
                <span>{"© 2026 Gestión Inmobiliaria RD"}</span>
                <span style="color: var(--border-default);">{"·"}</span>
                <span style="color: var(--text-tertiary); font-size: var(--text-xs);">
                    {"Presione "}
                    <kbd style="padding: 1px 4px; border: 1px solid var(--border-default); border-radius: 3px; font-size: 0.65rem; font-family: var(--font-body);">{"Esc"}</kbd>
                    {" para cerrar formularios"}
                </span>
            </div>
        </footer>
    }
}
