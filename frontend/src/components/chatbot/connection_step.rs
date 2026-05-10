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

    let (dot_class, status_label) = match props.connection_status.as_str() {
        "connected" => ("bg-emerald-500", "Conectado"),
        "qr_pending" => ("bg-amber-400", "Esperando escaneo QR"),
        "disconnected" => ("bg-red-500", "Desconectado"),
        "logged_out" => ("bg-red-500", "Sesión cerrada"),
        _ => ("bg-gray-400", "Desconocido"),
    };

    html! {
        <div class="gi-card" style="padding: var(--space-5);">
            <h3 class="text-base font-semibold mb-4">
                {"Conexión WhatsApp"}
            </h3>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <div class="flex items-center gap-2 mb-4">
                <span class={classes!("w-2.5", "h-2.5", "rounded-full", dot_class)} />
                <span class="text-sm text-[var(--text-secondary)]">
                    {status_label}
                </span>
            </div>

            if let Some(qr) = (*qr_code).as_ref() {
                <div class="flex justify-center mb-4">
                    <img
                        src={format!("data:image/png;base64,{qr}")}
                        alt="Código QR de WhatsApp"
                        class="rounded-lg bg-white p-2"
                        style="width: 256px; height: 256px; image-rendering: pixelated;"
                    />
                </div>
                <p class="text-xs text-[var(--text-tertiary)] text-center">
                    {"Escanee el código QR con WhatsApp para conectar."}
                </p>
            }

            <div class="flex gap-3 mt-4">
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
