use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::chatbot::activation_step::ActivationStep;
use crate::components::chatbot::capabilities_step::CapabilitiesStep;
use crate::components::chatbot::connection_step::ConnectionStep;
use crate::components::chatbot::knowledge_step::KnowledgeStep;
use crate::components::chatbot::persona_step::{PersonaStep, PersonaUpdate};
use crate::components::chatbot::sender_policy_step::SenderPolicyStep;
use crate::components::chatbot::test_chat_step::TestChatStep;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::chatbot;
use crate::types::chatbot::{
    Capabilities, ChatbotConfigResponse, ChatbotConfigUpdateRequest, FaqEntry,
};

const STEPS: &[&str] = &[
    "Conexión",
    "Personalidad",
    "Capacidades",
    "Conocimiento",
    "Remitentes",
    "Prueba",
    "Activar",
];

#[component]
fn StatusDashboard(props: &StatusDashboardProps) -> Html {
    let status_color = match props.connection_status.as_str() {
        "connected" => "var(--color-success)",
        "qr_pending" => "var(--color-warning)",
        "logged_out" => "var(--color-error)",
        _ => "var(--text-tertiary)",
    };

    let status_label = match props.connection_status.as_str() {
        "connected" => "Conectado",
        "qr_pending" => "Esperando QR",
        "disconnected" => "Desconectado",
        "logged_out" => "Sesión cerrada",
        _ => "Desconocido",
    };

    html! {
        <div class="gi-card" style="padding: var(--space-4); margin-bottom: var(--space-5);">
            <h3 style="font-size: var(--text-sm); font-weight: 600; color: var(--text-secondary); margin-bottom: var(--space-3);">
                {"Estado del Chatbot"}
            </h3>
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: var(--space-3);">
                <DashboardStat
                    label="Conexión"
                    value={status_label.to_string()}
                    color={status_color.to_string()}
                />
                <DashboardStat
                    label="Conversaciones"
                    value={props.message_count.to_string()}
                    color={"var(--text-primary)".to_string()}
                />
                <DashboardStat
                    label="Recibos Pendientes"
                    value={props.pending_receipts.to_string()}
                    color={if props.pending_receipts > 0 { "var(--color-warning)".to_string() } else { "var(--text-primary)".to_string() }}
                />
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct StatusDashboardProps {
    pub connection_status: String,
    pub message_count: usize,
    pub pending_receipts: usize,
}

#[derive(Properties, PartialEq)]
struct DashboardStatProps {
    pub label: String,
    pub value: String,
    pub color: String,
}

#[component]
fn DashboardStat(props: &DashboardStatProps) -> Html {
    html! {
        <div style="text-align: center;">
            <div style={format!("font-size: var(--text-lg); font-weight: 600; color: {};", props.color)}>
                {&props.value}
            </div>
            <div style="font-size: var(--text-xs); color: var(--text-tertiary);">
                {&props.label}
            </div>
        </div>
    }
}

#[component]
fn StepNavigation(props: &StepNavigationProps) -> Html {
    html! {
        <div style="display: flex; gap: var(--space-1); margin-bottom: var(--space-5); overflow-x: auto; padding-bottom: var(--space-2);">
            {for STEPS.iter().enumerate().map(|(idx, label)| {
                let is_active = idx == props.current_step;
                let is_completed = idx < props.current_step;
                let on_click = {
                    let on_step_click = props.on_step_click.clone();
                    Callback::from(move |_: MouseEvent| {
                        on_step_click.emit(idx);
                    })
                };
                html! {
                    <button
                        type="button"
                        onclick={on_click}
                        style={format!(
                            "padding: var(--space-2) var(--space-3); border-radius: var(--radius-md); border: 1px solid {}; background: {}; color: {}; font-size: var(--text-xs); font-weight: 500; cursor: pointer; white-space: nowrap; transition: all 0.2s;",
                            if is_active { "var(--color-primary)" } else { "var(--border-default)" },
                            if is_active { "var(--color-primary)" } else if is_completed { "var(--surface-raised)" } else { "transparent" },
                            if is_active { "white" } else { "var(--text-secondary)" },
                        )}
                    >
                        {format!("{}. {}", idx + 1, label)}
                    </button>
                }
            })}
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct StepNavigationProps {
    pub current_step: usize,
    pub on_step_click: Callback<usize>,
}

#[component]
pub fn ChatbotConfig() -> Html {
    let toasts = use_context::<ToastContext>();
    let current_step = use_state(|| 0_usize);
    let config = use_state(|| Option::<ChatbotConfigResponse>::None);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let saving = use_state(|| false);
    let message_count = use_state(|| 0_usize);
    let pending_receipts = use_state(|| 0_usize);

    // Load config and dashboard data on mount
    {
        let config = config.clone();
        let loading = loading.clone();
        let error = error.clone();
        let message_count = message_count.clone();
        let pending_receipts = pending_receipts.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                match chatbot::get_config().await {
                    Ok(cfg) => config.set(Some(cfg)),
                    Err(e) => error.set(Some(e)),
                }
                if let Ok(convos) = chatbot::list_conversations().await {
                    message_count.set(convos.len());
                }
                if let Ok(receipts) = chatbot::pending_receipts().await {
                    pending_receipts.set(receipts.len());
                }
                loading.set(false);
            });
        });
    }

    let on_step_click = {
        let current_step = current_step.clone();
        Callback::from(move |step: usize| {
            current_step.set(step);
        })
    };

    let on_next = {
        let current_step = current_step.clone();
        Callback::from(move |_: MouseEvent| {
            let step = *current_step;
            if step < STEPS.len() - 1 {
                current_step.set(step + 1);
            }
        })
    };

    let on_prev = {
        let current_step = current_step.clone();
        Callback::from(move |_: MouseEvent| {
            let step = *current_step;
            if step > 0 {
                current_step.set(step - 1);
            }
        })
    };

    let save_config = {
        let config = config.clone();
        let saving = saving.clone();
        let error = error.clone();
        let toasts = toasts;
        move |update: ChatbotConfigUpdateRequest| {
            let config = config.clone();
            let saving = saving.clone();
            let error = error.clone();
            let toasts = toasts.clone();
            saving.set(true);
            spawn_local(async move {
                match chatbot::update_config(&update).await {
                    Ok(new_cfg) => {
                        config.set(Some(new_cfg));
                        error.set(None);
                        if let Some(t) = &toasts {
                            t.dispatch(ToastAction::Push(
                                "Configuración guardada".into(),
                                ToastKind::Success,
                            ));
                        }
                    }
                    Err(e) => error.set(Some(e)),
                }
                saving.set(false);
            });
        }
    };

    if *loading {
        return html! {
            <div>
                <div class="gi-page-header">
                    <h1 class="gi-page-title">{"Configuración del Chatbot"}</h1>
                </div>
                <div class="gi-spinner" style="margin: var(--space-6) auto;"></div>
            </div>
        };
    }

    let cfg = match (*config).as_ref() {
        Some(c) => c.clone(),
        None => {
            return html! {
                <div>
                    <div class="gi-page-header">
                        <h1 class="gi-page-title">{"Configuración del Chatbot"}</h1>
                    </div>
                    if let Some(err) = (*error).as_ref() {
                        <ErrorBanner message={err.clone()} onclose={Callback::from({
                            let error = error.clone(); move |_: MouseEvent| error.set(None)
                        })} />
                    }
                    <p style="color: var(--text-secondary); font-size: var(--text-sm);">
                        {"No se pudo cargar la configuración del chatbot."}
                    </p>
                </div>
            };
        }
    };

    let connection_status = cfg.connection_status.clone();

    let on_status_change = {
        let config = config;
        Callback::from(move |new_status: String| {
            if let Some(mut cfg) = (*config).clone() {
                cfg.connection_status = new_status;
                config.set(Some(cfg));
            }
        })
    };

    let on_persona_change = {
        let save_config = save_config.clone();
        Callback::from(move |update: PersonaUpdate| {
            save_config(ChatbotConfigUpdateRequest {
                activo: None,
                display_name: update.display_name,
                language: update.language,
                tone: update.tone,
                greeting: update.greeting,
                system_prompt: update.system_prompt,
                faqs: None,
                policies: None,
                sender_policy: None,
                allowlist: None,
                capabilities: None,
                handoff_keywords: None,
                history_limit: None,
                retention_days: None,
            });
        })
    };

    let on_capabilities_change = {
        let save_config = save_config.clone();
        Callback::from(move |caps: Capabilities| {
            save_config(ChatbotConfigUpdateRequest {
                activo: None,
                display_name: None,
                language: None,
                tone: None,
                greeting: None,
                system_prompt: None,
                faqs: None,
                policies: None,
                sender_policy: None,
                allowlist: None,
                capabilities: Some(caps),
                handoff_keywords: None,
                history_limit: None,
                retention_days: None,
            });
        })
    };

    let on_faqs_change = {
        let save_config = save_config.clone();
        Callback::from(move |faqs: Vec<FaqEntry>| {
            save_config(ChatbotConfigUpdateRequest {
                activo: None,
                display_name: None,
                language: None,
                tone: None,
                greeting: None,
                system_prompt: None,
                faqs: Some(faqs),
                policies: None,
                sender_policy: None,
                allowlist: None,
                capabilities: None,
                handoff_keywords: None,
                history_limit: None,
                retention_days: None,
            });
        })
    };

    let on_policies_change = {
        let save_config = save_config.clone();
        Callback::from(move |policies: String| {
            save_config(ChatbotConfigUpdateRequest {
                activo: None,
                display_name: None,
                language: None,
                tone: None,
                greeting: None,
                system_prompt: None,
                faqs: None,
                policies: Some(policies),
                sender_policy: None,
                allowlist: None,
                capabilities: None,
                handoff_keywords: None,
                history_limit: None,
                retention_days: None,
            });
        })
    };

    let on_policy_change = {
        let save_config = save_config.clone();
        Callback::from(move |policy: String| {
            save_config(ChatbotConfigUpdateRequest {
                activo: None,
                display_name: None,
                language: None,
                tone: None,
                greeting: None,
                system_prompt: None,
                faqs: None,
                policies: None,
                sender_policy: Some(policy),
                allowlist: None,
                capabilities: None,
                handoff_keywords: None,
                history_limit: None,
                retention_days: None,
            });
        })
    };

    let on_allowlist_change = {
        let save_config = save_config.clone();
        Callback::from(move |allowlist: Vec<String>| {
            save_config(ChatbotConfigUpdateRequest {
                activo: None,
                display_name: None,
                language: None,
                tone: None,
                greeting: None,
                system_prompt: None,
                faqs: None,
                policies: None,
                sender_policy: None,
                allowlist: Some(allowlist),
                capabilities: None,
                handoff_keywords: None,
                history_limit: None,
                retention_days: None,
            });
        })
    };

    let on_activation_toggle = {
        let save_config = save_config;
        Callback::from(move |activo: bool| {
            save_config(ChatbotConfigUpdateRequest {
                activo: Some(activo),
                display_name: None,
                language: None,
                tone: None,
                greeting: None,
                system_prompt: None,
                faqs: None,
                policies: None,
                sender_policy: None,
                allowlist: None,
                capabilities: None,
                handoff_keywords: None,
                history_limit: None,
                retention_days: None,
            });
        })
    };

    let step_content = match *current_step {
        0 => html! {
            <ConnectionStep
                connection_status={connection_status.clone()}
                on_status_change={on_status_change}
            />
        },
        1 => html! {
            <PersonaStep
                config={cfg}
                on_change={on_persona_change}
            />
        },
        2 => html! {
            <CapabilitiesStep
                capabilities={cfg.capabilities}
                on_change={on_capabilities_change}
            />
        },
        3 => html! {
            <KnowledgeStep
                faqs={cfg.faqs.unwrap_or_default()}
                policies={cfg.policies.unwrap_or_default()}
                on_faqs_change={on_faqs_change}
                on_policies_change={on_policies_change}
            />
        },
        4 => html! {
            <SenderPolicyStep
                sender_policy={cfg.sender_policy}
                allowlist={cfg.allowlist.unwrap_or_default()}
                on_policy_change={on_policy_change}
                on_allowlist_change={on_allowlist_change}
            />
        },
        5 => html! { <TestChatStep /> },
        6 => html! {
            <ActivationStep
                activo={cfg.activo}
                connection_status={connection_status.clone()}
                on_toggle={on_activation_toggle}
            />
        },
        _ => html! {},
    };

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Configuración del Chatbot"}</h1>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <StatusDashboard
                connection_status={connection_status}
                message_count={*message_count}
                pending_receipts={*pending_receipts}
            />

            <StepNavigation
                current_step={*current_step}
                on_step_click={on_step_click}
            />

            {step_content}

            <WizardFooter
                current_step={*current_step}
                total_steps={STEPS.len()}
                saving={*saving}
                on_prev={on_prev}
                on_next={on_next}
            />
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct WizardFooterProps {
    pub current_step: usize,
    pub total_steps: usize,
    pub saving: bool,
    pub on_prev: Callback<MouseEvent>,
    pub on_next: Callback<MouseEvent>,
}

#[component]
fn WizardFooter(props: &WizardFooterProps) -> Html {
    html! {
        <div style="display: flex; justify-content: space-between; margin-top: var(--space-4);">
            if props.current_step > 0 {
                <button
                    type="button"
                    class="gi-btn gi-btn-secondary"
                    onclick={props.on_prev.clone()}
                >
                    {"← Anterior"}
                </button>
            } else {
                <div></div>
            }

            <div style="display: flex; align-items: center; gap: var(--space-3);">
                if props.saving {
                    <span style="font-size: var(--text-xs); color: var(--text-tertiary);">
                        {"Guardando..."}
                    </span>
                }
                if props.current_step < props.total_steps - 1 {
                    <button
                        type="button"
                        class="gi-btn gi-btn-primary"
                        onclick={props.on_next.clone()}
                    >
                        {"Siguiente →"}
                    </button>
                }
            </div>
        </div>
    }
}
