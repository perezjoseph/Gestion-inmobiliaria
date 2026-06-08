use yew::prelude::*;

#[component]
pub fn LandingTransparency() -> Html {
    html! {
        <section
            class="px-4 py-12 text-center"
            style="background-color: var(--surface-raised);"
        >
            <div class="max-w-3xl mx-auto">
                <h2
                    class="text-2xl md:text-3xl font-bold mb-4"
                    style="font-family: var(--font-display); color: var(--text-primary);"
                >
                    {"Código abierto, hecho con cariño"}
                </h2>
                <p
                    class="text-lg mb-6"
                    style="font-family: var(--font-body); color: var(--text-secondary);"
                >
                    {"Este proyecto es de código abierto — lo construimos porque nos pareció útil y divertido. No hay letra pequeña, ni planes de pago escondidos. Si te sirve, úsalo."}
                </p>
                <a
                    href="https://github.com/jpilier/Gestion-inmobiliaria"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg font-semibold"
                    style="border: 1px solid var(--border-default); color: var(--text-primary);"
                >
                    {"Ver código en GitHub"}
                </a>
            </div>
        </section>
    }
}
