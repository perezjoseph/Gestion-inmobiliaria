use yew::prelude::*;

#[derive(Properties, PartialEq, Eq)]
pub struct FormErrorSummaryProps {
    pub errors: Vec<String>,
}

#[function_component]
pub fn FormErrorSummary(props: &FormErrorSummaryProps) -> Html {
    if props.errors.is_empty() {
        return html! {};
    }
    let count = props.errors.len();
    html! {
        <div class="gi-form-error-summary" role="alert" aria-live="assertive">
            <p class="gi-form-error-summary-title">
                {format!("Se encontraron {} {} en el formulario:", count, if count == 1 { "error" } else { "errores" })}
            </p>
            <ul class="gi-form-error-summary-list">
                { for props.errors.iter().map(|e| html! { <li>{e}</li> }) }
            </ul>
        </div>
    }
}
