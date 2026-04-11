use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthAction, AuthContext, Route};
use crate::components::auth::login_form::LoginForm;
use crate::services::auth::is_authenticated;
use crate::types::usuario::LoginResponse;

#[function_component]
pub fn Login() -> Html {
    let navigator = use_navigator().unwrap();
    let auth = use_context::<AuthContext>().unwrap();

    {
        let navigator = navigator.clone();
        use_effect_with((), move |_| {
            if is_authenticated() {
                navigator.push(&Route::Dashboard);
            }
        });
    }

    let on_success = {
        let auth = auth.clone();
        let navigator = navigator.clone();
        Callback::from(move |response: LoginResponse| {
            auth.dispatch(AuthAction::Login {
                token: response.token,
                user: response.user,
            });
            navigator.push(&Route::Dashboard);
        })
    };

    html! {
        <div style="min-height: 100vh; display: flex; align-items: center; justify-content: center; background-color: var(--surface-base);">
            <div style="width: 100%; max-width: 420px; padding: var(--space-4);">
                <div class="gi-card" style="padding: var(--space-7) var(--space-6);">
                    <div style="text-align: center; margin-bottom: var(--space-6);">
                        <div class="gi-logo-mark" style="margin: 0 auto var(--space-3);">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M3 21V9l9-7 9 7v12"/>
                                <path d="M9 21V12h6v9"/>
                            </svg>
                        </div>
                        <h1 class="text-display" style="font-size: var(--text-xl); font-weight: 700; color: var(--color-primary-500);">
                            {"Gestión Inmobiliaria"}
                        </h1>
                        <p style="font-size: var(--text-sm); color: var(--text-tertiary); margin-top: var(--space-1);">
                            {"Inicie sesión para continuar"}
                        </p>
                    </div>
                    <LoginForm on_success={on_success} />
                    <p style="text-align: center; font-size: var(--text-sm); color: var(--text-secondary); margin-top: var(--space-4);">
                        <Link<Route> to={Route::Registro} classes="gi-btn-text">
                            {"¿No tiene cuenta? Regístrese"}
                        </Link<Route>>
                    </p>
                </div>
            </div>
        </div>
    }
}
