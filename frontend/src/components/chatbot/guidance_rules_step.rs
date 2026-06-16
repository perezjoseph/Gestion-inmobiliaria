use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::chatbot::toggle_switch::ToggleSwitch;
use crate::services::chatbot::{create_guidance_rule, delete_guidance_rule, update_guidance_rule};
use crate::types::chatbot::{
    CreateGuidanceRuleRequest, GuidanceCategory, GuidanceRule, UpdateGuidanceRuleRequest,
};

fn category_entries() -> Vec<(GuidanceCategory, &'static str, &'static str)> {
    vec![
        (
            GuidanceCategory::EstiloComunicacion,
            "Estilo de comunicación",
            "💬",
        ),
        (
            GuidanceCategory::ContextoClarificacion,
            "Contexto y clarificación",
            "🔍",
        ),
        (GuidanceCategory::Escalamiento, "Escalamiento", "📢"),
        (GuidanceCategory::Politicas, "Políticas", "📋"),
    ]
}

#[derive(Properties, PartialEq)]
pub struct GuidanceRulesStepProps {
    pub rules: Vec<GuidanceRule>,
    pub on_change: Callback<()>,
}

#[component]
pub fn GuidanceRulesStep(props: &GuidanceRulesStepProps) -> Html {
    let active_count = props.rules.iter().filter(|r| r.enabled).count();
    let total_count = props.rules.len();

    html! {
        <div class="flex flex-col gap-5">
            <div class="flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <h3
                        class="text-base font-semibold text-[var(--text-primary)]"
                        style="font-family: var(--font-display);"
                    >
                        {"Reglas del agente"}
                    </h3>
                    <span
                        class="gi-guidance-counter"
                        aria-live="polite"
                    >
                        {format!("{active_count}/30")}
                    </span>
                </div>
                if total_count > 0 {
                    <span class="text-xs text-[var(--text-tertiary)]">
                        {format!("{total_count} regla{}", if total_count == 1 { "" } else { "s" })}
                    </span>
                }
            </div>
            <div class="flex flex-col gap-3">
                {for category_entries().iter().map(|(cat, label, icon)| {
                    let cat_rules: Vec<GuidanceRule> = props.rules.iter()
                        .filter(|r| r.category == *cat)
                        .cloned()
                        .collect();
                    html! {
                        <CategorySection
                            category={cat.clone()}
                            label={AttrValue::from(*label)}
                            icon={AttrValue::from(*icon)}
                            rules={cat_rules}
                            on_change={props.on_change.clone()}
                        />
                    }
                })}
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct CategorySectionProps {
    category: GuidanceCategory,
    label: AttrValue,
    icon: AttrValue,
    rules: Vec<GuidanceRule>,
    on_change: Callback<()>,
}

#[component]
fn CategorySection(props: &CategorySectionProps) -> Html {
    let expanded = use_state(|| true);
    let adding = use_state(|| false);

    let active_in_cat = props.rules.iter().filter(|r| r.enabled).count();
    let total_in_cat = props.rules.len();

    let toggle_expand = {
        let expanded = expanded.clone();
        Callback::from(move |_: MouseEvent| {
            expanded.set(!*expanded);
        })
    };

    let on_add_click = {
        let adding = adding.clone();
        Callback::from(move |_: MouseEvent| {
            adding.set(true);
        })
    };

    let on_add_cancel = {
        let adding = adding.clone();
        Callback::from(move |(): ()| {
            adding.set(false);
        })
    };

    let on_add_save = {
        let adding = adding.clone();
        let category = props.category.clone();
        let on_change = props.on_change.clone();
        Callback::from(move |instruction: String| {
            let category = category.clone();
            let on_change = on_change.clone();
            let adding = adding.clone();
            spawn_local(async move {
                let req = CreateGuidanceRuleRequest {
                    category,
                    instruction,
                    enabled: Some(true),
                };
                if create_guidance_rule(&req).await.is_ok() {
                    on_change.emit(());
                }
                adding.set(false);
            });
        })
    };

    let section_id = format!(
        "category-header-{}",
        props.label.to_lowercase().replace(' ', "-")
    );

    let has_rules = total_in_cat > 0;

    html! {
        <div
            class="gi-guidance-category"
            role="region"
            aria-labelledby={section_id.clone()}
        >
            <button
                type="button"
                id={section_id}
                class="gi-guidance-category-header"
                onclick={toggle_expand}
                aria-expanded={(*expanded).to_string()}
            >
                <span class="gi-guidance-category-icon" aria-hidden="true">
                    {&props.icon}
                </span>
                <span class="gi-guidance-category-title">
                    {&props.label}
                </span>
                if has_rules {
                    <span class="gi-guidance-category-badge">
                        {format!("{active_in_cat}/{total_in_cat}")}
                    </span>
                }
                <span class="gi-guidance-chevron" aria-hidden="true" data-expanded={(*expanded).to_string()}>
                    {"›"}
                </span>
            </button>
            if *expanded {
                <div class="gi-guidance-category-body" role="list">
                    if has_rules {
                        {for props.rules.iter().map(|rule| {
                            html! {
                                <RuleRow
                                    rule={rule.clone()}
                                    on_change={props.on_change.clone()}
                                />
                            }
                        })}
                    }
                    if *adding {
                        <InlineRuleForm
                            on_save={on_add_save.clone()}
                            on_cancel={on_add_cancel}
                        />
                    } else {
                        <button
                            type="button"
                            class="gi-guidance-add-btn"
                            onclick={on_add_click}
                        >
                            <span class="gi-guidance-add-icon" aria-hidden="true">{"+"}</span>
                            {"Agregar regla"}
                        </button>
                    }
                </div>
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct RuleRowProps {
    rule: GuidanceRule,
    on_change: Callback<()>,
}

#[component]
fn RuleRow(props: &RuleRowProps) -> Html {
    let editing = use_state(|| false);

    let on_toggle = {
        let id = props.rule.id.clone();
        let enabled = props.rule.enabled;
        let on_change = props.on_change.clone();
        Callback::from(move |_: InputEvent| {
            let id = id.clone();
            let on_change = on_change.clone();
            spawn_local(async move {
                let req = UpdateGuidanceRuleRequest {
                    instruction: None,
                    enabled: Some(!enabled),
                    sort_order: None,
                };
                if update_guidance_rule(&id, &req).await.is_ok() {
                    on_change.emit(());
                }
            });
        })
    };

    let on_delete = {
        let id = props.rule.id.clone();
        let on_change = props.on_change.clone();
        Callback::from(move |_: MouseEvent| {
            let id = id.clone();
            let on_change = on_change.clone();
            spawn_local(async move {
                if delete_guidance_rule(&id).await.is_ok() {
                    on_change.emit(());
                }
            });
        })
    };

    let on_edit_click = {
        let editing = editing.clone();
        Callback::from(move |_: MouseEvent| {
            editing.set(true);
        })
    };

    let on_edit_save = {
        let editing = editing.clone();
        let id = props.rule.id.clone();
        let on_change = props.on_change.clone();
        Callback::from(move |instruction: String| {
            let id = id.clone();
            let on_change = on_change.clone();
            let editing = editing.clone();
            spawn_local(async move {
                let req = UpdateGuidanceRuleRequest {
                    instruction: Some(instruction),
                    enabled: None,
                    sort_order: None,
                };
                if update_guidance_rule(&id, &req).await.is_ok() {
                    on_change.emit(());
                }
                editing.set(false);
            });
        })
    };

    let on_edit_cancel = {
        let editing = editing.clone();
        Callback::from(move |(): ()| {
            editing.set(false);
        })
    };

    if *editing {
        return html! {
            <InlineRuleForm
                initial_value={AttrValue::from(props.rule.instruction.clone())}
                on_save={on_edit_save}
                on_cancel={on_edit_cancel}
            />
        };
    }

    let opacity_class = if props.rule.enabled {
        ""
    } else {
        " gi-guidance-rule-disabled"
    };

    html! {
        <div class={format!("gi-guidance-rule{opacity_class}")} role="listitem">
            <div class="gi-guidance-rule-toggle">
                <ToggleSwitch
                    checked={props.rule.enabled}
                    on_toggle={on_toggle}
                    label={AttrValue::from(props.rule.instruction.clone())}
                />
            </div>
            <span class="gi-guidance-rule-text">
                {&props.rule.instruction}
            </span>
            if props.rule.is_template {
                <span class="gi-guidance-rule-lock" title="Regla predefinida" aria-label="Regla predefinida">
                    {"🔒"}
                </span>
            }
            if !props.rule.is_template {
                <div class="gi-guidance-rule-actions">
                    <button
                        type="button"
                        class="gi-guidance-action-btn"
                        title="Editar regla"
                        aria-label="Editar regla"
                        onclick={on_edit_click}
                    >
                        {"✏️"}
                    </button>
                    <button
                        type="button"
                        class="gi-guidance-action-btn gi-guidance-action-btn--danger"
                        title="Eliminar regla"
                        aria-label="Eliminar regla"
                        onclick={on_delete}
                    >
                        {"🗑️"}
                    </button>
                </div>
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct InlineRuleFormProps {
    #[prop_or_default]
    initial_value: AttrValue,
    on_save: Callback<String>,
    on_cancel: Callback<()>,
}

#[component]
fn InlineRuleForm(props: &InlineRuleFormProps) -> Html {
    let text = use_state(|| (*props.initial_value).to_string());
    let char_count = text.len();

    let on_input = {
        let text = text.clone();
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            let val = el.value();
            if val.len() <= 500 {
                text.set(val);
            } else {
                text.set(val[..500].to_string());
            }
        })
    };

    let on_save_click = {
        let text = text.clone();
        let on_save = props.on_save.clone();
        Callback::from(move |_: MouseEvent| {
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() {
                on_save.emit(trimmed);
            }
        })
    };

    let on_cancel_click = {
        let on_cancel = props.on_cancel.clone();
        Callback::from(move |_: MouseEvent| {
            on_cancel.emit(());
        })
    };

    let counter_class = if char_count >= 450 {
        "gi-guidance-form-counter gi-guidance-form-counter--warn"
    } else {
        "gi-guidance-form-counter"
    };

    html! {
        <div
            class="gi-guidance-form"
            role="form"
            aria-label="Crear nueva regla"
        >
            <textarea
                class="gi-input gi-guidance-form-textarea"
                rows="3"
                maxlength="500"
                placeholder="Escribe la instrucción para el agente..."
                aria-label="Instrucción de la regla"
                value={(*text).clone()}
                oninput={on_input}
            />
            <div class="gi-guidance-form-footer">
                <span class={counter_class}>
                    {format!("{char_count}/500")}
                </span>
                <div class="gi-guidance-form-actions">
                    <button
                        type="button"
                        class="gi-btn-secondary gi-guidance-form-btn"
                        onclick={on_cancel_click}
                    >
                        {"Cancelar"}
                    </button>
                    <button
                        type="button"
                        class="gi-btn-primary gi-guidance-form-btn"
                        disabled={text.trim().is_empty()}
                        onclick={on_save_click}
                    >
                        {"Guardar"}
                    </button>
                </div>
            </div>
        </div>
    }
}
