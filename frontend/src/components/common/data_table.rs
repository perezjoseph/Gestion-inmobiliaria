use yew::prelude::*;

use super::sortable_header::SortableHeader;

#[derive(Properties, PartialEq)]
pub struct DataTableProps {
    pub headers: Vec<String>,
    #[prop_or_default]
    pub children: Html,
    #[prop_or_default]
    pub sortable_fields: Vec<String>,
    #[prop_or_default]
    pub current_sort: Option<String>,
    #[prop_or_default]
    pub current_order: Option<String>,
    #[prop_or_default]
    pub on_sort: Option<Callback<(String, String)>>,
}

#[component]
pub fn DataTable(props: &DataTableProps) -> Html {
    let has_sort = props.on_sort.is_some() && !props.sortable_fields.is_empty();

    html! {
        <div class="gi-table-wrap" aria-live="polite" style="border-radius: 12px; border: 1px solid var(--border-subtle); overflow: hidden;">
            <table class="gi-table">
                <thead>
                    <tr>
                        { for props.headers.iter().enumerate().filter(|(_, h)| !h.is_empty()).map(|(i, header)| {
                            let field = props.sortable_fields.get(i).cloned().unwrap_or_default();
                            if has_sort && !field.is_empty() {
                                let Some(ref on_sort) = props.on_sort else {
                                    return html! { <th>{header}</th> };
                                };
                                html! {
                                    <SortableHeader
                                        label={header.clone()}
                                        field={field}
                                        current_sort={props.current_sort.clone()}
                                        current_order={props.current_order.clone()}
                                        on_sort={on_sort.clone()}
                                    />
                                }
                            } else {
                                html! { <th>{header}</th> }
                            }
                        })}
                    </tr>
                </thead>
                <tbody>
                    {props.children.clone()}
                </tbody>
            </table>
        </div>
    }
}
