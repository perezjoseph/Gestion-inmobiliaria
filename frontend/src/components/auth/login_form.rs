use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::form_error_summary::FormErrorSummary;
use crate::services::auth::login;
use crate::types::usuario::{LoginRequest, LoginResponse};

#[derive(Properties, PartialEq)]
pub struct LoginFormProps {
    pub on_success: Callback<LoginResponse>,
}

fn validate_email(value: &str) -> Option<String> {
    if value.trim().is_empty() || !value.contains('@') {
        Some("Correo electrónico inválido".into())
    } else {
        None
    }
}

fn validate_password(value: &str) -> Option<String> {
    if value.len() < 6 {
        Some("La contraseña debe tener al menos 6 caracteres".into())
    } else {
        None
    }
}

fn friendly_server_error(err: &str) -> String {
    if err.contains("401") || err.to_lowercase().contains("unauthorized") {
        "Correo o contraseña incorrectos. Verifique sus datos e intente de nuevo.".to_string()
    } else if err.to_lowercase().contains("network") || err.to_lowercase().contains("fetch") {
        "No se pudo conectar al servidor. Verifique su conexión a internet.".to_string()
    } else {
        format!("Error al iniciar sesión: {err}")
    }
}

#[component]
pub fn LoginForm(props: &LoginFormProps) -> Html {
    let email = use_state(String::new);
    let password = use_state(String::new);
    let email_error = use_state(|| Option::<String>::None);
    let password_error = use_state(|| Option::<String>::None);
    let server_error = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);
    let on_success = props.on_success.clone();

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

    let on_email_blur = {
        let email = email.clone();
        let email_error = email_error.clone();
        Callback::from(move |_: FocusEvent| {
            email_error.set(validate_email(&email));
        })
    };

    let on_password_blur = {
        let password = password.clone();
        let password_error = password_error.clone();
        Callback::from(move |_: FocusEvent| {
            password_error.set(validate_password(&password));
        })
    };

    let on_submit = {
        let email = email.clone();
        let password = password.clone();
        let email_error = email_error.clone();
        let password_error = password_error.clone();
        let server_error = server_error.clone();
        let loading = loading.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let e_err = validate_email(&email);
            let p_err = validate_password(&password);
            email_error.set(e_err.clone());
            password_error.set(p_err.clone());
            if e_err.is_some() || p_err.is_some() {
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
                        server_error.set(Some(friendly_server_error(&err)));
                    }
                }
                loading.set(false);
            });
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

    let form_errors: Vec<String> = [(*email_error).clone(), (*password_error).clone()]
        .into_iter()
        .flatten()
        .collect();

    html! {
        <form onsubmit={on_submit} style="display: flex; flex-direction: column; gap: var(--space-4);">
            if let Some(err) = (*server_error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={close_error} />
            }
            <FormErrorSummary errors={form_errors} />
            <div>
                <label class="gi-label" for="login-email">{"Correo electrónico"}</label>
                <input
                    id="login-email"
                    type="email"
                    value={(*email).clone()}
                    oninput={on_email_change}
                    onblur={on_email_blur}
                    class={email_cls}
                    placeholder="correo@ejemplo.com"
                    autocomplete="email"
                    aria-describedby={email_error.is_some().then_some("email-error")}
                />
                if let Some(err) = (*email_error).as_ref() {
                    <p id="email-error" class="gi-field-error">{err}</p>
                }
            </div>
            <div>
                <label class="gi-label" for="login-password">{"Contraseña"}</label>
                <input
                    id="login-password"
                    type="password"
                    value={(*password).clone()}
                    oninput={on_password_change}
                    onblur={on_password_blur}
                    class={pass_cls}
                    autocomplete="current-password"
                    aria-describedby={password_error.is_some().then_some("password-error")}
                />
                if let Some(err) = (*password_error).as_ref() {
                    <p id="password-error" class="gi-field-error">{err}</p>
                }
            </div>
            <button type="submit" disabled={*loading} class="gi-btn gi-btn-primary"
                style="width: 100%; padding: var(--space-3) var(--space-4);">
                if *loading { {"Ingresando..."} } else { {"Iniciar Sesión"} }
            </button>
        </form>
    }
}
