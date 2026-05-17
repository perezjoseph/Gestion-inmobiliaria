use yew::prelude::*;

use crate::hooks::use_online;

/// Wraps any submit button or action control and disables it when the browser
/// is offline. Shows a "Sin conexión" tooltip on hover while disabled.
///
/// Usage:
/// ```ignore
/// <OfflineGuard>
///     <button type="submit" class="gi-btn gi-btn-primary">{"Guardar"}</button>
/// </OfflineGuard>
/// ```
#[derive(Properties, PartialEq)]
pub struct OfflineGuardProps {
    pub children: Children,
}

#[component]
pub fn OfflineGuard(props: &OfflineGuardProps) -> Html {
    let online = use_online();

    if online {
        html! { <>{ for props.children.iter() }</> }
    } else {
        html! {
            <span
                style="position: relative; display: inline-block; cursor: not-allowed;"
                title="Sin conexión"
            >
                <span style="pointer-events: none; opacity: 0.5;">
                    { for props.children.iter() }
                </span>
            </span>
        }
    }
}
