use yew::prelude::*;

use crate::types::chatbot::FaqEntry;

#[derive(Properties, PartialEq)]
pub struct KnowledgeStepProps {
    pub faqs: Vec<FaqEntry>,
    pub policies: String,
    pub on_faqs_change: Callback<Vec<FaqEntry>>,
    pub on_policies_change: Callback<String>,
}

#[component]
pub fn KnowledgeStep(props: &KnowledgeStepProps) -> Html {
    let faqs = use_state(|| props.faqs.clone());
    let policies = use_state(|| props.policies.clone());

    let on_add_faq = {
        let faqs = faqs.clone();
        let on_faqs_change = props.on_faqs_change.clone();
        Callback::from(move |_: MouseEvent| {
            let mut new_faqs = (*faqs).clone();
            new_faqs.push(FaqEntry {
                question: String::new(),
                answer: String::new(),
            });
            faqs.set(new_faqs.clone());
            on_faqs_change.emit(new_faqs);
        })
    };

    let on_remove_faq = {
        let faqs = faqs.clone();
        let on_faqs_change = props.on_faqs_change.clone();
        move |idx: usize| {
            let faqs = faqs.clone();
            let on_faqs_change = on_faqs_change.clone();
            Callback::from(move |_: MouseEvent| {
                let mut new_faqs = (*faqs).clone();
                if idx < new_faqs.len() {
                    new_faqs.remove(idx);
                }
                faqs.set(new_faqs.clone());
                on_faqs_change.emit(new_faqs);
            })
        }
    };

    let on_faq_question = {
        let faqs = faqs.clone();
        let on_faqs_change = props.on_faqs_change.clone();
        move |idx: usize| {
            let faqs = faqs.clone();
            let on_faqs_change = on_faqs_change.clone();
            Callback::from(move |e: InputEvent| {
                let el: web_sys::HtmlInputElement = e.target_unchecked_into();
                let mut new_faqs = (*faqs).clone();
                if let Some(entry) = new_faqs.get_mut(idx) {
                    entry.question = el.value();
                }
                faqs.set(new_faqs.clone());
                on_faqs_change.emit(new_faqs);
            })
        }
    };

    let on_faq_answer = {
        let faqs = faqs.clone();
        let on_faqs_change = props.on_faqs_change.clone();
        move |idx: usize| {
            let faqs = faqs.clone();
            let on_faqs_change = on_faqs_change.clone();
            Callback::from(move |e: InputEvent| {
                let el: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                let mut new_faqs = (*faqs).clone();
                if let Some(entry) = new_faqs.get_mut(idx) {
                    entry.answer = el.value();
                }
                faqs.set(new_faqs.clone());
                on_faqs_change.emit(new_faqs);
            })
        }
    };

    let on_policies_input = {
        let policies = policies.clone();
        let on_policies_change = props.on_policies_change.clone();
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            let val = el.value();
            policies.set(val.clone());
            on_policies_change.emit(val);
        })
    };

    let faq_count = faqs.len();

    html! {
        <div class="gi-card" style="padding: var(--space-5);">
            <h3 style="font-size: var(--text-base); font-weight: 600; margin-bottom: var(--space-4);">
                {"Base de Conocimiento"}
            </h3>

            <div style="margin-bottom: var(--space-5);">
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-3);">
                    <label class="gi-label" style="margin-bottom: 0;">
                        {format!("Preguntas Frecuentes ({faq_count}/50)")}
                    </label>
                    if faq_count < 50 {
                        <button
                            type="button"
                            class="gi-btn gi-btn-secondary"
                            style="font-size: var(--text-xs); padding: var(--space-1) var(--space-2);"
                            onclick={on_add_faq}
                        >
                            {"+ Agregar"}
                        </button>
                    }
                </div>

                <div style="display: flex; flex-direction: column; gap: var(--space-3);">
                    {for (*faqs).iter().enumerate().map(|(idx, faq)| {
                        html! {
                            <div style="border: 1px solid var(--border-default); border-radius: var(--radius-md); padding: var(--space-3);">
                                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-2);">
                                    <span style="font-size: var(--text-xs); color: var(--text-tertiary);">
                                        {format!("FAQ #{}", idx + 1)}
                                    </span>
                                    <button
                                        type="button"
                                        class="gi-btn gi-btn-secondary"
                                        style="font-size: var(--text-xs); padding: 2px 6px;"
                                        onclick={on_remove_faq(idx)}
                                    >
                                        {"✕"}
                                    </button>
                                </div>
                                <input
                                    type="text"
                                    class="gi-input"
                                    maxlength="200"
                                    placeholder="Pregunta (máx. 200 caracteres)"
                                    value={faq.question.clone()}
                                    oninput={on_faq_question(idx)}
                                    style="margin-bottom: var(--space-2);"
                                />
                                <textarea
                                    class="gi-input"
                                    rows="2"
                                    maxlength="1000"
                                    placeholder="Respuesta (máx. 1000 caracteres)"
                                    value={faq.answer.clone()}
                                    oninput={on_faq_answer(idx)}
                                />
                            </div>
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
