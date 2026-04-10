use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ActivationStepProps {
    pub activo: bool,
    pub connection_status: String,
    pub on_toggle: Callback<bool>,
}

#[component]
pub fn ActivationStep(props: &ActivationStepProps) -> Html {
    let is_connected = props.connection_status == "connected";

    let on_toggle = {
        let on_toggle = props.on_toggle.clone();
        let activo = props.activo;
        Callback::from(move |_: InputEvent| {
            on_toggle.emit(!activo);
        })
    };

    html! {
        <div class="gi-card" style="padding: var(--space-5);">
            <h3 class="text-base font-semibold mb-4">
                {"Activación"}
            </h3>

            if !is_connected {
                <div class="rounded-lg mb-4 p-3" style="background: var(--surface-raised);">
                    <p class="text-sm text-amber-600">
                        {"⚠ Debe conectar WhatsApp antes de activar el chatbot."}
                    </p>
                </div>
            }

            <div class="flex items-center justify-between p-4 rounded-lg border border-[var(--border-default)]">
                <div>
                    <div class="text-sm font-medium text-[var(--text-primary)]">
                        {"Chatbot Activo"}
                    </div>
                    <div class="text-xs text-[var(--text-tertiary)]">
                        {if props.activo {
                            "El chatbot está respondiendo mensajes de inquilinos"
                        } else {
                            "El chatbot está desactivado y no responderá mensajes"
                        }}
                    </div>
                </div>
                <ToggleSwitch
                    checked={props.activo}
                    disabled={!is_connected}
                    on_toggle={on_toggle}
                />
            </div>

            if props.activo && is_connected {
                <div class="mt-4 p-3 rounded-lg" style="background: var(--surface-raised);">
                    <p class="text-sm text-emerald-600">
                        {"✓ El chatbot está activo y listo para recibir mensajes."}
                    </p>
                </div>
            }
        </div>
    }
}

// ---------------------------------------------------------------------------
// ToggleSwitch — accessible toggle with ARIA
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct ToggleSwitchProps {
    checked: bool,
    disabled: bool,
    on_toggle: Callback<InputEvent>,
}

#[component]
fn ToggleSwitch(props: &ToggleSwitchProps) -> Html {
    let track_bg = if props.checked {
        "var(--color-success)"
    } else {
        "var(--border-default)"
    };

    let cursor = if props.disabled {
        "not-allowed"
    } else {
        "pointer"
    };

    let knob_left = if props.checked { "23px" } else { "3px" };

    html! {
        <label
            class="relative inline-block"
            style="width: 44px; height: 24px;"
            role="switch"
            aria-checked={props.checked.to_string()}
        >
            <input
                type="checkbox"
                checked={props.checked}
                oninput={props.on_toggle.clone()}
                disabled={props.disabled}
                class="opacity-0 w-0 h-0 absolute"
            />
            <span
                class="absolute inset-0 rounded-xl transition-colors"
                style={format!("background-color: {track_bg}; cursor: {cursor};")}
            >
                <span
                    class="absolute rounded-full bg-white transition-all"
                    style={format!("height: 18px; width: 18px; left: {knob_left}; bottom: 3px;")}
                />
            </span>
        </label>
    }
}
