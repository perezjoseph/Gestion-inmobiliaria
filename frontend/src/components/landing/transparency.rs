use yew::prelude::*;

#[component]
pub fn LandingTransparency() -> Html {
    html! {
        <section class="gi-l-os">
            <div class="gi-l-container-narrow gi-l-os-inner">
                <p class="gi-l-os-eyebrow">{"Desde Santo Domingo para el mundo"}</p>
                <h2 class="gi-l-section-title">{"CÃ³digo abierto, hecho con cariÃ±o"}</h2>
                <p class="gi-l-os-text">
                    {"Este proyecto es de cÃ³digo abierto. Lo construimos porque nos pareciÃ³ Ãºtil y divertido. No hay letra pequeÃ±a ni planes de pago escondidos. Si te sirve, Ãºsalo."}
                </p>
                <a
                    href="https://github.com/perezjoseph/Gestion-inmobiliaria"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="gi-l-btn gi-l-btn-secondary"
                >
                    {"Ver cÃ³digo en GitHub"}
                </a>
            </div>
        </section>
    }
}
