use yew::prelude::*;

use crate::hooks::use_online;

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
                title="Sin conexiÃ³n"
            >
                <span style="pointer-events: none; opacity: 0.5;">
                    { for props.children.iter() }
                </span>
            </span>
        }
    }
}
