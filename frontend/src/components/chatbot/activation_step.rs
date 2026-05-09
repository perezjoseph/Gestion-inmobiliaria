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
            <h3 style="font-size: var(--text-base); font-weight: 600; margin-bottom: var(--space-4);">
                {"Activación"}
            </h3>

            if !is_connected {
                <div style="padding: var(--space-3); background: var(--surface-raised); border-radius: var(--radius-md); margin-bottom: var(--space-4);">
                    <p style="font-size: var(--text-sm); color: var(--color-warning);">
                        {"⚠️ Debe conectar WhatsApp antes de activar el chatbot."}
                    </p>
                </div>
            }

            <div style="display: flex; align-items: center; justify-content: space-between; padding: var(--space-4); border: 1px solid var(--border-default); border-radius: var(--radius-md);">
                <div>
                    <div style="font-size: var(--text-sm); font-weight: 500; color: var(--text-primary);">
                        {"Chatbot Activo"}
                    </div>
                    <div style="font-size: var(--text-xs); color: var(--text-tertiary);">
                        {if props.activo {
                            "El chatbot está respondiendo mensajes de inquilinos"
                        } else {
                            "El chatbot está desactivado y no responderá mensajes"
                        }}
                    </div>
                </div>
                <label style="position: relative; display: inline-block; width: 44px; height: 24px;">
                    <input
                        type="checkbox"
                        checked={props.activo}
                        oninput={on_toggle}
                        disabled={!is_connected}
                        style="opacity: 0; width: 0; height: 0;"
                    />
                    <span style={format!(
                        "position: absolute; cursor: {}; top: 0; left: 0; right: 0; bottom: 0; background-color: {}; border-radius: 12px; transition: 0.3s;",
                        if is_connected { "pointer" } else { "not-allowed" },
                        if props.activo { "var(--color-success)" } else { "var(--border-default)" }
                    )}>
                        <span style={format!(
                            "position: absolute; height: 18px; width: 18px; left: {}; bottom: 3px; background-color: white; border-radius: 50%; transition: 0.3s;",
                            if props.activo { "23px" } else { "3px" }
                        )}></span>
                    </span>
                </label>
            </div>

            if props.activo && is_connected {
                <div style="margin-top: var(--space-4); padding: var(--space-3); background: var(--surface-raised); border-radius: var(--radius-md);">
                    <p style="font-size: var(--text-sm); color: var(--color-success);">
                        {"✓ El chatbot está activo y listo para recibir mensajes."}
                    </p>
                </div>
            }
        </div>
    }
}
