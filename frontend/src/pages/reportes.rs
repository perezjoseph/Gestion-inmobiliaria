use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::ReportSkeleton;
use crate::services::api::{api_download, api_get};
use crate::types::reporte::{IngresoReportSummary, RentabilidadReportSummary};

#[derive(Clone, Copy, PartialEq)]
enum ReportTab {
    Ingresos,
    Rentabilidad,
}

#[component]
pub fn Reportes() -> Html {
    let active_tab = use_state(|| ReportTab::Ingresos);

    let on_tab_ingresos = {
        let active_tab = active_tab.clone();
        Callback::from(move |_: MouseEvent| active_tab.set(ReportTab::Ingresos))
    };

    let on_tab_rentabilidad = {
        let active_tab = active_tab.clone();
        Callback::from(move |_: MouseEvent| active_tab.set(ReportTab::Rentabilidad))
    };

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Reportes"}</h1>
            </div>

            <div class="gi-tab-bar" style="display: flex; gap: var(--space-1); margin-bottom: var(--space-4); border-bottom: 1px solid var(--border-primary);">
                <button
                    class={classes!("gi-tab-btn", (*active_tab == ReportTab::Ingresos).then_some("gi-tab-btn-active"))}
                    onclick={on_tab_ingresos}
                    style="padding: var(--space-2) var(--space-4); border: none; background: none; cursor: pointer; font-weight: 500; border-bottom: 2px solid transparent;"
                >
                    {"Ingresos"}
                </button>
                <button
                    class={classes!("gi-tab-btn", (*active_tab == ReportTab::Rentabilidad).then_some("gi-tab-btn-active"))}
                    onclick={on_tab_rentabilidad}
                    style="padding: var(--space-2) var(--space-4); border: none; background: none; cursor: pointer; font-weight: 500; border-bottom: 2px solid transparent;"
                >
                    {"Rentabilidad"}
                </button>
            </div>

            {match *active_tab {
                ReportTab::Ingresos => html! { <IngresosTab /> },
                ReportTab::Rentabilidad => html! { <RentabilidadTab /> },
            }}
        </div>
    }
}

#[component]
fn IngresosTab() -> Html {
    let mes = use_state(|| {
        let d = js_sys::Date::new_0();
        d.get_month() + 1
    });
    let anio = use_state(|| {
        let d = js_sys::Date::new_0();
        #[allow(clippy::cast_possible_wrap)]
        {
            d.get_full_year() as i32
        }
    });
    let report = use_state(|| Option::<IngresoReportSummary>::None);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);

    let on_generate = {
        let mes = mes.clone();
        let anio = anio.clone();
        let report = report.clone();
        let error = error.clone();
        let loading = loading.clone();
        Callback::from(move |_: MouseEvent| {
            let m = *mes;
            let a = *anio;
            let report = report.clone();
            let error = error.clone();
            let loading = loading.clone();
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                let url = format!("/reportes/ingresos?mes={m}&anio={a}");
                match api_get::<IngresoReportSummary>(&url).await {
                    Ok(data) => report.set(Some(data)),
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        })
    };

    let on_export_pdf = {
        let mes = mes.clone();
        let anio = anio.clone();
        let error = error.clone();
        Callback::from(move |_: MouseEvent| {
            let m = *mes;
            let a = *anio;
            let error = error.clone();
            spawn_local(async move {
                let path = format!("/reportes/ingresos/pdf?mes={m}&anio={a}");
                if let Err(e) = api_download(&path, "reporte-ingresos.pdf").await {
                    error.set(Some(e));
                }
            });
        })
    };

    let on_export_xlsx = {
        let mes = mes.clone();
        let anio = anio.clone();
        let error = error.clone();
        Callback::from(move |_: MouseEvent| {
            let m = *mes;
            let a = *anio;
            let error = error.clone();
            spawn_local(async move {
                let path = format!("/reportes/ingresos/xlsx?mes={m}&anio={a}");
                if let Err(e) = api_download(&path, "reporte-ingresos.xlsx").await {
                    error.set(Some(e));
                }
            });
        })
    };

    let on_mes_change = {
        let mes = mes.clone();
        Callback::from(move |e: Event| {
            let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
            if let Ok(v) = el.value().parse::<u32>() {
                mes.set(v);
            }
        })
    };

    let on_anio_change = {
        let anio = anio.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().parse::<i32>() {
                anio.set(v);
            }
        })
    };

    html! {
        <div>
            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <IngresosFilterBar
                mes={*mes}
                anio={*anio}
                loading={*loading}
                on_mes_change={on_mes_change}
                on_anio_change={on_anio_change}
                on_generate={on_generate}
            />

            if *loading {
                <ReportSkeleton />
            }

            if let Some(ref r) = *report {
                <IngresosResult report={r.clone()} on_export_pdf={on_export_pdf} on_export_xlsx={on_export_xlsx} />
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct IngresosFilterBarProps {
    mes: u32,
    anio: i32,
    loading: bool,
    on_mes_change: Callback<Event>,
    on_anio_change: Callback<InputEvent>,
    on_generate: Callback<MouseEvent>,
}

#[component]
fn IngresosFilterBar(props: &IngresosFilterBarProps) -> Html {
    let meses = [
        (1, "Enero"),
        (2, "Febrero"),
        (3, "Marzo"),
        (4, "Abril"),
        (5, "Mayo"),
        (6, "Junio"),
        (7, "Julio"),
        (8, "Agosto"),
        (9, "Septiembre"),
        (10, "Octubre"),
        (11, "Noviembre"),
        (12, "Diciembre"),
    ];

    html! {
        <div class="gi-filter-bar" style="margin-bottom: var(--space-4);">
            <div style="display: flex; gap: var(--space-3); align-items: end; flex-wrap: wrap;">
                <div>
                    <label class="gi-label">{"Mes"}</label>
                    <select onchange={props.on_mes_change.clone()} class="gi-input">
                        { for meses.iter().map(|(val, label)| {
                            html! { <option value={val.to_string()} selected={props.mes == *val}>{*label}</option> }
                        })}
                    </select>
                </div>
                <div>
                    <label class="gi-label">{"Año"}</label>
                    <input
                        type="number"
                        class="gi-input"
                        value={props.anio.to_string()}
                        oninput={props.on_anio_change.clone()}
                        min="2020"
                        max="2100"
                        style="width: 100px;"
                    />
                </div>
                <div>
                    <button
                        onclick={props.on_generate.clone()}
                        class="gi-btn gi-btn-primary"
                        disabled={props.loading}
                    >
                        {if props.loading { "Generando..." } else { "Generar Reporte" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq, Clone)]
struct IngresosResultProps {
    report: IngresoReportSummary,
    on_export_pdf: Callback<MouseEvent>,
    on_export_xlsx: Callback<MouseEvent>,
}

#[component]
fn IngresosResult(props: &IngresosResultProps) -> Html {
    let r = &props.report;
    html! {
        <div>
            <div style="display: flex; gap: var(--space-3); margin-bottom: var(--space-4);">
                <button onclick={props.on_export_pdf.clone()} class="gi-btn gi-btn-secondary">
                    {"Descargar PDF"}
                </button>
                <button onclick={props.on_export_xlsx.clone()} class="gi-btn gi-btn-secondary">
                    {"Descargar Excel"}
                </button>
            </div>
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--space-4); margin-bottom: var(--space-4);">
                <div class="gi-card" style="padding: var(--space-4);">
                    <p class="gi-label" style="margin-bottom: var(--space-1);">{"Total Pagado"}</p>
                    <p style="font-size: var(--text-xl); font-weight: 600; color: var(--color-success);">
                        {format!("RD$ {:.2}", r.total_pagado)}
                    </p>
                </div>
                <div class="gi-card" style="padding: var(--space-4);">
                    <p class="gi-label" style="margin-bottom: var(--space-1);">{"Total Pendiente"}</p>
                    <p style="font-size: var(--text-xl); font-weight: 600; color: var(--text-secondary);">
                        {format!("RD$ {:.2}", r.total_pendiente)}
                    </p>
                </div>
                <div class="gi-card" style="padding: var(--space-4);">
                    <p class="gi-label" style="margin-bottom: var(--space-1);">{"Total Atrasado"}</p>
                    <p style="font-size: var(--text-xl); font-weight: 600; color: var(--color-danger);">
                        {format!("RD$ {:.2}", r.total_atrasado)}
                    </p>
                </div>
                <div class="gi-card" style="padding: var(--space-4);">
                    <p class="gi-label" style="margin-bottom: var(--space-1);">{"Tasa Ocupación"}</p>
                    <p style="font-size: var(--text-xl); font-weight: 600;">
                        {format!("{:.1}%", r.tasa_ocupacion)}
                    </p>
                </div>
            </div>
            if !r.rows.is_empty() {
                <div class="gi-card" style="overflow-x: auto;">
                    <table class="gi-table" style="width: 100%;">
                        <thead>
                            <tr>
                                <th>{"Propiedad"}</th>
                                <th>{"Inquilino"}</th>
                                <th style="text-align: right;">{"Monto"}</th>
                                <th>{"Moneda"}</th>
                                <th>{"Estado"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            { for r.rows.iter().map(|row| html! {
                                <tr>
                                    <td>{&row.propiedad_titulo}</td>
                                    <td>{&row.inquilino_nombre}</td>
                                    <td style="text-align: right;">{format!("{:.2}", row.monto)}</td>
                                    <td>{&row.moneda}</td>
                                    <td>{&row.estado}</td>
                                </tr>
                            })}
                        </tbody>
                    </table>
                </div>
            }
        </div>
    }
}

#[component]
fn RentabilidadTab() -> Html {
    let mes = use_state(|| {
        let d = js_sys::Date::new_0();
        d.get_month() + 1
    });
    let anio = use_state(|| {
        let d = js_sys::Date::new_0();
        #[allow(clippy::cast_possible_wrap)]
        {
            d.get_full_year() as i32
        }
    });
    let report = use_state(|| Option::<RentabilidadReportSummary>::None);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);

    let on_generate = {
        let mes = mes.clone();
        let anio = anio.clone();
        let report = report.clone();
        let error = error.clone();
        let loading = loading.clone();
        Callback::from(move |_: MouseEvent| {
            let m = *mes;
            let a = *anio;
            let report = report.clone();
            let error = error.clone();
            let loading = loading.clone();
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                let url = format!("/reportes/rentabilidad?mes={m}&anio={a}");
                match api_get::<RentabilidadReportSummary>(&url).await {
                    Ok(data) => report.set(Some(data)),
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        })
    };

    let on_export_pdf = {
        let mes = mes.clone();
        let anio = anio.clone();
        let error = error.clone();
        Callback::from(move |_: MouseEvent| {
            let m = *mes;
            let a = *anio;
            let error = error.clone();
            spawn_local(async move {
                let path = format!("/reportes/rentabilidad/pdf?mes={m}&anio={a}");
                if let Err(e) = api_download(&path, "reporte-rentabilidad.pdf").await {
                    error.set(Some(e));
                }
            });
        })
    };

    let on_export_xlsx = {
        let mes = mes.clone();
        let anio = anio.clone();
        let error = error.clone();
        Callback::from(move |_: MouseEvent| {
            let m = *mes;
            let a = *anio;
            let error = error.clone();
            spawn_local(async move {
                let path = format!("/reportes/rentabilidad/xlsx?mes={m}&anio={a}");
                if let Err(e) = api_download(&path, "reporte-rentabilidad.xlsx").await {
                    error.set(Some(e));
                }
            });
        })
    };

    let on_mes_change = {
        let mes = mes.clone();
        Callback::from(move |e: Event| {
            let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
            if let Ok(v) = el.value().parse::<u32>() {
                mes.set(v);
            }
        })
    };

    let on_anio_change = {
        let anio = anio.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().parse::<i32>() {
                anio.set(v);
            }
        })
    };

    html! {
        <div>
            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <RentabilidadFilterBar
                mes={*mes}
                anio={*anio}
                loading={*loading}
                on_mes_change={on_mes_change}
                on_anio_change={on_anio_change}
                on_generate={on_generate}
            />

            if *loading {
                <ReportSkeleton />
            }

            if let Some(ref r) = *report {
                <RentabilidadResult report={r.clone()} on_export_pdf={on_export_pdf} on_export_xlsx={on_export_xlsx} />
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct RentabilidadFilterBarProps {
    mes: u32,
    anio: i32,
    loading: bool,
    on_mes_change: Callback<Event>,
    on_anio_change: Callback<InputEvent>,
    on_generate: Callback<MouseEvent>,
}

#[component]
fn RentabilidadFilterBar(props: &RentabilidadFilterBarProps) -> Html {
    let meses = [
        (1, "Enero"),
        (2, "Febrero"),
        (3, "Marzo"),
        (4, "Abril"),
        (5, "Mayo"),
        (6, "Junio"),
        (7, "Julio"),
        (8, "Agosto"),
        (9, "Septiembre"),
        (10, "Octubre"),
        (11, "Noviembre"),
        (12, "Diciembre"),
    ];

    html! {
        <div class="gi-filter-bar" style="margin-bottom: var(--space-4);">
            <div style="display: flex; gap: var(--space-3); align-items: end; flex-wrap: wrap;">
                <div>
                    <label class="gi-label">{"Mes"}</label>
                    <select onchange={props.on_mes_change.clone()} class="gi-input">
                        { for meses.iter().map(|(val, label)| {
                            html! { <option value={val.to_string()} selected={props.mes == *val}>{*label}</option> }
                        })}
                    </select>
                </div>
                <div>
                    <label class="gi-label">{"Año"}</label>
                    <input
                        type="number"
                        class="gi-input"
                        value={props.anio.to_string()}
                        oninput={props.on_anio_change.clone()}
                        min="2020"
                        max="2100"
                        style="width: 100px;"
                    />
                </div>
                <div>
                    <button
                        onclick={props.on_generate.clone()}
                        class="gi-btn gi-btn-primary"
                        disabled={props.loading}
                    >
                        {if props.loading { "Generando..." } else { "Generar Reporte" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq, Clone)]
struct RentabilidadResultProps {
    report: RentabilidadReportSummary,
    on_export_pdf: Callback<MouseEvent>,
    on_export_xlsx: Callback<MouseEvent>,
}

#[component]
fn RentabilidadResult(props: &RentabilidadResultProps) -> Html {
    let r = &props.report;

    html! {
        <div>
            <div style="display: flex; gap: var(--space-3); margin-bottom: var(--space-4);">
                <button onclick={props.on_export_pdf.clone()} class="gi-btn gi-btn-secondary">
                    {"Descargar PDF"}
                </button>
                <button onclick={props.on_export_xlsx.clone()} class="gi-btn gi-btn-secondary">
                    {"Descargar Excel"}
                </button>
            </div>

            <RentabilidadSummaryCards
                total_ingresos={r.total_ingresos}
                total_gastos={r.total_gastos}
                total_neto={r.total_neto}
            />

            <RentabilidadTable rows={r.rows.clone()} />
        </div>
    }
}

#[derive(Properties, PartialEq)]
#[allow(clippy::struct_field_names)]
struct RentabilidadSummaryCardsProps {
    total_ingresos: f64,
    total_gastos: f64,
    total_neto: f64,
}

#[component]
fn RentabilidadSummaryCards(props: &RentabilidadSummaryCardsProps) -> Html {
    html! {
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--space-4); margin-bottom: var(--space-4);">
            <div class="gi-card" style="padding: var(--space-4);">
                <p class="gi-label" style="margin-bottom: var(--space-1);">{"Total Ingresos"}</p>
                <p style="font-size: var(--text-xl); font-weight: 600; color: var(--color-success);">
                    {format_currency(props.total_ingresos)}
                </p>
            </div>
            <div class="gi-card" style="padding: var(--space-4);">
                <p class="gi-label" style="margin-bottom: var(--space-1);">{"Total Gastos"}</p>
                <p style="font-size: var(--text-xl); font-weight: 600; color: var(--color-danger);">
                    {format_currency(props.total_gastos)}
                </p>
            </div>
            <div class="gi-card" style="padding: var(--space-4);">
                <p class="gi-label" style="margin-bottom: var(--space-1);">{"Ingreso Neto"}</p>
                <p style={format!("font-size: var(--text-xl); font-weight: 600; color: {};",
                    if props.total_neto >= 0.0 { "var(--color-success)" } else { "var(--color-danger)" })}>
                    {format_currency(props.total_neto)}
                </p>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct RentabilidadTableProps {
    rows: Vec<crate::types::reporte::RentabilidadReportRow>,
}

#[component]
fn RentabilidadTable(props: &RentabilidadTableProps) -> Html {
    if props.rows.is_empty() {
        return html! {
            <div class="gi-card" style="padding: var(--space-6); text-align: center; color: var(--text-secondary);">
                {"No se encontraron datos para el período seleccionado."}
            </div>
        };
    }

    html! {
        <div class="gi-card" style="overflow-x: auto;">
            <table class="gi-table" style="width: 100%;">
                <thead>
                    <tr>
                        <th>{"Propiedad"}</th>
                        <th style="text-align: right;">{"Ingresos"}</th>
                        <th style="text-align: right;">{"Gastos"}</th>
                        <th style="text-align: right;">{"Ingreso Neto"}</th>
                        <th>{"Moneda"}</th>
                    </tr>
                </thead>
                <tbody>
                    { for props.rows.iter().map(|row| {
                        let neto_color = if row.ingreso_neto >= 0.0 { "var(--color-success)" } else { "var(--color-danger)" };
                        html! {
                            <tr>
                                <td>{&row.propiedad_titulo}</td>
                                <td style="text-align: right;">{format_amount(row.total_ingresos, &row.moneda)}</td>
                                <td style="text-align: right;">{format_amount(row.total_gastos, &row.moneda)}</td>
                                <td style={format!("text-align: right; color: {neto_color}; font-weight: 600;")}>
                                    {format_amount(row.ingreso_neto, &row.moneda)}
                                </td>
                                <td>{&row.moneda}</td>
                            </tr>
                        }
                    })}
                </tbody>
            </table>
        </div>
    }
}

fn format_currency(value: f64) -> String {
    if value >= 0.0 {
        format!("RD$ {value:.2}")
    } else {
        format!("-RD$ {:.2}", value.abs())
    }
}

fn format_amount(value: f64, moneda: &str) -> String {
    let symbol = match moneda {
        "USD" => "US$",
        _ => "RD$",
    };
    if value >= 0.0 {
        format!("{symbol} {value:.2}")
    } else {
        format!("-{symbol} {:.2}", value.abs())
    }
}
