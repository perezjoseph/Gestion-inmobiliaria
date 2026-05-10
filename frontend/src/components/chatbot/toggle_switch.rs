use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ToggleSwitchProps {
    pub checked: bool,
    #[prop_or_default]
    pub disabled: bool,
    pub on_toggle: Callback<InputEvent>,
    #[prop_or_default]
    pub label: Option<AttrValue>,
    #[prop_or_default]
    pub size: ToggleSize,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ToggleSize {
    #[default]
    Md,
    Lg,
}

#[component]
pub fn ToggleSwitch(props: &ToggleSwitchProps) -> Html {
    let (track_w, track_h, knob, offset_on, offset_off) = match props.size {
        ToggleSize::Md => (44_u32, 24_u32, 18_u32, 23_i32, 3_i32),
        ToggleSize::Lg => (56, 30, 22, 30, 4),
    };

    let track_bg = if props.checked {
        "var(--color-success)"
    } else {
        "var(--border-strong)"
    };
    let cursor = if props.disabled {
        "not-allowed"
    } else {
        "pointer"
    };
    let knob_left = if props.checked { offset_on } else { offset_off };
    let knob_bottom =
        (i32::try_from(track_h).unwrap_or(24) - i32::try_from(knob).unwrap_or(18)) / 2;

    html! {
        <label
            class="relative inline-block"
            style={format!("width: {track_w}px; height: {track_h}px;")}
            role="switch"
            aria-checked={props.checked.to_string()}
            aria-label={props.label.clone().unwrap_or_default()}
        >
            <input
                type="checkbox"
                checked={props.checked}
                oninput={props.on_toggle.clone()}
                disabled={props.disabled}
                class="opacity-0 w-0 h-0 absolute"
            />
            <span
                class="absolute inset-0 rounded-xl transition-colors"
                style={format!("background-color: {track_bg}; cursor: {cursor}; opacity: {};", if props.disabled { "0.5" } else { "1" })}
            >
                <span
                    class="absolute rounded-full transition-all"
                    style={format!(
                        "height: {knob}px; width: {knob}px; left: {knob_left}px; bottom: {knob_bottom}px; background-color: var(--surface-raised); box-shadow: var(--shadow-sm);"
                    )}
                />
            </span>
        </label>
    }
}
