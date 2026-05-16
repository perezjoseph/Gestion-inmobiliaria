use yew::prelude::*;

/// A data point for the line chart.
#[derive(Clone, PartialEq)]
pub struct ChartPoint {
    pub label: AttrValue,
    pub value: f64,
}

#[derive(Properties, PartialEq)]
pub struct LineChartProps {
    pub points: Vec<ChartPoint>,
    #[prop_or(AttrValue::from(""))]
    pub title: AttrValue,
    #[prop_or(AttrValue::from("%"))]
    pub suffix: AttrValue,
}

const CHART_WIDTH: f64 = 600.0;
const CHART_HEIGHT: f64 = 200.0;
const PADDING_LEFT: f64 = 40.0;
const PADDING_RIGHT: f64 = 16.0;
const PADDING_TOP: f64 = 16.0;
const PADDING_BOTTOM: f64 = 32.0;

#[component]
pub fn LineChart(props: &LineChartProps) -> Html {
    if props.points.is_empty() {
        return html! {
            <div class="gi-chart-empty">
                <p class="gi-text-sm gi-text-tertiary">{"Sin datos disponibles"}</p>
            </div>
        };
    }

    let points = &props.points;
    let max_val = points
        .iter()
        .map(|p| p.value)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let min_val = points.iter().map(|p| p.value).fold(f64::MAX, f64::min);
    let range = (max_val - min_val).max(1.0);

    let usable_w = CHART_WIDTH - PADDING_LEFT - PADDING_RIGHT;
    let usable_h = CHART_HEIGHT - PADDING_TOP - PADDING_BOTTOM;

    let x_step = if points.len() > 1 {
        usable_w / (points.len() - 1) as f64
    } else {
        usable_w
    };

    // Build the polyline path
    let path_data: String = points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let x = PADDING_LEFT + i as f64 * x_step;
            let y = PADDING_TOP + usable_h - ((p.value - min_val) / range) * usable_h;
            if i == 0 {
                format!("M {x:.1} {y:.1}")
            } else {
                format!(" L {x:.1} {y:.1}")
            }
        })
        .collect();

    // Build the filled area path
    let area_data = {
        let baseline_y = PADDING_TOP + usable_h;
        let first_x = PADDING_LEFT;
        let last_x = PADDING_LEFT + (points.len() - 1) as f64 * x_step;
        format!("{path_data} L {last_x:.1} {baseline_y:.1} L {first_x:.1} {baseline_y:.1} Z")
    };

    // Y-axis grid lines (4 steps)
    let grid_lines: Vec<Html> = (0..=4)
        .map(|i| {
            let frac = i as f64 / 4.0;
            let y = PADDING_TOP + usable_h - frac * usable_h;
            let val = min_val + frac * range;
            html! {
                <g key={i}>
                    <line
                        x1={format!("{:.1}", PADDING_LEFT)}
                        y1={format!("{y:.1}")}
                        x2={format!("{:.1}", CHART_WIDTH - PADDING_RIGHT)}
                        y2={format!("{y:.1}")}
                        stroke="var(--border-subtle)"
                        stroke-width="0.5"
                        stroke-dasharray="4 2"
                    />
                    <text
                        x={format!("{:.1}", PADDING_LEFT - 6.0)}
                        y={format!("{:.1}", y + 3.0)}
                        text-anchor="end"
                        font-size="9"
                        fill="var(--text-tertiary)"
                    >
                        {format!("{:.0}", val)}
                    </text>
                </g>
            }
        })
        .collect();

    // X-axis labels
    let x_labels: Vec<Html> = points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let x = PADDING_LEFT + i as f64 * x_step;
            let y = CHART_HEIGHT - 6.0;
            html! {
                <text
                    key={i}
                    x={format!("{x:.1}")}
                    y={format!("{y:.1}")}
                    text-anchor="middle"
                    font-size="9"
                    fill="var(--text-tertiary)"
                >
                    {&p.label}
                </text>
            }
        })
        .collect();

    // Data point dots
    let dots: Vec<Html> = points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let x = PADDING_LEFT + i as f64 * x_step;
            let y = PADDING_TOP + usable_h - ((p.value - min_val) / range) * usable_h;
            html! {
                <circle
                    key={i}
                    cx={format!("{x:.1}")}
                    cy={format!("{y:.1}")}
                    r="3"
                    fill="var(--color-primary)"
                    stroke="var(--bg-surface)"
                    stroke-width="1.5"
                >
                    <title>{format!("{}: {:.1}{}", p.label, p.value, props.suffix)}</title>
                </circle>
            }
        })
        .collect();

    html! {
        <div class="gi-line-chart">
            if !props.title.is_empty() {
                <h3 class="gi-section-header-title gi-mb-3">{&props.title}</h3>
            }
            <svg
                viewBox={format!("0 0 {CHART_WIDTH} {CHART_HEIGHT}")}
                preserveAspectRatio="xMidYMid meet"
                style="width: 100%; height: auto;"
                role="img"
                aria-label={props.title.clone()}
            >
                { for grid_lines }
                <path
                    d={area_data}
                    fill="var(--color-primary)"
                    opacity="0.08"
                />
                <path
                    d={path_data}
                    fill="none"
                    stroke="var(--color-primary)"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                />
                { for dots }
                { for x_labels }
            </svg>
        </div>
    }
}
