use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::currency_display::CurrencyDisplay;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::loading::Loading;
use crate::services::api::{BASE_URL, api_get};
use crate::types::reporte::IngresoReportSummary;

#[function_component]
pub fn Reportes() -> Html {
    let mes = use_state(|| {
        let d = js_sys::Date::new_0();
        d.get_month() + 1
    });
    let anio = use_state(|| {
        let d = js_sys::Date::new_0();
        d.get_full_year() as i32
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
        Callback::from(move |_: MouseEvent| {
            let url = format!(
                "{BASE_URL}/reportes/ingresos/pdf?mes={}&anio={}",
                *mes, *anio
            );
            let _ = web_sys::window().and_then(|w| w.open_with_url(&url).ok());
        })
    };

    let on_export_xlsx = {
        let mes = mes.clone();
        let anio = anio.clone();
        Callback::from(move |_: MouseEvent| {
            let url = format!(
                "{BASE_URL}/reportes/ingresos/xlsx?mes={}&anio={}",
                *mes, *anio
            );
            let _ = web_sys::window().and_then(|w| w.open_with_url(&url).ok());
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
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Reportes de Ingresos"}</h1>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <div class="gi-filter-bar">
                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: var(--space-3); align-items: end;">
                    <div>
                        <label class="gi-label">{"Mes"}</label>
                        <select onchange={on_mes_change} class="gi-input">
                            { for (1..=12u32).map(|m| {
                                let label = mes_label(m);
                                html! { <option value={m.to_string()} selected={m == *mes}>{label}</option> }
                            })}
                        </select>
                    </div>
                    <div>
                        <label class="gi-label">{"Año"}</label>
                        <input type="number" value={anio.to_string()} oninput={on_anio_change}
                            class="gi-input" min="2020" max="2099" />
                    </div>
                    <div style="display: flex; gap: var(--space-2);">
                        <button onclick={on_generate} class="gi-btn gi-btn-primary" disabled={*loading}>
                            { if *loading { "Generando..." } else { "Generar Reporte" } }
                        </button>
                    </div>
                </div>
            </div>

            if *loading {
                <Loading />
            }

            if let Some(ref r) = *report {
                <div class="gi-card" style="padding: var(--space-5); margin-top: var(--space-4);">
                    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-4);">
                        <h2 style="font-size: var(--text-base); font-weight: 600; color: var(--text-primary);">
                            {"Resultado del Reporte"}
                        </h2>
                        <div style="display: flex; gap: var(--space-2);">
                            <button onclick={on_export_pdf} class="gi-btn gi-btn-sm gi-btn-secondary">{"📄 Exportar PDF"}</button>
                            <button onclick={on_export_xlsx} class="gi-btn gi-btn-sm gi-btn-secondary">{"📊 Exportar Excel"}</button>
                        </div>
                    </div>

                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: var(--space-3); margin-bottom: var(--space-4);">
                        <div class="gi-inline-stat">
                            <div style="font-size: var(--text-xs); color: var(--text-secondary);">{"Total Pagado"}</div>
                            <div style="font-size: var(--text-lg); font-weight: 700; color: var(--color-success);">
                                <CurrencyDisplay monto={r.total_pagado} moneda={"DOP".to_string()} />
                            </div>
                        </div>
                        <div class="gi-inline-stat">
                            <div style="font-size: var(--text-xs); color: var(--text-secondary);">{"Total Pendiente"}</div>
                            <div style="font-size: var(--text-lg); font-weight: 700; color: var(--color-warning);">
                                <CurrencyDisplay monto={r.total_pendiente} moneda={"DOP".to_string()} />
                            </div>
                        </div>
                        <div class="gi-inline-stat">
                            <div style="font-size: var(--text-xs); color: var(--text-secondary);">{"Total Atrasado"}</div>
                            <div style="font-size: var(--text-lg); font-weight: 700; color: var(--color-error);">
                                <CurrencyDisplay monto={r.total_atrasado} moneda={"DOP".to_string()} />
                            </div>
                        </div>
                        <div class="gi-inline-stat">
                            <div style="font-size: var(--text-xs); color: var(--text-secondary);">{"Tasa Ocupación"}</div>
                            <div style="font-size: var(--text-lg); font-weight: 700; color: var(--text-primary);">
                                {format!("{:.1}%", r.tasa_ocupacion)}
                            </div>
                        </div>
                    </div>

                    if r.rows.is_empty() {
                        <p style="text-align: center; color: var(--text-tertiary); padding: var(--space-4);">
                            {"Sin registros para el período seleccionado."}
                        </p>
                    } else {
                        <div style="overflow-x: auto;">
                            <table class="gi-table" style="width: 100%;">
                                <thead>
                                    <tr>
                                        <th style="padding: var(--space-3) var(--space-4);">{"Propiedad"}</th>
                                        <th style="padding: var(--space-3) var(--space-4);">{"Inquilino"}</th>
                                        <th style="padding: var(--space-3) var(--space-4);">{"Monto"}</th>
                                        <th style="padding: var(--space-3) var(--space-4);">{"Estado"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    { for r.rows.iter().map(|row| html! {
                                        <tr>
                                            <td style="padding: var(--space-3) var(--space-4); font-size: var(--text-sm);">{&row.propiedad_titulo}</td>
                                            <td style="padding: var(--space-3) var(--space-4); font-size: var(--text-sm);">{&row.inquilino_nombre}</td>
                                            <td class="tabular-nums" style="padding: var(--space-3) var(--space-4); font-size: var(--text-sm);"><CurrencyDisplay monto={row.monto} moneda={row.moneda.clone()} /></td>
                                            <td style="padding: var(--space-3) var(--space-4); font-size: var(--text-sm);">{estado_label(&row.estado)}</td>
                                        </tr>
                                    })}
                                </tbody>
                            </table>
                        </div>
                    }

                    <div style="margin-top: var(--space-3); font-size: var(--text-xs); color: var(--text-tertiary);">
                        {format!("Generado: {} — Por: {}", r.generated_at, r.generated_by)}
                    </div>
                </div>
            }
        </div>
    }
}

fn mes_label(m: u32) -> &'static str {
    match m {
        1 => "Enero",
        2 => "Febrero",
        3 => "Marzo",
        4 => "Abril",
        5 => "Mayo",
        6 => "Junio",
        7 => "Julio",
        8 => "Agosto",
        9 => "Septiembre",
        10 => "Octubre",
        11 => "Noviembre",
        12 => "Diciembre",
        _ => "—",
    }
}

fn estado_label(estado: &str) -> &str {
    match estado {
        "pagado" => "Pagado",
        "pendiente" => "Pendiente",
        "atrasado" => "Atrasado",
        _ => estado,
    }
}
