use yew::prelude::*;

#[component]
pub fn LandingFooter() -> Html {
    html! {
        <footer class="gi-l-footer">
            <p class="gi-l-footer-brand">{"Gestión Inmobiliaria"}</p>
            <p class="gi-l-footer-meta">{"© 2026 · Proyecto de código abierto"}</p>
        </footer>
    }
}
