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
        move |idx: usize| {
            let allowlist = allowlist.clone();
            let on_allowlist_change = on_allowlist_change.clone();
            Callback::from(move |_: MouseEvent| {
                let mut new_list = (*allowlist).clone();
                if idx < new_list.len() {
                    new_list.remove(idx);
                }
                allowlist.set(new_list.clone());
                on_allowlist_change.emit(new_list);
            })
        }
    };

    let on_phone_input = {
        let new_phone = new_phone.clone();
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlInputElement = e.target_unchecked_into();
            new_phone.set(el.value());
        })
    };

    let radio_option =
        |value: &'static str, label: &str, description: &str| {
            let is_selected = *policy == value;
            html! {
                <label style={format!(
                    "display: flex; align-items: flex-start; gap: var(--space-3); cursor: pointer; padding: var(--space-3); border-radius: var(--radius-md); border: 1px solid {};",
                    if is_selected { "var(--color-primary)" } else { "var(--border-default)" }
                )}>
                    <input
                        type="radio"
                        name="sender_policy"
                        checked={is_selected}
                        oninput={on_policy_select(value)}
                        style="margin-top: 2px;"
                    />
                    <div>
                        <div style="font-size: var(--text-sm); font-weight: 500; color: var(--text-primary);">
                            {label}
                        </div>
                        <div style="font-size: var(--text-xs); color: var(--text-tertiary);">
                            {description}
                        </div>
                    </div>
                </label>
            }
        };

    html! {
        <div class="gi-card" style="padding: var(--space-5);">
            <h3 style="font-size: var(--text-base); font-weight: 600; margin-bottom: var(--space-4);">
                {"Política de Remitentes"}
            </h3>

            <div style="display: flex; flex-direction: column; gap: var(--space-3); margin-bottom: var(--space-4);">
                {radio_option(
                    "tenants_only",
                    "Solo Inquilinos",
                    "Solo los inquilinos registrados pueden interactuar con el bot",
                )}
                {radio_option(
                    "tenants_and_prospects",
                    "Inquilinos y Prospectos",
                    "Cualquier persona puede enviar mensajes al bot",
                )}
                {radio_option(
                    "allowlist",
                    "Lista Permitida",
                    "Solo los números en la lista pueden interactuar",
                )}
            </div>

            if *policy == "allowlist" {
                <div style="border-top: 1px solid var(--border-default); padding-top: var(--space-4);">
                    <label class="gi-label">{"Números Permitidos"}</label>
                    <div style="display: flex; gap: var(--space-2); margin-bottom: var(--space-3);">
                        <input
                            type="text"
                            class="gi-input"
                            placeholder="+18091234567"
                            value={(*new_phone).clone()}
                            oninput={on_phone_input}
                            style="flex: 1;"
                        />
                        <button
                            type="button"
                            class="gi-btn gi-btn-secondary"
                            onclick={on_add_phone}
                        >
                            {"Agregar"}
                        </button>
                    </div>

                    if !allowlist.is_empty() {
                        <div style="display: flex; flex-wrap: wrap; gap: var(--space-2);">
                            {for (*allowlist).iter().enumerate().map(|(idx, phone)| {
                                html! {
                                    <span style="display: inline-flex; align-items: center; gap: var(--space-1); padding: var(--space-1) var(--space-2); background: var(--surface-raised); border-radius: var(--radius-sm); font-size: var(--text-xs);">
                                        {phone.clone()}
                                        <button
                                            type="button"
                                            style="background: none; border: none; cursor: pointer; color: var(--text-tertiary); font-size: var(--text-xs);"
                                            onclick={on_remove_phone(idx)}
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
        </div>
    }
}
