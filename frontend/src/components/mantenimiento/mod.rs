use yew::prelude::*;

pub fn prioridad_badge(prioridad: &str) -> Html {
    let (class, label) = match prioridad {
        "urgente" => ("gi-badge gi-badge-error", "Urgente"),
        "alta" => ("gi-badge gi-badge-warning", "Alta"),
        "media" => ("gi-badge gi-badge-neutral", "Media"),
        "baja" => ("gi-badge gi-badge-success", "Baja"),
        _ => ("gi-badge gi-badge-neutral", "Desconocida"),
    };
    html! { <span class={class}>{label}</span> }
}

pub fn estado_badge(estado: &str) -> Html {
    let (class, label) = match estado {
        "pendiente" => ("gi-badge gi-badge-warning", "Pendiente"),
        "en_progreso" => ("gi-badge gi-badge-info", "En Progreso"),
        "completado" => ("gi-badge gi-badge-success", "Completado"),
        _ => ("gi-badge gi-badge-neutral", "Desconocido"),
    };
    html! { <span class={class}>{label}</span> }
}
