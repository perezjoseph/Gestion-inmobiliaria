use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::currency_display::CurrencyDisplay;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::TableSkeleton;
use crate::services::api::api_get;
use crate::types::deserialize_f64_from_any;

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoriaResumen {
    pub categoria: String,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub total: f64,
    pub count: u64,
    #[serde(default)]
    pub moneda: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortColumn {
    Categoria,
    Total,
    Count,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortDir {
    Asc,
    Desc,
}

fn categoria_label(cat: &str) -> &str {
    match cat {
        "mantenimiento" => "Mantenimiento",
        "servicios" => "Servicios",
        "impuestos" => "Impuestos",
        "seguro" => "Seguro",
        "administracion" => "Administración",
        "otros" => "Otros",
        other => other,
    }
}

#[component]
pub fn CategoriasGastos() -> Html {
    let data = use_state(Vec::<CategoriaResumen>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let sort_col = use_state(|| SortColumn::Total);
    let sort_dir = use_state(|| SortDir::Desc);

    {
        let data = data.clone();
        let loading = loading.clone();
        let error = error.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                match api_get::<Vec<CategoriaResumen>>("/gastos/resumen-categorias").await {
                    Ok(rows) => data.set(rows),
                    Err(e) => error.set(Some(e)),
                }
                loading.set(false);
            });
        });
    }

    let sorted_data = {
        let mut rows = (*data).clone();
        let col = *sort_col;
        let dir = *sort_dir;
        rows.sort_by(|a, b| {
            let cmp = match col {
                SortColumn::Categoria => {
                    categoria_label(&a.categoria).cmp(categoria_label(&b.categoria))
                }
                SortColumn::Total => a
                    .total
                    .partial_cmp(&b.total)
                    .unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::Count => a.count.cmp(&b.count),
            };
            if dir == SortDir::Desc {
                cmp.reverse()
            } else {
                cmp
            }
        });
        rows
    };

    let make_sort_handler = |col: SortColumn| {
        let sort_col = sort_col.clone();
        let sort_dir = sort_dir.clone();
        Callback::from(move |_: MouseEvent| {
            if *sort_col == col {
                sort_dir.set(if *sort_dir == SortDir::Asc {
                    SortDir::Desc
                } else {
                    SortDir::Asc
                });
            } else {
                sort_col.set(col);
                sort_dir.set(SortDir::Desc);
            }
        })
    };

    let sort_indicator = |col: SortColumn| -> &'static str {
        if *sort_col == col {
            if *sort_dir == SortDir::Asc {
                " ▲"
            } else {
                " ▼"
            }
        } else {
            ""
        }
    };

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Resumen por Categoría de Gastos"}</h1>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            if *loading {
                <TableSkeleton />
            } else if sorted_data.is_empty() {
                <p style="text-align: center; color: var(--text-tertiary); padding: var(--space-6);">
                    {"No hay gastos registrados."}
                </p>
            } else {
                <div class="gi-card" style="padding: var(--space-5); margin-top: var(--space-4);">
                    <div style="overflow-x: auto;">
                        <table class="gi-table" style="width: 100%;">
                            <thead>
                                <tr>
                                    <th style="padding: var(--space-3) var(--space-4); cursor: pointer;"
                                        onclick={make_sort_handler(SortColumn::Categoria)}>
                                        {format!("Categoría{}", sort_indicator(SortColumn::Categoria))}
                                    </th>
                                    <th style="padding: var(--space-3) var(--space-4); cursor: pointer; text-align: right;"
                                        onclick={make_sort_handler(SortColumn::Total)}>
                                        {format!("Total{}", sort_indicator(SortColumn::Total))}
                                    </th>
                                    <th style="padding: var(--space-3) var(--space-4); cursor: pointer; text-align: right;"
                                        onclick={make_sort_handler(SortColumn::Count)}>
                                        {format!("Cantidad{}", sort_indicator(SortColumn::Count))}
                                    </th>
                                </tr>
                            </thead>
                            <tbody>
                                { for sorted_data.iter().map(|row| {
                                    let moneda = row.moneda.clone().unwrap_or_else(|| "DOP".into());
                                    html! {
                                        <tr>
                                            <td style="padding: var(--space-3) var(--space-4); font-size: var(--text-sm);">
                                                {categoria_label(&row.categoria)}
                                            </td>
                                            <td class="tabular-nums" style="padding: var(--space-3) var(--space-4); font-size: var(--text-sm); text-align: right;">
                                                <CurrencyDisplay monto={row.total} moneda={moneda} />
                                            </td>
                                            <td class="tabular-nums" style="padding: var(--space-3) var(--space-4); font-size: var(--text-sm); text-align: right;">
                                                {row.count}
                                            </td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>
                    </div>
                </div>
            }
        </div>
    }
}
