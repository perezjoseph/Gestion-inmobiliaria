use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct HelpTooltipProps {
    pub text: AttrValue,
    #[prop_or_default]
    pub id: Option<AttrValue>,
}

/// A small "?" icon that reveals a tooltip on hover/focus.
/// Use `id` prop to connect via `aria-describedby` on the associated input.
#[component]
pub fn HelpTooltip(props: &HelpTooltipProps) -> Html {
    let visible = use_state(|| false);

    let on_enter = {
        let visible = visible.clone();
        Callback::from(move |_: MouseEvent| visible.set(true))
    };
    let on_leave = {
        let visible = visible.clone();
        Callback::from(move |_: MouseEvent| visible.set(false))
    };
    let on_focus = {
        let visible = visible.clone();
        Callback::from(move |_: FocusEvent| visible.set(true))
    };
    let on_blur = {
        let visible = visible.clone();
        Callback::from(move |_: FocusEvent| visible.set(false))
    };

    html! {
        <span class="gi-help-tooltip-wrap"
            onmouseenter={on_enter}
            onmouseleave={on_leave}
        >
            <button
                type="button"
                class="gi-help-tooltip-trigger"
                onfocus={on_focus}
                onblur={on_blur}
                aria-label="Ayuda"
                tabindex="0"
            >
                {"?"}
            </button>
            if *visible {
                <span
                    class="gi-help-tooltip-content"
                    role="tooltip"
                    id={props.id.as_ref().map(|id| id.to_string())}
                >
                    {&props.text}
                </span>
            }
        </span>
    }
}
