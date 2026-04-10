use yew::prelude::*;
use yew_router::prelude::*;

use crate::components::common::toast::{ToastContainer, ToastContext, ToastState};
use crate::components::layout::footer::Footer;
use crate::components::layout::navbar::Navbar;
use crate::components::layout::sidebar::Sidebar;
use crate::pages::contratos::Contratos;
use crate::pages::dashboard::Dashboard;
use crate::pages::inquilinos::Inquilinos;
use crate::pages::login::Login;
use crate::pages::pagos::Pagos;
use crate::pages::propiedades::Propiedades;
use crate::pages::registro::Registro;
use crate::services::auth::{clear_token, get_token, set_token};
use crate::types::usuario::User;

#[derive(Clone, Debug, PartialEq)]
pub struct AuthState {
    pub token: Option<String>,
    pub user: Option<User>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            token: get_token(),
            user: None,
        }
    }
}

impl Reducible for AuthState {
    type Action = AuthAction;

    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        match action {
            AuthAction::Login { token, user } => {
                set_token(&token);
                Self {
                    token: Some(token),
                    user: Some(user),
                }
                .into()
            }
            AuthAction::Logout => {
                clear_token();
                Self {
                    token: None,
                    user: None,
                }
                .into()
            }
            AuthAction::SetUser(user) => Self {
                token: self.token.clone(),
                user: Some(user),
            }
            .into(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum AuthAction {
    Login {
        token: String,
        user: User,
    },
    Logout,
    #[allow(dead_code)]
    SetUser(User),
}

pub type AuthContext = UseReducerHandle<AuthState>;

pub type ThemeContext = UseStateHandle<bool>;

#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Login,
    #[at("/dashboard")]
    Dashboard,
    #[at("/propiedades")]
    Propiedades,
    #[at("/inquilinos")]
    Inquilinos,
    #[at("/contratos")]
    Contratos,
    #[at("/pagos")]
    Pagos,
    #[at("/registro")]
    Registro,
    #[not_found]
    #[at("/404")]
    NotFound,
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Login => html! { <Login /> },
        Route::Registro => html! { <Registro /> },
        Route::Dashboard => html! { <ProtectedRoute><Dashboard /></ProtectedRoute> },
        Route::Propiedades => html! { <ProtectedRoute><Propiedades /></ProtectedRoute> },
        Route::Inquilinos => html! { <ProtectedRoute><Inquilinos /></ProtectedRoute> },
        Route::Contratos => html! { <ProtectedRoute><Contratos /></ProtectedRoute> },
        Route::Pagos => html! { <ProtectedRoute><Pagos /></ProtectedRoute> },
        Route::NotFound => {
            html! {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-icon">{"🏝️"}</div>
                    <div class="gi-empty-state-title">{"404 — Página no encontrada"}</div>
                    <p class="gi-empty-state-text">{"La página que busca no existe o fue movida."}</p>
                    <div style="margin-top: var(--space-5);">
                        <Link<Route> to={Route::Login} classes="gi-btn gi-btn-primary">
                            {"Volver al inicio"}
                        </Link<Route>>
                    </div>
                </div>
            }
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct ProtectedRouteProps {
    pub children: Html,
}

#[function_component]
pub fn ProtectedRoute(props: &ProtectedRouteProps) -> Html {
    let navigator = use_navigator().unwrap();
    let auth = use_context::<AuthContext>();
    let sidebar_open = use_state(|| false);

    let is_authed = auth.as_ref().map(|a| a.token.is_some()).unwrap_or(false);

    {
        let navigator = navigator.clone();
        use_effect_with(is_authed, move |authed| {
            if !*authed {
                navigator.push(&Route::Login);
            }
        });
    }

    if !is_authed {
        return html! {};
    }

    let user_name = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.nombre.clone())
        .unwrap_or_default();

    let user_role = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();

    let on_toggle_sidebar = {
        let sidebar_open = sidebar_open.clone();
        Callback::from(move |_: MouseEvent| {
            sidebar_open.set(!*sidebar_open);
        })
    };

    let on_close_sidebar = {
        let sidebar_open = sidebar_open.clone();
        Callback::from(move |_: MouseEvent| {
            sidebar_open.set(false);
        })
    };

    let on_nav_click = {
        let sidebar_open = sidebar_open.clone();
        Callback::from(move |_: ()| {
            sidebar_open.set(false);
        })
    };

    html! {
        <div class="flex min-h-screen" style="background-color: var(--surface-base);">
            // Mobile overlay
            if *sidebar_open {
                <div class="gi-sidebar-overlay open" onclick={on_close_sidebar}></div>
            }
            <Sidebar is_open={*sidebar_open} on_nav_click={on_nav_click} />
            <div class="flex-1 flex flex-col min-w-0">
                <Navbar
                    user_name={user_name}
                    user_role={user_role}
                    on_toggle_sidebar={on_toggle_sidebar}
                />
                <main class="flex-1" style="padding: var(--space-5) var(--space-6);">
                    {props.children.clone()}
                </main>
                <Footer />
            </div>
        </div>
    }
}

#[function_component]
pub fn App() -> Html {
    let auth = use_reducer(AuthState::default);
    let toasts = use_reducer(ToastState::default);

    let is_dark = use_state(|| {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
            .map(|el| {
                el.get_attribute("data-theme")
                    .map(|t| t == "dark")
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    });

    html! {
        <ContextProvider<AuthContext> context={auth}>
            <ContextProvider<ThemeContext> context={is_dark}>
                <ContextProvider<ToastContext> context={toasts}>
                    <BrowserRouter>
                        <Switch<Route> render={switch} />
                    </BrowserRouter>
                    <ToastContainer />
                </ContextProvider<ToastContext>>
            </ContextProvider<ThemeContext>>
        </ContextProvider<AuthContext>>
    }
}
