use yew::prelude::*;

/// A single card in the mobile card list.
#[derive(Properties, PartialEq)]
pub struct MobileCardProps {
    /// Primary label (entity name/title). Shown bold.
    pub title: AttrValue,
    /// Secondary detail line (amount, date, etc).
    #[prop_or_default]
    pub subtitle: AttrValue,
    /// Optional status badge HTML (pass a rendered gi-badge span).
    #[prop_or_default]
    pub badge: Html,
    /// Optional tertiary info line.
    #[prop_or_default]
    pub detail: AttrValue,
    /// Click handler for the card (e.g., navigate to detail).
    #[prop_or_default]
    pub onclick: Option<Callback<MouseEvent>>,
}

#[component]
pub fn MobileCard(props: &MobileCardProps) -> Html {
    let click = props.onclick.clone();
    let on_click = Callback::from(move |e: MouseEvent| {
        if let Some(ref cb) = click {
            cb.emit(e);
        }
    });
    let interactive_class = if props.onclick.is_some() {
        "gi-mobile-card gi-mobile-card-interactive"
    } else {
        "gi-mobile-card"
    };

    html! {
        <div class={interactive_class} onclick={on_click} role="article">
            <div class="gi-mobile-card-content">
                <div class="gi-mobile-card-title">{&props.title}</div>
                if !props.subtitle.is_empty() {
                    <div class="gi-mobile-card-subtitle">{&props.subtitle}</div>
                }
                if !props.detail.is_empty() {
                    <div class="gi-mobile-card-detail">{&props.detail}</div>
                }
            </div>
            <div class="gi-mobile-card-badge">
                {props.badge.clone()}
            </div>
        </div>
    }
}

/// Container for a list of mobile cards. Only visible at ≤768px (via CSS).
/// Pair with `DataTable` which is hidden on mobile via `.gi-mobile-hidden`.
#[derive(Properties, PartialEq)]
pub struct MobileCardListProps {
    #[prop_or_default]
    pub children: Html,
}

#[component]
pub fn MobileCardList(props: &MobileCardListProps) -> Html {
    html! {
        <div class="gi-mobile-card-list">
            {props.children.clone()}
        </div>
    }
}
