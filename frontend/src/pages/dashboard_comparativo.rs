use yew::prelude::*;

/// Página de Dashboard Comparativo
/// Compara el rendimiento financiero entre múltiples propiedades.
#[component]
pub fn DashboardComparativo() -> Html {
    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Dashboard Comparativo"}</h1>
                <p class="gi-page-subtitle">
                    {"Compare el rendimiento financiero entre sus propiedades."}
                </p>
            </div>
            <div class="gi-empty-state">
                <div class="gi-empty-state-icon">{"📊"}</div>
                <div class="gi-empty-state-title">{"Comparación de Propiedades"}</div>
                <p class="gi-empty-state-text">
                    {"Visualice ingresos, gastos, rentabilidad y ocupación por propiedad con filtros de fecha y tipo."}
                </p>
            </div>
        </div>
    }
}
