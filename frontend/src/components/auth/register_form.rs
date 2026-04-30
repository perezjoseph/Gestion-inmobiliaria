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

fn validate_required(value: &str, field_name: &str) -> Option<String> {
    if value.trim().is_empty() {
        Some(format!("{field_name} es obligatorio"))
    } else {
        None
    }
}

#[allow(dead_code)]
pub fn validate_form(
    nombre: &str,
    email: &str,
    password: &str,
    confirm: &str,
    tipo: &str,
) -> Option<RegisterRequest> {
    if validate_nombre(nombre).is_some()
        || validate_email(email).is_some()
        || validate_password(password).is_some()
        || validate_confirm_password(password, confirm).is_some()
        || tipo.is_empty()
    {
        return None;
    }

    Some(RegisterRequest {
        nombre: nombre.to_string(),
        email: email.to_string(),
        password: password.to_string(),
        tipo: tipo.to_string(),
        cedula: None,
        telefono: None,
        nombre_organizacion: None,
        rnc: None,
        razon_social: None,
        nombre_comercial: None,
        direccion_fiscal: None,
        representante_legal: None,
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
    if err.contains("ya está registrad") || err.contains("409") {
        "Este correo electrónico o documento fiscal ya está registrado".to_string()
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
    // Common fields
    let nombre = use_state(String::new);
    let email = use_state(String::new);
    let password = use_state(String::new);
    let confirm_password = use_state(String::new);
    let tipo = use_state(|| "persona_fisica".to_string());

    // persona_fisica fields
    let cedula = use_state(String::new);
    let telefono = use_state(String::new);
    let nombre_org = use_state(String::new);

    // persona_juridica fields
    let rnc = use_state(String::new);
    let razon_social = use_state(String::new);
    let nombre_comercial = use_state(String::new);
    let direccion_fiscal = use_state(String::new);
    let representante_legal = use_state(String::new);

    // Validation errors
    let nombre_error = use_state(|| Option::<String>::None);
    let email_error = use_state(|| Option::<String>::None);
    let password_error = use_state(|| Option::<String>::None);
    let confirm_error = use_state(|| Option::<String>::None);
    let cedula_error = use_state(|| Option::<String>::None);
    let telefono_error = use_state(|| Option::<String>::None);
    let nombre_org_error = use_state(|| Option::<String>::None);
    let rnc_error = use_state(|| Option::<String>::None);
    let razon_social_error = use_state(|| Option::<String>::None);
    let nombre_comercial_error = use_state(|| Option::<String>::None);
    let direccion_fiscal_error = use_state(|| Option::<String>::None);
    let representante_legal_error = use_state(|| Option::<String>::None);
    let server_error = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);
    let on_success = props.on_success.clone();

    // Input change handlers
    let make_handler = |state: UseStateHandle<String>| {
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            state.set(input.value());
        })
    };

    let on_nombre_change = make_handler(nombre.clone());
    let on_email_change = make_handler(email.clone());
    let on_password_change = make_handler(password.clone());
    let on_confirm_change = make_handler(confirm_password.clone());
    let on_cedula_change = make_handler(cedula.clone());
    let on_telefono_change = make_handler(telefono.clone());
    let on_nombre_org_change = make_handler(nombre_org.clone());
    let on_rnc_change = make_handler(rnc.clone());
    let on_razon_social_change = make_handler(razon_social.clone());
    let on_nombre_comercial_change = make_handler(nombre_comercial.clone());
    let on_direccion_fiscal_change = make_handler(direccion_fiscal.clone());
    let on_representante_legal_change = make_handler(representante_legal.clone());

    let on_tipo_change = {
        let tipo = tipo.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            tipo.set(select.value());
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
        let tipo = tipo.clone();
        let cedula = cedula.clone();
        let telefono = telefono.clone();
        let nombre_org = nombre_org.clone();
        let rnc = rnc.clone();
        let razon_social = razon_social.clone();
        let nombre_comercial = nombre_comercial.clone();
        let direccion_fiscal = direccion_fiscal.clone();
        let representante_legal = representante_legal.clone();
        let nombre_error = nombre_error.clone();
        let email_error = email_error.clone();
        let password_error = password_error.clone();
        let confirm_error = confirm_error.clone();
        let cedula_error = cedula_error.clone();
        let telefono_error = telefono_error.clone();
        let nombre_org_error = nombre_org_error.clone();
        let rnc_error = rnc_error.clone();
        let razon_social_error = razon_social_error.clone();
        let nombre_comercial_error = nombre_comercial_error.clone();
        let direccion_fiscal_error = direccion_fiscal_error.clone();
        let representante_legal_error = representante_legal_error.clone();
        let server_error = server_error.clone();
        let loading = loading.clone();
        let on_success = on_success.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            // Validate common fields
            let n_err = validate_nombre(&nombre);
            let e_err = validate_email(&email);
            let p_err = validate_password(&password);
            let c_err = validate_confirm_password(&password, &confirm_password);

            nombre_error.set(n_err.clone());
            email_error.set(e_err.clone());
            password_error.set(p_err.clone());
            confirm_error.set(c_err.clone());

            let mut has_error =
                n_err.is_some() || e_err.is_some() || p_err.is_some() || c_err.is_some();

            // Validate tipo-specific fields
            let is_fisica = *tipo == "persona_fisica";

            if is_fisica {
                let ced_err = validate_required(&cedula, "La cédula");
                let tel_err = validate_required(&telefono, "El teléfono");
                let org_err = validate_required(&nombre_org, "El nombre de la organización");
                cedula_error.set(ced_err.clone());
                telefono_error.set(tel_err.clone());
                nombre_org_error.set(org_err.clone());
                // Clear juridica errors
                rnc_error.set(None);
                razon_social_error.set(None);
                nombre_comercial_error.set(None);
                direccion_fiscal_error.set(None);
                representante_legal_error.set(None);
                if ced_err.is_some() || tel_err.is_some() || org_err.is_some() {
                    has_error = true;
                }
            } else {
                let rnc_e = validate_required(&rnc, "El RNC");
                let rs_e = validate_required(&razon_social, "La razón social");
                let nc_e = validate_required(&nombre_comercial, "El nombre comercial");
                let df_e = validate_required(&direccion_fiscal, "La dirección fiscal");
                let rl_e = validate_required(&representante_legal, "El representante legal");
                rnc_error.set(rnc_e.clone());
                razon_social_error.set(rs_e.clone());
                nombre_comercial_error.set(nc_e.clone());
                direccion_fiscal_error.set(df_e.clone());
                representante_legal_error.set(rl_e.clone());
                // Clear fisica errors
                cedula_error.set(None);
                telefono_error.set(None);
                nombre_org_error.set(None);
                if rnc_e.is_some()
                    || rs_e.is_some()
                    || nc_e.is_some()
                    || df_e.is_some()
                    || rl_e.is_some()
                {
                    has_error = true;
                }
            }

            if has_error {
                return;
            }

            let request = if is_fisica {
                RegisterRequest {
                    nombre: (*nombre).clone(),
                    email: (*email).clone(),
                    password: (*password).clone(),
                    tipo: "persona_fisica".to_string(),
                    cedula: Some((*cedula).clone()),
                    telefono: Some((*telefono).clone()),
                    nombre_organizacion: Some((*nombre_org).clone()),
                    rnc: None,
                    razon_social: None,
                    nombre_comercial: None,
                    direccion_fiscal: None,
                    representante_legal: None,
                }
            } else {
                RegisterRequest {
                    nombre: (*nombre).clone(),
                    email: (*email).clone(),
                    password: (*password).clone(),
                    tipo: "persona_juridica".to_string(),
                    cedula: None,
                    telefono: None,
                    nombre_organizacion: None,
                    rnc: Some((*rnc).clone()),
                    razon_social: Some((*razon_social).clone()),
                    nombre_comercial: Some((*nombre_comercial).clone()),
                    direccion_fiscal: Some((*direccion_fiscal).clone()),
                    representante_legal: Some((*representante_legal).clone()),
                }
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
    let is_fisica = *tipo == "persona_fisica";

    html! {
        <form onsubmit={on_submit} style="display: flex; flex-direction: column; gap: var(--space-4);">
            if let Some(err) = (*server_error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={close_error} />
            }

            // Common fields: Nombre
            <div>
                <label class="gi-label">{"Nombre"}</label>
                <input type="text" value={(*nombre).clone()} oninput={on_nombre_change}
                    class={nombre_cls} placeholder="Su nombre completo" />
                if let Some(err) = (*nombre_error).as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
            // Common fields: Email
            <div>
                <label class="gi-label">{"Correo electrónico"}</label>
                <input type="email" value={(*email).clone()} oninput={on_email_change}
                    class={email_cls} placeholder="correo@ejemplo.com" />
                if let Some(err) = (*email_error).as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
            // Common fields: Password
            <div>
                <label class="gi-label">{"Contraseña"}</label>
                <input type="password" value={(*password).clone()} oninput={on_password_change}
                    class={pass_cls} />
                if let Some(err) = (*password_error).as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
            // Common fields: Confirm password
            <div>
                <label class="gi-label">{"Confirmar contraseña"}</label>
                <input type="password" value={(*confirm_password).clone()} oninput={on_confirm_change}
                    class={confirm_cls} />
                if let Some(err) = (*confirm_error).as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
            // Organization type selector
            <div>
                <label class="gi-label">{"Tipo de persona"}</label>
                <select class="gi-input" onchange={on_tipo_change} value={(*tipo).clone()}>
                    <option value="persona_fisica" selected={is_fisica}>{"Persona Física"}</option>
                    <option value="persona_juridica" selected={!is_fisica}>{"Persona Jurídica"}</option>
                </select>
            </div>
            // Conditional fields based on tipo
            if is_fisica {
                <PersonaFisicaFields
                    cedula={(*cedula).clone()}
                    telefono={(*telefono).clone()}
                    nombre_org={(*nombre_org).clone()}
                    on_cedula_change={on_cedula_change}
                    on_telefono_change={on_telefono_change}
                    on_nombre_org_change={on_nombre_org_change}
                    cedula_error={(*cedula_error).clone()}
                    telefono_error={(*telefono_error).clone()}
                    nombre_org_error={(*nombre_org_error).clone()}
                />
            } else {
                <PersonaJuridicaFields
                    rnc={(*rnc).clone()}
                    razon_social={(*razon_social).clone()}
                    nombre_comercial={(*nombre_comercial).clone()}
                    direccion_fiscal={(*direccion_fiscal).clone()}
                    representante_legal={(*representante_legal).clone()}
                    on_rnc_change={on_rnc_change}
                    on_razon_social_change={on_razon_social_change}
                    on_nombre_comercial_change={on_nombre_comercial_change}
                    on_direccion_fiscal_change={on_direccion_fiscal_change}
                    on_representante_legal_change={on_representante_legal_change}
                    rnc_error={(*rnc_error).clone()}
                    razon_social_error={(*razon_social_error).clone()}
                    nombre_comercial_error={(*nombre_comercial_error).clone()}
                    direccion_fiscal_error={(*direccion_fiscal_error).clone()}
                    representante_legal_error={(*representante_legal_error).clone()}
                />
            }
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

// ── Sub-components to keep html! blocks small ──────────────────

#[derive(Properties, PartialEq)]
struct PersonaFisicaFieldsProps {
    cedula: String,
    telefono: String,
    nombre_org: String,
    on_cedula_change: Callback<InputEvent>,
    on_telefono_change: Callback<InputEvent>,
    on_nombre_org_change: Callback<InputEvent>,
    cedula_error: Option<String>,
    telefono_error: Option<String>,
    nombre_org_error: Option<String>,
}

#[component]
fn PersonaFisicaFields(props: &PersonaFisicaFieldsProps) -> Html {
    let ced_cls = input_class(props.cedula_error.is_some());
    let tel_cls = input_class(props.telefono_error.is_some());
    let org_cls = input_class(props.nombre_org_error.is_some());

    html! {
        <>
            <div>
                <label class="gi-label">{"Cédula *"}</label>
                <input type="text" value={props.cedula.clone()} oninput={props.on_cedula_change.clone()}
                    class={ced_cls} placeholder="000-0000000-0" />
                if let Some(err) = props.cedula_error.as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
            <div>
                <label class="gi-label">{"Teléfono *"}</label>
                <input type="tel" value={props.telefono.clone()} oninput={props.on_telefono_change.clone()}
                    class={tel_cls} placeholder="809-555-1234" />
                if let Some(err) = props.telefono_error.as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
            <div>
                <label class="gi-label">{"Nombre de la organización *"}</label>
                <input type="text" value={props.nombre_org.clone()} oninput={props.on_nombre_org_change.clone()}
                    class={org_cls} placeholder="Mi Inmobiliaria" />
                if let Some(err) = props.nombre_org_error.as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
        </>
    }
}

#[derive(Properties, PartialEq)]
struct PersonaJuridicaFieldsProps {
    rnc: String,
    razon_social: String,
    nombre_comercial: String,
    direccion_fiscal: String,
    representante_legal: String,
    on_rnc_change: Callback<InputEvent>,
    on_razon_social_change: Callback<InputEvent>,
    on_nombre_comercial_change: Callback<InputEvent>,
    on_direccion_fiscal_change: Callback<InputEvent>,
    on_representante_legal_change: Callback<InputEvent>,
    rnc_error: Option<String>,
    razon_social_error: Option<String>,
    nombre_comercial_error: Option<String>,
    direccion_fiscal_error: Option<String>,
    representante_legal_error: Option<String>,
}

#[component]
fn PersonaJuridicaFields(props: &PersonaJuridicaFieldsProps) -> Html {
    let rnc_cls = input_class(props.rnc_error.is_some());
    let rs_cls = input_class(props.razon_social_error.is_some());
    let nc_cls = input_class(props.nombre_comercial_error.is_some());
    let df_cls = input_class(props.direccion_fiscal_error.is_some());
    let rl_cls = input_class(props.representante_legal_error.is_some());

    html! {
        <>
            <div>
                <label class="gi-label">{"RNC *"}</label>
                <input type="text" value={props.rnc.clone()} oninput={props.on_rnc_change.clone()}
                    class={rnc_cls} placeholder="000000000" />
                if let Some(err) = props.rnc_error.as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
            <div>
                <label class="gi-label">{"Razón social *"}</label>
                <input type="text" value={props.razon_social.clone()} oninput={props.on_razon_social_change.clone()}
                    class={rs_cls} placeholder="Empresa S.R.L." />
                if let Some(err) = props.razon_social_error.as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
            <div>
                <label class="gi-label">{"Nombre comercial *"}</label>
                <input type="text" value={props.nombre_comercial.clone()} oninput={props.on_nombre_comercial_change.clone()}
                    class={nc_cls} placeholder="Mi Empresa" />
                if let Some(err) = props.nombre_comercial_error.as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
            <div>
                <label class="gi-label">{"Dirección fiscal *"}</label>
                <input type="text" value={props.direccion_fiscal.clone()} oninput={props.on_direccion_fiscal_change.clone()}
                    class={df_cls} placeholder="Calle Principal #1, Santo Domingo" />
                if let Some(err) = props.direccion_fiscal_error.as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
            <div>
                <label class="gi-label">{"Representante legal *"}</label>
                <input type="text" value={props.representante_legal.clone()} oninput={props.on_representante_legal_change.clone()}
                    class={rl_cls} placeholder="Nombre del representante" />
                if let Some(err) = props.representante_legal_error.as_ref() {
                    <p class="gi-field-error">{err}</p>
                }
            </div>
        </>
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn test_password(label: &str) -> String {
        format!("test_pw_{label}")
    }

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
        let result = validate_form("Juan", "juan@test.com", &pw, &pw, "persona_fisica");
        assert!(result.is_some());
        if let Some(req) = result {
            assert_eq!(req.nombre, "Juan");
            assert_eq!(req.email, "juan@test.com");
            assert_eq!(req.password, pw);
            assert_eq!(req.tipo, "persona_fisica");
        }
    }

    #[test]
    fn test_validate_form_empty_nombre() {
        let pw = test_password("empty_name");
        assert!(validate_form("", "juan@test.com", &pw, &pw, "persona_fisica").is_none());
    }

    #[test]
    fn test_validate_form_invalid_email() {
        let pw = test_password("bad_email");
        assert!(validate_form("Juan", "invalid", &pw, &pw, "persona_fisica").is_none());
    }

    #[test]
    fn test_validate_form_short_password() {
        let pw = short_password();
        assert!(validate_form("Juan", "juan@test.com", &pw, &pw, "persona_fisica").is_none());
    }

    #[test]
    fn test_validate_form_password_mismatch() {
        let pw1 = test_password("mismatch_a");
        let pw2 = test_password("mismatch_b");
        assert!(validate_form("Juan", "juan@test.com", &pw1, &pw2, "persona_fisica").is_none());
    }

    #[test]
    fn test_validate_form_empty_tipo() {
        let pw = test_password("no_tipo");
        assert!(validate_form("Juan", "juan@test.com", &pw, &pw, "").is_none());
    }

    #[test]
    fn test_validate_required_empty() {
        assert_eq!(
            validate_required("", "La cédula"),
            Some("La cédula es obligatorio".into())
        );
    }

    #[test]
    fn test_validate_required_whitespace() {
        assert_eq!(
            validate_required("   ", "El RNC"),
            Some("El RNC es obligatorio".into())
        );
    }

    #[test]
    fn test_validate_required_valid() {
        assert_eq!(validate_required("12345", "La cédula"), None);
    }
}
