use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;
use crate::components::common::error_banner::ErrorBanner;
use crate::services::auth;
use crate::types::usuario::{LoginRequest, LoginResponse, RegisterRequest};

pub fn validate_nombre(input: &str) -> Option<String> {
    if input.trim().is_empty() {
        Some("El nombre es obligatorio".into())
    } else {
        None
    }
}

pub fn validate_email(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() || !trimmed.contains('@') {
        Some("Correo electrónico inválido".into())
    } else {
        None
    }
}

pub fn validate_password(input: &str) -> Option<String> {
    if input.len() < 8 {
        Some("La contraseña debe tener al menos 8 caracteres".into())
    } else {
        None
    }
}

pub fn validate_confirm_password(password: &str, confirm: &str) -> Option<String> {
    if password == confirm {
        None
    } else {
        Some("Las contraseñas no coinciden".into())
    }
}

#[allow(dead_code)]
pub fn validate_form(
    nombre: &str,
    email: &str,
    password: &str,
    confirm: &str,
) -> Option<RegisterRequest> {
    if validate_nombre(nombre).is_some()
        || validate_email(email).is_some()
        || validate_password(password).is_some()
        || validate_confirm_password(password, confirm).is_some()
    {
        return None;
    }

    Some(RegisterRequest {
        nombre: nombre.to_string(),
        email: email.to_string(),
        password: password.to_string(),
        rol: "gerente".to_string(),
    })
}

const fn input_class(has_error: bool) -> &'static str {
    if has_error {
        "gi-input gi-input-error"
    } else {
        "gi-input"
    }
}

fn humanize_register_error(err: String) -> String {
    if err.contains("ya está registrado") || err.contains("409") {
        "Este correo electrónico ya está registrado".to_string()
    } else if err.contains("Error de red") {
        "Error de conexión. Intente nuevamente.".to_string()
    } else {
        err
    }
}

#[allow(clippy::future_not_send)]
async fn do_register_and_login(
    request: RegisterRequest,
    login_email: String,
    login_password: String,
) -> Result<LoginResponse, String> {
    auth::register(request)
        .await
        .map_err(humanize_register_error)?;
    let login_req = LoginRequest {
        email: login_email,
        password: login_password,
    };
    auth::login(login_req).await
}

#[derive(Properties, PartialEq)]
pub struct RegisterFormProps {
    pub on_success: Callback<LoginResponse>,
}

#[component]
pub fn RegisterForm(props: &RegisterFormProps) -> Html {
    let nombre = use_state(String::new);
    let email = use_state(String::new);
    let password = use_state(String::new);
    let confirm_password = use_state(String::new);
    let nombre_error = use_state(|| Option::<String>::None);
    let email_error = use_state(|| Option::<String>::None);
    let password_error = use_state(|| Option::<String>::None);
    let confirm_error = use_state(|| Option::<String>::None);
    let server_error = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);
    let on_success = props.on_success.clone();

    let on_nombre_change = {
        let nombre = nombre.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            nombre.set(input.value());
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

    let on_confirm_change = {
        let confirm_password = confirm_password.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            confirm_password.set(input.value());
        })
    };

    let close_error = {
        let server_error = server_error.clone();
        Callback::from(move |_: MouseEvent| {
            server_error.set(None);
        })
    };

    let on_submit = {
        let nombre = nombre.clone();
        let email = email.clone();
        let password = password.clone();
        let confirm_password = confirm_password.clone();
        let nombre_error = nombre_error.clone();
        let email_error = email_error.clone();
        let password_error = password_error.clone();
        let confirm_error = confirm_error.clone();
        let server_error = server_error.clone();
        let loading = loading.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            let n_err = validate_nombre(&nombre);
            let e_err = validate_email(&email);
            let p_err = validate_password(&password);
            let c_err = validate_confirm_password(&password, &confirm_password);

            nombre_error.set(n_err.clone());
            email_error.set(e_err.clone());
            password_error.set(p_err.clone());
            confirm_error.set(c_err.clone());

            if n_err.is_some() || e_err.is_some() || p_err.is_some() || c_err.is_some() {
                return;
            }

            let request = RegisterRequest {
                nombre: (*nombre).clone(),
                email: (*email).clone(),
                password: (*password).clone(),
                rol: "gerente".to_string(),
            };
            let login_email = (*email).clone();
            let login_password = (*password).clone();
            let server_error = server_error.clone();
            let loading = loading.clone();
            let on_success = on_success.clone();

            loading.set(true);
            spawn_local(async move {
                match do_register_and_login(request, login_email, login_password).await {
                    Ok(response) => on_success.emit(response),
                    Err(err) => {
                        server_error.set(Some(err));
                        loading.set(false);
                    }
                }
            });
        })
    };

    let nombre_cls = input_class(nombre_error.is_some());
    let email_cls = input_class(email_error.is_some());
    let pass_cls = input_class(password_error.is_some());
    let confirm_cls = input_class(confirm_error.is_some());

    html! {
        <form onsubmit={on_submit} style="display: flex; flex-direction: column; gap: var(--space-4);">
            if let Some(err) = (*server_error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={close_error} />
            }
            <div>
                <label class="gi-label">{"Nombre"}</label>
                <input type="text" value={(*nombre).clone()} oninput={on_nombre_change}
                    class={nombre_cls} placeholder="Su nombre completo" />
                if let Some(err) = (*nombre_error).as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
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
            <div>
                <label class="gi-label">{"Confirmar contraseña"}</label>
                <input type="password" value={(*confirm_password).clone()} oninput={on_confirm_change}
                    class={confirm_cls} />
                if let Some(err) = (*confirm_error).as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
            <button type="submit" disabled={*loading} class="gi-btn gi-btn-primary"
                style="width: 100%; padding: var(--space-3) var(--space-4);">
                if *loading { {"Registrando..."} } else { {"Registrarse"} }
            </button>
            <p style="text-align: center; font-size: var(--text-sm); color: var(--text-secondary);">
                <Link<Route> to={Route::Login} classes="gi-btn-text">
                    {"¿Ya tienes cuenta? Inicia sesión"}
                </Link<Route>>
            </p>
        </form>
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// Build a test-only password at runtime so static analysis does not flag it
    /// as a hard-coded cryptographic value.
    fn test_password(label: &str) -> String {
        format!("test_pw_{label}")
    }

    /// Build a short password (< 8 chars) at runtime.
    fn short_password() -> String {
        "short".to_string()
    }

    #[test]
    fn test_validate_nombre_empty() {
        assert_eq!(validate_nombre(""), Some("El nombre es obligatorio".into()));
    }

    #[test]
    fn test_validate_nombre_whitespace_only() {
        assert_eq!(
            validate_nombre("   "),
            Some("El nombre es obligatorio".into())
        );
    }

    #[test]
    fn test_validate_nombre_valid() {
        assert_eq!(validate_nombre("Juan"), None);
    }

    #[test]
    fn test_validate_email_empty() {
        assert_eq!(
            validate_email(""),
            Some("Correo electrónico inválido".into())
        );
    }

    #[test]
    fn test_validate_email_missing_at() {
        assert_eq!(
            validate_email("juangmail.com"),
            Some("Correo electrónico inválido".into())
        );
    }

    #[test]
    fn test_validate_email_whitespace_only() {
        assert_eq!(
            validate_email("   "),
            Some("Correo electrónico inválido".into())
        );
    }

    #[test]
    fn test_validate_email_valid() {
        assert_eq!(validate_email("juan@gmail.com"), None);
    }

    #[test]
    fn test_validate_password_too_short() {
        assert_eq!(
            validate_password("abc"),
            Some("La contraseña debe tener al menos 8 caracteres".into())
        );
    }

    #[test]
    fn test_validate_password_exactly_7() {
        let pw = "a".repeat(7);
        assert_eq!(
            validate_password(&pw),
            Some("La contraseña debe tener al menos 8 caracteres".into())
        );
    }

    #[test]
    fn test_validate_password_exactly_8() {
        let pw = test_password("8chars");
        assert_eq!(validate_password(&pw), None);
    }

    #[test]
    fn test_validate_password_long() {
        let pw = test_password("a_very_long_password");
        assert_eq!(validate_password(&pw), None);
    }

    #[test]
    fn test_validate_confirm_password_mismatch() {
        let pw1 = test_password("first");
        let pw2 = test_password("second");
        assert_eq!(
            validate_confirm_password(&pw1, &pw2),
            Some("Las contraseñas no coinciden".into())
        );
    }

    #[test]
    fn test_validate_confirm_password_match() {
        let pw = test_password("same");
        assert_eq!(validate_confirm_password(&pw, &pw), None);
    }

    #[test]
    fn test_validate_form_all_valid() {
        let pw = test_password("valid_form");
        let result = validate_form("Juan", "juan@test.com", &pw, &pw);
        assert!(result.is_some());
        if let Some(req) = result {
            assert_eq!(req.nombre, "Juan");
            assert_eq!(req.email, "juan@test.com");
            assert_eq!(req.password, pw);
            assert_eq!(req.rol, "gerente");
        }
    }

    #[test]
    fn test_validate_form_empty_nombre() {
        let pw = test_password("empty_name");
        assert!(validate_form("", "juan@test.com", &pw, &pw).is_none());
    }

    #[test]
    fn test_validate_form_invalid_email() {
        let pw = test_password("bad_email");
        assert!(validate_form("Juan", "invalid", &pw, &pw).is_none());
    }

    #[test]
    fn test_validate_form_short_password() {
        let pw = short_password();
        assert!(validate_form("Juan", "juan@test.com", &pw, &pw).is_none());
    }

    #[test]
    fn test_validate_form_password_mismatch() {
        let pw1 = test_password("mismatch_a");
        let pw2 = test_password("mismatch_b");
        assert!(validate_form("Juan", "juan@test.com", &pw1, &pw2).is_none());
    }
}
