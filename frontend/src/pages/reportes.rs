use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::ReportSkeleton;
use crate::services::api::{api_download, api_get};
use crate::types::reporte::IngresoReportSummary;

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

// ---------------------------------------------------------------------------
// Ingresos Tab (existing functionality)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Stub components — to be implemented by task 11.5
// ---------------------------------------------------------------------------

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
    let _ = props;
    html! { <div class="gi-filter-bar">{"TODO: Ingresos filter"}</div> }
}

#[derive(Properties, PartialEq, Clone)]
struct IngresosResultProps {
    report: IngresoReportSummary,
    on_export_pdf: Callback<MouseEvent>,
    on_export_xlsx: Callback<MouseEvent>,
}

#[component]
fn IngresosResult(props: &IngresosResultProps) -> Html {
    let _ = props;
    html! { <div>{"TODO: Ingresos result"}</div> }
}

#[component]
fn RentabilidadTab() -> Html {
    html! { <div>{"TODO: Rentabilidad"}</div> }
}
