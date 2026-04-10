use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::ProfileSkeleton;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_get, api_put};
use crate::types::usuario::User;

#[component]
fn PasswordChangeForm() -> Html {
    let toasts = use_context::<ToastContext>();
    let password_actual = use_state(String::new);
    let password_nuevo = use_state(String::new);
    let password_confirmar = use_state(String::new);
    let pwd_error = use_state(|| Option::<String>::None);

    let on_change_password = {
        let password_actual = password_actual.clone();
        let password_nuevo = password_nuevo.clone();
        let password_confirmar = password_confirmar.clone();
        let pwd_error = pwd_error.clone();
        let toasts = toasts;
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if *password_nuevo != *password_confirmar {
                pwd_error.set(Some("Las contraseñas no coinciden".into()));
                return;
            }
            if password_nuevo.len() < 6 {
                pwd_error.set(Some(
                    "La contraseña debe tener al menos 6 caracteres".into(),
                ));
                return;
            }
            let body = serde_json::json!({
                "passwordActual": *password_actual,
                "passwordNuevo": *password_nuevo,
            });
            let pwd_error = pwd_error.clone();
            let password_actual = password_actual.clone();
            let password_nuevo = password_nuevo.clone();
            let password_confirmar = password_confirmar.clone();
            let toasts = toasts.clone();
            spawn_local(async move {
                match api_put::<serde_json::Value, _>("/perfil/password", &body).await {
                    Ok(_) => {
                        password_actual.set(String::new());
                        password_nuevo.set(String::new());
                        password_confirmar.set(String::new());
                        pwd_error.set(None);
                        if let Some(t) = &toasts {
                            t.dispatch(ToastAction::Push(
                                "Contraseña actualizada".into(),
                                ToastKind::Success,
                            ));
                        }
                    }
                    Err(err) => pwd_error.set(Some(err)),
                }
            });
        })
    };

    macro_rules! input_cb {
        ($state:expr) => {{
            let s = $state.clone();
            Callback::from(move |e: InputEvent| {
                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                s.set(input.value());
            })
        }};
    }

    html! {
        <div class="gi-card" style="padding: var(--space-5);">
            <h2 style="font-size: var(--text-base); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-4);">
                {"Cambiar Contraseña"}
            </h2>
            if let Some(err) = (*pwd_error).as_ref() {
                <div class="gi-error-banner" role="alert" style="margin-bottom: var(--space-3);">
                    <span style="font-size: var(--text-sm);">{err}</span>
                </div>
            }
            <form onsubmit={on_change_password} style="display: flex; flex-direction: column; gap: var(--space-3);">
                <div>
                    <label class="gi-label">{"Contraseña Actual"}</label>
                    <input type="password" value={(*password_actual).clone()} oninput={input_cb!(password_actual)} class="gi-input" />
                </div>
                <div>
                    <label class="gi-label">{"Nueva Contraseña"}</label>
                    <input type="password" value={(*password_nuevo).clone()} oninput={input_cb!(password_nuevo)} class="gi-input" />
                </div>
                <div>
                    <label class="gi-label">{"Confirmar Nueva Contraseña"}</label>
                    <input type="password" value={(*password_confirmar).clone()} oninput={input_cb!(password_confirmar)} class="gi-input" />
                </div>
                <div style="display: flex; justify-content: flex-end;">
                    <button type="submit" class="gi-btn gi-btn-primary">{"Cambiar Contraseña"}</button>
                </div>
            </form>
        </div>
    }
}

#[component]
pub fn Perfil() -> Html {
    let auth = use_context::<AuthContext>();
    let toasts = use_context::<ToastContext>();
    let user = use_state(|| Option::<User>::None);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);

    let nombre = use_state(String::new);
    let email = use_state(String::new);

    {
        let user = user.clone();
        let nombre = nombre.clone();
        let email = email.clone();
        let error = error.clone();
        let loading = loading.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                match api_get::<User>("/perfil").await {
                    Ok(u) => {
                        nombre.set(u.nombre.clone());
                        email.set(u.email.clone());
                        user.set(Some(u));
                    }
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        });
    }

    let on_save_profile = {
        let nombre = nombre.clone();
        let email = email.clone();
        let error = error.clone();
        let user = user;
        let toasts = toasts;
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let body = serde_json::json!({
                "nombre": *nombre,
                "email": *email,
            });
            let error = error.clone();
            let user = user.clone();
            let toasts = toasts.clone();
            spawn_local(async move {
                match api_put::<User, _>("/perfil", &body).await {
                    Ok(u) => {
                        user.set(Some(u));
                        if let Some(t) = &toasts {
                            t.dispatch(ToastAction::Push(
                                "Perfil actualizado".into(),
                                ToastKind::Success,
                            ));
                        }
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        })
    };

    macro_rules! input_cb {
        ($state:expr) => {{
            let s = $state.clone();
            Callback::from(move |e: InputEvent| {
                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                s.set(input.value());
            })
        }};
    }

    if *loading {
        return html! { <ProfileSkeleton /> };
    }

    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Mi Perfil"}</h1>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr)); gap: var(--space-5);">
                <div class="gi-card" style="padding: var(--space-5);">
                    <h2 style="font-size: var(--text-base); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-4);">
                        {"Datos Personales"}
                    </h2>
                    <form onsubmit={on_save_profile} style="display: flex; flex-direction: column; gap: var(--space-3);">
                        <div>
                            <label class="gi-label">{"Nombre"}</label>
                            <input type="text" value={(*nombre).clone()} oninput={input_cb!(nombre)} class="gi-input" />
                        </div>
                        <div>
                            <label class="gi-label">{"Email"}</label>
                            <input type="email" value={(*email).clone()} oninput={input_cb!(email)} class="gi-input" />
                        </div>
                        <div>
                            <label class="gi-label">{"Rol"}</label>
                            <input type="text" value={user_rol} class="gi-input" disabled=true style="opacity: 0.6;" />
                        </div>
                        <div style="display: flex; justify-content: flex-end;">
                            <button type="submit" class="gi-btn gi-btn-primary">{"Guardar Cambios"}</button>
                        </div>
                    </form>
                </div>

                <PasswordChangeForm />
            </div>
        </div>
    }
}
