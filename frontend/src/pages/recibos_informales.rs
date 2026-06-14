use yew::prelude::*;

#[component]
pub fn RecibosInformales() -> Html {
    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Recibos Informales"}</h1>
                <p class="gi-page-subtitle">
                    {"Registre pagos parciales y en efectivo sin comprobante fiscal."}
                </p>
            </div>
            <div class="gi-empty-state">
                <div class="gi-empty-state-icon">{"ðŸ§¾"}</div>
                <div class="gi-empty-state-title">{"Recibos Informales"}</div>
                <p class="gi-empty-state-text">
                    {"AquÃ puede registrar pagos parciales y generar recibos internos para organizaciones informales."}
                </p>
            </div>
        </div>
    }
}
