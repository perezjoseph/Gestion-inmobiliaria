use gloo_timers::callback::Interval;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;
use crate::services::api::api_get;
use crate::types::notificacion::ConteoNoLeidas;

const POLL_INTERVAL_MS: u32 = 60_000;

fn fetch_count(count: UseStateHandle<u64>) {
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(resp) = api_get::<ConteoNoLeidas>("/notificaciones/no-leidas/conteo").await {
            count.set(resp.count);
        }
    });
}

#[component]
pub fn NotificationBell() -> Html {
    let count = use_state(|| 0u64);
    let navigator = use_navigator();

    {
        let count = count.clone();
        use_effect_with((), move |()| {
            // Initial fetch on mount
            fetch_count(count.clone());

            // Poll every 60 seconds
            let interval = Interval::new(POLL_INTERVAL_MS, move || {
                fetch_count(count.clone());
            });

            // Cleanup: drop the interval when the component unmounts
            move || drop(interval)
        });
    }

    let on_click = {
        Callback::from(move |_: MouseEvent| {
            if let Some(nav) = &navigator {
                nav.push(&Route::Notificaciones);
            }
        })
    };

    html! {
        <button
            class="gi-btn gi-btn-ghost"
            onclick={on_click}
            aria-label="Notificaciones"
            title="Notificaciones"
            style="position: relative; padding: var(--space-2); border: none; display: flex; align-items: center;"
        >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/>
                <path d="M13.73 21a2 2 0 0 1-3.46 0"/>
            </svg>
            if *count > 0 {
                <span style="
                    position: absolute;
                    top: 0;
                    right: 0;
                    background: var(--color-danger, #ef4444);
                    color: white;
                    font-size: 0.65rem;
                    font-weight: 700;
                    min-width: 16px;
                    height: 16px;
                    border-radius: 9999px;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    padding: 0 4px;
                    line-height: 1;
                ">
                    {*count}
                </span>
            }
        </button>
    }
}
