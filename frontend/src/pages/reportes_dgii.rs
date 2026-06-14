use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::ReportSkeleton;
use crate::services::api::{api_get, api_post, api_put};
use crate::types::reportes_dgii::{
    EstadoRequest, ItbisNetoResult, PeriodoRequest, ReporteGenerado, ReportePreviewResponse,
};

#[component]
pub fn ReportesDgii() -> Html {
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

    let reporte_607 = use_state(|| Option::<Rc<ReporteGenerado>>::None);
    let reporte_606 = use_state(|| Option::<Rc<ReporteGenerado>>::None);
    let preview = use_state(|| Option::<ReportePreviewResponse>::None);
    let itbis_neto = use_state(|| Option::<ItbisNetoResult>::None);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);
    let submitting = use_state(|| false);

    let periodo = format!("{}{:02}", *anio, *mes);

    let on_generar_607 = {
        let mes = mes.clone();
        let anio = anio.clone();
        let reporte_607 = reporte_607.clone();
        let error = error.clone();
        let loading = loading.clone();
        Callback::from(move |_: MouseEvent| {
            let periodo = format!("{}{:02}", *anio, *mes);
            let reporte_607 = reporte_607.clone();
            let error = error.clone();
            let loading = loading.clone();
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                let body = PeriodoRequest { periodo };
                match api_post::<ReporteGenerado, _>("/reportes-dgii/607", &body).await {
                    Ok(data) => reporte_607.set(Some(Rc::new(data))),
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        })
    };

    let on_generar_606 = {
        let mes = mes.clone();
        let anio = anio.clone();
        let reporte_606 = reporte_606.clone();
        let error = error.clone();
        let loading = loading.clone();
        Callback::from(move |_: MouseEvent| {
            let periodo = format!("{}{:02}", *anio, *mes);
            let reporte_606 = reporte_606.clone();
            let error = error.clone();
            let loading = loading.clone();
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                let body = PeriodoRequest { periodo };
                match api_post::<ReporteGenerado, _>("/reportes-dgii/606", &body).await {
                    Ok(data) => reporte_606.set(Some(Rc::new(data))),
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        })
    };

    let on_preview = {
        let preview = preview.clone();
        let itbis_neto = itbis_neto.clone();
        let error = error.clone();
        let loading = loading.clone();
        Callback::from(move |_: MouseEvent| {
            let periodo = periodo.clone();
            let preview = preview.clone();
            let itbis_neto_state = itbis_neto.clone();
            let error = error.clone();
            let loading = loading.clone();
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                let url = format!("/reportes-dgii/preview/607/{periodo}");
                match api_get::<ReportePreviewResponse>(&url).await {
                    Ok(data) => preview.set(Some(data)),
                    Err(err) => {
                        let url_606 = format!("/reportes-dgii/preview/606/{periodo}");
                        match api_get::<ReportePreviewResponse>(&url_606).await {
                            Ok(data) => preview.set(Some(data)),
                            Err(_) => error.set(Some(err)),
                        }
                    }
                }
                let url_neto = format!("/reportes-dgii/itbis-neto/{periodo}");
                if let Ok(neto) = api_get::<ItbisNetoResult>(&url_neto).await {
                    itbis_neto_state.set(Some(neto));
                }
                loading.set(false);
            });
        })
    };

    let on_marcar_enviado = {
        let preview = preview.clone();
        let error = error.clone();
        let submitting = submitting.clone();
        Callback::from(move |_: MouseEvent| {
            let preview_val = (*preview).clone();
            let preview = preview.clone();
            let error = error.clone();
            let submitting = submitting.clone();
            if let Some(ref p) = preview_val {
                if p.estado == "enviado" {
                    return;
                }
                let id = p.id.clone();
                submitting.set(true);
                error.set(None);
                spawn_local(async move {
                    let body = EstadoRequest {
                        estado: "enviado".to_string(),
                    };
                    let url = format!("/reportes-dgii/{id}/estado");
                    match api_put::<ReportePreviewResponse, _>(&url, &body).await {
                        Ok(updated) => preview.set(Some(updated)),
                        Err(err) => error.set(Some(err)),
                    }
                    submitting.set(false);
                });
            }
        })
    };

    let on_download_607 = {
        let reporte_607 = reporte_607.clone();
        let anio = anio.clone();
        let mes = mes.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(ref r) = *reporte_607 {
                trigger_txt_download(&r.contenido, &format!("607_{}{:02}.txt", *anio, *mes));
            }
        })
    };

    let on_download_606 = {
        let reporte_606 = reporte_606.clone();
        let anio = anio.clone();
        let mes = mes.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(ref r) = *reporte_606 {
                trigger_txt_download(&r.contenido, &format!("606_{}{:02}.txt", *anio, *mes));
            }
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
                <h1 class="gi-page-title">{"Reportes DGII (606/607)"}</h1>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <PeriodSelector
                mes={*mes}
                anio={*anio}
                loading={*loading}
                on_mes_change={on_mes_change}
                on_anio_change={on_anio_change}
                on_generar_607={on_generar_607}
                on_generar_606={on_generar_606}
                on_preview={on_preview}
            />

            if *loading {
                <ReportSkeleton />
            }

            if let Some(ref neto) = *itbis_neto {
                <ItbisNetoCard neto={neto.clone()} />
            }

            if let Some(ref p) = *preview {
                <ReporteStatusCard
                    preview={p.clone()}
                    submitting={*submitting}
                    on_marcar_enviado={on_marcar_enviado}
                />
            }

            if let Some(ref r) = *reporte_607 {
                <ReporteResultSection
                    titulo={"Reporte 607 (Ingresos)"}
                    reporte={Rc::clone(r)}
                    on_download={on_download_607}
                />
            }

            if let Some(ref r) = *reporte_606 {
                <ReporteResultSection
                    titulo={"Reporte 606 (Compras)"}
                    reporte={Rc::clone(r)}
                    on_download={on_download_606}
                />
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct PeriodSelectorProps {
    mes: u32,
    anio: i32,
    loading: bool,
    on_mes_change: Callback<Event>,
    on_anio_change: Callback<InputEvent>,
    on_generar_607: Callback<MouseEvent>,
    on_generar_606: Callback<MouseEvent>,
    on_preview: Callback<MouseEvent>,
}

#[component]
fn PeriodSelector(props: &PeriodSelectorProps) -> Html {
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
                <div style="display: flex; gap: var(--space-2);">
                    <button
                        onclick={props.on_generar_607.clone()}
                        class="gi-btn gi-btn-primary"
                        disabled={props.loading}
                    >
                        {if props.loading { "Generando..." } else { "Generar 607" }}
                    </button>
                    <button
                        onclick={props.on_generar_606.clone()}
                        class="gi-btn gi-btn-primary"
                        disabled={props.loading}
                    >
                        {if props.loading { "Generando..." } else { "Generar 606" }}
                    </button>
                    <button
                        onclick={props.on_preview.clone()}
                        class="gi-btn gi-btn-secondary"
                        disabled={props.loading}
                    >
                        {"Ver Estado"}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ItbisNetoCardProps {
    neto: ItbisNetoResult,
}

#[component]
fn ItbisNetoCard(props: &ItbisNetoCardProps) -> Html {
    let neto = &props.neto;
    let color = if neto.itbis_neto >= 0.0 {
        "var(--color-danger)"
    } else {
        "var(--color-success)"
    };

    html! {
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--space-4); margin-bottom: var(--space-4);">
            <div class="gi-card" style="padding: var(--space-4);">
                <p class="gi-label" style="margin-bottom: var(--space-1);">{"ITBIS Cobrado (607)"}</p>
                <p style="font-size: var(--text-xl); font-weight: 600;">
                    {format!("RD$ {:.2}", neto.itbis_cobrado)}
                </p>
            </div>
            <div class="gi-card" style="padding: var(--space-4);">
                <p class="gi-label" style="margin-bottom: var(--space-1);">{"ITBIS Pagado (606)"}</p>
                <p style="font-size: var(--text-xl); font-weight: 600;">
                    {format!("RD$ {:.2}", neto.itbis_pagado)}
                </p>
            </div>
            <div class="gi-card" style="padding: var(--space-4);">
                <p class="gi-label" style="margin-bottom: var(--space-1);">{"ITBIS Neto"}</p>
                <p style={format!("font-size: var(--text-xl); font-weight: 600; color: {color};")}>
                    {format!("RD$ {:.2}", neto.itbis_neto)}
                </p>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ReporteStatusCardProps {
    preview: ReportePreviewResponse,
    submitting: bool,
    on_marcar_enviado: Callback<MouseEvent>,
}

#[component]
fn ReporteStatusCard(props: &ReporteStatusCardProps) -> Html {
    let p = &props.preview;
    let is_enviado = p.estado == "enviado";
    let badge_style = if is_enviado {
        "background: var(--color-success); color: white; padding: 2px 8px; border-radius: 4px; font-size: var(--text-sm);"
    } else {
        "background: var(--color-warning); color: white; padding: 2px 8px; border-radius: 4px; font-size: var(--text-sm);"
    };

    html! {
        <div class="gi-card" style="padding: var(--space-4); margin-bottom: var(--space-4);">
            <div style="display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: var(--space-3);">
                <div>
                    <p style="font-weight: 600; margin-bottom: var(--space-1);">
                        {format!("Reporte {} — Período {}", p.tipo_reporte, p.periodo)}
                    </p>
                    <p style="font-size: var(--text-sm); color: var(--text-secondary);">
                        {format!("Registros: {} | Total: RD$ {:.2} | ITBIS: RD$ {:.2}",
                            p.cantidad_registros, p.monto_total, p.itbis_total)}
                    </p>
                </div>
                <div style="display: flex; align-items: center; gap: var(--space-3);">
                    <span style={badge_style}>
                        {if is_enviado { "Enviado" } else { "Borrador" }}
                    </span>
                    if !is_enviado {
                        <button
                            onclick={props.on_marcar_enviado.clone()}
                            class="gi-btn gi-btn-primary"
                            disabled={props.submitting}
                        >
                            {if props.submitting { "Enviando..." } else { "Marcar como Enviado" }}
                        </button>
                    }
                </div>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ReporteResultSectionProps {
    titulo: AttrValue,
    reporte: Rc<ReporteGenerado>,
    on_download: Callback<MouseEvent>,
}

#[component]
fn ReporteResultSection(props: &ReporteResultSectionProps) -> Html {
    let r = &props.reporte;

    html! {
        <div style="margin-bottom: var(--space-5);">
            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--space-3);">
                <h2 style="font-size: var(--text-lg); font-weight: 600;">{&props.titulo}</h2>
                <button onclick={props.on_download.clone()} class="gi-btn gi-btn-secondary">
                    {"Descargar TXT"}
                </button>
            </div>

            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: var(--space-3); margin-bottom: var(--space-3);">
                <div class="gi-card" style="padding: var(--space-3);">
                    <p class="gi-label" style="margin-bottom: 2px;">{"Registros"}</p>
                    <p style="font-weight: 600;">{r.cantidad_registros}</p>
                </div>
                <div class="gi-card" style="padding: var(--space-3);">
                    <p class="gi-label" style="margin-bottom: 2px;">{"Monto Total"}</p>
                    <p style="font-weight: 600;">{format!("RD$ {:.2}", r.monto_total)}</p>
                </div>
                <div class="gi-card" style="padding: var(--space-3);">
                    <p class="gi-label" style="margin-bottom: 2px;">{"ITBIS Total"}</p>
                    <p style="font-weight: 600;">{format!("RD$ {:.2}", r.itbis_total)}</p>
                </div>
                <div class="gi-card" style="padding: var(--space-3);">
                    <p class="gi-label" style="margin-bottom: 2px;">{"Excluidos"}</p>
                    <p style="font-weight: 600; color: var(--color-warning);">{r.excluidos.len()}</p>
                </div>
            </div>

            if !r.preview.is_empty() {
                <PreviewTable rows={Rc::new(r.preview.clone())} />
            }

            if !r.excluidos.is_empty() {
                <ExcluidosTable excluidos={Rc::new(r.excluidos.clone())} />
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct PreviewTableProps {
    rows: Rc<Vec<RegistroPreview>>,
}

use crate::types::reportes_dgii::RegistroPreview;

#[component]
fn PreviewTable(props: &PreviewTableProps) -> Html {
    html! {
        <div class="gi-card" style="overflow-x: auto; margin-bottom: var(--space-3);">
            <table class="gi-table" style="width: 100%;">
                <thead>
                    <tr>
                        <th>{"#"}</th>
                        <th>{"Campos del Registro"}</th>
                    </tr>
                </thead>
                <tbody>
                    { for props.rows.iter().enumerate().map(|(i, row)| {
                        html! {
                            <tr>
                                <td>{i + 1}</td>
                                <td style="font-family: monospace; font-size: var(--text-sm);">
                                    {row.campos.join(" | ")}
                                </td>
                            </tr>
                        }
                    })}
                </tbody>
            </table>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ExcluidosTableProps {
    excluidos: Rc<Vec<RegistroExcluido>>,
}

use crate::types::reportes_dgii::RegistroExcluido;

#[component]
fn ExcluidosTable(props: &ExcluidosTableProps) -> Html {
    html! {
        <div class="gi-card" style="overflow-x: auto;">
            <div style="padding: var(--space-3); border-bottom: 1px solid var(--border-primary);">
                <p style="font-weight: 600; color: var(--color-warning);">
                    {"Registros Excluidos"}
                </p>
                <p style="font-size: var(--text-sm); color: var(--text-secondary);">
                    {"Estos registros no se incluyeron por datos fiscales incompletos (falta RNC o fecha_comprobante)."}
                </p>
            </div>
            <table class="gi-table" style="width: 100%;">
                <thead>
                    <tr>
                        <th>{"Referencia"}</th>
                        <th>{"Razón de Exclusión"}</th>
                    </tr>
                </thead>
                <tbody>
                    { for props.excluidos.iter().map(|exc| {
                        html! {
                            <tr>
                                <td>{&exc.referencia}</td>
                                <td>{&exc.razon}</td>
                            </tr>
                        }
                    })}
                </tbody>
            </table>
        </div>
    }
}

fn trigger_txt_download(content: &str, filename: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };

    let uint8 = js_sys::Uint8Array::from(content.as_bytes());
    let array = js_sys::Array::new();
    array.push(&uint8.buffer());

    #[allow(deprecated)]
    let opts = {
        let mut o = web_sys::BlobPropertyBag::new();
        o.type_("text/plain;charset=utf-8");
        o
    };

    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&array, &opts) else {
        return;
    };

    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };

    if let Ok(el) = document.create_element("a") {
        let a: web_sys::HtmlElement = el.unchecked_into();
        let _ = a.set_attribute("href", &url);
        let _ = a.set_attribute("download", filename);
        a.click();
    }

    let _ = web_sys::Url::revoke_object_url(&url);
}
