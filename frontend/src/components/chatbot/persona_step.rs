use yew::prelude::*;

use crate::types::chatbot::ChatbotConfigResponse;

#[derive(Properties, PartialEq)]
pub struct PersonaStepProps {
    pub config: ChatbotConfigResponse,
    pub on_change: Callback<PersonaUpdate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonaUpdate {
    pub display_name: Option<String>,
    pub language: Option<String>,
    pub tone: Option<String>,
    pub greeting: Option<String>,
    pub system_prompt: Option<String>,
}

#[component]
pub fn PersonaStep(props: &PersonaStepProps) -> Html {
    let initial_name = props.config.display_name.clone().unwrap_or_default();
    let initial_tone = props.config.tone.clone().unwrap_or_default();
    let initial_greeting = props.config.greeting.clone().unwrap_or_default();
    let initial_prompt = props.config.system_prompt.clone().unwrap_or_default();

    let display_name = use_state(|| initial_name.clone());
    let tone = use_state(|| initial_tone.clone());
    let greeting = use_state(|| initial_greeting.clone());
    let system_prompt = use_state(|| initial_prompt.clone());
    let dirty = use_state(|| false);

    let mark_dirty = {
        let dirty = dirty.clone();
        move || dirty.set(true)
    };

    let on_name_input = {
        let display_name = display_name.clone();
        let mark_dirty = mark_dirty.clone();
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlInputElement = e.target_unchecked_into();
            display_name.set(el.value());
            mark_dirty();
        })
    };

    let on_tone_input = {
        let tone = tone.clone();
        let mark_dirty = mark_dirty.clone();
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlInputElement = e.target_unchecked_into();
            tone.set(el.value());
            mark_dirty();
        })
    };

    let on_greeting_input = {
        let greeting = greeting.clone();
        let mark_dirty = mark_dirty.clone();
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            greeting.set(el.value());
            mark_dirty();
        })
    };

    let on_prompt_input = {
        let system_prompt = system_prompt.clone();
        let mark_dirty = mark_dirty;
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            system_prompt.set(el.value());
            mark_dirty();
        })
    };

    let on_save = {
        let on_change = props.on_change.clone();
        let display_name = display_name.clone();
        let tone = tone.clone();
        let greeting = greeting.clone();
        let system_prompt = system_prompt.clone();
        let dirty = dirty.clone();
        Callback::from(move |_: MouseEvent| {
            on_change.emit(PersonaUpdate {
                display_name: Some((*display_name).clone()),
                language: None,
                tone: Some((*tone).clone()),
                greeting: Some((*greeting).clone()),
                system_prompt: Some((*system_prompt).clone()),
            });
            dirty.set(false);
        })
    };

    html! {
        <div class="gi-card" style="padding: var(--space-5);">
            <h3 class="text-base font-semibold mb-4">
                {"Personalidad del Chatbot"}
            </h3>

            <div class="flex flex-col gap-4">
                <div>
                    <label class="gi-label">{"Nombre del Bot (1-100 caracteres)"}</label>
                    <input
                        type="text"
                        class="gi-input"
                        maxlength="100"
                        placeholder="Ej: Asistente de Pagos"
                        value={(*display_name).clone()}
                        oninput={on_name_input}
                    />
                </div>

                <div>
                    <label class="gi-label">{"Tono (1-50 caracteres)"}</label>
                    <input
                        type="text"
                        class="gi-input"
                        maxlength="50"
                        placeholder="Ej: profesional, amigable"
                        value={(*tone).clone()}
                        oninput={on_tone_input}
                    />
                </div>

                <div>
                    <label class="gi-label">{"Saludo Inicial"}</label>
                    <textarea
                        class="gi-input"
                        rows="3"
                        placeholder="Mensaje de bienvenida para nuevos contactos"
                        value={(*greeting).clone()}
                        oninput={on_greeting_input}
                    />
                </div>

                <div>
                    <label class="gi-label">{"Prompt del Sistema (opcional)"}</label>
                    <textarea
                        class="gi-input"
                        rows="5"
                        placeholder="Instrucciones personalizadas para el comportamiento del bot"
                        value={(*system_prompt).clone()}
                        oninput={on_prompt_input}
                    />
                </div>
            </div>

            if *dirty {
                <div class="flex justify-end mt-4">
                    <button
                        type="button"
                        class="gi-btn gi-btn-primary"
                        onclick={on_save}
                    >
                        {"Guardar"}
                    </button>
                </div>
            }
        </div>
    }
}
