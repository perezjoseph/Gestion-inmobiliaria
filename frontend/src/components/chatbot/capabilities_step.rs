use yew::prelude::*;

use crate::components::chatbot::toggle_switch::ToggleSwitch;
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
        <div class="flex flex-col gap-6">
            <CapabilityGroup
                title="Cuando el inquilino escribe"
                subtitle="Respuestas automáticas a mensajes entrantes."
            >
                <CapabilityRow
                    label="Consulta de balance"
                    description="El inquilino pregunta, el bot responde con el saldo pendiente."
                    icon={balance_icon()}
                    checked={caps.balance_queries}
                    on_toggle={make_toggle("balance_queries")}
                />
                <CapabilityRow
                    label="Recibos de pago (OCR)"
                    description="El inquilino envía una foto del recibo, el bot extrae monto y referencia."
                    icon={receipt_icon()}
                    checked={caps.receipt_ocr}
                    on_toggle={make_toggle("receipt_ocr")}
                />
                <CapabilityRow
                    label="Solicitudes de mantenimiento"
                    description="El inquilino reporta un problema, el bot lo registra en la unidad."
                    icon={wrench_icon()}
                    checked={caps.maintenance_requests}
                    on_toggle={make_toggle("maintenance_requests")}
                />
                <CapabilityRow
                    label="Transferencia a una persona"
                    description="Palabras clave (hablar con alguien, humano, agente) pausan al bot."
                    icon={handoff_icon()}
                    checked={caps.human_handoff}
                    on_toggle={make_toggle("human_handoff")}
                />
            </CapabilityGroup>

            <CapabilityGroup
                title="Por iniciativa del bot"
                subtitle="Mensajes que el bot envía sin que el inquilino escriba primero."
            >
                <CapabilityRow
                    label="Recordatorios de pago"
                    description="Aviso automático tres días antes del vencimiento."
                    icon={bell_icon()}
                    checked={caps.payment_reminders}
                    on_toggle={make_toggle("payment_reminders")}
                />
            </CapabilityGroup>
        </div>
    }
}

// ---------------------------------------------------------------------------
// CapabilityGroup — section wrapper
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct CapabilityGroupProps {
    title: &'static str,
    subtitle: &'static str,
    children: Children,
}

#[component]
fn CapabilityGroup(props: &CapabilityGroupProps) -> Html {
    html! {
        <section>
            <header class="mb-3">
                <h4 class="text-sm font-semibold text-[var(--text-primary)]">
                    {props.title}
                </h4>
                <p class="text-xs text-[var(--text-tertiary)] mt-0.5">
                    {props.subtitle}
                </p>
            </header>
            <div class="flex flex-col" style="border: 1px solid var(--border-default); border-radius: var(--radius-md, 8px); overflow: hidden;">
                {for props.children.iter()}
            </div>
        </section>
    }
}

// ---------------------------------------------------------------------------
// CapabilityRow — single capability with icon, toggle, description
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct CapabilityRowProps {
    label: &'static str,
    description: &'static str,
    icon: Html,
    checked: bool,
    on_toggle: Callback<InputEvent>,
}

#[component]
fn CapabilityRow(props: &CapabilityRowProps) -> Html {
    html! {
        <div
            class="flex items-center gap-3 px-4 py-3"
            style="border-bottom: 1px solid var(--border-subtle); background: var(--surface-raised);"
        >
            <div
                class="flex items-center justify-center rounded-lg flex-shrink-0"
                style={format!(
                    "width: 32px; height: 32px; background: {bg}; color: {fg};",
                    bg = if props.checked { "var(--color-primary-100)" } else { "var(--border-subtle)" },
                    fg = if props.checked { "var(--color-primary-700)" } else { "var(--text-tertiary)" },
                )}
                aria-hidden="true"
            >
                {props.icon.clone()}
            </div>
            <div class="flex-1 min-w-0">
                <div class="text-sm font-medium text-[var(--text-primary)]">
                    {props.label}
                </div>
                <div class="text-xs text-[var(--text-tertiary)] mt-0.5">
                    {props.description}
                </div>
            </div>
            <ToggleSwitch
                checked={props.checked}
                on_toggle={props.on_toggle.clone()}
                label={AttrValue::from(props.label)}
            />
        </div>
    }
}

// ---------------------------------------------------------------------------
// Icons — inline SVGs, stroke-based so they inherit `color`
// ---------------------------------------------------------------------------

fn balance_icon() -> Html {
    html! {
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="1" x2="12" y2="23"/>
            <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>
        </svg>
    }
}

fn receipt_icon() -> Html {
    html! {
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M4 2v20l3-2 3 2 3-2 3 2 3-2 3 2V2l-3 2-3-2-3 2-3-2-3 2-3-2z"/>
            <line x1="8" y1="9" x2="16" y2="9"/>
            <line x1="8" y1="13" x2="16" y2="13"/>
        </svg>
    }
}

fn wrench_icon() -> Html {
    html! {
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>
        </svg>
    }
}

fn handoff_icon() -> Html {
    html! {
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M16 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
            <circle cx="8.5" cy="7" r="4"/>
            <polyline points="17 11 19 13 23 9"/>
        </svg>
    }
}

fn bell_icon() -> Html {
    html! {
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/>
            <path d="M13.73 21a2 2 0 0 1-3.46 0"/>
        </svg>
    }
}
