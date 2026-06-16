use yew::prelude::*;

#[component]
pub fn LandingStats() -> Html {
    html! {
        <section class="gi-l-trust">
            <div class="gi-l-container gi-l-trust-row">
                <div class="gi-l-trust-item">
                    <span class="gi-l-dot gi-l-dot--ok"></span>
                    <span>{"En desarrollo activo"}</span>
                </div>
                <div class="gi-l-trust-item">
                    <span class="gi-l-dot gi-l-dot--accent"></span>
                    <span>{"Gratis, sin límites"}</span>
                </div>
                <div class="gi-l-trust-item">
                    <span class="gi-l-dot gi-l-dot--ok"></span>
                    <span>{"Hecho en RD"}</span>
                </div>
            </div>
        </section>
    }
}
