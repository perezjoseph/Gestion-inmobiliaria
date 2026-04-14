pub fn can_write(rol: &str) -> bool {
    rol == "admin" || rol == "gerente"
}

pub fn can_delete(rol: &str) -> bool {
    rol == "admin"
}

pub fn format_date_display(iso: &str) -> String {
    if iso.len() >= 10 {
        let parts: Vec<&str> = iso[..10].split('-').collect();
        if parts.len() == 3 {
            return format!("{}/{}/{}", parts[2], parts[1], parts[0]);
        }
    }
    iso.to_string()
}

pub fn format_currency(moneda: &str, monto: f64) -> String {
    if moneda == "DOP" {
        format!("RD$ {monto:.2}")
    } else {
        format!("US$ {monto:.2}")
    }
}

pub fn input_class(has_error: bool) -> &'static str {
    if has_error {
        "gi-input gi-input-error"
    } else {
        "gi-input"
    }
}

pub fn field_error(error: &Option<String>) -> yew::Html {
    match error {
        Some(msg) => yew::html! { <p class="gi-field-error">{msg}</p> },
        None => yew::html! {},
    }
}

pub type EscapeHandler = std::rc::Rc<std::cell::RefCell<Option<Box<dyn Fn()>>>>;
