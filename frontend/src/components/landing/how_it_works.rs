use yew::prelude::*;

struct Step {
    title: &'static str,
    description: &'static str,
}

const STEPS: &[Step] = &[
    Step {
        title: "Registra tus propiedades",
        description: "Añade inmuebles, unidades, y precios. La estructura se arma sola.",
    },
    Step {
        title: "Conecta inquilinos",
        description: "Crea contratos con fechas, montos, y asocia cada inquilino a su espacio.",
    },
    Step {
        title: "Controla todo",
        description: "Pagos, gastos, mantenimiento, reportes. Todo fluye desde un solo panel.",
    },
];

#[component]
pub fn LandingHowItWorks() -> Html {
    html! {
        <section class="gi-l-how">
            <div class="gi-l-container">
                <h2 class="gi-l-section-title">{"Tres pasos para empezar"}</h2>
                <div class="gi-l-steps">
                    { for STEPS.iter().enumerate().map(|(i, step)| html! {
                        <div class={classes!("gi-l-step", (i < STEPS.len() - 1).then_some("gi-l-step--connected"))}>
                            <div class="gi-l-step-marker"></div>
                            <h3 class="gi-l-step-title">{ step.title }</h3>
                            <p class="gi-l-step-desc">{ step.description }</p>
                        </div>
                    })}
                </div>
            </div>
        </section>
    }
}
