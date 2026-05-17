use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct BulkActionBarProps {
    pub selected_count: usize,
    pub on_clear: Callback<MouseEvent>,
    #[prop_or_default]
    pub children: Html,
}

/// A floating action bar that appears when items are selected in a table.
/// Pass action buttons as children.
#[component]
pub fn BulkActionBar(props: &BulkActionBarProps) -> Html {
    if props.selected_count == 0 {
        return html! {};
    }

    html! {
        <div class="gi-bulk-bar" role="toolbar" aria-label="Acciones en lote">
            <span class="gi-bulk-bar-count">
                {format!("{} seleccionado{}", props.selected_count, if props.selected_count > 1 { "s" } else { "" })}
            </span>
            <div class="gi-bulk-bar-actions">
                {props.children.clone()}
            </div>
            <button
                onclick={props.on_clear.clone()}
                class="gi-btn gi-btn-ghost gi-btn-sm"
                aria-label="Deseleccionar todo"
            >
                {"✕ Limpiar"}
            </button>
        </div>
    }
}
