use yew::prelude::*;

/// A contextual stat strip displayed above the filter bar on CRUD pages.
/// Shows domain-specific summary info that differentiates each section.
///
/// Example usage:
/// ```text
/// <DomainHeader>
///     <DomainStat label="Atrasados" value="3" variant={DomainStatVariant::Error} />
///     <DomainStat label="Pendiente este mes" value="RD$45,000" />
/// </DomainHeader>
/// ```
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

/// Visual variant for a domain stat.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DomainStatVariant {
    #[default]
    Neutral,
    Success,
    Warning,
    Error,
}

#[derive(Properties, PartialEq)]
pub struct DomainStatProps {
    /// Short label (e.g., "Atrasados", "Ocupación").
    pub label: AttrValue,
    /// Value to display (e.g., "3", "85%", "RD$45,000").
    pub value: AttrValue,
    /// Visual variant for emphasis.
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
