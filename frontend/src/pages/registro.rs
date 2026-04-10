use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthAction, AuthContext, Route};
use crate::components::auth::register_form::RegisterForm;
use crate::services::auth::is_authenticated;
use crate::types::usuario::LoginResponse;

#[function_component]
pub fn Registro() -> Html {
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
                        <div style="font-size: 2.5rem; margin-bottom: var(--space-3);">{"🏠"}</div>
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
