use yew::prelude::*;

use crate::components::chatbot::toggle_switch::{ToggleSize, ToggleSwitch};

#[derive(Properties, PartialEq)]
pub struct BotStateCardProps {
    pub activo: bool,
    pub connection_status: String,
    pub capabilities_enabled: usize,
    pub capabilities_total: usize,
    pub sender_scope: AttrValue,
    pub conversations_count: usize,
    pub pending_receipts: usize,
    pub on_toggle: Callback<bool>,
}

#[component]
pub fn BotStateCard(props: &BotStateCardProps) -> Html {
    let is_connected = props.connection_status == "connected";
    let can_activate = is_connected;

    let on_toggle = {
        let on_toggle = props.on_toggle.clone();
        let activo = props.activo;
        Callback::from(move |_: InputEvent| {
            on_toggle.emit(!activo);
        })
    };

    let (state_label, state_color, state_bg) = if props.activo && is_connected {
        (
            "Activo",
            "var(--color-success-dark)",
            "var(--color-success-light)",
        )
    } else if !is_connected {
        (
            "Sin conexión",
            "var(--color-error-dark)",
            "var(--color-error-light)",
        )
    } else {
        ("Pausado", "var(--text-secondary)", "var(--border-subtle)")
    };

    html! {
        <section
            class="rounded-lg p-5"
            style="background: var(--surface-raised); border: 1px solid var(--border-default);"
        >
            <div class="flex items-start justify-between gap-4 mb-4">
                <div>
                    <div class="flex items-center gap-2 mb-1">
                        <span
                            class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-semibold"
                            style={format!("background: {state_bg}; color: {state_color};")}
                        >
                            <span
                                class="inline-block rounded-full"
                                style={format!("width: 6px; height: 6px; background: {state_color};")}
                                aria-hidden="true"
                            />
                            {state_label}
                        </span>
                    </div>
                    <h2 class="text-xl font-semibold text-[var(--text-primary)]" style="font-family: var(--font-display);">
                        {"Chatbot de WhatsApp"}
                    </h2>
                    <p class="text-sm text-[var(--text-secondary)] mt-1">
                        {state_description(props.activo, is_connected)}
                    </p>
                </div>

                <div class="flex flex-col items-end gap-1">
                    <ToggleSwitch
                        checked={props.activo && is_connected}
                        disabled={!can_activate}
                        on_toggle={on_toggle}
                        label={AttrValue::from("Activar chatbot")}
                        size={ToggleSize::Lg}
                    />
                    <span class="text-xs text-[var(--text-tertiary)]">
                        {if props.activo && is_connected { "Respondiendo" } else { "Sin responder" }}
                    </span>
                </div>
            </div>

            if !is_connected {
                <div
                    class="flex items-start gap-2 px-3 py-2 rounded-md text-sm"
                    style="background: var(--color-warning-light); color: var(--color-warning-dark);"
                    role="alert"
                >
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="flex-shrink-0 mt-0.5">
                        <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
                        <line x1="12" y1="9" x2="12" y2="13"/>
                        <line x1="12" y1="17" x2="12.01" y2="17"/>
                    </svg>
                    <span>
                        {"Vincule WhatsApp antes de activar el chatbot. Vea la sección "}
                        <strong>{"Conexión"}</strong>
                        {" abajo."}
                    </span>
                </div>
            }

            <div
                class="grid mt-4"
                style="grid-template-columns: repeat(3, minmax(0, 1fr)); gap: var(--space-4); border-top: 1px solid var(--border-subtle); padding-top: var(--space-4);"
            >
                <StatChip
                    label="Capacidades activas"
                    value={format!("{} / {}", props.capabilities_enabled, props.capabilities_total)}
                />
                <StatChip
                    label="Responde a"
                    value={props.sender_scope.to_string()}
                />
                <StatChip
                    label="Esta semana"
                    value={format!(
                        "{} conv · {} recibos",
                        props.conversations_count, props.pending_receipts,
                    )}
                />
            </div>
        </section>
    }
}

const fn state_description(activo: bool, is_connected: bool) -> &'static str {
    match (activo, is_connected) {
        (true, true) => "El bot está respondiendo mensajes entrantes según la configuración.",
        (false, true) => "El bot está configurado y conectado, pero no está respondiendo.",
        (_, false) => "El bot no puede responder sin una sesión de WhatsApp vinculada.",
    }
}

#[derive(Properties, PartialEq)]
struct StatChipProps {
    label: &'static str,
    value: String,
}

#[component]
fn StatChip(props: &StatChipProps) -> Html {
    html! {
        <div class="flex flex-col">
            <span class="text-xs text-[var(--text-tertiary)] uppercase tracking-wide">
                {props.label}
            </span>
            <span class="text-sm font-semibold text-[var(--text-primary)] mt-1">
                {&props.value}
            </span>
        </div>
    }
}
