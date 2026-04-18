use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ErrorBannerProps {
    pub message: String,
    #[prop_or_default]
    pub onclose: Option<Callback<MouseEvent>>,
    #[prop_or_default]
    pub on_retry: Option<Callback<MouseEvent>>,
}

#[function_component]
pub fn ErrorBanner(props: &ErrorBannerProps) -> Html {
    html! {
        <div class="gi-error-banner" role="alert">
            <span style="font-size: var(--text-sm);">{&props.message}</span>
            <div style="display: flex; align-items: center; gap: var(--space-2);">
                if let Some(on_retry) = &props.on_retry {
                    <button
                        onclick={on_retry.clone()}
                        class="gi-btn gi-btn-ghost gi-btn-sm"
                    >
                        {"Reintentar"}
                    </button>
                }
                if let Some(onclose) = &props.onclose {
                    <button
                        onclick={onclose.clone()}
                        style="background: none; border: none; color: var(--color-error); cursor: pointer; font-size: var(--text-lg); font-weight: 700; line-height: 1; padding: var(--space-1);"
                        aria-label="Cerrar"
                    >
                        {"×"}
                    </button>
                }
            </div>
        </div>
    }
}
