use yew::prelude::*;

struct FeatureBlock {
    number: &'static str,
    title: &'static str,
    points: &'static [&'static str],
}

const BLOCKS: &[FeatureBlock] = &[
    FeatureBlock {
        number: "01",
        title: "Propiedades, inquilinos y contratos",
        points: &[
            "Registra inmuebles con unidades, precios y estado.",
            "Asocia inquilinos con cédula y datos de contacto.",
            "Contratos con fechas, montos y renovación automática.",
        ],
    },
    FeatureBlock {
        number: "02",
        title: "Pagos, gastos y reportes",
        points: &[
            "Cobros en DOP o USD con seguimiento de atrasos.",
            "Gastos por categoría vinculados a cada propiedad.",
            "Reportes de ocupación, ingresos y cumplimiento fiscal.",
        ],
    },
    FeatureBlock {
        number: "03",
        title: "Mantenimiento y operaciones",
        points: &[
            "Solicitudes de reparación con prioridad y seguimiento.",
            "Dashboard en tiempo real con alertas de vencimiento.",
            "Documentos, plantillas y comprobantes fiscales (NCF).",
        ],
    },
];

#[component]
pub fn LandingFeatures() -> Html {
    html! {
        <section class="gi-l-features">
            <div class="gi-l-container">
                <div class="gi-l-section-head">
                    <h2 class="gi-l-section-title">{"Todo lo que necesitas para administrar"}</h2>
                </div>
                <div class="gi-l-feature-blocks">
                    { for BLOCKS.iter().map(|block| html! {
                        <div class="gi-l-feature-block">
                            <span class="gi-l-feature-block-num">{ block.number }</span>
                            <div class="gi-l-feature-block-content">
                                <h3 class="gi-l-feature-block-title">{ block.title }</h3>
                                <ul class="gi-l-feature-block-list">
                                    { for block.points.iter().map(|point| html! {
                                        <li class="gi-l-feature-block-item">{ *point }</li>
                                    })}
                                </ul>
                            </div>
                        </div>
                    })}
                </div>
            </div>
        </section>
    }
}
