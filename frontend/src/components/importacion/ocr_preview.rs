use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::services::api::{confirmar_preview, descartar_preview};
use crate::types::importacion::ImportResult;
use crate::types::ocr::{ConfirmPreviewRequest, ImportPreview};

#[derive(Properties, PartialEq)]
pub struct OcrPreviewProps {
    pub preview: ImportPreview,
    pub on_confirmed: Callback<ImportResult>,
    pub on_discarded: Callback<()>,
}

#[component]
pub fn OcrPreview(props: &OcrPreviewProps) -> Html {
    let field_values: UseStateHandle<HashMap<String, String>> = {
        let initial: HashMap<String, String> = props
            .preview
            .fields
            .iter()
            .map(|f| (f.name.clone(), f.value.clone()))
            .collect();
        use_state(|| initial)
    };
    let submitting = use_state(|| false);
    let error = use_state(|| Option::<String>::None);

    let on_field_change = {
        let field_values = field_values.clone();
        Callback::from(move |(name, value): (String, String)| {
            let mut vals = (*field_values).clone();
            vals.insert(name, value);
            field_values.set(vals);
        })
    };

    let on_confirmar = {
        let preview_id = props.preview.preview_id.clone();
        let field_values = field_values.clone();
        let submitting = submitting.clone();
        let error = error.clone();
        let on_confirmed = props.on_confirmed.clone();
        Callback::from(move |_: MouseEvent| {
            let preview_id = preview_id.clone();
            let corrections: HashMap<String, String> = (*field_values).clone();
            let submitting = submitting.clone();
            let error = error.clone();
            let on_confirmed = on_confirmed.clone();

            submitting.set(true);
            error.set(None);

            spawn_local(async move {
                let request = ConfirmPreviewRequest {
                    preview_id,
                    corrections: Some(corrections),
                };
                match confirmar_preview(&request).await {
                    Ok(val) => {
                        if let Ok(result) = serde_json::from_value::<ImportResult>(val) {
                            on_confirmed.emit(result);
                        } else {
                            on_confirmed.emit(ImportResult {
                                total_filas: 1,
                                exitosos: 1,
                                fallidos: vec![],
                            });
                        }
                    }
                    Err(err) => error.set(Some(err)),
                }
                submitting.set(false);
            });
        })
    };

    let on_descartar = {
        let preview_id = props.preview.preview_id.clone();
        let submitting = submitting.clone();
        let error = error.clone();
        let on_discarded = props.on_discarded.clone();
        Callback::from(move |_: MouseEvent| {
            let preview_id = preview_id.clone();
            let submitting = submitting.clone();
            let error = error.clone();
            let on_discarded = on_discarded.clone();

            submitting.set(true);
            error.set(None);

            spawn_local(async move {
                match descartar_preview(&preview_id).await {
                    Ok(()) => on_discarded.emit(()),
                    Err(err) => error.set(Some(err)),
                }
                submitting.set(false);
            });
        })
    };

    let doc_label = match props.preview.document_type.as_str() {
        "deposito_bancario" => "Depósito Bancario",
        "recibo_gasto" => "Recibo de Gasto",
        _ => "Documento",
    };

    html! {
        <div class="gi-card" style="padding: var(--space-5); margin-top: var(--space-4);">
            <h3 style="font-size: var(--text-base); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-3);">
                {"Vista Previa OCR — "}{doc_label}
            </h3>
            <p style="font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-4);">
                {"Revise y corrija los datos extraídos antes de confirmar."}
            </p>

            if let Some(err) = (*error).as_ref() {
                <div style="background: var(--color-error-bg, #fef2f2); border: 1px solid var(--color-error, #ef4444); border-radius: 6px; padding: var(--space-3); margin-bottom: var(--space-4); color: var(--color-error, #ef4444); font-size: var(--text-sm);">
                    {err}
                </div>
            }

            <div style="display: grid; gap: var(--space-3);">
                { for props.preview.fields.iter().map(|field| {
                    let name = field.name.clone();
                    let confidence = field.confidence;
                    let current_value = field_values
                        .get(&field.name)
                        .cloned()
                        .unwrap_or_default();
                    let on_change = on_field_change.clone();
                    let low_confidence = confidence < 0.7;
                    let border_style = if low_confidence {
                        "border: 2px solid #f59e0b;"
                    } else {
                        ""
                    };
                    let bg_style = if low_confidence {
                        "background-color: #fffbeb;"
                    } else {
                        ""
                    };

                    html! {
                        <div>
                            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-1);">
                                <label class="gi-label" style="margin-bottom: 0;">
                                    {&field.label}
                                </label>
                                <span style={format!(
                                    "font-size: var(--text-xs); {}",
                                    if low_confidence { "color: #f59e0b; font-weight: 600;" } else { "color: var(--text-tertiary);" }
                                )}>
                                    {format!("Confianza: {:.0}%", confidence * 100.0)}
                                </span>
                            </div>
                            <input
                                type="text"
                                class="gi-input"
                                style={format!("{border_style} {bg_style}")}
                                value={current_value}
                                oninput={Callback::from(move |e: InputEvent| {
                                    let el: web_sys::HtmlInputElement = e.target_unchecked_into();
                                    on_change.emit((name.clone(), el.value()));
                                })}
                            />
                            if low_confidence {
                                <span style="font-size: var(--text-xs); color: #f59e0b;">
                                    {"⚠ Baja confianza — verifique este campo"}
                                </span>
                            }
                        </div>
                    }
                })}
            </div>

            <div style="display: flex; gap: var(--space-3); margin-top: var(--space-5);">
                <button
                    class="gi-btn gi-btn-primary"
                    onclick={on_confirmar}
                    disabled={*submitting}
                >
                    { if *submitting { "Procesando..." } else { "Confirmar" } }
                </button>
                <button
                    class="gi-btn gi-btn-secondary"
                    onclick={on_descartar}
                    disabled={*submitting}
                >
                    {"Descartar"}
                </button>
            </div>
        </div>
    }
}
