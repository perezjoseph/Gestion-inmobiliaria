use web_sys::HtmlSelectElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PaginationProps {
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
    pub on_page_change: Callback<u64>,
    pub on_per_page_change: Callback<u64>,
}

#[function_component]
pub fn Pagination(props: &PaginationProps) -> Html {
    let total = props.total;
    let page = props.page;
    let per_page = props.per_page;

    if total == 0 {
        return html! {};
    }

    let total_pages = total.div_ceil(per_page);
    let start = (page - 1) * per_page + 1;
    let end = (page * per_page).min(total);

    let on_prev = {
        let cb = props.on_page_change.clone();
        Callback::from(move |_: MouseEvent| {
            if page > 1 {
                cb.emit(page - 1);
            }
        })
    };

    let on_next = {
        let cb = props.on_page_change.clone();
        Callback::from(move |_: MouseEvent| {
            if page < total_pages {
                cb.emit(page + 1);
            }
        })
    };

    let on_per_page = {
        let cb = props.on_per_page_change.clone();
        Callback::from(move |e: Event| {
            let target: HtmlSelectElement = e.target_unchecked_into();
            if let Ok(val) = target.value().parse::<u64>() {
                cb.emit(val);
            }
        })
    };

    let page_numbers = build_page_numbers(page, total_pages);

    html! {
        <div class="flex flex-wrap items-center justify-between gap-4 py-3 px-1"
             style="font-size: var(--text-sm); color: var(--text-secondary);">
            <span>
                {format!("Mostrando {start}–{end} de {total}")}
            </span>

            <div class="flex items-center gap-2">
                <button
                    onclick={on_prev}
                    disabled={page <= 1}
                    class="gi-btn gi-btn-sm gi-btn-ghost"
                    aria-label="Página anterior"
                >
                    {"← Anterior"}
                </button>

                { for page_numbers.iter().map(|&pn| {
                    if pn == 0 {
                        html! { <span class="px-1">{"…"}</span> }
                    } else {
                        let cb = props.on_page_change.clone();
                        let is_current = pn == page;
                        let onclick = Callback::from(move |_: MouseEvent| cb.emit(pn));
                        html! {
                            <button
                                onclick={onclick}
                                disabled={is_current}
                                class={classes!(
                                    "gi-btn", "gi-btn-sm",
                                    if is_current { "gi-btn-primary" } else { "gi-btn-ghost" }
                                )}
                                aria-label={format!("Página {pn}")}
                                aria-current={if is_current { Some("page") } else { None }}
                            >
                                {pn}
                            </button>
                        }
                    }
                })}

                <button
                    onclick={on_next}
                    disabled={page >= total_pages}
                    class="gi-btn gi-btn-sm gi-btn-ghost"
                    aria-label="Página siguiente"
                >
                    {"Siguiente →"}
                </button>
            </div>

            <div class="flex items-center gap-2">
                <label for="per-page-select">{"Por página:"}</label>
                <select
                    id="per-page-select"
                    onchange={on_per_page}
                    class="gi-input"
                    style="width: auto; padding: var(--space-1) var(--space-2);"
                >
                    { for [10u64, 20, 50].iter().map(|&size| {
                        html! {
                            <option value={size.to_string()} selected={size == per_page}>
                                {size}
                            </option>
                        }
                    })}
                </select>
            </div>
        </div>
    }
}

fn build_page_numbers(current: u64, total: u64) -> Vec<u64> {
    if total <= 7 {
        return (1..=total).collect();
    }

    let mut pages = Vec::with_capacity(9);
    pages.push(1);

    if current > 3 {
        pages.push(0);
    }

    let range_start = current.saturating_sub(1).max(2);
    let range_end = (current + 1).min(total - 1);

    for p in range_start..=range_end {
        pages.push(p);
    }

    if current < total - 2 {
        pages.push(0);
    }

    pages.push(total);
    pages
}
