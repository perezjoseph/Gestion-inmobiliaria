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
        <section class="px-4 py-12 max-w-6xl mx-auto">
            <h2
                class="text-2xl md:text-3xl font-bold text-center mb-8"
                style="font-family: var(--font-display); color: var(--text-primary);"
            >
                {"Todo lo que necesitas para administrar"}
            </h2>
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                { for FEATURES.iter().map(|f| html! {
                    <div
                        class="p-5 rounded-lg"
                        style="background-color: var(--surface-raised); border: 1px solid var(--border-subtle);"
                    >
                        <div class="text-2xl mb-2">{ f.icon }</div>
                        <h3
                            class="font-semibold mb-1"
                            style="color: var(--text-primary);"
                        >
                            { f.title }
                        </h3>
                        <p class="text-sm" style="color: var(--text-secondary);">
                            { f.description }
                        </p>
                    </div>
                })}
            </div>
        </section>
    }
}
