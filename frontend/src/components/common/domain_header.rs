use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct DomainHeaderProps {
    #[prop_or_default]
    pub children: Html,
}

#[component]
pub fn DomainHeader(props: &DomainHeaderProps) -> Html {
    html! {
        <div class="gi-domain-header">
            {props.children.clone()}
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DomainStatVariant {
    #[default]
    Neutral,
    Success,
    Warning,
    Error,
}

#[derive(Properties, PartialEq, Eq)]
pub struct DomainStatProps {
    pub label: AttrValue,
    pub value: AttrValue,
    #[prop_or_default]
    pub variant: DomainStatVariant,
}

#[component]
pub fn DomainStat(props: &DomainStatProps) -> Html {
    let value_class = match props.variant {
        DomainStatVariant::Neutral => "gi-domain-stat-value",
        DomainStatVariant::Success => "gi-domain-stat-value gi-domain-stat-success",
        DomainStatVariant::Warning => "gi-domain-stat-value gi-domain-stat-warning",
        DomainStatVariant::Error => "gi-domain-stat-value gi-domain-stat-error",
    };

    html! {
        <div class="gi-domain-stat">
            <span class={value_class}>{&props.value}</span>
            <span class="gi-domain-stat-label">{&props.label}</span>
        </div>
    }
}
