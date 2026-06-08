use yew::prelude::*;

#[component]
pub fn LandingPreview() -> Html {
    html! {
        <section class="px-4 py-12 max-w-5xl mx-auto">
            <h2
                class="text-2xl md:text-3xl font-bold text-center mb-6"
                style="font-family: var(--font-display); color: var(--text-primary);"
            >
                {"Así se ve por dentro"}
            </h2>
            <div
                class="rounded-xl overflow-hidden mb-6"
                style="box-shadow: var(--shadow-lg); border: 1px solid var(--border-subtle);"
            >
                <img
                    src="/assets/dashboard-preview.gif"
                    alt="Demo animada del dashboard de Gestión Inmobiliaria"
                    class="w-full h-auto"
                    loading="lazy"
                />
            </div>
            <div class="text-center">
                <a
                    href="https://demo.gestion-inmobiliaria.com"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg font-semibold"
                    style="border: 1px solid var(--border-default); color: var(--text-primary);"
                >
                    {"Ver demo en vivo"}
                </a>
            </div>
        </section>
    }
}
