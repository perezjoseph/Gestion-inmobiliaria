use gloo_timers::callback::Timeout;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::chatbot::activation_step::ActivationStep;
use crate::components::chatbot::capabilities_step::CapabilitiesStep;
use crate::components::chatbot::connection_step::ConnectionStep;
use crate::components::chatbot::knowledge_step::KnowledgeStep;
use crate::components::chatbot::persona_step::{PersonaStep, PersonaUpdate};
use crate::components::chatbot::sender_policy_step::SenderPolicyStep;
use crate::components::chatbot::test_chat_step::TestChatStep;
use crate::components::common::delete_confirm_modal::DeleteConfirmModal;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::Skeleton;
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

// ---------------------------------------------------------------------------
// Save status indicator
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum SaveStatus {
    Idle,
    Dirty,
    Saving,
    Saved,
}

// ---------------------------------------------------------------------------
// StatusBar — compact horizontal status (no hero-metric template)
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct StatusBarProps {
    pub connection_status: AttrValue,
    pub message_count: usize,
    pub pending_receipts: usize,
}

#[component]
fn StatusBar(props: &StatusBarProps) -> Html {
    let (dot_class, label) = match props.connection_status.as_str() {
        "connected" => ("bg-emerald-500", "Conectado"),
        "qr_pending" => ("bg-amber-400", "Esperando QR"),
        "logged_out" => ("bg-red-500", "Sesión cerrada"),
        "disconnected" => ("bg-red-500", "Desconectado"),
        _ => ("bg-gray-400", "Desconocido"),
    };

    html! {
        <div class="flex items-center gap-3 px-4 py-2 rounded-lg border border-[var(--border-subtle)] mb-5 text-sm">
            <span class={classes!("inline-block", "w-2", "h-2", "rounded-full", dot_class)} />
            <span class="font-medium text-[var(--text-primary)]">{label}</span>
            <span class="text-[var(--text-tertiary)]">{"|"}</span>
            <span class="text-[var(--text-secondary)]">
                {format!("{} conversaciones", props.message_count)}
            </span>
            <span class="text-[var(--text-tertiary)]">{"·"}</span>
            <span class={classes!(
                "text-[var(--text-secondary)]",
                (props.pending_receipts > 0).then_some("text-amber-600 font-medium")
            )}>
                {format!("{} recibos pendientes", props.pending_receipts)}
            </span>
        </div>
    }
}

// ---------------------------------------------------------------------------
// StepperNav — accessible stepper with progress line
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct StepperNavProps {
    pub current_step: usize,
    pub on_step_click: Callback<usize>,
}

#[component]
fn StepperNav(props: &StepperNavProps) -> Html {
    let on_keydown = {
        let current = props.current_step;
        let on_step_click = props.on_step_click.clone();
        Callback::from(move |e: KeyboardEvent| {
            let next = match e.key().as_str() {
                "ArrowRight" | "ArrowDown" => {
                    e.prevent_default();
                    if current < STEPS.len() - 1 {
                        Some(current + 1)
                    } else {
                        None
                    }
                }
                "ArrowLeft" | "ArrowUp" => {
                    e.prevent_default();
                    if current > 0 { Some(current - 1) } else { None }
                }
                _ => None,
            };
            if let Some(step) = next {
                on_step_click.emit(step);
            }
        })
    };

    html! {
        <div
            role="tablist"
            aria-label="Pasos de configuración"
            class="flex items-center gap-1 mb-6 overflow-x-auto pb-2"
            onkeydown={on_keydown}
        >
            {for STEPS.iter().enumerate().map(|(idx, label)| {
                let is_active = idx == props.current_step;
                let is_completed = idx < props.current_step;
                let on_click = {
                    let cb = props.on_step_click.clone();
                    Callback::from(move |_: MouseEvent| cb.emit(idx))
                };

                html! {
                    <StepTab
                        index={idx}
                        label={AttrValue::from(*label)}
                        is_active={is_active}
                        is_completed={is_completed}
                        onclick={on_click}
                    />
                }
            })}
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct StepTabProps {
    pub index: usize,
    pub label: AttrValue,
    pub is_active: bool,
    pub is_completed: bool,
    pub onclick: Callback<MouseEvent>,
}

#[component]
fn StepTab(props: &StepTabProps) -> Html {
    let btn_class = if props.is_active {
        "px-3 py-1.5 rounded-md text-xs font-semibold bg-[var(--color-primary-500)] text-white cursor-pointer border-none outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-focus)] whitespace-nowrap"
    } else if props.is_completed {
        "px-3 py-1.5 rounded-md text-xs font-medium bg-[var(--surface-sidebar-active)] text-emerald-400 cursor-pointer border-none outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-focus)] whitespace-nowrap"
    } else {
        "px-3 py-1.5 rounded-md text-xs text-[var(--text-tertiary)] cursor-pointer border border-[var(--border-default)] bg-transparent outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-focus)] whitespace-nowrap"
    };

    html! {
        <button
            type="button"
            role="tab"
            aria-selected={props.is_active.to_string()}
            tabindex={if props.is_active { "0" } else { "-1" }}
            onclick={props.onclick.clone()}
            class={btn_class}
        >
            if props.is_completed {
                {"✓ "}{&props.label}
            } else {
                {format!("{}. ", props.index + 1)}{&props.label}
            }
        </button>
    }
}

// ---------------------------------------------------------------------------
// WizardFooter — with SVG arrows and save status
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct WizardFooterProps {
    pub current_step: usize,
    pub total_steps: usize,
    pub save_status: SaveStatus,
    pub on_prev: Callback<MouseEvent>,
    pub on_next: Callback<MouseEvent>,
}

#[component]
fn WizardFooter(props: &WizardFooterProps) -> Html {
    let status_indicator = match props.save_status {
        SaveStatus::Dirty => html! {
            <span class="text-xs text-amber-600 font-medium">{"Cambios sin guardar"}</span>
        },
        SaveStatus::Saving => html! {
            <span class="text-xs text-[var(--text-tertiary)]">{"Guardando..."}</span>
        },
        SaveStatus::Saved => html! {
            <span class="text-xs text-emerald-600">{"Guardado ✓"}</span>
        },
        SaveStatus::Idle => html! {},
    };

    html! {
        <div class="flex justify-between items-center mt-4">
            if props.current_step > 0 {
                <button
                    type="button"
                    class="gi-btn gi-btn-secondary flex items-center gap-1"
                    onclick={props.on_prev.clone()}
                >
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="15 18 9 12 15 6"/>
                    </svg>
                    {"Anterior"}
                </button>
            } else {
                <div />
            }

            <div class="flex items-center gap-3">
                {status_indicator}
                if props.current_step < props.total_steps - 1 {
                    <button
                        type="button"
                        class="gi-btn gi-btn-primary flex items-center gap-1"
                        onclick={props.on_next.clone()}
                    >
                        {"Siguiente"}
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <polyline points="9 18 15 12 9 6"/>
                        </svg>
                    </button>
                }
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// ChatbotConfigSkeleton — loading skeleton
// ---------------------------------------------------------------------------

#[component]
fn ChatbotConfigSkeleton() -> Html {
    html! {
        <div aria-busy="true" aria-label="Cargando configuración del chatbot">
            <div class="gi-page-header">
                <Skeleton width="260px" height="2rem" radius="6px" />
            </div>
            <div class="flex items-center gap-3 px-4 py-2 rounded-lg border border-[var(--border-subtle)] mb-5">
                <Skeleton width="8px" height="8px" radius="50%" />
                <Skeleton width="80px" height="0.875rem" radius="4px" />
                <Skeleton width="140px" height="0.875rem" radius="4px" />
            </div>
            <div class="flex items-center gap-2 mb-6">
                {for (0..7).map(|_| html! {
                    <div class="flex items-center gap-2">
                        <Skeleton width="28px" height="28px" radius="50%" />
                        <Skeleton width="60px" height="0.75rem" radius="4px" />
                    </div>
                })}
            </div>
            <div class="gi-card" style="padding: var(--space-5);">
                <Skeleton width="200px" height="1.25rem" radius="4px" />
                <div class="flex flex-col gap-3 mt-4">
                    <Skeleton height="2.5rem" radius="6px" />
                    <Skeleton height="2.5rem" radius="6px" />
                    <Skeleton height="2.5rem" radius="6px" />
                </div>
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// ChatbotConfig — main page component
// ---------------------------------------------------------------------------

#[component]
pub fn ChatbotConfig() -> Html {
    let toasts = use_context::<ToastContext>();
    let current_step = use_state(|| 0_usize);
    let config = use_state(|| Option::<ChatbotConfigResponse>::None);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let save_status = use_state(|| SaveStatus::Idle);
    let message_count = use_state(|| 0_usize);
    let pending_receipts = use_state(|| 0_usize);
    let debounce_handle = use_state(|| Option::<Timeout>::None);
    let confirm_toggle = use_state(|| Option::<bool>::None);

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

    // Step navigation
    let on_step_click = {
        let current_step = current_step.clone();
        Callback::from(move |step: usize| current_step.set(step))
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

    // Debounced save: 600ms delay
    let save_config = {
        let config = config.clone();
        let save_status = save_status.clone();
        let error = error.clone();
        let toasts = toasts;
        let debounce_handle = debounce_handle;
        move |update: ChatbotConfigUpdateRequest| {
            let config = config.clone();
            let save_status = save_status.clone();
            let error = error.clone();
            let toasts = toasts.clone();
            let debounce_handle = debounce_handle.clone();

            // Mark dirty immediately
            save_status.set(SaveStatus::Dirty);

            // Clear previous timeout
            debounce_handle.set(None);

            let timeout = Timeout::new(600, move || {
                save_status.set(SaveStatus::Saving);
                spawn_local(async move {
                    match chatbot::update_config(&update).await {
                        Ok(new_cfg) => {
                            config.set(Some(new_cfg));
                            error.set(None);
                            save_status.set(SaveStatus::Saved);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Configuración guardada".into(),
                                    ToastKind::Success,
                                ));
                            }
                        }
                        Err(e) => {
                            error.set(Some(e));
                            save_status.set(SaveStatus::Idle);
                        }
                    }
                });
            });
            debounce_handle.set(Some(timeout));
        }
    };

    // Activation toggle with confirmation
    let on_activation_request = {
        let confirm_toggle = confirm_toggle.clone();
        Callback::from(move |activo: bool| {
            confirm_toggle.set(Some(activo));
        })
    };

    let on_confirm_activation = {
        let confirm_toggle = confirm_toggle.clone();
        let save_config = save_config.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(activo) = *confirm_toggle {
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
            }
            confirm_toggle.set(None);
        })
    };

    let on_cancel_activation = {
        let confirm_toggle = confirm_toggle.clone();
        Callback::from(move |_: MouseEvent| {
            confirm_toggle.set(None);
        })
    };

    // Loading state
    if *loading {
        return html! { <ChatbotConfigSkeleton /> };
    }

    // Config not loaded
    let cfg = match (*config).as_ref() {
        Some(c) => c.clone(),
        None => {
            return html! {
                <div>
                    if let Some(err) = (*error).as_ref() {
                        <ErrorBanner message={err.clone()} onclose={Callback::from({
                            let error = error.clone(); move |_: MouseEvent| error.set(None)
                        })} />
                    }
                    <p class="text-sm text-[var(--text-secondary)]">
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
            if let Some(mut c) = (*config).clone() {
                c.connection_status = new_status;
                config.set(Some(c));
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
        let save_config = save_config;
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
                faqs={cfg.faqs.clone().unwrap_or_default()}
                policies={cfg.policies.unwrap_or_default()}
                on_faqs_change={on_faqs_change}
                on_policies_change={on_policies_change}
            />
        },
        4 => html! {
            <SenderPolicyStep
                sender_policy={cfg.sender_policy.clone()}
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
                on_toggle={on_activation_request}
            />
        },
        _ => html! {},
    };

    let confirm_message = if confirm_toggle.is_some_and(|v| v) {
        "¿Está seguro de que desea activar el chatbot? Los inquilinos comenzarán a recibir respuestas automáticas."
    } else {
        "¿Está seguro de que desea desactivar el chatbot? Los inquilinos dejarán de recibir respuestas automáticas."
    };

    let confirm_label = if confirm_toggle.is_some_and(|v| v) {
        "Activar"
    } else {
        "Desactivar"
    };

    html! {
        <div>
            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <StatusBar
                connection_status={AttrValue::from(connection_status.clone())}
                message_count={*message_count}
                pending_receipts={*pending_receipts}
            />

            <StepperNav
                current_step={*current_step}
                on_step_click={on_step_click}
            />

            {step_content}

            <WizardFooter
                current_step={*current_step}
                total_steps={STEPS.len()}
                save_status={*save_status}
                on_prev={on_prev}
                on_next={on_next}
            />

            if confirm_toggle.is_some() {
                <DeleteConfirmModal
                    message={confirm_message.to_string()}
                    confirm_label={Some(confirm_label.to_string())}
                    on_confirm={on_confirm_activation}
                    on_cancel={on_cancel_activation}
                />
            }
        </div>
    }
}
