use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::error_banner::ErrorBanner;
use crate::services::chatbot;

#[derive(Properties, PartialEq)]
pub struct ConnectionStepProps {
    pub connection_status: String,
    pub connected_phone: Option<String>,
    pub connected_at: Option<String>,
    pub on_status_change: Callback<(String, Option<String>, Option<String>)>,
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
                        on_status_change.emit((
                            resp.status,
                            resp.connected_phone,
                            resp.connected_at,
                        ));
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
                        on_status_change.emit((resp.status, None, None));
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e)),
                }
                loading.set(false);
            });
        })
    };

    let is_connected = props.connection_status == "connected";
    let is_qr_pending = props.connection_status == "qr_pending" || qr_code.is_some();

    html! {
        <section class="gi-card" style="padding: var(--space-5);">
            <header class="mb-4">
                <h3 class="text-base font-semibold text-[var(--text-primary)]">
                    {"Conexión WhatsApp"}
                </h3>
                <p class="text-xs text-[var(--text-tertiary)] mt-1">
                    {"Vincule el número que el bot usará para responder a sus inquilinos."}
                </p>
            </header>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            if is_connected {
                <ConnectedPanel
                    phone={props.connected_phone.clone()}
                    connected_at={props.connected_at.clone()}
                    loading={*loading}
                    on_disconnect={on_disconnect}
                />
            } else if is_qr_pending {
                <QrScanPanel
                    qr_code={(*qr_code).clone()}
                    loading={*loading}
                    on_disconnect={on_disconnect}
                />
            } else {
                <DisconnectedPanel
                    loading={*loading}
                    on_connect={on_connect}
                />
            }
        </section>
    }
}

// ---------------------------------------------------------------------------
// ConnectedPanel
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct ConnectedPanelProps {
    phone: Option<String>,
    connected_at: Option<String>,
    loading: bool,
    on_disconnect: Callback<MouseEvent>,
}

#[component]
fn ConnectedPanel(props: &ConnectedPanelProps) -> Html {
    let phone_display = props
        .phone
        .as_deref()
        .map_or_else(|| "Número no disponible".to_string(), format_phone);
    let connected_label = props
        .connected_at
        .as_deref()
        .map(format_date_es)
        .map_or_else(String::new, |d| format!("Vinculado desde {d}"));

    html! {
        <div class="flex items-start justify-between gap-4">
            <div class="flex items-start gap-3">
                <div
                    class="flex items-center justify-center rounded-full"
                    style="width: 40px; height: 40px; background: var(--color-success-light); color: var(--color-success-dark);"
                    aria-hidden="true"
                >
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="20 6 9 17 4 12"/>
                    </svg>
                </div>
                <div>
                    <div class="text-sm font-semibold text-[var(--text-primary)]">
                        {phone_display}
                    </div>
                    if !connected_label.is_empty() {
                        <div class="text-xs text-[var(--text-tertiary)] mt-0.5">
                            {connected_label}
                        </div>
                    }
                    <div class="text-xs mt-1" style="color: var(--color-success-dark);">
                        {"Sesión activa"}
                    </div>
                </div>
            </div>
            <button
                type="button"
                class="gi-btn gi-btn-secondary"
                onclick={props.on_disconnect.clone()}
                disabled={props.loading}
            >
                {"Desconectar"}
            </button>
        </div>
    }
}

// ---------------------------------------------------------------------------
// QrScanPanel — QR code with numbered steps beside it
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct QrScanPanelProps {
    qr_code: Option<String>,
    loading: bool,
    on_disconnect: Callback<MouseEvent>,
}

#[component]
fn QrScanPanel(props: &QrScanPanelProps) -> Html {
    let qr_visual = props.qr_code.as_ref().map_or_else(
        || html! {
            <div
                class="flex items-center justify-center rounded-lg gi-spinner"
                style="width: 220px; height: 220px; border: 1px dashed var(--border-default);"
                aria-label="Generando código QR"
            />
        },
        |qr| html! {
            <img
                src={format!("data:image/png;base64,{qr}")}
                alt="Código QR para vincular WhatsApp"
                class="rounded-lg"
                style="width: 220px; height: 220px; background: #fff; padding: 8px; image-rendering: pixelated; border: 1px solid var(--border-default);"
            />
        },
    );

    html! {
        <div class="grid gap-5" style="display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1.4fr);">
            <div class="flex flex-col items-center gap-2">
                {qr_visual}
                <p class="text-xs" style="color: var(--color-warning-dark);">
                    {"El código caduca en pocos segundos"}
                </p>
            </div>

            <ol class="flex flex-col gap-3 text-sm" style="counter-reset: step;">
                <StepInstruction
                    step={1}
                    title="Abra WhatsApp"
                    body="Preferiblemente un número dedicado al negocio, no su línea personal."
                />
                <StepInstruction
                    step={2}
                    title="Configuración → Dispositivos vinculados"
                    body="Toque “Vincular un dispositivo”."
                />
                <StepInstruction
                    step={3}
                    title="Escanee el código"
                    body="Apunte la cámara al QR. La vinculación se mantiene activa hasta que cierre la sesión."
                />
                <div class="mt-1">
                    <button
                        type="button"
                        class="gi-btn gi-btn-secondary text-xs"
                        onclick={props.on_disconnect.clone()}
                        disabled={props.loading}
                    >
                        {"Cancelar vinculación"}
                    </button>
                </div>
            </ol>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct StepInstructionProps {
    step: u8,
    title: &'static str,
    body: &'static str,
}

#[component]
fn StepInstruction(props: &StepInstructionProps) -> Html {
    html! {
        <li class="flex items-start gap-3">
            <span
                class="flex items-center justify-center rounded-full text-xs font-semibold flex-shrink-0"
                style="width: 22px; height: 22px; background: var(--color-primary-100); color: var(--color-primary-700);"
                aria-hidden="true"
            >
                {props.step}
            </span>
            <div class="flex-1">
                <div class="font-medium text-[var(--text-primary)]">{props.title}</div>
                <div class="text-xs text-[var(--text-secondary)] mt-0.5 leading-relaxed">
                    {props.body}
                </div>
            </div>
        </li>
    }
}

// ---------------------------------------------------------------------------
// DisconnectedPanel
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct DisconnectedPanelProps {
    loading: bool,
    on_connect: Callback<MouseEvent>,
}

#[component]
fn DisconnectedPanel(props: &DisconnectedPanelProps) -> Html {
    html! {
        <div class="flex items-center justify-between gap-4">
            <div class="flex items-start gap-3">
                <div
                    class="flex items-center justify-center rounded-full"
                    style="width: 40px; height: 40px; background: var(--border-subtle); color: var(--text-secondary);"
                    aria-hidden="true"
                >
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <line x1="5" y1="12" x2="19" y2="12"/>
                    </svg>
                </div>
                <div>
                    <div class="text-sm font-medium text-[var(--text-primary)]">
                        {"Sin conexión a WhatsApp"}
                    </div>
                    <div class="text-xs text-[var(--text-tertiary)] mt-0.5">
                        {"Vincule un número para que el bot pueda recibir y enviar mensajes."}
                    </div>
                </div>
            </div>
            <button
                type="button"
                class="gi-btn gi-btn-primary"
                onclick={props.on_connect.clone()}
                disabled={props.loading}
            >
                {if props.loading { "Generando QR..." } else { "Vincular WhatsApp" }}
            </button>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format a phone string like `18091234567` or `+18091234567` as `+1 809 123 4567`.
fn format_phone(phone: &str) -> String {
    let digits: String = phone.chars().filter(char::is_ascii_digit).collect();
    match digits.len() {
        11 if digits.starts_with('1') => {
            format!("+1 {} {} {}", &digits[1..4], &digits[4..7], &digits[7..11])
        }
        10 => format!("+1 {} {} {}", &digits[0..3], &digits[3..6], &digits[6..10]),
        _ => phone.to_string(),
    }
}

/// Format an ISO-8601 date as `DD/MM/YYYY`. Falls back to the input on parse failure.
fn format_date_es(iso: &str) -> String {
    // Accept `YYYY-MM-DD...` — just swap to `DD/MM/YYYY`.
    let date_part = iso.split('T').next().unwrap_or(iso);
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() == 3 {
        return format!("{}/{}/{}", parts[2], parts[1], parts[0]);
    }
    iso.to_string()
}
