use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthAction, AuthContext, Route};
use crate::components::auth::register_form::RegisterForm;
use crate::services::auth::is_authenticated;
use crate::types::usuario::LoginResponse;

#[component]
pub fn Registro() -> Html {
    let Some(navigator) = use_navigator() else {
        return html! {};
    };
    let Some(auth) = use_context::<AuthContext>() else {
        return html! {};
    };

    {
        let navigator = navigator.clone();
        use_effect_with((), move |()| {
            if is_authenticated() {
                navigator.push(&Route::Dashboard);
            }
        });
    }

    let on_success = {
        let auth = auth;
        let navigator = navigator;
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
            <div style="width: 100%; max-width: 480px; padding: var(--space-4);">
                <div class="gi-card" style="padding: var(--space-7) var(--space-6); max-height: 90vh; overflow-y: auto;">
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
                            {"Cree su cuenta para comenzar"}
                        </p>
                    </div>
                    <RegisterForm on_success={on_success} />
                </div>
            </div>
        </div>
    }
}
