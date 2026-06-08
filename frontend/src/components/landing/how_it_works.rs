use yew::prelude::*;

struct Step {
    number: &'static str,
    title: &'static str,
    description: &'static str,
}

const STEPS: &[Step] = &[
    Step {
        number: "1",
        title: "Registra tus propiedades",
        description: "Añade tus inmuebles con dirección, unidades y precio. Todo organizado desde el inicio.",
    },
    Step {
        number: "2",
        title: "Organiza inquilinos y contratos",
        description: "Asocia inquilinos a tus propiedades con contratos claros: fechas, montos y estado.",
    },
    Step {
        number: "3",
        title: "Controla pagos y gastos",
        description: "Registra cobros, da seguimiento a pagos atrasados y lleva el control de cada gasto.",
    },
];

#[component]
pub fn LandingHowItWorks() -> Html {
    html! {
        <section class="px-4 py-12 max-w-5xl mx-auto">
            <h2
                class="text-2xl md:text-3xl font-bold text-center mb-10"
                style="font-family: var(--font-display); color: var(--text-primary);"
            >
                {"Cómo funciona"}
            </h2>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-8">
                { for STEPS.iter().map(|step| html! {
                    <div class="text-center">
                        <div
                            class="w-12 h-12 rounded-full flex items-center justify-center text-xl font-bold mx-auto mb-4 text-white"
                            style="background-color: #3d8b8b;"
                        >
                            { step.number }
                        </div>
                        <h3
                            class="text-lg font-semibold mb-2"
                            style="color: var(--text-primary);"
                        >
                            { step.title }
                        </h3>
                        <p class="text-sm" style="color: var(--text-secondary);">
                            { step.description }
                        </p>
                    </div>
                })}
            </div>
        </section>
    }
}
