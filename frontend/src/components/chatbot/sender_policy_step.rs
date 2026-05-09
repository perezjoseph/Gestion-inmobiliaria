use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SenderPolicyStepProps {
    pub sender_policy: String,
    pub allowlist: Vec<String>,
    pub on_policy_change: Callback<String>,
    pub on_allowlist_change: Callback<Vec<String>>,
}

#[component]
pub fn SenderPolicyStep(props: &SenderPolicyStepProps) -> Html {
    let policy = use_state(|| props.sender_policy.clone());
    let allowlist = use_state(|| props.allowlist.clone());
    let new_phone = use_state(String::new);

    let on_policy_select = {
        let policy = policy.clone();
        let on_policy_change = props.on_policy_change.clone();
        move |value: &'static str| {
            let policy = policy.clone();
            let on_policy_change = on_policy_change.clone();
            Callback::from(move |_: InputEvent| {
                policy.set(value.to_string());
                on_policy_change.emit(value.to_string());
            })
        }
    };

    let on_add_phone = {
        let new_phone = new_phone.clone();
        let allowlist = allowlist.clone();
        let on_allowlist_change = props.on_allowlist_change.clone();
        Callback::from(move |_: MouseEvent| {
            let phone = (*new_phone).trim().to_string();
            if phone.is_empty() {
                return;
            }
            let mut new_list = (*allowlist).clone();
            if !new_list.contains(&phone) {
                new_list.push(phone);
            }
            allowlist.set(new_list.clone());
            on_allowlist_change.emit(new_list);
            new_phone.set(String::new());
        })
    };

    let on_remove_phone = {
        let allowlist = allowlist.clone();
        let on_allowlist_change = props.on_allowlist_change.clone();
        Callback::from(move |idx: usize| {
            let mut new_list = (*allowlist).clone();
            if idx < new_list.len() {
                new_list.remove(idx);
            }
            allowlist.set(new_list.clone());
            on_allowlist_change.emit(new_list);
        })
    };

    let on_phone_input = {
        let new_phone = new_phone.clone();
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlInputElement = e.target_unchecked_into();
            new_phone.set(el.value());
        })
    };

    html! {
        <div class="gi-card" style="padding: var(--space-5);">
            <h3 class="text-base font-semibold mb-4">
                {"Política de Remitentes"}
            </h3>

            <div class="flex flex-col gap-3 mb-4">
                <RadioOption
                    value="tenants_only"
                    label="Solo Inquilinos"
                    description="Solo los inquilinos registrados pueden interactuar con el bot"
                    selected={*policy == "tenants_only"}
                    on_select={on_policy_select("tenants_only")}
                />
                <RadioOption
                    value="tenants_and_prospects"
                    label="Inquilinos y Prospectos"
                    description="Cualquier persona puede enviar mensajes al bot"
                    selected={*policy == "tenants_and_prospects"}
                    on_select={on_policy_select("tenants_and_prospects")}
                />
                <RadioOption
                    value="allowlist"
                    label="Lista Permitida"
                    description="Solo los números en la lista pueden interactuar"
                    selected={*policy == "allowlist"}
                    on_select={on_policy_select("allowlist")}
                />
            </div>

            if *policy == "allowlist" {
                <AllowlistEditor
                    allowlist={(*allowlist).clone()}
                    new_phone={(*new_phone).clone()}
                    on_phone_input={on_phone_input}
                    on_add_phone={on_add_phone}
                    on_remove_phone={on_remove_phone}
                />
            }
        </div>
    }
}

// ---------------------------------------------------------------------------
// RadioOption sub-component
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct RadioOptionProps {
    value: &'static str,
    label: &'static str,
    description: &'static str,
    selected: bool,
    on_select: Callback<InputEvent>,
}

#[component]
fn RadioOption(props: &RadioOptionProps) -> Html {
    let border_class = if props.selected {
        "border-[var(--color-primary-500)]"
    } else {
        "border-[var(--border-default)]"
    };

    html! {
        <label class={classes!("flex", "items-start", "gap-3", "cursor-pointer", "p-3", "rounded-lg", "border", border_class)}>
            <input
                type="radio"
                name="sender_policy"
                checked={props.selected}
                oninput={props.on_select.clone()}
                class="mt-0.5"
            />
            <div>
                <div class="text-sm font-medium text-[var(--text-primary)]">
                    {props.label}
                </div>
                <div class="text-xs text-[var(--text-tertiary)]">
                    {props.description}
                </div>
            </div>
        </label>
    }
}

// ---------------------------------------------------------------------------
// AllowlistEditor sub-component
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct AllowlistEditorProps {
    allowlist: Vec<String>,
    new_phone: String,
    on_phone_input: Callback<InputEvent>,
    on_add_phone: Callback<MouseEvent>,
    #[prop_or_default]
    on_remove_phone: Callback<usize>,
}

#[component]
fn AllowlistEditor(props: &AllowlistEditorProps) -> Html {
    html! {
        <div class="border-t border-[var(--border-default)] pt-4">
            <label class="gi-label">{"Números Permitidos"}</label>
            <div class="flex gap-2 mb-3">
                <input
                    type="text"
                    class="gi-input flex-1"
                    placeholder="+18091234567"
                    value={props.new_phone.clone()}
                    oninput={props.on_phone_input.clone()}
                />
                <button
                    type="button"
                    class="gi-btn gi-btn-secondary"
                    onclick={props.on_add_phone.clone()}
                >
                    {"Agregar"}
                </button>
            </div>

            if !props.allowlist.is_empty() {
                <div class="flex flex-wrap gap-2">
                    {for props.allowlist.iter().enumerate().map(|(idx, phone)| {
                        let on_remove = props.on_remove_phone.clone();
                        let on_click = Callback::from(move |_: MouseEvent| {
                            on_remove.emit(idx);
                        });
                        html! {
                            <span class="inline-flex items-center gap-1 px-2 py-1 rounded-md text-xs"
                                style="background: var(--surface-raised);">
                                {phone.clone()}
                                <button
                                    type="button"
                                    class="bg-transparent border-none cursor-pointer text-[var(--text-tertiary)] text-xs"
                                    onclick={on_click}
                                    aria-label={format!("Eliminar {phone}")}
                                >
                                    {"✕"}
                                </button>
                            </span>
                        }
                    })}
                </div>
            }
        </div>
    }
}
