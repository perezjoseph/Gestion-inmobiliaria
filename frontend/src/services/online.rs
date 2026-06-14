use gloo_events::EventListener;
use web_sys::window;
use yew::prelude::*;

pub fn is_online() -> bool {
    window()
        .and_then(|w| w.navigator().on_line().then_some(true))
        .unwrap_or(false)
}

pub fn subscribe_online_status(
    on_change: Callback<bool>,
) -> Option<(EventListener, EventListener)> {
    let win = window()?;

    let cb_online = on_change.clone();
    let online_listener = EventListener::new(&win, "online", move |_| {
        cb_online.emit(true);
    });

    let offline_listener = EventListener::new(&win, "offline", move |_| {
        on_change.emit(false);
    });

    Some((online_listener, offline_listener))
}
