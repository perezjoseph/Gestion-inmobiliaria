use gloo_timers::callback::Interval;
use yew::prelude::*;

#[function_component]
pub fn OfflineBanner() -> Html {
    let is_offline = use_state(|| false);

    {
        let is_offline = is_offline.clone();
        use_effect_with((), move |_| {
            let check_online = move || {
                let online = web_sys::window()
                    .map(|w| w.navigator().on_line())
                    .unwrap_or(true);
                is_offline.set(!online);
            };
            check_online();
            let interval = Interval::new(3000, check_online);
            move || drop(interval)
        });
    }

    if !*is_offline {
        return html! {};
    }

    html! {
        <div style="position: fixed; bottom: 0; left: 0; right: 0; z-index: 9999; padding: var(--space-3) var(--space-4); background: var(--color-warning); color: var(--color-sand-900); text-align: center; font-size: var(--text-sm); font-weight: 500;">
            {"⚠ Sin conexión a internet — Se requiere conexión a internet para realizar cambios"}
        </div>
    }
}
