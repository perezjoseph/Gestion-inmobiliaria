use yew::prelude::*;

struct FeatureItem {
    icon: &'static str,
    title: &'static str,
    description: &'static str,
}

const FEATURES: &[FeatureItem] = &[
    FeatureItem {
        icon: "🏠",
        title: "Propiedades y Unidades",
        description: "Registra inmuebles, divide en unidades y controla el estado de cada uno.",
    },
    FeatureItem {
        icon: "👤",
        title: "Inquilinos y Contratos",
        description: "Gestiona inquilinos con sus contratos, fechas de vigencia y montos mensuales.",
    },
    FeatureItem {
        icon: "💰",
        title: "Pagos y Cobros",
        description: "Registra cobros en DOP o USD, identifica atrasos y genera recibos.",
    },
    FeatureItem {
        icon: "📊",
        title: "Gastos y Reportes",
        description: "Controla gastos por categoría y genera informes de ingresos y ocupación.",
    },
    FeatureItem {
        icon: "🔧",
        title: "Mantenimiento",
        description: "Solicitudes de reparación con seguimiento de estado y prioridad.",
    },
    FeatureItem {
        icon: "📈",
        title: "Dashboard en tiempo real",
        description: "Vista general de tu portafolio: ocupación, cobros pendientes y vencimientos.",
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
                <div class="gi-l-feature-grid">
                    { for FEATURES.iter().enumerate().map(|(i, f)| {
                        let class = if i == 0 {
                            "gi-l-feature gi-l-feature--wide"
                        } else {
                            "gi-l-feature"
                        };
                        html! {
                            <div class={class}>
                                <div class="gi-l-feature-icon">{ f.icon }</div>
                                <div class="gi-l-feature-text">
                                    <h3 class="gi-l-feature-title">{ f.title }</h3>
                                    <p class="gi-l-feature-desc">{ f.description }</p>
                                </div>
                            </div>
                        }
                    })}
                </div>
            </div>
        </section>
    }
}
