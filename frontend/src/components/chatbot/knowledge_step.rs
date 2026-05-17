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
        <div class="flex flex-col gap-5">
            <div>
                <div class="flex justify-between items-center mb-3">
                    <div>
                        <h4 class="text-sm font-semibold text-[var(--text-primary)]">
                            {"Preguntas frecuentes"}
                        </h4>
                        <p class="text-xs text-[var(--text-tertiary)] mt-0.5">
                            {format!("{faq_count} de 50 entradas. El bot responderá con estas respuestas cuando detecte la pregunta.")}
                        </p>
                    </div>
                    if faq_count < 50 {
                        <button
                            type="button"
                            class="gi-btn gi-btn-secondary text-xs"
                            onclick={on_add_faq}
                        >
                            {"+ Agregar pregunta"}
                        </button>
                    }
                </div>

                if faq_count == 0 {
                    <div
                        class="rounded-lg px-4 py-6 text-center"
                        style="border: 1px dashed var(--border-default); background: var(--surface-raised);"
                    >
                        <p class="text-sm text-[var(--text-secondary)]">
                            {"Sin preguntas frecuentes."}
                        </p>
                        <p class="text-xs text-[var(--text-tertiary)] mt-1">
                            {"Agregue respuestas a preguntas comunes para que el bot responda sin consultar la IA."}
                        </p>
                    </div>
                } else {
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
                }
            </div>

            <div>
                <label for="knowledge-policies" class="gi-label">
                    {"Políticas y reglas"}
                </label>
                <p class="text-xs text-[var(--text-tertiary)] mb-2">
                    {"Instrucciones que el bot seguirá siempre (ej. no dar información financiera sin verificar)."}
                </p>
                <textarea
                    id="knowledge-policies"
                    class="gi-input"
                    rows="6"
                    maxlength="5000"
                    placeholder="Ej: Nunca comparta datos bancarios por chat. Si un inquilino solicita cancelar contrato, transfiera a una persona."
                    value={(*policies).clone()}
                    oninput={on_policies_input}
                />
                <p class="text-xs text-[var(--text-tertiary)] mt-1 text-right">
                    {format!("{}/5000", policies.len())}
                </p>
            </div>

            if is_dirty {
                <div class="flex justify-end">
                    <button
                        type="button"
                        class="gi-btn gi-btn-primary"
                        onclick={on_save}
                    >
                        {"Guardar conocimiento"}
                    </button>
                </div>
            }
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
        <div
            class="rounded-lg p-3"
            style="border: 1px solid var(--border-default); background: var(--surface-raised);"
        >
            <div class="flex justify-between items-center mb-2">
                <span class="text-xs font-medium text-[var(--text-tertiary)]">
                    {format!("Pregunta {idx_label}")}
                </span>
                if *confirming {
                    <span class="flex items-center gap-2 text-xs">
                        <span class="text-[var(--text-secondary)]">{"¿Eliminar?"}</span>
                        <button
                            type="button"
                            class="gi-btn gi-btn-danger text-xs"
                            style="padding: 2px 8px;"
                            onclick={on_confirm_remove}
                        >
                            {"Sí"}
                        </button>
                        <button
                            type="button"
                            class="gi-btn gi-btn-secondary text-xs"
                            style="padding: 2px 8px;"
                            onclick={on_cancel_remove}
                        >
                            {"No"}
                        </button>
                    </span>
                } else {
                    <button
                        type="button"
                        class="bg-transparent border-none cursor-pointer p-1 rounded"
                        style="color: var(--text-tertiary);"
                        onclick={on_request_remove}
                        aria-label={format!("Eliminar pregunta {idx_label}")}
                    >
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <polyline points="3 6 5 6 21 6"/>
                            <path d="M19 6l-2 14a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L5 6"/>
                            <path d="M10 11v6"/>
                            <path d="M14 11v6"/>
                        </svg>
                    </button>
                }
            </div>
            <input
                type="text"
                class="gi-input mb-2"
                maxlength="200"
                placeholder="Ej: ¿Cuál es la fecha de corte?"
                value={props.question.clone()}
                oninput={props.on_question_change.clone()}
            />
            <textarea
                class="gi-input"
                rows="2"
                maxlength="1000"
                placeholder="Respuesta que el bot dará (máx. 1000 caracteres)"
                value={props.answer.clone()}
                oninput={props.on_answer_change.clone()}
            />
        </div>
    }
}
