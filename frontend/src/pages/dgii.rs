use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::components::common::error_banner::ErrorBanner;
use crate::services::api::api_get;
use crate::types::dgii::{DgiiConsulta, DgiiNombreResult};

#[component]
pub fn Dgii() -> Html {
    let rnc_input = use_state(String::new);
    let nombre_input = use_state(String::new);
    let rnc_result = use_state(|| Option::<DgiiConsulta>::None);
    let nombre_result = use_state(|| Option::<DgiiNombreResult>::None);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);

    let auth = use_context::<AuthContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map_or("", |u| u.rol.as_str());
    let can_write = user_rol == "admin" || user_rol == "gerente";

    if !can_write {
        return html! {
            <div class="gi-empty-state">
                <div class="gi-empty-state-title">{"No tiene permisos para acceder a esta sección"}</div>
            </div>
        };
    }

    let on_rnc_change = {
        let rnc_input = rnc_input.clone();
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlInputElement = e.target_unchecked_into();
            rnc_input.set(el.value());
        })
    };

    let on_nombre_change = {
        let nombre_input = nombre_input.clone();
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlInputElement = e.target_unchecked_into();
            nombre_input.set(el.value());
        })
    };

    let on_search_rnc = {
        let rnc_input = rnc_input.clone();
        let rnc_result = rnc_result.clone();
        let nombre_result = nombre_result.clone();
        let error = error.clone();
        let loading = loading.clone();
        Callback::from(move |_: MouseEvent| {
            let value = (*rnc_input).clone();
            if value.is_empty() {
                return;
            }
            let rnc_result = rnc_result.clone();
            let nombre_result = nombre_result.clone();
            let error = error.clone();
            let loading = loading.clone();
            spawn_local(async move {
                loading.set(true);
                error.set(None);
                nombre_result.set(None);
                let url = format!("/dgii/consulta?rnc={value}");
                match api_get::<DgiiConsulta>(&url).await {
                    Ok(resp) => rnc_result.set(Some(resp)),
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        })
    };

    let on_search_nombre = {
        let nombre_input = nombre_input.clone();
        let rnc_result = rnc_result.clone();
        let nombre_result = nombre_result.clone();
        let error = error.clone();
        let loading = loading.clone();
        Callback::from(move |_: MouseEvent| {
            let value = (*nombre_input).clone();
            if value.is_empty() {
                return;
            }
            let rnc_result = rnc_result.clone();
            let nombre_result = nombre_result.clone();
            let error = error.clone();
            let loading = loading.clone();
            spawn_local(async move {
                loading.set(true);
                error.set(None);
                rnc_result.set(None);
                let url = format!("/dgii/consulta/nombre?buscar={value}");
                match api_get::<DgiiNombreResult>(&url).await {
                    Ok(resp) => nombre_result.set(Some(resp)),
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        })
    };

    let on_invalidar_cache = {
        let error = error.clone();
        let loading = loading.clone();
        Callback::from(move |_: MouseEvent| {
            let error = error.clone();
            let loading = loading.clone();
            spawn_local(async move {
                loading.set(true);
                if let Err(err) = api_get::<serde_json::Value>("/dgii/cache/invalidar").await {
                    error.set(Some(err));
                }
                loading.set(false);
            });
        })
    };

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Consulta DGII"}</h1>
                <button class="gi-btn gi-btn-ghost" onclick={on_invalidar_cache}>{"Invalidar Caché"}</button>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-4);">
                <DgiiSearchSection
                    label="Buscar por RNC"
                    placeholder="Ingrese RNC o Cédula"
                    value={(*rnc_input).clone()}
                    oninput={on_rnc_change}
                    onclick={on_search_rnc}
                    loading={*loading}
                />
            </div>

            <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-4);">
                <DgiiSearchSection
                    label="Buscar por nombre"
                    placeholder="Ingrese nombre o razón social"
                    value={(*nombre_input).clone()}
                    oninput={on_nombre_change}
                    onclick={on_search_nombre}
                    loading={*loading}
                />
            </div>

            if let Some(data) = (*rnc_result).as_ref() {
                <DgiiRncResultCard data={data.clone()} />
            }

            if let Some(data) = (*nombre_result).as_ref() {
                <DgiiNombreResultTable data={data.clone()} />
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
#[allow(dead_code)]
struct DgiiSearchSectionProps {
    label: AttrValue,
    placeholder: AttrValue,
    value: AttrValue,
    oninput: Callback<InputEvent>,
    onclick: Callback<MouseEvent>,
    loading: bool,
}

#[component]
fn DgiiSearchSection(props: &DgiiSearchSectionProps) -> Html {
    html! {
        <div>
            <label class="gi-label">{&props.label}</label>
            <div style="display: flex; gap: var(--space-3); align-items: center;">
                <input
                    type="text"
                    class="gi-input"
                    placeholder={props.placeholder.clone()}
                    value={props.value.clone()}
                    oninput={props.oninput.clone()}
                />
                <button
                    class="gi-btn gi-btn-primary"
                    onclick={props.onclick.clone()}
                    disabled={props.loading}
                >
                    {"Buscar"}
                </button>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
#[allow(dead_code)]
struct DgiiRncResultCardProps {
    data: DgiiConsulta,
}

#[component]
fn DgiiRncResultCard(props: &DgiiRncResultCardProps) -> Html {
    let d = &props.data;
    html! {
        <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-4);">
            <h3 style="margin-bottom: var(--space-4); font-size: var(--text-lg);">{"Resultado"}</h3>
            <DgiiField label="Cédula/RNC" value={d.cedula_rnc.clone()} />
            <DgiiField label="Nombre/Razón Social" value={d.nombre_razon_social.clone()} />
            <DgiiField label="Nombre Comercial" value={d.nombre_comercial.clone().unwrap_or_default()} />
            <DgiiField label="Estado" value={d.estado.clone()} />
            <DgiiField label="Régimen de Pagos" value={d.regimen_de_pagos.clone().unwrap_or_default()} />
            <DgiiField label="Actividad Económica" value={d.actividad_economica.clone().unwrap_or_default()} />
            if d.cached {
                <p style="font-size: var(--text-xs); color: var(--text-secondary); margin-top: var(--space-2);">{"(resultado desde caché)"}</p>
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
#[allow(dead_code)]
struct DgiiFieldProps {
    label: AttrValue,
    value: AttrValue,
}

#[component]
fn DgiiField(props: &DgiiFieldProps) -> Html {
    html! {
        <div style="display: flex; padding: var(--space-2) 0; border-bottom: 1px solid var(--border-color);">
            <span style="font-weight: 600; min-width: 180px; font-size: var(--text-sm); color: var(--text-secondary);">
                {&props.label}
            </span>
            <span style="font-size: var(--text-sm);">{&props.value}</span>
        </div>
    }
}

#[derive(Properties, PartialEq)]
#[allow(dead_code)]
struct DgiiNombreResultTableProps {
    data: DgiiNombreResult,
}

#[component]
fn DgiiNombreResultTable(props: &DgiiNombreResultTableProps) -> Html {
    if props.data.resultados.is_empty() {
        return html! {
            <div class="gi-card" style="padding: var(--space-6);">
                <p style="font-size: var(--text-sm); color: var(--text-secondary);">{"Sin resultados encontrados."}</p>
            </div>
        };
    }
    html! {
        <div class="gi-card" style="padding: var(--space-6);">
            <h3 style="margin-bottom: var(--space-4); font-size: var(--text-lg);">{"Resultados por Nombre"}</h3>
            <table class="gi-table">
                <thead>
                    <tr>
                        <th style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{"Cédula/RNC"}</th>
                        <th style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{"Nombre/Razón Social"}</th>
                        <th style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{"Nombre Comercial"}</th>
                        <th style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{"Estado"}</th>
                    </tr>
                </thead>
                <tbody>
                    { for props.data.resultados.iter().map(|item| {
                        html! {
                            <tr>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {&item.cedula_rnc}
                                </td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {&item.nombre_razon_social}
                                </td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {item.nombre_comercial.clone().unwrap_or_default()}
                                </td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    {&item.estado}
                                </td>
                            </tr>
                        }
                    })}
                </tbody>
            </table>
        </div>
    }
}
