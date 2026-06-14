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
        description: "AÃ±ade tus inmuebles con direcciÃ³n, unidades y precio. Todo organizado desde el inicio.",
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
        <section class="gi-l-how">
            <div class="gi-l-container">
                <div class="gi-l-section-head">
                    <h2 class="gi-l-section-title">{"CÃ³mo funciona"}</h2>
                </div>
                <div class="gi-l-steps">
                    { for STEPS.iter().map(|step| html! {
                        <div class="gi-l-step">
                            <span class="gi-l-step-num">{ step.number }</span>
                            <h3 class="gi-l-step-title">{ step.title }</h3>
                            <p class="gi-l-step-desc">{ step.description }</p>
                        </div>
                    })}
                </div>
            </div>
        </section>
    }
}
