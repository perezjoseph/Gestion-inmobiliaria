use yew::prelude::*;

use crate::types::chatbot::Capabilities;

#[derive(Properties, PartialEq)]
pub struct CapabilitiesStepProps {
    pub capabilities: Capabilities,
    pub on_change: Callback<Capabilities>,
}

#[component]
pub fn CapabilitiesStep(props: &CapabilitiesStepProps) -> Html {
    let caps = use_state(|| props.capabilities.clone());

    let make_toggle = |field: &'static str| {
        let caps = caps.clone();
        let on_change = props.on_change.clone();
        Callback::from(move |_: InputEvent| {
            let mut new_caps = (*caps).clone();
            match field {
                "receipt_ocr" => new_caps.receipt_ocr = !new_caps.receipt_ocr,
                "balance_queries" => new_caps.balance_queries = !new_caps.balance_queries,
                "payment_reminders" => new_caps.payment_reminders = !new_caps.payment_reminders,
                "maintenance_requests" => {
                    new_caps.maintenance_requests = !new_caps.maintenance_requests;
                }
                "human_handoff" => new_caps.human_handoff = !new_caps.human_handoff,
                _ => {}
            }
            caps.set(new_caps.clone());
            on_change.emit(new_caps);
        })
    };

    let toggle_row = |label: &str, description: &str, checked: bool, field: &'static str| {
        html! {
            <label style="display: flex; align-items: flex-start; gap: var(--space-3); cursor: pointer; padding: var(--space-3); border-radius: var(--radius-md); border: 1px solid var(--border-default);">
                <input
                    type="checkbox"
                    checked={checked}
                    oninput={make_toggle(field)}
                    style="margin-top: 2px;"
                />
                <div>
                    <div style="font-size: var(--text-sm); font-weight: 500; color: var(--text-primary);">
                        {label}
                    </div>
                    <div style="font-size: var(--text-xs); color: var(--text-tertiary);">
                        {description}
                    </div>
                </div>
            </label>
        }
    };

    html! {
        <div class="gi-card" style="padding: var(--space-5);">
            <h3 style="font-size: var(--text-base); font-weight: 600; margin-bottom: var(--space-4);">
                {"Capacidades"}
            </h3>

            <div style="display: flex; flex-direction: column; gap: var(--space-3);">
                {toggle_row(
                    "Recibos de Pago (OCR)",
                    "Permite a los inquilinos enviar fotos de recibos para registro automático",
                    caps.receipt_ocr,
                    "receipt_ocr",
                )}
                {toggle_row(
                    "Consulta de Balance",
                    "Permite a los inquilinos consultar su saldo pendiente",
                    caps.balance_queries,
                    "balance_queries",
                )}
                {toggle_row(
                    "Recordatorios de Pago",
                    "Envía recordatorios automáticos de pagos próximos a vencer",
                    caps.payment_reminders,
                    "payment_reminders",
                )}
                {toggle_row(
                    "Solicitudes de Mantenimiento",
                    "Permite a los inquilinos reportar problemas de mantenimiento",
                    caps.maintenance_requests,
                    "maintenance_requests",
                )}
                {toggle_row(
                    "Transferencia a Humano",
                    "Permite al inquilino solicitar hablar con una persona",
                    caps.human_handoff,
                    "human_handoff",
                )}
            </div>
        </div>
    }
}
