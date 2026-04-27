use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ConfidenceInputProps {
    pub value: AttrValue,
    pub confidence: Option<f64>,
    pub oninput: Callback<InputEvent>,
    #[prop_or(AttrValue::from("text"))]
    pub input_type: AttrValue,
    #[prop_or_default]
    pub placeholder: AttrValue,
    #[prop_or_default]
    pub class: AttrValue,
}

fn is_low_confidence(confidence: Option<f64>) -> bool {
    matches!(confidence, Some(c) if c < 0.7)
}

#[component]
pub fn ConfidenceInput(props: &ConfidenceInputProps) -> Html {
    let low = is_low_confidence(props.confidence);

    let input_style = if low {
        "border-color: #f59e0b; background-color: #fffbeb;"
    } else {
        ""
    };

    let badge = if low {
        let pct = (props.confidence.unwrap_or(0.0) * 100.0) as u32;
        html! {
            <span style="font-size: var(--text-xs); color: #b45309; background: #fef3c7; padding: 1px 6px; border-radius: 4px; white-space: nowrap;">
                {format!("Confianza: {pct}%")}
            </span>
        }
    } else {
        html! {}
    };

    let combined_class = if props.class.is_empty() {
        "gi-input".to_string()
    } else {
        format!("gi-input {}", props.class)
    };

    html! {
        <div style="display: flex; align-items: center; gap: var(--space-2);">
            <input
                type={props.input_type.clone()}
                value={props.value.clone()}
                oninput={props.oninput.clone()}
                placeholder={props.placeholder.clone()}
                class={combined_class}
                style={input_style}
            />
            {badge}
        </div>
    }
}
