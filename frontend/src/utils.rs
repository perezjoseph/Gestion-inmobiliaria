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

/// Convert ISO (YYYY-MM-DD) to display (DD/MM/YYYY) for form inputs.
pub fn iso_to_display(iso: &str) -> String {
    if iso.len() >= 10 {
        let parts: Vec<&str> = iso[..10].split('-').collect();
        if parts.len() == 3 {
            return format!("{}/{}/{}", parts[2], parts[1], parts[0]);
        }
    }
    iso.to_string()
}

/// Convert display (DD/MM/YYYY) to ISO (YYYY-MM-DD) for API submission.
/// Returns the input as-is if it doesn't match the expected format.
pub fn display_to_iso(display: &str) -> String {
    let parts: Vec<&str> = display.split('/').collect();
    if parts.len() == 3 && parts[0].len() == 2 && parts[1].len() == 2 && parts[2].len() == 4 {
        return format!("{}-{}-{}", parts[2], parts[1], parts[0]);
    }
    display.to_string()
}

pub fn format_currency(moneda: &str, monto: f64) -> String {
    if moneda == "DOP" {
        format!("RD$ {monto:.2}")
    } else {
        format!("US$ {monto:.2}")
    }
}

pub const fn input_class(has_error: bool) -> &'static str {
    if has_error {
        "gi-input gi-input-error"
    } else {
        "gi-input"
    }
}

#[allow(clippy::ref_option)]
pub fn field_error(error: &Option<String>) -> yew::Html {
    error.as_ref().map_or_else(
        || yew::html! {},
        |msg| yew::html! { <p class="gi-field-error">{msg}</p> },
    )
}

pub type EscapeHandler = std::rc::Rc<std::cell::RefCell<Option<Box<dyn Fn()>>>>;
