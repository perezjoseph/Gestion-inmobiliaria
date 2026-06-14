use yew::prelude::*;

#[derive(Properties, PartialEq, Eq)]
pub struct CondominiosProps {
    pub propiedad_id: String,
}

#[component]
pub fn Condominios(props: &CondominiosProps) -> Html {
    let _propiedad_id = props.propiedad_id.clone();

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Cuotas de Condominio"}</h1>
                <p class="gi-page-subtitle">
                    {"Administre las cuotas de condominio para esta propiedad."}
                </p>
            </div>
            <div class="gi-empty-state">
                <div class="gi-empty-state-icon">{"ðŸ¢"}</div>
                <div class="gi-empty-state-title">{"Cuotas de Condominio"}</div>
                <p class="gi-empty-state-text">
                    {"Agregue y gestione las cuotas de condominio, configure el pass-through a inquilinos."}
                </p>
            </div>
        </div>
    }
}
