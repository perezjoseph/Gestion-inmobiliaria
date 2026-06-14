use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::chatbot::toggle_switch::ToggleSwitch;
use crate::services::chatbot::{create_guidance_rule, delete_guidance_rule, update_guidance_rule};
use crate::types::chatbot::{
    CreateGuidanceRuleRequest, GuidanceCategory, GuidanceRule, UpdateGuidanceRuleRequest,
};

fn category_entries() -> Vec<(GuidanceCategory, &'static str)> {
    vec![
        (
            GuidanceCategory::EstiloComunicacion,
            "Estilo de comunicación",
        ),
        (
            GuidanceCategory::ContextoClarificacion,
            "Contexto y clarificación",
        ),
        (GuidanceCategory::Escalamiento, "Escalamiento"),
        (GuidanceCategory::Politicas, "Políticas"),
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

    html! {
        <div class="flex flex-col gap-4">
            <div class="flex items-center justify-between">
                <h3 class="text-base font-semibold text-[var(--text-primary)]">
                    <span aria-live="polite">
                        {format!("Reglas del agente ({active_count}/30 activas)")}
                    </span>
                </h3>
            </div>
            {for category_entries().iter().map(|(cat, label)| {
                let cat_rules: Vec<GuidanceRule> = props.rules.iter()
                    .filter(|r| r.category == *cat)
                    .cloned()
                    .collect();
                html! {
                    <CategorySection
                        category={cat.clone()}
                        label={AttrValue::from(*label)}
                        rules={cat_rules}
                        on_change={props.on_change.clone()}
                    />
                }
            })}
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct CategorySectionProps {
    category: GuidanceCategory,
    label: AttrValue,
    rules: Vec<GuidanceRule>,
    on_change: Callback<()>,
}

#[component]
fn CategorySection(props: &CategorySectionProps) -> Html {
    let expanded = use_state(|| true);
    let adding = use_state(|| false);

    let active_in_cat = props.rules.iter().filter(|r| r.enabled).count();

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

    let chevron = if *expanded { "▼" } else { "▶" };
    let section_id = format!(
        "category-header-{}",
        props.label.to_lowercase().replace(' ', "-")
    );

    html! {
        <div
            class="rounded-lg"
            style="border: 1px solid var(--border-subtle); background: var(--surface-raised);"
            role="region"
            aria-labelledby={section_id.clone()}
        >
            <button
                type="button"
                id={section_id}
                class="flex items-center gap-2 w-full px-4 py-3 text-left"
                onclick={toggle_expand}
                aria-expanded={(*expanded).to_string()}
            >
                <span class="text-xs text-[var(--text-tertiary)]" aria-hidden="true">{chevron}</span>
                <span class="text-sm font-medium text-[var(--text-primary)] flex-1">
                    {format!("{} ({active_in_cat} activas)", &props.label)}
                </span>
            </button>
            if *expanded {
                <div class="flex flex-col gap-1 px-4 pb-3" role="list">
                    {for props.rules.iter().map(|rule| {
                        html! {
                            <RuleRow
                                rule={rule.clone()}
                                on_change={props.on_change.clone()}
                            />
                        }
                    })}
                    if *adding {
                        <InlineRuleForm
                            on_save={on_add_save.clone()}
                            on_cancel={on_add_cancel}
                        />
                    } else {
                        <button
                            type="button"
                            class="text-xs text-[var(--color-primary-500)] hover:underline mt-1 self-start"
                            onclick={on_add_click}
                        >
                            {"+ Agregar regla"}
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

    html! {
        <div class="flex items-start gap-2 py-1.5 group" role="listitem">
            <ToggleSwitch
                checked={props.rule.enabled}
                on_toggle={on_toggle}
                label={AttrValue::from(props.rule.instruction.clone())}
            />
            if props.rule.is_template {
                <span class="text-xs mt-0.5" title="Regla predefinida" aria-hidden="true">{"🔒"}</span>
            }
            <span class="text-sm text-[var(--text-primary)] flex-1 leading-snug mt-0.5">
                {&props.rule.instruction}
            </span>
            if !props.rule.is_template {
                <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                        type="button"
                        class="text-xs p-1 rounded hover:bg-[var(--surface-base)]"
                        title="Editar regla"
                        aria-label="Editar regla"
                        onclick={on_edit_click}
                    >
                        {"✏️"}
                    </button>
                    <button
                        type="button"
                        class="text-xs p-1 rounded hover:bg-[var(--surface-base)]"
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

    html! {
        <div
            class="flex flex-col gap-2 p-2 rounded-md"
            style="background: var(--surface-base); border: 1px solid var(--border-subtle);"
            role="form"
            aria-label="Crear nueva regla"
        >
            <textarea
                class="gi-input text-sm"
                rows="2"
                maxlength="500"
                placeholder="Escribe la instrucción para el agente..."
                aria-label="Instrucción de la regla"
                value={(*text).clone()}
                oninput={on_input}
            />
            <div class="flex items-center justify-between">
                <span class="text-xs text-[var(--text-tertiary)]">
                    {format!("{char_count}/500")}
                </span>
                <div class="flex gap-2">
                    <button
                        type="button"
                        class="text-xs px-2 py-1 rounded text-[var(--text-secondary)] hover:bg-[var(--surface-raised)]"
                        onclick={on_cancel_click}
                    >
                        {"Cancelar"}
                    </button>
                    <button
                        type="button"
                        class="text-xs px-2 py-1 rounded gi-btn-primary"
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
