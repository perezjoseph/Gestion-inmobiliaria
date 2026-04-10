use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct DataTableProps {
    pub headers: Vec<String>,
    #[prop_or_default]
    pub children: Html,
}

#[function_component]
pub fn DataTable(props: &DataTableProps) -> Html {
    html! {
        <div class="gi-table-wrap" style="border-radius: 12px; border: 1px solid var(--border-subtle); overflow: hidden;">
            <table class="gi-table">
                <thead>
                    <tr>
                        { for props.headers.iter().filter(|h| !h.is_empty()).map(|header| html! {
                            <th>{header}</th>
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
