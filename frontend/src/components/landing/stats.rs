use yew::prelude::*;

#[component]
pub fn LandingStats() -> Html {
    html! {
        <section
            class="px-4 py-4 border-y"
            style="border-color: var(--border-subtle); background-color: var(--surface-raised);"
        >
            <div class="max-w-4xl mx-auto flex flex-wrap items-center justify-center gap-6 text-sm"
                 style="color: var(--text-tertiary);">
                <div class="flex items-center gap-2">
                    <span>{"⭐"}</span>
                    <span>{"GitHub Stars"}</span>
                </div>
                <div class="flex items-center gap-2">
                    <span
                        class="w-2 h-2 rounded-full inline-block"
                        style="background-color: #22c55e;"
                    ></span>
                    <span>{"Proyecto en desarrollo activo"}</span>
                </div>
                <div class="flex items-center gap-2">
                    <span>{"🆓"}</span>
                    <span>{"100% gratis y código abierto"}</span>
                </div>
            </div>
        </section>
    }
}
