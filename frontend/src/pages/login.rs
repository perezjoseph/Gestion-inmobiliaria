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
        <div class="gi-login-page">
            <div class="gi-login-container">
                <div class="gi-card gi-p-6">
                    <div class="gi-login-header">
                        <div class="gi-logo-mark gi-mb-3" style="margin-inline: auto;">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M3 21V9l9-7 9 7v12"/>
                                <path d="M9 21V12h6v9"/>
                            </svg>
                        </div>
                        <h1 class="text-display gi-login-title">
                            {"Gestión Inmobiliaria"}
                        </h1>
                        <p class="gi-login-subtitle">
                            {"Inicie sesión para continuar"}
                        </p>
                    </div>
                    <LoginForm on_success={on_success} />
                    <p class="gi-login-footer">
                        <Link<Route> to={Route::Registro} classes="gi-btn-text">
                            {"¿No tiene cuenta? Regístrese"}
                        </Link<Route>>
                    </p>
                </div>
            </div>
        </div>
    }
}
