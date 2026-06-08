use yew::prelude::*;

#[component]
pub fn LandingFooter() -> Html {
    html! {
        <footer
            class="px-4 py-8 text-center"
            style="border-top: 1px solid var(--border-subtle); color: var(--text-tertiary);"
        >
            <p class="font-semibold" style="color: var(--text-secondary);">
                {"Gestión Inmobiliaria"}
            </p>
            <p class="text-sm mt-1">
                {"© 2025 — Proyecto de código abierto"}
            </p>
        </footer>
    }
}
