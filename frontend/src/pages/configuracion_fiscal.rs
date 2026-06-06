use yew::prelude::*;

/// Página de Configuración Fiscal
/// Permite configurar tipo fiscal, RNC/cédula y rangos NCF de la organización.
#[component]
pub fn ConfiguracionFiscal() -> Html {
    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Configuración Fiscal"}</h1>
                <p class="gi-page-subtitle">
                    {"Administre el tipo fiscal, RNC/cédula y rangos de NCF de su organización."}
                </p>
            </div>
            <div class="gi-empty-state">
                <div class="gi-empty-state-icon">{"🏛️"}</div>
                <div class="gi-empty-state-title">{"Configuración Fiscal"}</div>
                <p class="gi-empty-state-text">
                    {"Configure su tipo fiscal (Persona Jurídica, Persona Física o Informal) y los datos asociados."}
                </p>
            </div>
        </div>
    }
}
