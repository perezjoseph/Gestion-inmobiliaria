use yew::prelude::*;

#[derive(Properties, PartialEq, Eq)]
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

#[derive(Properties, PartialEq, Eq)]
pub struct TableSkeletonProps {
    pub title_width: AttrValue,
    #[prop_or(5)]
    pub columns: usize,
    #[prop_or(6)]
    pub rows: usize,
    #[prop_or(false)]
    pub has_filter: bool,
}

#[function_component]
pub fn TableSkeleton(props: &TableSkeletonProps) -> Html {
    let col_widths = [
        "40%", "25%", "20%", "15%", "30%", "35%", "22%", "18%",
    ];

    html! {
        <div aria-busy="true" aria-label="Cargando contenido">
            <div class="gi-page-header">
                <Skeleton width={props.title_width.to_string()} height="2rem" radius="6px" />
            </div>

            if props.has_filter {
                <div class="gi-filter-bar" style="margin-bottom: var(--space-4);">
                    <div style="display: flex; gap: var(--space-3); align-items: end;">
                        <div>
                            <Skeleton width="60px" height="0.75rem" radius="4px" />
                            <Skeleton width="140px" height="2.25rem" radius="6px" />
                        </div>
                        <div>
                            <Skeleton width="60px" height="0.75rem" radius="4px" />
                            <Skeleton width="140px" height="2.25rem" radius="6px" />
                        </div>
                    </div>
                </div>
            }

            <div class="gi-table-wrap" style="border-radius: 12px; border: 1px solid var(--border-subtle); overflow: hidden;">
                <table class="gi-table" style="width: 100%;">
                    <thead>
                        <tr>
                            { for (0..props.columns).map(|i| {
                                let w = col_widths.get(i).unwrap_or(&"20%");
                                html! {
                                    <th style="padding: var(--space-3) var(--space-5);">
                                        <Skeleton width={w.to_string()} height="0.75rem" radius="4px" />
                                    </th>
                                }
                            })}
                        </tr>
                    </thead>
                    <tbody>
                        { for (0..props.rows).map(|_| {
                            html! {
                                <tr>
                                    { for (0..props.columns).map(|i| {
                                        let w = col_widths.get(i).unwrap_or(&"20%");
                                        html! {
                                            <td style="padding: var(--space-3) var(--space-5);">
                                                <Skeleton width={w.to_string()} height="0.875rem" radius="4px" />
                                            </td>
                                        }
                                    })}
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>

            <div style="display: flex; justify-content: space-between; align-items: center; padding: var(--space-3) 0;">
                <Skeleton width="120px" height="0.75rem" radius="4px" />
                <div style="display: flex; gap: var(--space-2);">
                    <Skeleton width="32px" height="32px" radius="6px" />
                    <Skeleton width="32px" height="32px" radius="6px" />
                    <Skeleton width="32px" height="32px" radius="6px" />
                </div>
            </div>
        </div>
    }
}

#[function_component]
pub fn ProfileSkeleton() -> Html {
    html! {
        <div aria-busy="true" aria-label="Cargando perfil">
            <div class="gi-page-header">
                <Skeleton width="140px" height="2rem" radius="6px" />
            </div>
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr)); gap: var(--space-5);">
                <div class="gi-card" style="padding: var(--space-5);">
                    <Skeleton width="160px" height="1rem" radius="4px" />
                    <div style="display: flex; flex-direction: column; gap: var(--space-3); margin-top: var(--space-4);">
                        { for (0..3).map(|_| html! {
                            <div>
                                <Skeleton width="80px" height="0.75rem" radius="4px" />
                                <Skeleton height="2.25rem" radius="6px" />
                            </div>
                        })}
                        <div style="display: flex; justify-content: flex-end;">
                            <Skeleton width="140px" height="2.25rem" radius="6px" />
                        </div>
                    </div>
                </div>
                <div class="gi-card" style="padding: var(--space-5);">
                    <Skeleton width="180px" height="1rem" radius="4px" />
                    <div style="display: flex; flex-direction: column; gap: var(--space-3); margin-top: var(--space-4);">
                        { for (0..3).map(|_| html! {
                            <div>
                                <Skeleton width="120px" height="0.75rem" radius="4px" />
                                <Skeleton height="2.25rem" radius="6px" />
                            </div>
                        })}
                        <div style="display: flex; justify-content: flex-end;">
                            <Skeleton width="160px" height="2.25rem" radius="6px" />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[function_component]
pub fn ReportSkeleton() -> Html {
    html! {
        <div aria-busy="true" aria-label="Cargando reporte">
            <div style="display: flex; flex-direction: column; gap: var(--space-2);">
                <Skeleton height="2.25rem" radius="8px" />
                <Skeleton height="2.25rem" radius="8px" />
                <Skeleton height="2.25rem" radius="8px" />
            </div>
        </div>
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
