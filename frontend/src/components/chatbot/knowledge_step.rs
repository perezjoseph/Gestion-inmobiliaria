use gloo_timers::callback::Timeout;
use yew::prelude::*;

use crate::types::chatbot::FaqEntry;

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
pub struct KnowledgeStepProps {
    pub faqs: Vec<FaqEntry>,
    pub policies: String,
    pub on_faqs_change: Callback<Vec<FaqEntry>>,
    pub on_policies_change: Callback<String>,
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

#[component]
pub fn KnowledgeStep(props: &KnowledgeStepProps) -> Html {
    let faqs = use_state(|| props.faqs.clone());
    let policies = use_state(|| props.policies.clone());
    let dirty = use_state(|| false);

    // --- FAQ mutations (local only, mark dirty) ---

    let on_add_faq = {
        let faqs = faqs.clone();
        let dirty = dirty.clone();
        Callback::from(move |_: MouseEvent| {
            let mut new_faqs = (*faqs).clone();
            new_faqs.push(FaqEntry {
                question: String::new(),
                answer: String::new(),
            });
            faqs.set(new_faqs);
            dirty.set(true);
        })
    };

    let on_faq_question = {
        let faqs = faqs.clone();
        let dirty = dirty.clone();
        move |idx: usize| {
            let faqs = faqs.clone();
            let dirty = dirty.clone();
            Callback::from(move |e: InputEvent| {
                let el: web_sys::HtmlInputElement = e.target_unchecked_into();
                let mut new_faqs = (*faqs).clone();
                if let Some(entry) = new_faqs.get_mut(idx) {
                    entry.question = el.value();
                }
                faqs.set(new_faqs);
                dirty.set(true);
            })
        }
    };

    let on_faq_answer = {
        let faqs = faqs.clone();
        let dirty = dirty.clone();
        move |idx: usize| {
            let faqs = faqs.clone();
            let dirty = dirty.clone();
            Callback::from(move |e: InputEvent| {
                let el: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                let mut new_faqs = (*faqs).clone();
                if let Some(entry) = new_faqs.get_mut(idx) {
                    entry.answer = el.value();
                }
                faqs.set(new_faqs);
                dirty.set(true);
            })
        }
    };

    let on_remove_confirmed = {
        let faqs = faqs.clone();
        let dirty = dirty.clone();
        move |idx: usize| {
            let faqs = faqs.clone();
            let dirty = dirty.clone();
            Callback::from(move |_: MouseEvent| {
                let mut new_faqs = (*faqs).clone();
                if idx < new_faqs.len() {
                    new_faqs.remove(idx);
                }
                faqs.set(new_faqs);
                dirty.set(true);
            })
        }
    };

    // --- Policies (local only, mark dirty) ---

    let on_policies_input = {
        let policies = policies.clone();
        let dirty = dirty.clone();
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            policies.set(el.value());
            dirty.set(true);
        })
    };

    // --- Save: emit to parent only on explicit click ---

    let on_save = {
        let faqs = faqs.clone();
        let policies = policies.clone();
        let dirty = dirty.clone();
        let on_faqs_change = props.on_faqs_change.clone();
        let on_policies_change = props.on_policies_change.clone();
        Callback::from(move |_: MouseEvent| {
            on_faqs_change.emit((*faqs).clone());
            on_policies_change.emit((*policies).clone());
            dirty.set(false);
        })
    };

    let faq_count = faqs.len();
    let is_dirty = *dirty;

    html! {
        <div class="gi-card" style="padding: var(--space-5);">
            <div class="flex items-center justify-between mb-4">
                <h3 class="text-base font-semibold">
                    {"Base de Conocimiento"}
                </h3>
                if is_dirty {
                    <button
                        type="button"
                        class="gi-btn gi-btn-primary"
                        onclick={on_save}
                    >
                        {"Guardar Cambios"}
                    </button>
                }
            </div>

            <div class="mb-5">
                <div class="flex justify-between items-center mb-3">
                    <label class="gi-label" style="margin-bottom: 0;">
                        {format!("Preguntas Frecuentes ({faq_count}/50)")}
                    </label>
                    if faq_count < 50 {
                        <button
                            type="button"
                            class="gi-btn gi-btn-secondary text-xs"
                            style="padding: var(--space-1) var(--space-2);"
                            onclick={on_add_faq}
                        >
                            {"+ Agregar"}
                        </button>
                    }
                </div>

                <div class="flex flex-col gap-3">
                    {for (*faqs).iter().enumerate().map(|(idx, faq)| {
                        html! {
                            <FaqItem
                                key={idx}
                                idx={idx}
                                question={AttrValue::from(faq.question.clone())}
                                answer={AttrValue::from(faq.answer.clone())}
                                on_question_change={on_faq_question(idx)}
                                on_answer_change={on_faq_answer(idx)}
                                on_remove={on_remove_confirmed(idx)}
                            />
                        }
                    })}
                </div>
            </div>

            <div>
                <label class="gi-label">{"Políticas (máx. 5000 caracteres)"}</label>
                <textarea
                    class="gi-input"
                    rows="6"
                    maxlength="5000"
                    placeholder="Reglas y políticas que el bot debe seguir al responder"
                    value={(*policies).clone()}
                    oninput={on_policies_input}
                />
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// FaqItem sub-component (inline confirmation on delete)
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct FaqItemProps {
    idx: usize,
    question: AttrValue,
    answer: AttrValue,
    on_question_change: Callback<InputEvent>,
    on_answer_change: Callback<InputEvent>,
    on_remove: Callback<MouseEvent>,
}

#[component]
fn FaqItem(props: &FaqItemProps) -> Html {
    let confirming = use_state(|| false);

    // When user clicks ✕, show inline confirmation
    let on_request_remove = {
        let confirming = confirming.clone();
        Callback::from(move |_: MouseEvent| {
            confirming.set(true);
        })
    };

    // Auto-revert confirmation after 3 seconds
    {
        let confirming = confirming.clone();
        use_effect_with(*confirming, move |is_confirming| {
            let handle = if *is_confirming {
                Some(Timeout::new(3_000, move || {
                    confirming.set(false);
                }))
            } else {
                None
            };
            move || drop(handle)
        });
    }

    let on_cancel_remove = {
        let confirming = confirming.clone();
        Callback::from(move |_: MouseEvent| {
            confirming.set(false);
        })
    };

    let on_confirm_remove = {
        let confirming = confirming.clone();
        let on_remove = props.on_remove.clone();
        Callback::from(move |e: MouseEvent| {
            confirming.set(false);
            on_remove.emit(e);
        })
    };

    let idx_label = props.idx + 1;

    html! {
        <div class="border border-[var(--border-default)] rounded-lg p-3">
            <div class="flex justify-between items-center mb-2">
                <span class="text-xs text-[var(--text-tertiary)]">
                    {format!("FAQ #{idx_label}")}
                </span>
                if *confirming {
                    <span class="flex items-center gap-1 text-xs">
                        <span class="text-[var(--text-secondary)]">{"¿Eliminar?"}</span>
                        <button
                            type="button"
                            class="gi-btn gi-btn-danger text-xs"
                            style="padding: 2px 6px;"
                            onclick={on_confirm_remove}
                        >
                            {"Sí"}
                        </button>
                        <button
                            type="button"
                            class="gi-btn gi-btn-secondary text-xs"
                            style="padding: 2px 6px;"
                            onclick={on_cancel_remove}
                        >
                            {"No"}
                        </button>
                    </span>
                } else {
                    <button
                        type="button"
                        class="gi-btn gi-btn-secondary text-xs"
                        style="padding: 2px 6px;"
                        onclick={on_request_remove}
                        aria-label={format!("Eliminar FAQ #{idx_label}")}
                    >
                        {"✕"}
                    </button>
                }
            </div>
            <input
                type="text"
                class="gi-input mb-2"
                maxlength="200"
                placeholder="Pregunta (máx. 200 caracteres)"
                value={props.question.clone()}
                oninput={props.on_question_change.clone()}
            />
            <textarea
                class="gi-input"
                rows="2"
                maxlength="1000"
                placeholder="Respuesta (máx. 1000 caracteres)"
                value={props.answer.clone()}
                oninput={props.on_answer_change.clone()}
            />
        </div>
    }
}
