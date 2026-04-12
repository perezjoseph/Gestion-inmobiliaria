use gloo_net::http::Request;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::error_banner::ErrorBanner;
use crate::services::api::BASE_URL;
use crate::types::importacion::ImportResult;

fn get_token() -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()
        .flatten()?
        .get_item("jwt_token")
        .ok()
        .flatten()
}

async fn handle_upload_response(resp: gloo_net::http::Response) -> Result<ImportResult, String> {
    if resp.ok() {
        resp.json::<ImportResult>()
            .await
            .map_err(|err| format!("Error al procesar respuesta: {err}"))
    } else {
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "Error desconocido".into());
        Err(text)
    }
}

async fn perform_upload(file: web_sys::File, entity_type: String) -> Result<ImportResult, String> {
    let form_data =
        web_sys::FormData::new().map_err(|_| "Error al crear formulario".to_string())?;
    let _ = form_data.append_with_blob("file", &file);

    let url = format!("{BASE_URL}/importar/{entity_type}");
    let mut builder = Request::post(&url);
    if let Some(token) = get_token() {
        builder = builder.header("Authorization", &format!("Bearer {token}"));
    }
    let req = builder
        .body(form_data)
        .map_err(|err| format!("Error al preparar solicitud: {err}"))?;
    let resp = req
        .send()
        .await
        .map_err(|err| format!("Error de red: {err}"))?;
    handle_upload_response(resp).await
}

#[function_component]
pub fn Importar() -> Html {
    let entity_type = use_state(|| "propiedades".to_string());
    let result = use_state(|| Option::<ImportResult>::None);
    let error = use_state(|| Option::<String>::None);
    let uploading = use_state(|| false);

    let on_entity_change = {
        let entity_type = entity_type.clone();
        Callback::from(move |e: Event| {
            let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
            entity_type.set(el.value());
        })
    };

    let on_upload = {
        let entity_type = entity_type.clone();
        let result = result.clone();
        let error = error.clone();
        let uploading = uploading.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let Some(files) = input.files() else { return };
            let Some(file) = files.get(0) else { return };

            let entity_type = (*entity_type).clone();
            let result = result.clone();
            let error = error.clone();
            let uploading = uploading.clone();

            uploading.set(true);
            error.set(None);
            result.set(None);

            spawn_local(async move {
                match perform_upload(file, entity_type).await {
                    Ok(r) => result.set(Some(r)),
                    Err(err) => error.set(Some(err)),
                }
                uploading.set(false);
            });

            input.set_value("");
        })
    };

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Importación Masiva"}</h1>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <div class="gi-card" style="padding: var(--space-6);">
                <h2 style="font-size: var(--text-base); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-4);">
                    {"Importar datos desde archivo CSV o XLSX"}
                </h2>

                <div style="display: flex; gap: var(--space-4); align-items: end; flex-wrap: wrap;">
                    <div>
                        <label class="gi-label">{"Tipo de Entidad"}</label>
                        <select onchange={on_entity_change} class="gi-input">
                            <option value="propiedades" selected={*entity_type == "propiedades"}>{"Propiedades"}</option>
                            <option value="inquilinos" selected={*entity_type == "inquilinos"}>{"Inquilinos"}</option>
                        </select>
                    </div>
                    <div>
                        <label
                            class="gi-btn gi-btn-primary"
                            style={if *uploading { "opacity: 0.5; pointer-events: none;" } else { "" }}
                        >
                            { if *uploading { "Importando..." } else { "📁 Seleccionar Archivo" } }
                            <input
                                type="file"
                                accept=".csv,.xlsx"
                                onchange={on_upload}
                                style="display: none;"
                            />
                        </label>
                    </div>
                </div>

                <div style="margin-top: var(--space-3); font-size: var(--text-xs); color: var(--text-tertiary);">
                    {"Formatos aceptados: CSV (.csv) y Excel (.xlsx). La primera fila debe contener los encabezados."}
                </div>
            </div>

            if let Some(ref r) = *result {
                <div class="gi-card" style="padding: var(--space-5); margin-top: var(--space-4);">
                    <h3 style="font-size: var(--text-base); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-3);">
                        {"Resultado de la Importación"}
                    </h3>
                    <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--space-3); margin-bottom: var(--space-3);">
                        <div class="gi-inline-stat">
                            <div style="font-size: var(--text-xs); color: var(--text-secondary);">{"Total Filas"}</div>
                            <div style="font-size: var(--text-lg); font-weight: 700;">{r.total_filas}</div>
                        </div>
                        <div class="gi-inline-stat">
                            <div style="font-size: var(--text-xs); color: var(--text-secondary);">{"Exitosos"}</div>
                            <div style="font-size: var(--text-lg); font-weight: 700; color: var(--color-success);">{r.exitosos}</div>
                        </div>
                        <div class="gi-inline-stat">
                            <div style="font-size: var(--text-xs); color: var(--text-secondary);">{"Fallidos"}</div>
                            <div style="font-size: var(--text-lg); font-weight: 700; color: var(--color-error);">{r.fallidos.len()}</div>
                        </div>
                    </div>
                    if !r.fallidos.is_empty() {
                        <div style="max-height: 300px; overflow-y: auto;">
                            <table class="gi-table" style="width: 100%;">
                                <thead>
                                    <tr>
                                        <th style="padding: var(--space-2) var(--space-3);">{"Fila"}</th>
                                        <th style="padding: var(--space-2) var(--space-3);">{"Error"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    { for r.fallidos.iter().map(|f| html! {
                                        <tr>
                                            <td style="padding: var(--space-2) var(--space-3); font-size: var(--text-sm);">{f.fila}</td>
                                            <td style="padding: var(--space-2) var(--space-3); font-size: var(--text-sm); color: var(--color-error);">{&f.error}</td>
                                        </tr>
                                    })}
                                </tbody>
                            </table>
                        </div>
                    }
                </div>
            }
        </div>
    }
}
