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

    html! {
        <div class="gi-card" style="padding: var(--space-5);">
            <h3 class="text-base font-semibold mb-4">
                {"Capacidades"}
            </h3>

            <div class="flex flex-col gap-3">
                <ToggleRow
                    label="Recibos de Pago (OCR)"
                    description="Permite a los inquilinos enviar fotos de recibos para registro automático"
                    checked={caps.receipt_ocr}
                    on_toggle={make_toggle("receipt_ocr")}
                />
                <ToggleRow
                    label="Consulta de Balance"
                    description="Permite a los inquilinos consultar su saldo pendiente"
                    checked={caps.balance_queries}
                    on_toggle={make_toggle("balance_queries")}
                />
                <ToggleRow
                    label="Recordatorios de Pago"
                    description="Envía recordatorios automáticos de pagos próximos a vencer"
                    checked={caps.payment_reminders}
                    on_toggle={make_toggle("payment_reminders")}
                />
                <ToggleRow
                    label="Solicitudes de Mantenimiento"
                    description="Permite a los inquilinos reportar problemas de mantenimiento"
                    checked={caps.maintenance_requests}
                    on_toggle={make_toggle("maintenance_requests")}
                />
                <ToggleRow
                    label="Transferencia a Humano"
                    description="Permite al inquilino solicitar hablar con una persona"
                    checked={caps.human_handoff}
                    on_toggle={make_toggle("human_handoff")}
                />
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ToggleRowProps {
    label: &'static str,
    description: &'static str,
    checked: bool,
    on_toggle: Callback<InputEvent>,
}

#[component]
fn ToggleRow(props: &ToggleRowProps) -> Html {
    html! {
        <label class="flex items-start gap-3 cursor-pointer p-3 rounded-lg border border-[var(--border-default)]">
            <input
                type="checkbox"
                checked={props.checked}
                oninput={props.on_toggle.clone()}
                class="mt-0.5"
            />
            <div>
                <div class="text-sm font-medium text-[var(--text-primary)]">
                    {props.label}
                </div>
                <div class="text-xs text-[var(--text-tertiary)]">
                    {props.description}
                </div>
            </div>
        </label>
    }
}
