use yew::prelude::*;

#[derive(Properties, PartialEq, Eq)]
pub struct ComplianceBadgeProps {
    pub porcentaje: u8,
}

const fn nivel(porcentaje: u8) -> &'static str {
    if porcentaje >= 80 {
        "high"
    } else if porcentaje >= 50 {
        "medium"
    } else {
        "low"
    }
}

#[component]
pub fn ComplianceBadge(props: &ComplianceBadgeProps) -> Html {
    let pct = props.porcentaje.min(100);
    let level = nivel(pct);
    let width_style = format!("width: {pct}%;");

    html! {
        <div class="inline-flex items-center" style="gap: var(--space-2);">
            <div class="gi-compliance-meter" style="flex: 1; min-width: 60px;">
                <div
                    class="gi-compliance-meter-fill"
                    data-level={level}
                    style={width_style}
                />
            </div>
            <span style="font-size: var(--text-xs); font-weight: 500; color: var(--text-secondary); white-space: nowrap;"
                  aria-label={format!("Cumplimiento: {pct}%")}>
                {format!("{pct}%")}
            </span>
        </div>
    }
}
