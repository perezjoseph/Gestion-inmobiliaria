use gloo_timers::callback::Timeout;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::chatbot::bot_state_card::BotStateCard;
use crate::components::chatbot::capabilities_step::CapabilitiesStep;
use crate::components::chatbot::connection_step::ConnectionStep;
use crate::components::chatbot::guidance_rules_step::GuidanceRulesStep;
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum SaveStatus {
    Idle,
    Dirty,
    Saving,
    Saved,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum Section {
    Connection,
    Persona,
    GuidanceRules,
    Capabilities,
    Knowledge,
    Senders,
}

impl Section {
    const fn title(self) -> &'static str {
        match self {
            Self::Connection => "ConexiÃ³n",
            Self::Persona => "Personalidad",
            Self::GuidanceRules => "Reglas del agente",
            Self::Capabilities => "Capacidades",
            Self::Knowledge => "Conocimiento",
            Self::Senders => "Remitentes",
        }
    }

    const fn id(self) -> &'static str {
        match self {
            Self::Connection => "section-connection",
            Self::Persona => "section-persona",
            Self::GuidanceRules => "section-guidance-rules",
            Self::Capabilities => "section-capabilities",
            Self::Knowledge => "section-knowledge",
            Self::Senders => "section-senders",
        }
    }
}

#[component]
fn ChatbotConfigSkeleton() -> Html {
    html! {
        <div aria-busy="true" aria-label="Cargando configuraciÃ³n del chatbot" class="flex flex-col gap-5">
            <div class="rounded-lg p-5" style="background: var(--surface-raised); border: 1px solid var(--border-default);">
                <Skeleton width="60px" height="1rem" radius="999px" />
                <div class="mt-3">
                    <Skeleton width="260px" height="1.75rem" radius="6px" />
                </div>
                <div class="mt-2">
                    <Skeleton width="100%" height="1rem" radius="4px" />
                </div>
            </div>
            {for (0..2).map(|_| html! {
                <div class="rounded-lg p-4" style="background: var(--surface-raised); border: 1px solid var(--border-default);">
                    <Skeleton width="180px" height="1.25rem" radius="4px" />
                    <div class="mt-3">
                        <Skeleton height="2.5rem" radius="6px" />
                    </div>
                </div>
            })}
        </div>
    }
}

#[component]
pub fn ChatbotConfig() -> Html {
    let toasts = use_context::<ToastContext>();
    let config = use_state(|| Option::<ChatbotConfigResponse>::None);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let save_status = use_state(|| SaveStatus::Idle);
    let conversations_count = use_state(|| 0_usize);
    let pending_receipts = use_state(|| 0_usize);
    let connected_phone = use_state(|| Option::<String>::None);
    let connected_at = use_state(|| Option::<String>::None);
    let debounce_handle = use_state(|| Option::<Timeout>::None);
    let confirm_toggle = use_state(|| Option::<bool>::None);
    let open_section = use_state(|| Option::<Section>::None);

    {
        let config = config.clone();
        let loading = loading.clone();
        let error = error.clone();
        let conversations_count = conversations_count.clone();
        let pending_receipts = pending_receipts.clone();
        let connected_phone = connected_phone.clone();
        let connected_at = connected_at.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                let (cfg_res, status_res) =
                    futures::future::join(chatbot::get_config(), chatbot::status()).await;
                match cfg_res {
                    Ok(mut cfg) => {
                        if let Ok(status_resp) = &status_res {
                            cfg.connection_status.clone_from(&status_resp.status);
                        }
                        config.set(Some(cfg));
                    }
                    Err(e) => error.set(Some(e)),
                }
                if let Ok(status_resp) = status_res {
                    connected_phone.set(status_resp.connected_phone);
                    connected_at.set(status_resp.connected_at);
                }
                if let Ok(convos) = chatbot::list_conversations().await {
                    conversations_count.set(convos.len());
                }
                if let Ok(receipts) = chatbot::pending_receipts().await {
                    pending_receipts.set(receipts.len());
                }
                loading.set(false);
            });
        });
    }

    let save_config: std::rc::Rc<dyn Fn(ChatbotConfigUpdateRequest)> = {
        let config = config.clone();
        let save_status = save_status.clone();
        let error = error.clone();
        let toasts = toasts;
        let debounce_handle = debounce_handle;
        let inner = move |update: ChatbotConfigUpdateRequest| {
            let config = config.clone();
            let save_status = save_status.clone();
            let error = error.clone();
            let toasts = toasts.clone();
            let debounce_handle = debounce_handle.clone();

            save_status.set(SaveStatus::Dirty);
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
                                    "Cambios guardados".into(),
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
        };
        std::rc::Rc::new(inner)
    };

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
                save_config(activation_update(activo));
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

    let on_toggle_section = {
        let open_section = open_section.clone();
        Callback::from(move |section: Section| {
            if open_section.as_ref() == Some(&section) {
                open_section.set(None);
            } else {
                open_section.set(Some(section));
            }
        })
    };

    if *loading {
        return html! { <ChatbotConfigSkeleton /> };
    }

    let cfg = match (*config).as_ref() {
        Some(c) => c.clone(),
        None => return render_load_error(&error),
    };

    let on_status_change = {
        let config = config;
        let connected_phone = connected_phone.clone();
        let connected_at = connected_at.clone();
        Callback::from(
            move |(new_status, phone, at): (String, Option<String>, Option<String>)| {
                if let Some(mut c) = (*config).clone() {
                    c.connection_status = new_status;
                    config.set(Some(c));
                }
                connected_phone.set(phone);
                connected_at.set(at);
            },
        )
    };

    let capabilities_enabled = count_enabled(&cfg.capabilities);
    let sender_scope = sender_scope_label(&cfg.sender_policy, cfg.allowlist.as_deref());

    let confirm_message = if confirm_toggle.is_some_and(|v| v) {
        "Â¿Activar el chatbot? Los inquilinos comenzarÃ¡n a recibir respuestas automÃ¡ticas de inmediato."
    } else {
        "Â¿Desactivar el chatbot? Los inquilinos dejarÃ¡n de recibir respuestas automÃ¡ticas."
    };

    let confirm_label = if confirm_toggle.is_some_and(|v| v) {
        "Activar"
    } else {
        "Desactivar"
    };

    html! {
        <div class="flex flex-col gap-5">
            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <BotStateCard
                activo={cfg.activo}
                connection_status={cfg.connection_status.clone()}
                capabilities_enabled={capabilities_enabled}
                capabilities_total={5}
                sender_scope={AttrValue::from(sender_scope)}
                conversations_count={*conversations_count}
                pending_receipts={*pending_receipts}
                on_toggle={on_activation_request}
            />

            <ControlPanelLayout
                cfg={cfg.clone()}
                connected_phone={(*connected_phone).clone()}
                connected_at={(*connected_at).clone()}
                open_section={*open_section}
                save_status={*save_status}
                save_config={save_config}
                on_status_change={on_status_change}
                on_toggle_section={on_toggle_section}
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

#[derive(Properties)]
struct ControlPanelLayoutProps {
    cfg: ChatbotConfigResponse,
    connected_phone: Option<String>,
    connected_at: Option<String>,
    open_section: Option<Section>,
    save_status: SaveStatus,
    save_config: std::rc::Rc<dyn Fn(ChatbotConfigUpdateRequest)>,
    on_status_change: Callback<(String, Option<String>, Option<String>)>,
    on_toggle_section: Callback<Section>,
}

impl PartialEq for ControlPanelLayoutProps {
    fn eq(&self, other: &Self) -> bool {
        self.cfg == other.cfg
            && self.connected_phone == other.connected_phone
            && self.connected_at == other.connected_at
            && self.open_section == other.open_section
            && self.save_status == other.save_status
            && std::rc::Rc::ptr_eq(&self.save_config, &other.save_config)
            && self.on_status_change == other.on_status_change
            && self.on_toggle_section == other.on_toggle_section
    }
}

#[component]
fn ControlPanelLayout(props: &ControlPanelLayoutProps) -> Html {
    html! {
        <div
            class="gap-5"
            style="display: grid; grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr);"
        >
            <SettingsColumn
                cfg={props.cfg.clone()}
                connected_phone={props.connected_phone.clone()}
                connected_at={props.connected_at.clone()}
                open_section={props.open_section}
                save_status={props.save_status}
                save_config={props.save_config.clone()}
                on_status_change={props.on_status_change.clone()}
                on_toggle_section={props.on_toggle_section.clone()}
            />

            <div
                class="rounded-lg p-4"
                style="background: var(--surface-raised); border: 1px solid var(--border-default); position: sticky; top: var(--space-4); align-self: flex-start; max-height: calc(100vh - 120px); overflow-y: auto;"
            >
                <TestChatStep />
            </div>
        </div>
    }
}

#[derive(Properties)]
struct SettingsColumnProps {
    cfg: ChatbotConfigResponse,
    connected_phone: Option<String>,
    connected_at: Option<String>,
    open_section: Option<Section>,
    save_status: SaveStatus,
    save_config: std::rc::Rc<dyn Fn(ChatbotConfigUpdateRequest)>,
    on_status_change: Callback<(String, Option<String>, Option<String>)>,
    on_toggle_section: Callback<Section>,
}

impl PartialEq for SettingsColumnProps {
    fn eq(&self, other: &Self) -> bool {
        self.cfg == other.cfg
            && self.connected_phone == other.connected_phone
            && self.connected_at == other.connected_at
            && self.open_section == other.open_section
            && self.save_status == other.save_status
            && std::rc::Rc::ptr_eq(&self.save_config, &other.save_config)
            && self.on_status_change == other.on_status_change
            && self.on_toggle_section == other.on_toggle_section
    }
}

#[component]
fn SettingsColumn(props: &SettingsColumnProps) -> Html {
    let cfg = &props.cfg;

    html! {
        <div class="flex flex-col gap-3">
            <div class="flex items-center justify-between">
                <h3 class="text-sm font-semibold text-[var(--text-primary)]">
                    {"ConfiguraciÃ³n"}
                </h3>
                <SaveIndicator status={props.save_status} />
            </div>

            <ConnectionSection
                status={cfg.connection_status.clone()}
                phone={props.connected_phone.clone()}
                connected_at={props.connected_at.clone()}
                on_status_change={props.on_status_change.clone()}
            />

            <PersonaSection
                open={props.open_section == Some(Section::Persona)}
                cfg={cfg.clone()}
                save_config={props.save_config.clone()}
                on_toggle={props.on_toggle_section.clone()}
            />

            <GuidanceRulesSection
                open={props.open_section == Some(Section::GuidanceRules)}
                rules={cfg.guidance_rules.clone()}
                on_toggle={props.on_toggle_section.clone()}
            />

            <CapabilitiesSection
                open={props.open_section == Some(Section::Capabilities)}
                capabilities={cfg.capabilities.clone()}
                save_config={props.save_config.clone()}
                on_toggle={props.on_toggle_section.clone()}
            />

            <KnowledgeSection
                open={props.open_section == Some(Section::Knowledge)}
                faqs={cfg.faqs.clone().unwrap_or_default()}
                policies={cfg.policies.clone().unwrap_or_default()}
                save_config={props.save_config.clone()}
                on_toggle={props.on_toggle_section.clone()}
            />

            <SendersSection
                open={props.open_section == Some(Section::Senders)}
                sender_policy={cfg.sender_policy.clone()}
                allowlist={cfg.allowlist.clone().unwrap_or_default()}
                save_config={props.save_config.clone()}
                on_toggle={props.on_toggle_section.clone()}
            />
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ConnectionSectionProps {
    status: String,
    phone: Option<String>,
    connected_at: Option<String>,
    on_status_change: Callback<(String, Option<String>, Option<String>)>,
}

#[component]
fn ConnectionSection(props: &ConnectionSectionProps) -> Html {
    html! {
        <ConnectionStep
            connection_status={props.status.clone()}
            connected_phone={props.phone.clone()}
            connected_at={props.connected_at.clone()}
            on_status_change={props.on_status_change.clone()}
        />
    }
}

#[derive(Properties)]
struct AccordionSectionProps {
    section: Section,
    open: bool,
    summary: AttrValue,
    children: Children,
    on_toggle: Callback<Section>,
}

impl PartialEq for AccordionSectionProps {
    fn eq(&self, other: &Self) -> bool {
        self.section == other.section
            && self.open == other.open
            && self.summary == other.summary
            && self.children == other.children
            && self.on_toggle == other.on_toggle
    }
}

#[component]
fn AccordionSection(props: &AccordionSectionProps) -> Html {
    let on_click = {
        let on_toggle = props.on_toggle.clone();
        let section = props.section;
        Callback::from(move |_: MouseEvent| on_toggle.emit(section))
    };

    let panel_id = props.section.id();

    html! {
        <section
            class="rounded-lg"
            style="background: var(--surface-raised); border: 1px solid var(--border-default); overflow: hidden;"
        >
            <button
                type="button"
                class="w-full flex items-center justify-between gap-3 px-4 py-3 bg-transparent border-none cursor-pointer text-left"
                aria-expanded={props.open.to_string()}
                aria-controls={panel_id}
                onclick={on_click}
            >
                <div class="flex-1 min-w-0">
                    <div class="text-sm font-semibold text-[var(--text-primary)]">
                        {props.section.title()}
                    </div>
                    <div class="text-xs text-[var(--text-tertiary)] mt-0.5 truncate">
                        {&props.summary}
                    </div>
                </div>
                <Chevron open={props.open} />
            </button>
            if props.open {
                <div
                    id={panel_id}
                    role="region"
                    aria-label={props.section.title()}
                    class="px-4 pb-4 pt-2"
                    style="border-top: 1px solid var(--border-subtle);"
                >
                    {for props.children.iter()}
                </div>
            }
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ChevronProps {
    open: bool,
}

#[component]
fn Chevron(props: &ChevronProps) -> Html {
    let transform = if props.open {
        "rotate(180deg)"
    } else {
        "rotate(0)"
    };
    html! {
        <svg
            width="16" height="16" viewBox="0 0 24 24" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
            style={format!("color: var(--text-secondary); transform: {transform}; transition: transform 200ms var(--ease-out);")}
            aria-hidden="true"
        >
            <polyline points="6 9 12 15 18 9"/>
        </svg>
    }
}

#[derive(Properties)]
struct PersonaSectionProps {
    open: bool,
    cfg: ChatbotConfigResponse,
    save_config: std::rc::Rc<dyn Fn(ChatbotConfigUpdateRequest)>,
    on_toggle: Callback<Section>,
}

impl PartialEq for PersonaSectionProps {
    fn eq(&self, other: &Self) -> bool {
        self.open == other.open
            && self.cfg == other.cfg
            && std::rc::Rc::ptr_eq(&self.save_config, &other.save_config)
            && self.on_toggle == other.on_toggle
    }
}

#[component]
fn PersonaSection(props: &PersonaSectionProps) -> Html {
    let summary = persona_summary(&props.cfg);

    let on_change = {
        let save_config = props.save_config.clone();
        Callback::from(move |update: PersonaUpdate| {
            save_config(ChatbotConfigUpdateRequest {
                activo: None,
                display_name: update.display_name,
                language: update.language,
                tone: update.tone,
                greeting: update.greeting,
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

    html! {
        <AccordionSection
            section={Section::Persona}
            open={props.open}
            summary={AttrValue::from(summary)}
            on_toggle={props.on_toggle.clone()}
        >
            <PersonaStep config={props.cfg.clone()} on_change={on_change} />
        </AccordionSection>
    }
}

#[derive(Properties, PartialEq)]
struct GuidanceRulesSectionProps {
    open: bool,
    rules: Vec<crate::types::chatbot::GuidanceRule>,
    on_toggle: Callback<Section>,
}

#[component]
fn GuidanceRulesSection(props: &GuidanceRulesSectionProps) -> Html {
    let active_count = props.rules.iter().filter(|r| r.enabled).count();
    let summary = format!("{active_count} de {} reglas activas", props.rules.len());

    let on_change = Callback::from(|()| {});

    html! {
        <AccordionSection
            section={Section::GuidanceRules}
            open={props.open}
            summary={AttrValue::from(summary)}
            on_toggle={props.on_toggle.clone()}
        >
            <GuidanceRulesStep rules={props.rules.clone()} on_change={on_change} />
        </AccordionSection>
    }
}

#[derive(Properties)]
struct CapabilitiesSectionProps {
    open: bool,
    capabilities: Capabilities,
    save_config: std::rc::Rc<dyn Fn(ChatbotConfigUpdateRequest)>,
    on_toggle: Callback<Section>,
}

impl PartialEq for CapabilitiesSectionProps {
    fn eq(&self, other: &Self) -> bool {
        self.open == other.open
            && self.capabilities == other.capabilities
            && std::rc::Rc::ptr_eq(&self.save_config, &other.save_config)
            && self.on_toggle == other.on_toggle
    }
}

#[component]
fn CapabilitiesSection(props: &CapabilitiesSectionProps) -> Html {
    let enabled = count_enabled(&props.capabilities);
    let summary = format!("{enabled} de 5 capacidades activas");

    let on_change = {
        let save_config = props.save_config.clone();
        Callback::from(move |caps: Capabilities| {
            save_config(ChatbotConfigUpdateRequest {
                activo: None,
                display_name: None,
                language: None,
                tone: None,
                greeting: None,
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

    html! {
        <AccordionSection
            section={Section::Capabilities}
            open={props.open}
            summary={AttrValue::from(summary)}
            on_toggle={props.on_toggle.clone()}
        >
            <CapabilitiesStep capabilities={props.capabilities.clone()} on_change={on_change} />
        </AccordionSection>
    }
}

#[derive(Properties)]
struct KnowledgeSectionProps {
    open: bool,
    faqs: Vec<FaqEntry>,
    policies: String,
    save_config: std::rc::Rc<dyn Fn(ChatbotConfigUpdateRequest)>,
    on_toggle: Callback<Section>,
}

impl PartialEq for KnowledgeSectionProps {
    fn eq(&self, other: &Self) -> bool {
        self.open == other.open
            && self.faqs == other.faqs
            && self.policies == other.policies
            && std::rc::Rc::ptr_eq(&self.save_config, &other.save_config)
            && self.on_toggle == other.on_toggle
    }
}

#[component]
fn KnowledgeSection(props: &KnowledgeSectionProps) -> Html {
    let summary = knowledge_summary(props.faqs.len(), !props.policies.trim().is_empty());

    let on_faqs_change = {
        let save_config = props.save_config.clone();
        Callback::from(move |faqs: Vec<FaqEntry>| {
            save_config(ChatbotConfigUpdateRequest {
                activo: None,
                display_name: None,
                language: None,
                tone: None,
                greeting: None,
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
        let save_config = props.save_config.clone();
        Callback::from(move |policies: String| {
            save_config(ChatbotConfigUpdateRequest {
                activo: None,
                display_name: None,
                language: None,
                tone: None,
                greeting: None,
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

    html! {
        <AccordionSection
            section={Section::Knowledge}
            open={props.open}
            summary={AttrValue::from(summary)}
            on_toggle={props.on_toggle.clone()}
        >
            <KnowledgeStep
                faqs={props.faqs.clone()}
                policies={props.policies.clone()}
                on_faqs_change={on_faqs_change}
                on_policies_change={on_policies_change}
            />
        </AccordionSection>
    }
}

#[derive(Properties)]
struct SendersSectionProps {
    open: bool,
    sender_policy: String,
    allowlist: Vec<String>,
    save_config: std::rc::Rc<dyn Fn(ChatbotConfigUpdateRequest)>,
    on_toggle: Callback<Section>,
}

impl PartialEq for SendersSectionProps {
    fn eq(&self, other: &Self) -> bool {
        self.open == other.open
            && self.sender_policy == other.sender_policy
            && self.allowlist == other.allowlist
            && std::rc::Rc::ptr_eq(&self.save_config, &other.save_config)
            && self.on_toggle == other.on_toggle
    }
}

#[component]
fn SendersSection(props: &SendersSectionProps) -> Html {
    let summary = sender_scope_label(&props.sender_policy, Some(&props.allowlist));

    let on_policy_change = {
        let save_config = props.save_config.clone();
        Callback::from(move |policy: String| {
            save_config(ChatbotConfigUpdateRequest {
                activo: None,
                display_name: None,
                language: None,
                tone: None,
                greeting: None,
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
        let save_config = props.save_config.clone();
        Callback::from(move |allowlist: Vec<String>| {
            save_config(ChatbotConfigUpdateRequest {
                activo: None,
                display_name: None,
                language: None,
                tone: None,
                greeting: None,
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

    html! {
        <AccordionSection
            section={Section::Senders}
            open={props.open}
            summary={AttrValue::from(summary)}
            on_toggle={props.on_toggle.clone()}
        >
            <SenderPolicyStep
                sender_policy={props.sender_policy.clone()}
                allowlist={props.allowlist.clone()}
                on_policy_change={on_policy_change}
                on_allowlist_change={on_allowlist_change}
            />
        </AccordionSection>
    }
}

#[derive(Properties, PartialEq)]
struct SaveIndicatorProps {
    status: SaveStatus,
}

#[component]
fn SaveIndicator(props: &SaveIndicatorProps) -> Html {
    match props.status {
        SaveStatus::Dirty => html! {
            <span class="text-xs" style="color: var(--color-warning-dark);">
                {"Cambios sin guardar"}
            </span>
        },
        SaveStatus::Saving => html! {
            <span class="text-xs text-[var(--text-tertiary)]">{"Guardando..."}</span>
        },
        SaveStatus::Saved => html! {
            <span class="text-xs" style="color: var(--color-success-dark);">
                {"Guardado"}
            </span>
        },
        SaveStatus::Idle => html! {},
    }
}

fn render_load_error(error: &UseStateHandle<Option<String>>) -> Html {
    html! {
        <div>
            if let Some(err) = (**error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }
            <p class="text-sm text-[var(--text-secondary)]">
                {"No se pudo cargar la configuraciÃ³n del chatbot."}
            </p>
        </div>
    }
}

const fn count_enabled(caps: &Capabilities) -> usize {
    (caps.receipt_ocr as usize)
        + (caps.balance_queries as usize)
        + (caps.payment_reminders as usize)
        + (caps.maintenance_requests as usize)
        + (caps.human_handoff as usize)
}

const fn activation_update(activo: bool) -> ChatbotConfigUpdateRequest {
    ChatbotConfigUpdateRequest {
        activo: Some(activo),
        display_name: None,
        language: None,
        tone: None,
        greeting: None,
        faqs: None,
        policies: None,
        sender_policy: None,
        allowlist: None,
        capabilities: None,
        handoff_keywords: None,
        history_limit: None,
        retention_days: None,
    }
}

fn sender_scope_label(policy: &str, allowlist: Option<&[String]>) -> String {
    match policy {
        "tenants_only" => "Solo inquilinos".to_string(),
        "tenants_and_prospects" => "Inquilinos y prospectos".to_string(),
        "allowlist" => allowlist.map_or_else(
            || "Lista permitida".to_string(),
            |l| format!("{} nÃºmeros permitidos", l.len()),
        ),
        _ => "Sin configurar".to_string(),
    }
}

fn persona_summary(cfg: &ChatbotConfigResponse) -> String {
    let name = cfg.display_name.as_deref().unwrap_or("Sin nombre");
    let tone = cfg.tone.as_deref().unwrap_or("sin tono definido");
    format!("{name} Â· {tone}")
}

fn knowledge_summary(faq_count: usize, has_policies: bool) -> String {
    let policies_label = if has_policies {
        "con polÃ­ticas"
    } else {
        "sin polÃ­ticas"
    };
    format!("{faq_count} preguntas Â· {policies_label}")
}
