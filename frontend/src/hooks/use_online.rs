use yew::prelude::*;

use crate::services::online::{is_online, subscribe_online_status};

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

            let listeners = subscribe_online_status(cb);

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
