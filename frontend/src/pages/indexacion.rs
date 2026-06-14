use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::TableSkeleton;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_get, api_post};
use crate::types::indexacion::{
    AprobarRenovacionRequest, ContratoProximoVencer, PropuestaRenovacion,
};
use crate::utils::{can_write, format_currency, format_date_display};

fn push_toast(toasts: Option<&ToastContext>, msg: &str, kind: ToastKind) {
    if let Some(t) = toasts {
        t.dispatch(ToastAction::Push(msg.into(), kind));
    }
}

#[function_component(Indexacion)]
pub fn indexacion() -> Html {
    let contratos = use_state(Vec::<ContratoProximoVencer>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let reload = use_state(|| 0u32);
    let selected_contrato = use_state(|| Option::<String>::None);

    let auth = use_context::<AuthContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();

    {
        let contratos = contratos.clone();
        let loading = loading.clone();
        let error = error.clone();
        let reload_val = *reload;
        use_effect_with(reload_val, move |_| {
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                match api_get::<Vec<ContratoProximoVencer>>("/indexacion/proximos-vencer").await {
                    Ok(rows) => contratos.set(rows),
                    Err(e) => error.set(Some(e)),
                }
                loading.set(false);
            });
        });
    }

    let on_select = {
        let selected_contrato = selected_contrato.clone();
        Callback::from(move |id: String| {
            selected_contrato.set(Some(id));
        })
    };

    let on_close_modal = {
        let selected_contrato = selected_contrato.clone();
        Callback::from(move |()| {
            selected_contrato.set(None);
        })
    };

    let on_approved = {
        let reload = reload;
        #[allow(clippy::redundant_clone)]
        let selected_contrato = selected_contrato.clone();
        Callback::from(move |()| {
            selected_contrato.set(None);
            reload.set(*reload + 1);
        })
    };

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Indexación de Contratos"}</h1>
                <p class="gi-page-subtitle">{"Contratos próximos a vencer (60 días) — propuestas de ajuste basadas en IPC y Ley 85-25"}</p>
            </div>

            if *loading {
                <TableSkeleton title_width={AttrValue::from("200px")} rows={5} columns={6} />
            } else if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} />
            } else if contratos.is_empty() {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-icon">{"📋"}</div>
                    <div class="gi-empty-state-title">{"Sin contratos por renovar"}</div>
                    <p class="gi-empty-state-text">{"No hay contratos con vencimiento en los próximos 60 días."}</p>
                </div>
            } else {
                <ContratosProximosTable
                    contratos={(*contratos).clone()}
                    user_rol={user_rol.clone()}
                    on_select={on_select}
                />
            }

            if let Some(ref contrato_id) = *selected_contrato {
                <PropuestaModal
                    contrato_id={contrato_id.clone()}
                    on_close={on_close_modal}
                    on_approved={on_approved}
                />
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ContratosProximosTableProps {
    contratos: Vec<ContratoProximoVencer>,
    user_rol: String,
    on_select: Callback<String>,
}

#[function_component(ContratosProximosTable)]
fn contratos_proximos_table(props: &ContratosProximosTableProps) -> Html {
    html! {
        <div class="gi-card">
            <div class="gi-table-container">
                <table class="gi-table">
                    <thead>
                        <tr>
                            <th>{"Propiedad"}</th>
                            <th>{"Inquilino"}</th>
                            <th>{"Fecha Fin"}</th>
                            <th>{"Días Restantes"}</th>
                            <th>{"Monto Actual"}</th>
                            <th>{"Acciones"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for props.contratos.iter().map(|c| {
                            render_contrato_row(c, &props.user_rol, &props.on_select)
                        })}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

fn render_contrato_row(
    contrato: &ContratoProximoVencer,
    user_rol: &str,
    on_select: &Callback<String>,
) -> Html {
    let urgency_class = if contrato.dias_restantes <= 15 {
        "gi-badge gi-badge-error"
    } else if contrato.dias_restantes <= 30 {
        "gi-badge gi-badge-warning"
    } else {
        "gi-badge gi-badge-neutral"
    };

    let on_click = {
        let id = contrato.contrato_id.clone();
        let cb = on_select.clone();
        Callback::from(move |_: MouseEvent| cb.emit(id.clone()))
    };

    html! {
        <tr>
            <td>{&contrato.propiedad_titulo}</td>
            <td>{&contrato.inquilino_nombre}</td>
            <td class="tabular-nums">{format_date_display(&contrato.fecha_fin)}</td>
            <td>
                <span class={urgency_class}>
                    {format!("{} días", contrato.dias_restantes)}
                </span>
            </td>
            <td class="tabular-nums">{format_currency(&contrato.moneda, contrato.monto_actual)}</td>
            <td>
                if can_write(user_rol) {
                    <button onclick={on_click} class="gi-btn gi-btn-sm gi-btn-primary">
                        {"Ver Propuesta"}
                    </button>
                }
            </td>
        </tr>
    }
}

#[derive(Properties, PartialEq)]
struct PropuestaModalProps {
    contrato_id: String,
    on_close: Callback<()>,
    on_approved: Callback<()>,
}

#[function_component(PropuestaModal)]
fn propuesta_modal(props: &PropuestaModalProps) -> Html {
    let propuesta = use_state(|| Option::<PropuestaRenovacion>::None);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let override_monto = use_state(String::new);
    let submitting = use_state(|| false);
    let submit_error = use_state(|| Option::<String>::None);
    let toasts = use_context::<ToastContext>();

    let contrato_id = props.contrato_id.clone();

    {
        let propuesta = propuesta.clone();
        let loading = loading.clone();
        let error = error.clone();
        let contrato_id = contrato_id.clone();
        use_effect_with(contrato_id.clone(), move |_| {
            spawn_local(async move {
                match api_get::<PropuestaRenovacion>(&format!(
                    "/indexacion/propuesta/{contrato_id}"
                ))
                .await
                {
                    Ok(p) => propuesta.set(Some(p)),
                    Err(e) => error.set(Some(e)),
                }
                loading.set(false);
            });
        });
    }

    let on_close = {
        let cb = props.on_close.clone();
        Callback::from(move |_: MouseEvent| cb.emit(()))
    };

    let on_approve = {
        let propuesta = propuesta.clone();
        let submitting = submitting.clone();
        let submit_error = submit_error.clone();
        let on_approved = props.on_approved.clone();
        let contrato_id = contrato_id.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            if *submitting {
                return;
            }
            let Some(ref p) = *propuesta else { return };
            let monto = format!("{:.2}", p.monto_maximo);
            let submitting = submitting.clone();
            let submit_error = submit_error.clone();
            let on_approved = on_approved.clone();
            let contrato_id = contrato_id.clone();
            let toasts = toasts.clone();
            submitting.set(true);
            submit_error.set(None);
            spawn_local(async move {
                let req = AprobarRenovacionRequest {
                    monto_aprobado: monto,
                };
                match api_post::<serde_json::Value, _>(
                    &format!("/indexacion/aprobar/{contrato_id}"),
                    &req,
                )
                .await
                {
                    Ok(_) => {
                        push_toast(
                            toasts.as_ref(),
                            "Renovación aprobada exitosamente",
                            ToastKind::Success,
                        );
                        on_approved.emit(());
                    }
                    Err(e) => {
                        submit_error.set(Some(e));
                        submitting.set(false);
                    }
                }
            });
        })
    };

    let on_override = {
        let override_monto = override_monto.clone();
        let submitting = submitting.clone();
        let submit_error = submit_error.clone();
        let on_approved = props.on_approved.clone();
        #[allow(clippy::redundant_clone)]
        let contrato_id = contrato_id.clone();
        #[allow(clippy::redundant_clone)]
        let toasts = toasts.clone();
        let propuesta = propuesta.clone();
        Callback::from(move |_: MouseEvent| {
            if *submitting {
                return;
            }
            let monto_str = (*override_monto).clone();
            let Ok(monto_val) = monto_str.parse::<f64>() else {
                submit_error.set(Some("Monto inválido".into()));
                return;
            };
            if let Some(ref p) = *propuesta {
                if monto_val > p.monto_maximo {
                    submit_error.set(Some(
                        "El monto no puede exceder el máximo permitido por Ley 85-25".into(),
                    ));
                    return;
                }
                if monto_val < p.monto_actual {
                    submit_error.set(Some("El monto no puede ser menor al monto actual".into()));
                    return;
                }
            }
            let submitting = submitting.clone();
            let submit_error = submit_error.clone();
            let on_approved = on_approved.clone();
            let contrato_id = contrato_id.clone();
            let toasts = toasts.clone();
            submitting.set(true);
            submit_error.set(None);
            spawn_local(async move {
                let req = AprobarRenovacionRequest {
                    monto_aprobado: monto_str,
                };
                match api_post::<serde_json::Value, _>(
                    &format!("/indexacion/aprobar/{contrato_id}"),
                    &req,
                )
                .await
                {
                    Ok(_) => {
                        push_toast(
                            toasts.as_ref(),
                            "Renovación aprobada con monto personalizado",
                            ToastKind::Success,
                        );
                        on_approved.emit(());
                    }
                    Err(e) => {
                        submit_error.set(Some(e));
                        submitting.set(false);
                    }
                }
            });
        })
    };

    let on_override_input = {
        let override_monto = override_monto.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            override_monto.set(input.value());
        })
    };

    html! {
        <div class="gi-modal-overlay">
            <div class="gi-modal" style="max-width: 560px;">
                <h3 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">
                    {"Propuesta de Renovación"}
                </h3>

                if *loading {
                    <div class="gi-loading-container">
                        <div class="gi-spinner"></div>
                        <span>{"Calculando propuesta..."}</span>
                    </div>
                } else if let Some(err) = (*error).as_ref() {
                    <ErrorBanner message={err.clone()} />
                } else if let Some(ref p) = *propuesta {
                    <PropuestaDetail
                        propuesta={p.clone()}
                        override_monto={(*override_monto).clone()}
                        on_override_input={on_override_input}
                        submit_error={(*submit_error).clone()}
                        submitting={*submitting}
                        on_approve={on_approve}
                        on_override={on_override}
                    />
                }

                <div style="display: flex; justify-content: flex-end; margin-top: var(--space-4);">
                    <button onclick={on_close} class="gi-btn gi-btn-ghost" disabled={*submitting}>
                        {"Cerrar"}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct PropuestaDetailProps {
    propuesta: PropuestaRenovacion,
    override_monto: String,
    on_override_input: Callback<InputEvent>,
    submit_error: Option<String>,
    submitting: bool,
    on_approve: Callback<MouseEvent>,
    on_override: Callback<MouseEvent>,
}

#[function_component(PropuestaDetail)]
fn propuesta_detail(props: &PropuestaDetailProps) -> Html {
    let p = &props.propuesta;

    html! {
        <>
            if p.datos_stale {
                <div class="gi-alert gi-alert-warning" style="margin-bottom: var(--space-4);">
                    {"⚠️ Los datos de IPC están desactualizados (más de 90 días). La propuesta se basa en datos en caché."}
                </div>
            }

            <div style="padding: var(--space-4); background: var(--bg-subtle); border-radius: var(--radius-md); margin-bottom: var(--space-4);">
                <h4 style="font-weight: 600; margin-bottom: var(--space-3); color: var(--text-primary);">
                    {"Información de Auditoría"}
                </h4>
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-3);">
                    <div>
                        <span class="gi-label" style="display: block; margin-bottom: var(--space-1);">{"IPC Interanual"}</span>
                        <span class="tabular-nums" style="font-weight: 500;">{format!("{:.2}%", p.ipc_porcentaje)}</span>
                    </div>
                    <div>
                        <span class="gi-label" style="display: block; margin-bottom: var(--space-1);">{"Tope Legal (Ley 85-25)"}</span>
                        <span class="tabular-nums" style="font-weight: 500;">{"10%"}</span>
                        if p.tope_aplicado {
                            <span class="gi-badge gi-badge-warning" style="margin-left: var(--space-2);">{"Aplicado"}</span>
                        }
                    </div>
                    <div>
                        <span class="gi-label" style="display: block; margin-bottom: var(--space-1);">{"Monto Actual"}</span>
                        <span class="tabular-nums" style="font-weight: 500;">{format!("RD$ {:.2}", p.monto_actual)}</span>
                    </div>
                    <div>
                        <span class="gi-label" style="display: block; margin-bottom: var(--space-1);">{"Monto Propuesto (máximo)"}</span>
                        <span class="tabular-nums" style="font-weight: 500; color: var(--color-success);">{format!("RD$ {:.2}", p.monto_maximo)}</span>
                    </div>
                </div>
            </div>

            if let Some(ref err) = props.submit_error {
                <div class="gi-alert gi-alert-error" style="margin-bottom: var(--space-3);">
                    {err}
                </div>
            }

            <div style="display: flex; flex-direction: column; gap: var(--space-3);">
                <button
                    onclick={props.on_approve.clone()}
                    disabled={props.submitting}
                    class="gi-btn gi-btn-primary"
                    style="width: 100%;"
                >
                    {if props.submitting { "Procesando..." } else { "Aprobar Monto Propuesto" }}
                </button>

                <div style="border-top: 1px solid var(--border-subtle); padding-top: var(--space-3);">
                    <label class="gi-label" style="margin-bottom: var(--space-2); display: block;">
                        {"Monto personalizado (solo menor al máximo, per Ley 85-25)"}
                    </label>
                    <div style="display: flex; gap: var(--space-2);">
                        <input
                            type="number"
                            step="0.01"
                            min={format!("{:.2}", p.monto_actual)}
                            max={format!("{:.2}", p.monto_maximo)}
                            value={props.override_monto.clone()}
                            oninput={props.on_override_input.clone()}
                            class="gi-input"
                            placeholder={format!("Entre {:.2} y {:.2}", p.monto_actual, p.monto_maximo)}
                            disabled={props.submitting}
                        />
                        <button
                            onclick={props.on_override.clone()}
                            disabled={props.submitting || props.override_monto.is_empty()}
                            class="gi-btn gi-btn-secondary"
                        >
                            {"Aprobar"}
                        </button>
                    </div>
                </div>
            </div>
        </>
    }
}
