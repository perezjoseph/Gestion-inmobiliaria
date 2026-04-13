use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SkeletonProps {
    #[prop_or("100%".to_string())]
    pub width: String,
    #[prop_or("1rem".to_string())]
    pub height: String,
    #[prop_or("4px".to_string())]
    pub radius: String,
}

#[function_component]
pub fn Skeleton(props: &SkeletonProps) -> Html {
    html! {
        <div class="gi-skeleton"
            style={format!("width: {}; height: {}; border-radius: {};", props.width, props.height, props.radius)}
            aria-hidden="true"
        />
    }
}

#[function_component]
pub fn DashboardSkeleton() -> Html {
    html! {
        <div>
            <div class="gi-page-header">
                <Skeleton width="200px" height="2rem" radius="6px" />
            </div>
            <div class="gi-dashboard-header">
                <div class="gi-dashboard-hero">
                    <Skeleton width="100px" height="100px" radius="50%" />
                    <div style="flex: 1; display: flex; flex-direction: column; gap: var(--space-2);">
                        <Skeleton width="180px" height="0.75rem" />
                        <Skeleton width="240px" height="1.5rem" />
                        <Skeleton width="140px" height="0.75rem" />
                    </div>
                </div>
                <div class="gi-dashboard-secondary">
                    <Skeleton width="120px" height="0.75rem" />
                    <Skeleton width="60px" height="1.5rem" />
                    <Skeleton width="80px" height="0.75rem" />
                </div>
                <div class="gi-dashboard-secondary">
                    <Skeleton width="120px" height="0.75rem" />
                    <Skeleton width="100px" height="1rem" />
                    <Skeleton width="140px" height="0.75rem" />
                </div>
            </div>
            <div class="gi-card" style="padding: var(--space-5); margin-top: var(--space-5);">
                <Skeleton width="160px" height="1rem" radius="6px" />
                <div style="display: flex; flex-direction: column; gap: var(--space-2); margin-top: var(--space-4);">
                    <Skeleton height="3rem" radius="8px" />
                    <Skeleton height="3rem" radius="8px" />
                    <Skeleton height="3rem" radius="8px" />
                </div>
            </div>
        </div>
    }
}
