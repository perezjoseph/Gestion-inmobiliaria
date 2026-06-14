use yew::prelude::*;

use crate::types::chatbot::ChatbotConfigResponse;

const TONE_PRESETS: &[(&str, &str)] = &[
    ("Formal", "formal, profesional, usted"),
    ("Cercano", "cercano, amable, tÃº"),
    ("Directo", "directo, breve, sin rodeos"),
    ("Cordial", "cordial, respetuoso, servicial"),
    ("Amistoso", "amistoso, cÃ¡lido, conversacional"),
];

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
}

#[component]
pub fn PersonaStep(props: &PersonaStepProps) -> Html {
    let initial_name = props.config.display_name.clone().unwrap_or_default();
    let initial_tone = props.config.tone.clone().unwrap_or_default();
    let initial_greeting = props.config.greeting.clone().unwrap_or_default();

    let display_name = use_state(|| initial_name);
    let tone = use_state(|| initial_tone);
    let greeting = use_state(|| initial_greeting);
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

    let on_save = {
        let on_change = props.on_change.clone();
        let display_name = display_name.clone();
        let tone = tone.clone();
        let greeting = greeting.clone();
        let dirty = dirty.clone();
        Callback::from(move |_: MouseEvent| {
            on_change.emit(PersonaUpdate {
                display_name: Some((*display_name).clone()),
                language: None,
                tone: Some((*tone).clone()),
                greeting: Some((*greeting).clone()),
            });
            dirty.set(false);
        })
    };

    let on_chip_click = {
        let tone = tone.clone();
        let mark_dirty = mark_dirty;
        move |value: &'static str| {
            let tone = tone.clone();
            let mark_dirty = mark_dirty.clone();
            Callback::from(move |_: MouseEvent| {
                tone.set(value.to_string());
                mark_dirty();
            })
        }
    };

    let current_tone = (*tone).clone();
    let current_name = (*display_name).clone();
    let current_greeting = (*greeting).clone();

    html! {
        <div class="grid gap-5" style="display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);">
            <div class="flex flex-col gap-4">
                <div>
                    <label for="persona-name" class="gi-label">
                        {"Nombre del bot"}
                    </label>
                    <input
                        id="persona-name"
                        type="text"
                        class="gi-input"
                        maxlength="100"
                        placeholder="Ej: Asistente de Rent-DR"
                        value={(*display_name).clone()}
                        oninput={on_name_input}
                    />
                    <p class="text-xs text-[var(--text-tertiary)] mt-1">
                        {"AsÃ se presentarÃ¡ en el primer mensaje."}
                    </p>
                </div>

                <div>
                    <span class="gi-label">{"Tono"}</span>
                    <div class="flex flex-wrap gap-2 mb-2">
                        {for TONE_PRESETS.iter().map(|(label, value)| {
                            let selected = current_tone == *value;
                            html! {
                                <button
                                    type="button"
                                    class={classes!(
                                        "gi-chip",
                                        selected.then_some("gi-chip-selected"),
                                    )}
                                    aria-pressed={selected.to_string()}
                                    onclick={on_chip_click(value)}
                                >
                                    {label}
                                </button>
                            }
                        })}
                    </div>
                    <input
                        type="text"
                        class="gi-input"
                        maxlength="200"
                        placeholder="O escriba un tono propio"
                        value={(*tone).clone()}
                        oninput={on_tone_input}
                    />
                </div>

                <div>
                    <label for="persona-greeting" class="gi-label">
                        {"Saludo inicial"}
                    </label>
                    <textarea
                        id="persona-greeting"
                        class="gi-input"
                        rows="3"
                        maxlength="500"
                        placeholder="Mensaje de bienvenida para nuevos contactos"
                        value={(*greeting).clone()}
                        oninput={on_greeting_input}
                    />
                </div>

                if *dirty {
                    <div class="flex justify-end">
                        <button
                            type="button"
                            class="gi-btn gi-btn-primary"
                            onclick={on_save}
                        >
                            {"Guardar personalidad"}
                        </button>
                    </div>
                }
            </div>

            <PersonaPreview
                name={current_name}
                tone={current_tone}
                greeting={current_greeting}
            />
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct PersonaPreviewProps {
    name: String,
    tone: String,
    greeting: String,
}

#[component]
fn PersonaPreview(props: &PersonaPreviewProps) -> Html {
    let display_name = if props.name.trim().is_empty() {
        "Asistente".to_string()
    } else {
        props.name.clone()
    };

    let greeting = if props.greeting.trim().is_empty() {
        format!("Hola, soy {display_name}. Â¿En quÃ© puedo ayudarle?")
    } else {
        props.greeting.clone()
    };

    let sample_reply = sample_reply_for_tone(&props.tone, &display_name);

    html! {
        <aside
            class="rounded-lg flex flex-col"
            style="background: var(--surface-raised); border: 1px solid var(--border-default); min-height: 320px;"
            aria-label="Vista previa del tono del bot"
        >
            <header class="flex items-center gap-3 px-4 py-3" style="border-bottom: 1px solid var(--border-subtle);">
                <div
                    class="flex items-center justify-center rounded-full text-xs font-semibold"
                    style="width: 32px; height: 32px; background: var(--color-primary-500); color: var(--text-on-primary);"
                    aria-hidden="true"
                >
                    {initials_of(&display_name)}
                </div>
                <div class="flex flex-col">
                    <span class="text-sm font-semibold text-[var(--text-primary)]">
                        {display_name.clone()}
                    </span>
                    <span class="text-xs text-[var(--text-tertiary)]">
                        {"Vista previa"}
                    </span>
                </div>
            </header>

            <div class="flex flex-col gap-2 px-4 py-4 flex-1">
                <PreviewBubble
                    from_user={true}
                    text={AttrValue::from("Â¿CuÃ¡nto debo este mes?")}
                />
                <PreviewBubble
                    from_user={false}
                    text={AttrValue::from(greeting)}
                />
                <PreviewBubble
                    from_user={false}
                    text={AttrValue::from(sample_reply)}
                />
            </div>

            <footer class="px-4 py-2 text-xs text-[var(--text-tertiary)]" style="border-top: 1px solid var(--border-subtle);">
                {"Los mensajes reales pueden variar segÃºn la pregunta del inquilino."}
            </footer>
        </aside>
    }
}

#[derive(Properties, PartialEq)]
struct PreviewBubbleProps {
    from_user: bool,
    text: AttrValue,
}

#[component]
fn PreviewBubble(props: &PreviewBubbleProps) -> Html {
    let (align, style) = if props.from_user {
        (
            "self-end",
            "padding: var(--space-2) var(--space-3); background: var(--color-primary-500); color: var(--text-on-primary); max-width: 85%;",
        )
    } else {
        (
            "self-start",
            "padding: var(--space-2) var(--space-3); background: var(--surface-base); color: var(--text-primary); max-width: 85%; border: 1px solid var(--border-subtle);",
        )
    };

    html! {
        <div class={classes!(align, "rounded-lg", "text-sm", "leading-relaxed")} style={style}>
            {&props.text}
        </div>
    }
}

fn sample_reply_for_tone(tone: &str, name: &str) -> String {
    let t = tone.to_lowercase();
    if t.contains("formal") || t.contains("usted") {
        "Con gusto le ayudo. SegÃºn nuestro sistema, su saldo pendiente al dÃa de hoy es de RD$ 18,500.00. Â¿Desea que le envÃe el recibo correspondiente?".to_string()
    } else if t.contains("directo") || t.contains("breve") {
        "Debe RD$ 18,500.00. Vence el 15 de este mes.".to_string()
    } else if t.contains("cercano")
        || t.contains("amistoso")
        || t.contains("amable")
        || t.contains("tÃº")
        || t.contains("tu")
    {
        "Â¡Hola! Claro, revisÃ© tu cuenta y tienes pendiente RD$ 18,500.00 para este mes. Â¿Te envÃo los datos para pagar?".to_string()
    } else {
        format!(
            "Su saldo pendiente es de RD$ 18,500.00 con vencimiento el 15 de este mes. DÃgame si necesita los datos bancarios o puedo ayudarle con algo mÃ¡s. {name} a la orden."
        )
    }
}

fn initials_of(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .collect::<String>()
        .to_uppercase()
}
