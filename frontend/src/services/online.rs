use gloo_events::EventListener;
use web_sys::window;
use yew::prelude::*;

/// Returns the current browser online status via `navigator.onLine`.
pub fn is_online() -> bool {
    window()
        .and_then(|w| w.navigator().on_line().then_some(true))
        .unwrap_or(false)
}

/// Subscribes to browser `online`/`offline` events and invokes the callback
/// with the new status. Returns the two `EventListener` guards — drop them
/// to unsubscribe.
pub fn subscribe_online_status(on_change: Callback<bool>) -> (EventListener, EventListener) {
    let win = window().expect("window must exist");

    let cb_online = on_change.clone();
    let online_listener = EventListener::new(&win, "online", move |_| {
        cb_online.emit(true);
    });

    let offline_listener = EventListener::new(&win, "offline", move |_| {
        on_change.emit(false);
    });

    (online_listener, offline_listener)
}
