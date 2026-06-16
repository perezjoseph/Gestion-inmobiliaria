use yew::prelude::*;

struct FeatureBlock {
    title: &'static str,
    description: &'static str,
}

const BLOCKS: &[FeatureBlock] = &[
    FeatureBlock {
        title: "Propiedades y unidades",
        description: "Registra inmuebles con dirección, unidades individuales, precios en DOP o USD, y estado de ocupación. Todo visible de un vistazo.",
    },
    FeatureBlock {
        title: "Contratos e inquilinos",
        description: "Asocia inquilinos con cédula verificada. Contratos con fechas claras, montos, y renovación controlada. Sin sorpresas.",
    },
    FeatureBlock {
        title: "Pagos y cobros",
        description: "Seguimiento de cada pago: pendiente, al día, o atrasado. Alertas automáticas cuando algo se vence.",
    },
    FeatureBlock {
        title: "Gastos y mantenimiento",
        description: "Registra gastos por propiedad o unidad. Solicitudes de reparación con prioridad, notas, y costos asociados.",
    },
    FeatureBlock {
        title: "Reportes y comprobantes",
        description: "Ocupación, ingresos mensuales, cumplimiento fiscal. Genera comprobantes NCF para la DGII directamente.",
    },
];

#[component]
pub fn LandingFeatures() -> Html {
    html! {
        <section class="gi-l-features">
            <div class="gi-l-container">
                <h2 class="gi-l-section-title">{"Todo lo que necesitas"}</h2>
                <p class="gi-l-section-sub">{"Un sistema completo para administradores que manejan varias propiedades."}</p>
                <div class="gi-l-feature-list">
                    { for BLOCKS.iter().map(|block| html! {
                        <div class="gi-l-feature-item">
                            <h3 class="gi-l-feature-item-title">{ block.title }</h3>
                            <p class="gi-l-feature-item-desc">{ block.description }</p>
                        </div>
                    })}
                </div>
            </div>
        </section>
    }
}
