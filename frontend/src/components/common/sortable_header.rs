use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SortableHeaderProps {
    pub label: String,
    pub field: String,
    pub current_sort: Option<String>,
    pub current_order: Option<String>,
    pub on_sort: Callback<(String, String)>,
}

#[function_component]
pub fn SortableHeader(props: &SortableHeaderProps) -> Html {
    let is_active = props
        .current_sort
        .as_ref()
        .is_some_and(|s| s == &props.field);

    let current_order = if is_active {
        props.current_order.as_deref().unwrap_or("asc").to_string()
    } else {
        String::new()
    };

    let onclick = {
        let field = props.field.clone();
        let current_order = current_order.clone();
        let cb = props.on_sort.clone();
        Callback::from(move |_: MouseEvent| {
            let new_order = if is_active && current_order == "asc" {
                "desc"
            } else {
                "asc"
            };
            cb.emit((field.clone(), new_order.to_string()));
        })
    };

    let indicator = if is_active {
        if current_order == "asc" {
            " ▲"
        } else {
            " ▼"
        }
    } else {
        ""
    };

    html! {
        <th
            onclick={onclick}
            style="cursor: pointer; user-select: none;"
            aria-sort={if is_active {
                Some(if current_order == "asc" { "ascending" } else { "descending" })
            } else {
                None
            }}
        >
            <span class="inline-flex items-center gap-1">
                {&props.label}
                <span style="font-size: 0.7em; opacity: 0.8;">{indicator}</span>
            </span>
        </th>
    }
}
