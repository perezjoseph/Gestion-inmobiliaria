use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::error_banner::ErrorBanner;
use crate::services::auth::login;
use crate::types::usuario::{LoginRequest, LoginResponse};

#[derive(Properties, PartialEq)]
pub struct LoginFormProps {
    pub on_success: Callback<LoginResponse>,
}

#[function_component]
pub fn LoginForm(props: &LoginFormProps) -> Html {
    let email = use_state(String::new);
    let password = use_state(String::new);
    let email_error = use_state(|| Option::<String>::None);
    let password_error = use_state(|| Option::<String>::None);
    let server_error = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);
    let on_success = props.on_success.clone();

    let validate = {
        let email = email.clone();
        let password = password.clone();
        let email_error = email_error.clone();
        let password_error = password_error.clone();
        move || -> bool {
            let mut valid = true;
            if email.trim().is_empty() || !email.contains('@') {
                email_error.set(Some("Correo electrónico inválido".into()));
                valid = false;
            } else {
                email_error.set(None);
            }
            if password.len() < 6 {
                password_error.set(Some(
                    "La contraseña debe tener al menos 6 caracteres".into(),
                ));
                valid = false;
            } else {
                password_error.set(None);
            }
            valid
        }
    };

    let on_submit = {
        let email = email.clone();
        let password = password.clone();
        let server_error = server_error.clone();
        let loading = loading.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if !validate() {
                return;
            }
            let request = LoginRequest {
                email: (*email).clone(),
                password: (*password).clone(),
            };
            let server_error = server_error.clone();
            let loading = loading.clone();
            let on_success = on_success.clone();
            loading.set(true);
            spawn_local(async move {
                match login(request).await {
                    Ok(response) => {
                        on_success.emit(response);
                    }
                    Err(err) => {
                        server_error.set(Some(err));
                    }
                }
                loading.set(false);
            });
        })
    };

    let on_email_change = {
        let email = email.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            email.set(input.value());
        })
    };

    let on_password_change = {
        let password = password.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            password.set(input.value());
        })
    };

    let close_error = {
        let server_error = server_error.clone();
        Callback::from(move |_: MouseEvent| {
            server_error.set(None);
        })
    };

    let email_cls = if email_error.is_some() {
        "gi-input gi-input-error"
    } else {
        "gi-input"
    };
    let pass_cls = if password_error.is_some() {
        "gi-input gi-input-error"
    } else {
        "gi-input"
    };

    html! {
        <form onsubmit={on_submit} style="display: flex; flex-direction: column; gap: var(--space-4);">
            if let Some(err) = (*server_error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={close_error} />
            }
            <div>
                <label class="gi-label">{"Correo electrónico"}</label>
                <input type="email" value={(*email).clone()} oninput={on_email_change}
                    class={email_cls} placeholder="correo@ejemplo.com" />
                if let Some(err) = (*email_error).as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
            <div>
                <label class="gi-label">{"Contraseña"}</label>
                <input type="password" value={(*password).clone()} oninput={on_password_change}
                    class={pass_cls} />
                if let Some(err) = (*password_error).as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
            <button type="submit" disabled={*loading} class="gi-btn gi-btn-primary"
                style="width: 100%; padding: var(--space-3) var(--space-4);">
                if *loading { {"Ingresando..."} } else { {"Iniciar Sesión"} }
            </button>
        </form>
    }
}
