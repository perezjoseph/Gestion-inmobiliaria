use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::services::api::api_get;
use crate::types::documento::DocumentoResponse;
use crate::utils::format_date_display;

#[function_component(DocumentosPorVencer)]
pub fn documentos_por_vencer() -> Html {
    let docs = use_state(Vec::<DocumentoResponse>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);

    {
        let docs = docs.clone();
        let loading = loading.clone();
        let error = error.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                match api_get::<Vec<DocumentoResponse>>("/documentos/por-vencer").await {
                    Ok(mut rows) => {
                        rows.sort_by(|a, b| a.fecha_vencimiento.cmp(&b.fecha_vencimiento));
                        docs.set(rows);
                    }
                    Err(e) => error.set(Some(e)),
                }
                loading.set(false);
            });
        });
    }

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Documentos por Vencer"}</h1>
                <p class="gi-page-subtitle">{"Documentos cuya fecha de expiración está próxima (30 días)"}</p>
            </div>

            if *loading {
                <div class="gi-loading-container">
                    <div class="gi-spinner"></div>
                    <span>{"Cargando documentos..."}</span>
                </div>
            } else if let Some(err) = (*error).as_ref() {
                <div class="gi-alert gi-alert-error">
                    {format!("Error al cargar documentos: {err}")}
                </div>
            } else if docs.is_empty() {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-icon">{"📄"}</div>
                    <div class="gi-empty-state-title">{"Sin documentos por vencer"}</div>
                    <p class="gi-empty-state-text">{"No hay documentos próximos a vencer en los próximos 30 días."}</p>
                </div>
            } else {
                <DocumentosPorVencerTable docs={(*docs).clone()} />
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct TableProps {
    docs: Vec<DocumentoResponse>,
}

#[function_component(DocumentosPorVencerTable)]
fn documentos_por_vencer_table(props: &TableProps) -> Html {
    html! {
        <div class="gi-card">
            <div class="gi-table-container">
                <table class="gi-table">
                    <thead>
                        <tr>
                            <th>{"Archivo"}</th>
                            <th>{"Tipo"}</th>
                            <th>{"Entidad"}</th>
                            <th>{"Fecha de Vencimiento"}</th>
                            <th>{"Estado"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for props.docs.iter().map(render_row) }
                    </tbody>
                </table>
            </div>
        </div>
    }
}

fn render_row(doc: &DocumentoResponse) -> Html {
    let fecha = doc
        .fecha_vencimiento
        .as_deref()
        .map_or_else(|| "—".to_string(), format_date_display);

    let tipo_label = doc
        .tipo_documento
        .as_deref()
        .unwrap_or("—")
        .replace('_', " ");

    let estado_label = doc.estado_verificacion.as_deref().unwrap_or("pendiente");

    let badge_class = match estado_label {
        "verificado" => "gi-badge gi-badge-success",
        "rechazado" => "gi-badge gi-badge-error",
        _ => "gi-badge gi-badge-warning",
    };

    html! {
        <tr>
            <td>{&doc.filename}</td>
            <td>{tipo_label}</td>
            <td>{&doc.entity_type}</td>
            <td>{fecha}</td>
            <td><span class={badge_class}>{estado_label}</span></td>
        </tr>
    }
}
