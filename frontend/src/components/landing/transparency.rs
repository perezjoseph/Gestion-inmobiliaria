use yew::prelude::*;

#[component]
pub fn LandingTransparency() -> Html {
    html! {
        <section class="gi-l-os">
            <div class="gi-l-container-narrow gi-l-os-inner">
                <p class="gi-l-os-eyebrow">{"Desde Santo Domingo para el mundo"}</p>
                <h2 class="gi-l-section-title">{"Código abierto, hecho con cariño"}</h2>
                <p class="gi-l-os-text">
                    {"Este proyecto es de código abierto. Lo construimos porque nos pareció útil y divertido. No hay letra pequeña ni planes de pago escondidos. Si te sirve, úsalo."}
                </p>
                <a
                    href="https://github.com/perezjoseph/Gestion-inmobiliaria"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="gi-l-btn gi-l-btn-secondary"
                >
                    {"Ver código en GitHub"}
                </a>
            </div>
        </section>
    }
}
