use yew::prelude::*;

#[component]
pub fn LandingTransparency() -> Html {
    html! {
        <section class="gi-l-os">
            <div class="gi-l-container-narrow gi-l-os-inner">
                <h2 class="gi-l-section-title">{"Código abierto, hecho con cariño"}</h2>
                <p class="gi-l-os-text">
                    {"Este proyecto es libre. Lo construimos porque nos pareció útil. No hay letra pequeña, no hay planes de pago escondidos. Si te sirve, úsalo. Si quieres contribuir, bienvenido."}
                </p>
                <a
                    href="https://github.com/perezjoseph/Gestion-inmobiliaria"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="gi-l-btn gi-l-btn-secondary"
                >
                    {"Ver en GitHub"}
                </a>
            </div>
        </section>
    }
}
