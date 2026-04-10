use yew::prelude::*;

#[derive(Properties, PartialEq, Eq)]
pub struct VerificationBadgeProps {
    pub estado: AttrValue,
}

fn badge_class(estado: &str) -> &'static str {
    match estado {
        "verificado" => "gi-badge gi-badge-verificado",
        "pendiente" => "gi-badge gi-badge-pendiente",
        "rechazado" => "gi-badge gi-badge-rechazado",
        "vencido" => "gi-badge gi-badge-vencido",
        "faltante" => "gi-badge gi-badge-faltante",
        _ => "gi-badge",
    }
}

fn badge_label(estado: &str) -> &str {
    match estado {
        "verificado" => "Verificado",
        "pendiente" => "Pendiente",
        "rechazado" => "Rechazado",
        "vencido" => "Vencido",
        "faltante" => "Faltante",
        other => other,
    }
}

#[component]
pub fn VerificationBadge(props: &VerificationBadgeProps) -> Html {
    let estado = props.estado.as_str();
    let class = badge_class(estado);
    let label = badge_label(estado);

    html! {
        <span class={class} aria-label={format!("Estado: {label}")}>
            {label}
        </span>
    }
}
