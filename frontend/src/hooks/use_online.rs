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

            let guards = subscribe_online_status(cb);

            // Return cleanup closure that drops the listeners on unmount
            move || {
                drop(guards);
            }
        });
    }

    *state
}
