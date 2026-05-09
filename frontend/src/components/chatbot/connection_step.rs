use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::error_banner::ErrorBanner;
use crate::services::chatbot;

#[derive(Properties, PartialEq)]
pub struct ConnectionStepProps {
    pub connection_status: String,
    pub on_status_change: Callback<String>,
}

#[component]
pub fn ConnectionStep(props: &ConnectionStepProps) -> Html {
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);
    let qr_code = use_state(|| Option::<String>::None);

    let on_connect = {
        let error = error.clone();
        let loading = loading.clone();
        let qr_code = qr_code.clone();
        let on_status_change = props.on_status_change.clone();
        Callback::from(move |_: MouseEvent| {
            let error = error.clone();
            let loading = loading.clone();
            let qr_code = qr_code.clone();
            let on_status_change = on_status_change.clone();
            loading.set(true);
            spawn_local(async move {
                match chatbot::connect().await {
                    Ok(resp) => {
                        qr_code.set(resp.qr_code.clone());
                        on_status_change.emit(resp.status);
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e)),
                }
                loading.set(false);
            });
        })
    };

    let on_disconnect = {
        let error = error.clone();
        let loading = loading.clone();
        let qr_code = qr_code.clone();
        let on_status_change = props.on_status_change.clone();
        Callback::from(move |_: MouseEvent| {
            let error = error.clone();
            let loading = loading.clone();
            let qr_code = qr_code.clone();
            let on_status_change = on_status_change.clone();
            loading.set(true);
            spawn_local(async move {
                match chatbot::disconnect().await {
                    Ok(resp) => {
                        qr_code.set(None);
                        on_status_change.emit(resp.status);
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e)),
                }
                loading.set(false);
            });
        })
    };

    let status_color = match props.connection_status.as_str() {
        "connected" => "var(--color-success)",
        "qr_pending" => "var(--color-warning)",
        "logged_out" => "var(--color-error)",
        _ => "var(--text-tertiary)",
    };

    let status_label = match props.connection_status.as_str() {
        "connected" => "Conectado",
        "qr_pending" => "Esperando escaneo QR",
        "disconnected" => "Desconectado",
        "logged_out" => "Sesión cerrada",
        _ => "Desconocido",
    };

    html! {
        <div class="gi-card" style="padding: var(--space-5);">
            <h3 style="font-size: var(--text-base); font-weight: 600; margin-bottom: var(--space-4);">
                {"Conexión WhatsApp"}
            </h3>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <div style="display: flex; align-items: center; gap: var(--space-2); margin-bottom: var(--space-4);">
                <span style={format!("width: 10px; height: 10px; border-radius: 50%; background: {status_color};")}></span>
                <span style="font-size: var(--text-sm); color: var(--text-secondary);">
                    {status_label}
                </span>
            </div>

            if let Some(qr) = (*qr_code).as_ref() {
                <div style="display: flex; justify-content: center; margin-bottom: var(--space-4);">
                    <img
                        src={format!("data:image/png;base64,{qr}")}
                        alt="Código QR de WhatsApp"
                        style="width: 256px; height: 256px; border-radius: var(--radius-md);"
                    />
                </div>
                <p style="font-size: var(--text-xs); color: var(--text-tertiary); text-align: center;">
                    {"Escanee el código QR con WhatsApp para conectar."}
                </p>
            }

            <div style="display: flex; gap: var(--space-3); margin-top: var(--space-4);">
                if props.connection_status != "connected" {
                    <button
                        class="gi-btn gi-btn-primary"
                        onclick={on_connect}
                        disabled={*loading}
                    >
                        {if *loading { "Conectando..." } else { "Conectar" }}
                    </button>
                }
                if props.connection_status == "connected" || props.connection_status == "qr_pending" {
                    <button
                        class="gi-btn gi-btn-secondary"
                        onclick={on_disconnect}
                        disabled={*loading}
                    >
                        {"Desconectar"}
                    </button>
                }
            </div>
        </div>
    }
}
