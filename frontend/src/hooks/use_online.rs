use yew::prelude::*;

use crate::services::online::{is_online, subscribe_online_status};

/// Yew hook that tracks browser online/offline status.
///
/// Returns `true` when the browser reports connectivity, `false` otherwise.
/// Automatically subscribes to `online`/`offline` window events and cleans up
/// on unmount.
#[hook]
pub fn use_online() -> bool {
    let state = use_state(is_online);

    {
        #[allow(clippy::redundant_clone)]
        let state = state.clone();
        use_effect_with((), move |()| {
            let cb = {
                let state = state;
                Callback::from(move |online: bool| {
                    state.set(online);
                })
            };

            // Bind both listener guards by name so they are kept alive for the
            // lifetime of the effect and dropped together on unmount.
            let listeners = subscribe_online_status(cb);

            // Cleanup closure: dropping the tuple drops both
            // `online_listener` and `offline_listener`, removing the window
            // event handlers.
            move || {
                if let Some((online_listener, offline_listener)) = listeners {
                    drop(online_listener);
                    drop(offline_listener);
                }
            }
        });
    }

    *state
}
