use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;
use crate::components::common::currency_display::CurrencyDisplay;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::line_chart::{ChartPoint, LineChart};
use crate::components::common::skeleton::DashboardSkeleton;
use crate::services::api::api_get;
use crate::types::contrato::Contrato;
use crate::types::dashboard_extra::{GastosComparacion, IngresoComparacion, OcupacionMensual};
use crate::types::notificacion::PagoVencido;
use crate::types::pago::Pago;
use crate::types::{DashboardStats, PaginatedResponse};
use crate::utils::format_date_display;

const DETAILS_OPEN_KEY: &str = "gi_dashboard_details_open";

#[component]
pub fn Dashboard() -> Html {
    let stats = use_state(|| Option::<DashboardStats>::None);
    let overdue_pagos = use_state(Vec::<PagoVencido>::new);
    let ingreso_comp = use_state(|| Option::<IngresoComparacion>::None);
    let ocupacion_tendencia = use_state(Vec::<OcupacionMensual>::new);
    let gastos_comp = use_state(|| Option::<GastosComparacion>::None);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);

    {
        let stats = stats.clone();
        let overdue_pagos = overdue_pagos.clone();
        let ingreso_comp = ingreso_comp.clone();
        let ocupacion_tendencia = ocupacion_tendencia.clone();
        let gastos_comp = gastos_comp.clone();
        let error = error.clone();
        let loading = loading.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                match api_get::<DashboardStats>("/dashboard/stats").await {
                    Ok(data) => stats.set(Some(data)),
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
            {
                let overdue_pagos = overdue_pagos.clone();
                spawn_local(async move {
                    if let Ok(data) =
                        api_get::<Vec<PagoVencido>>("/notificaciones/pagos-vencidos").await
                    {
                        overdue_pagos.set(data);
                    }
                });
            }
            {
                spawn_local(async move {
                    if let Ok(data) =
                        api_get::<IngresoComparacion>("/dashboard/ingresos-comparacion").await
                    {
                        ingreso_comp.set(Some(data));
                    }
                });
            }
            {
                spawn_local(async move {
                    if let Ok(data) =
                        api_get::<Vec<OcupacionMensual>>("/dashboard/ocupacion-tendencia").await
                    {
                        ocupacion_tendencia.set(data);
                    }
                });
            }
            {
                spawn_local(async move {
                    if let Ok(data) =
                        api_get::<GastosComparacion>("/dashboard/gastos-comparacion").await
                    {
                        gastos_comp.set(Some(data));
                    }
                });
            }
        });
    }

    if *loading {
        return html! { <DashboardSkeleton /> };
    }

    let on_close_error = {
        let error = error.clone();
        Callback::from(move |_: MouseEvent| error.set(None))
    };

    let has_data = stats.is_some();

    // Collapsible state: read initial value from localStorage (default open)
    let details_open = use_state(|| {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|ls| ls.get_item(DETAILS_OPEN_KEY).ok().flatten())
            .is_none_or(|v| v != "false")
    });

    let on_toggle_details = {
        let details_open = details_open.clone();
        Callback::from(move |_: MouseEvent| {
            let new_val = !*details_open;
            details_open.set(new_val);
            if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
                let _ = ls.set_item(DETAILS_OPEN_KEY, if new_val { "true" } else { "false" });
            }
        })
    };

    html! {
        <div aria-live="polite" aria-atomic="true" aria-busy={(*loading).to_string()}>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Panel de Control"}</h1>
            </div>
            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={on_close_error.clone()} />
            }
            if has_data {
                if let Some(s) = (*stats).as_ref() {
                    <StatsHeader stats={s.clone()} ingreso_comp={(*ingreso_comp).clone()} />
                }
                <AttentionSection
                    overdue_pagos={(*overdue_pagos).clone()}
                    stats={(*stats).clone()}
                />
                // Visual separator between attention and details
                <div style="border-top: 1px solid var(--border-subtle); margin-top: var(--space-5); padding-top: var(--space-5);">
                    <div class="gi-collapsible-trigger" onclick={on_toggle_details.clone()}
                        role="button" tabindex="0"
                        aria-expanded={if *details_open { "true" } else { "false" }}
                        aria-controls="dashboard-details-panel">
                        <h2 class="gi-section-header-title">{"Detalles del Portafolio"}</h2>
                        <span style={format!(
                            "display: inline-block; transition: transform var(--duration-normal) var(--ease-out); transform: rotate({}deg);",
                            if *details_open { 180 } else { 0 }
                        )}>
                            {"▾"}
                        </span>
                    </div>
                    <div class="gi-collapsible-content" id="dashboard-details-panel"
                        style={if *details_open { "" } else { "display: none;" }}>
                        <OccupancyChart data={(*ocupacion_tendencia).clone()} />
                        <ContratosPorVencerWidget />
                        <UpcomingPaymentsWidget />
                        <ExpenseCard data={(*gastos_comp).clone()} />
                    </div>
                </div>
            } else {
                <WelcomeCard />
            }
        </div>
    }
}

#[component]
fn WelcomeCard() -> Html {
    html! {
        <div class="gi-welcome-card">
            <h2 class="text-display gi-text-xl gi-font-bold gi-text-primary gi-mb-2">
                {"Bienvenido a Gestión Inmobiliaria"}
            </h2>
            <p class="gi-text-sm gi-text-secondary gi-mb-6">
                {"Comience configurando su portafolio en tres pasos:"}
            </p>
            <div class="gi-flex-col gi-gap-3">
                <div class="gi-welcome-step">
                    <div class="gi-welcome-step-num">{"1"}</div>
                    <div>
                        <div class="gi-welcome-step-title">{"Agregue una propiedad"}</div>
                        <div class="gi-welcome-step-desc">{"Registre la dirección, tipo y precio de alquiler."}</div>
                    </div>
                </div>
                <div class="gi-welcome-step">
                    <div class="gi-welcome-step-num">{"2"}</div>
                    <div>
                        <div class="gi-welcome-step-title">{"Registre un inquilino"}</div>
                        <div class="gi-welcome-step-desc">{"Agregue los datos del arrendatario con su cédula."}</div>
                    </div>
                </div>
                <div class="gi-welcome-step">
                    <div class="gi-welcome-step-num">{"3"}</div>
                    <div>
                        <div class="gi-welcome-step-title">{"Cree un contrato"}</div>
                        <div class="gi-welcome-step-desc">{"Vincule propiedad e inquilino con fechas y monto mensual."}</div>
                    </div>
                </div>
            </div>
            <div class="gi-mt-6">
                <Link<Route> to={Route::Propiedades} classes="gi-btn gi-btn-primary">
                    {"Agregar primera propiedad"}
                </Link<Route>>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct StatsHeaderProps {
    stats: DashboardStats,
    ingreso_comp: Option<IngresoComparacion>,
}

#[component]
fn StatsHeader(props: &StatsHeaderProps) -> Html {
    let s = &props.stats;
    html! {
        <div class="gi-dashboard-header">
            <div class="gi-dashboard-hero">
                <div class="gi-occupancy-ring">
                    <svg viewBox="0 0 36 36" role="img" aria-label={format!("Ocupación del portafolio: {:.0}%", s.tasa_ocupacion)}>
                        <circle cx="18" cy="18" r="15.5" fill="none" stroke="var(--border-subtle)" stroke-width="3"/>
                        <circle cx="18" cy="18" r="15.5" fill="none" stroke="var(--color-success)" stroke-width="3"
                            stroke-dasharray={format!("{:.1} 100", s.tasa_ocupacion)} stroke-linecap="round"/>
                    </svg>
                    <div class="gi-occupancy-ring-label">{format!("{:.0}%", s.tasa_ocupacion)}</div>
                </div>
                <div class="gi-dashboard-hero-metric">
                    <p class="gi-stat-label">{"Ocupación del portafolio"}</p>
                    <p class="gi-stat-value">
                        <CurrencyDisplay monto={s.ingreso_mensual} moneda={"DOP".to_string()} />
                        <span class="gi-stat-suffix">{"/ mes"}</span>
                    </p>
                    <p class="gi-text-xs gi-text-tertiary gi-mt-1">
                        {format!("{} propiedades registradas", s.total_propiedades)}
                    </p>
                </div>
            </div>
            <div class={if s.pagos_atrasados > 0 { "gi-dashboard-secondary gi-dashboard-alert" } else { "gi-dashboard-secondary" }}>
                <p class="gi-stat-label">{if s.pagos_atrasados > 0 { "⚠ Pagos atrasados" } else { "Pagos atrasados" }}</p>
                <p class="gi-stat-value">{s.pagos_atrasados}</p>
                if s.pagos_atrasados > 0 {
                    <div class="gi-overdue-age">
                        <span class="gi-overdue-age-item">
                            <span class="gi-overdue-age-dot gi-overdue-age-dot-recent" />
                            {"< 15 días"}
                        </span>
                        <span class="gi-overdue-age-item">
                            <span class="gi-overdue-age-dot gi-overdue-age-dot-old" />
                            {"> 30 días"}
                        </span>
                    </div>
                    <span class="gi-mt-1">
                        <Link<Route> to={Route::Pagos} classes="gi-btn-text gi-text-xs">{"Ver detalles →"}</Link<Route>>
                    </span>
                } else {
                    <p class="gi-text-xs gi-text-tertiary gi-mt-1">{"Todo al día"}</p>
                }
            </div>
            if let Some(ref comp) = props.ingreso_comp {
                <div class="gi-dashboard-secondary">
                    <p class="gi-stat-label">{"Ingresos del Mes"}</p>
                    <p class="gi-stat-value gi-text-base">
                        <CurrencyDisplay monto={comp.cobrado} moneda={"DOP".to_string()} />
                    </p>
                    <p class="gi-text-xs gi-text-tertiary gi-mt-1">
                        {"Esperado: "}<CurrencyDisplay monto={comp.esperado} moneda={"DOP".to_string()} />
                    </p>
                </div>
            }
            <ComplianceCounters stats={s.clone()} />
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct AttentionSectionProps {
    overdue_pagos: Vec<PagoVencido>,
    stats: Option<DashboardStats>,
}

#[component]
fn AttentionSection(props: &AttentionSectionProps) -> Html {
    let overdue_count = props.overdue_pagos.len();
    let docs_vencidos = props.stats.as_ref().map_or(0, |s| s.documentos_vencidos);
    let docs_por_vencer = props.stats.as_ref().map_or(0, |s| s.documentos_por_vencer);
    let entidades_incompletas = props.stats.as_ref().map_or(0, |s| s.entidades_incompletas);

    let total_alerts: i64 = i64::try_from(overdue_count).unwrap_or(i64::MAX)
        + docs_vencidos
        + docs_por_vencer
        + entidades_incompletas;

    if total_alerts == 0 {
        return html! {
            <div class="gi-card-section gi-attention-clear">
                <p class="gi-text-sm" style="color: var(--color-success-dark); display: flex; align-items: center; gap: var(--space-2);">
                    <span>{"✓"}</span>
                    {"Todo al día. Sin asuntos pendientes."}
                </p>
            </div>
        };
    }

    html! {
        <div class="gi-card-section gi-attention-section">
            <div class="gi-section-header">
                <h2 class="gi-section-header-title">
                    {"Requiere Atención"}
                    <span class="gi-badge gi-badge-error" style="margin-left: var(--space-2); font-size: var(--text-xs);">
                        {total_alerts}
                    </span>
                </h2>
            </div>
            if overdue_count > 0 {
                <div class="gi-attention-group">
                    <div class="gi-attention-group-header">
                        <span class="gi-text-sm gi-font-semibold">{"Pagos Atrasados"}</span>
                        <Link<Route> to={Route::Pagos} classes="gi-btn-text gi-text-xs">{"Ver todos →"}</Link<Route>>
                    </div>
                    <div class="gi-flex-col gi-gap-2">
                        { for props.overdue_pagos.iter().take(3).map(|p| html! {
                            <div class="gi-overdue-row">
                                <div class="gi-min-w-0 gi-flex-1">
                                    <div class="gi-overdue-row-title">{&p.propiedad_titulo}</div>
                                    <div class="gi-overdue-row-detail">
                                        {format!("{} {} — ", p.inquilino_nombre, p.inquilino_apellido)}<CurrencyDisplay monto={p.monto} moneda={p.moneda.clone()} />
                                    </div>
                                </div>
                                <span class="gi-badge gi-badge-error">{format!("{} días", p.dias_vencido)}</span>
                            </div>
                        })}
                        if overdue_count > 3 {
                            <span style="align-self: flex-start;">
                                <Link<Route> to={Route::Pagos} classes="gi-btn gi-btn-primary gi-btn-sm">
                                    {format!("Ver {} más", overdue_count - 3)}
                                </Link<Route>>
                            </span>
                        }
                    </div>
                </div>
            }
            if docs_vencidos > 0 || docs_por_vencer > 0 || entidades_incompletas > 0 {
                <div class="gi-attention-group">
                    <div class="gi-attention-group-header">
                        <span class="gi-text-sm gi-font-semibold">{"Cumplimiento Documental"}</span>
                        <Link<Route> to={Route::DocumentosPorVencer} classes="gi-btn-text gi-text-xs">{"Ver detalles →"}</Link<Route>>
                    </div>
                    <div style="display: flex; gap: var(--space-4); flex-wrap: wrap;">
                        if docs_vencidos > 0 {
                            <span class="gi-attention-counter gi-attention-counter-error">
                                <span class="gi-attention-counter-num">{docs_vencidos}</span>
                                {" vencidos"}
                            </span>
                        }
                        if docs_por_vencer > 0 {
                            <span class="gi-attention-counter gi-attention-counter-warning">
                                <span class="gi-attention-counter-num">{docs_por_vencer}</span>
                                {" por vencer"}
                            </span>
                        }
                        if entidades_incompletas > 0 {
                            <span class="gi-attention-counter gi-attention-counter-warning">
                                <span class="gi-attention-counter-num">{entidades_incompletas}</span>
                                {" incompletas"}
                            </span>
                        }
                    </div>
                </div>
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ComplianceCountersProps {
    stats: DashboardStats,
}

#[component]
fn ComplianceCounters(props: &ComplianceCountersProps) -> Html {
    let s = &props.stats;
    html! {
        <div class="gi-card-section">
            <div class="gi-section-header">
                <h2 class="gi-section-header-title">{"Cumplimiento Documental"}</h2>
            </div>
            <div class="gi-dashboard-header">
                <div class={if s.documentos_vencidos > 0 { "gi-dashboard-secondary gi-dashboard-alert" } else { "gi-dashboard-secondary" }}>
                    <p class="gi-stat-label">{"Documentos vencidos"}</p>
                    <p class="gi-stat-value">{s.documentos_vencidos}</p>
                </div>
                <div class={if s.documentos_por_vencer > 0 { "gi-dashboard-secondary gi-dashboard-alert" } else { "gi-dashboard-secondary" }}>
                    <p class="gi-stat-label">{"Por vencer"}</p>
                    <p class="gi-stat-value">{s.documentos_por_vencer}</p>
                </div>
                <div class={if s.entidades_incompletas > 0 { "gi-dashboard-secondary gi-dashboard-alert" } else { "gi-dashboard-secondary" }}>
                    <p class="gi-stat-label">{"Entidades incompletas"}</p>
                    <p class="gi-stat-value">{s.entidades_incompletas}</p>
                </div>
            </div>
        </div>
    }
}

/// Computes the date 30 days from now as YYYY-MM-DD for the API query.
fn thirty_days_from_now() -> String {
    const THIRTY_DAYS_MS: f64 = 30.0 * 24.0 * 60.0 * 60.0 * 1000.0;
    let now = js_sys::Date::new_0();
    let future = js_sys::Date::new_0();
    future.set_time(now.get_time() + THIRTY_DAYS_MS);
    let y = future.get_full_year();
    let m = future.get_month() + 1; // 0-indexed
    let d = future.get_date();
    format!("{y:04}-{m:02}-{d:02}")
}

#[component]
fn UpcomingPaymentsWidget() -> Html {
    let pagos = use_state(Vec::<Pago>::new);
    let loading = use_state(|| true);

    {
        let pagos = pagos.clone();
        let loading = loading.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                let hasta = thirty_days_from_now();
                let url = format!("/pagos?estado=pendiente&fechaHasta={hasta}&perPage=10");
                if let Ok(resp) = api_get::<PaginatedResponse<Pago>>(&url).await {
                    let mut data = resp.data;
                    data.sort_by(|a, b| a.fecha_vencimiento.cmp(&b.fecha_vencimiento));
                    pagos.set(data);
                }
                loading.set(false);
            });
        });
    }

    if *loading {
        return html! {
            <div class="gi-card-section">
                <p class="gi-text-sm gi-text-tertiary gi-py-3">{"Cargando..."}</p>
            </div>
        };
    }

    html! {
        <div class="gi-card-section">
            <div class="gi-section-header">
                <h2 class="gi-section-header-title">{"Próximos Pagos"}</h2>
                <Link<Route> to={Route::Pagos} classes="gi-btn-text gi-text-xs">{"Ver todos →"}</Link<Route>>
            </div>
            if pagos.is_empty() {
                <p class="gi-text-sm gi-text-tertiary gi-py-3">{"Sin pagos pendientes en los próximos 30 días."}</p>
            } else {
                <div class="gi-flex-col gi-gap-2">
                    { for pagos.iter().map(|p| html! {
                        <UpcomingPaymentRow pago={p.clone()} />
                    })}
                </div>
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct UpcomingPaymentRowProps {
    pago: Pago,
}

#[component]
fn UpcomingPaymentRow(props: &UpcomingPaymentRowProps) -> Html {
    let p = &props.pago;
    html! {
        <div class="gi-overdue-row">
            <div class="gi-min-w-0 gi-flex-1">
                <div class="gi-overdue-row-title">
                    {"Vence: "}{format_date_display(&p.fecha_vencimiento)}
                </div>
                <div class="gi-overdue-row-detail">
                    <CurrencyDisplay monto={p.monto} moneda={p.moneda.clone()} />
                </div>
            </div>
            <span class="gi-badge gi-badge-warning">{"Pendiente"}</span>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct OccupancyChartProps {
    data: Vec<OcupacionMensual>,
}

#[component]
fn OccupancyChart(props: &OccupancyChartProps) -> Html {
    let points: Vec<ChartPoint> = props
        .data
        .iter()
        .map(|d| {
            let label = format!("{:02}/{}", d.mes, d.anio % 100);
            ChartPoint {
                label: AttrValue::from(label),
                value: d.tasa,
            }
        })
        .collect();

    html! {
        <div class="gi-card-section">
            <LineChart
                points={points}
                title={AttrValue::from("Ocupación últimos 12 meses")}
                suffix={AttrValue::from("%")}
            />
        </div>
    }
}

#[component]
fn ContratosPorVencerWidget() -> Html {
    let contratos_30 = use_state(Vec::<Contrato>::new);
    let contratos_60 = use_state(Vec::<Contrato>::new);
    let contratos_90 = use_state(Vec::<Contrato>::new);
    let loading = use_state(|| true);

    {
        let contratos_30 = contratos_30.clone();
        let contratos_60 = contratos_60.clone();
        let contratos_90 = contratos_90.clone();
        let loading = loading.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                if let Ok(data) = api_get::<Vec<Contrato>>("/contratos/por-vencer?dias=30").await {
                    contratos_30.set(data);
                }
            });
            {
                let contratos_60 = contratos_60.clone();
                spawn_local(async move {
                    if let Ok(data) =
                        api_get::<Vec<Contrato>>("/contratos/por-vencer?dias=60").await
                    {
                        contratos_60.set(data);
                    }
                });
            }
            {
                let contratos_90 = contratos_90.clone();
                spawn_local(async move {
                    if let Ok(data) =
                        api_get::<Vec<Contrato>>("/contratos/por-vencer?dias=90").await
                    {
                        contratos_90.set(data);
                    }
                    loading.set(false);
                });
            }
        });
    }

    if *loading {
        return html! {
            <div class="gi-card-section">
                <p class="gi-text-sm gi-text-tertiary gi-py-3">{"Cargando contratos..."}</p>
            </div>
        };
    }

    html! {
        <div class="gi-card-section">
            <div class="gi-section-header">
                <h2 class="gi-section-header-title">{"Contratos por vencer"}</h2>
                <Link<Route> to={Route::Contratos} classes="gi-btn-text gi-text-xs">{"Ver todos →"}</Link<Route>>
            </div>
            <ContratosBucket label="Próximos 30 días" contratos={(*contratos_30).clone()} />
            <ContratosBucket label="Próximos 60 días" contratos={(*contratos_60).clone()} />
            <ContratosBucket label="Próximos 90 días" contratos={(*contratos_90).clone()} />
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ContratosBucketProps {
    label: AttrValue,
    contratos: Vec<Contrato>,
}

#[component]
fn ContratosBucket(props: &ContratosBucketProps) -> Html {
    html! {
        <div class="gi-mb-3">
            <p class="gi-text-xs gi-font-bold gi-text-secondary gi-mb-1">
                {&props.label}{format!(" ({})", props.contratos.len())}
            </p>
            if props.contratos.is_empty() {
                <p class="gi-text-sm gi-text-tertiary">{"Sin contratos en este período."}</p>
            } else {
                <div class="gi-flex-col gi-gap-1">
                    { for props.contratos.iter().map(|c| html! {
                        <ContratoPorVencerRow contrato={c.clone()} />
                    })}
                </div>
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ContratoPorVencerRowProps {
    contrato: Contrato,
}

#[component]
fn ContratoPorVencerRow(props: &ContratoPorVencerRowProps) -> Html {
    let c = &props.contrato;
    html! {
        <div class="gi-overdue-row">
            <div class="gi-min-w-0 gi-flex-1">
                <div class="gi-overdue-row-title">
                    {"Vence: "}{format_date_display(&c.fecha_fin)}
                </div>
                <div class="gi-overdue-row-detail">
                    <CurrencyDisplay monto={c.monto_mensual} moneda={c.moneda.clone()} />
                    {" / mes"}
                </div>
            </div>
            <span class="gi-badge gi-badge-warning">{"Por vencer"}</span>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ExpenseCardProps {
    data: Option<GastosComparacion>,
}

#[component]
fn ExpenseCard(props: &ExpenseCardProps) -> Html {
    let Some(ref data) = props.data else {
        return html! {};
    };

    html! {
        <div class="gi-card-section">
            <div class="gi-section-header">
                <h2 class="gi-section-header-title">{"Resumen de Gastos"}</h2>
                <Link<Route> to={Route::Gastos} classes="gi-btn-text gi-text-xs">{"Ver todos →"}</Link<Route>>
            </div>
            <div class="gi-dashboard-header">
                <div class="gi-dashboard-secondary">
                    <p class="gi-stat-label">{"Pendientes"}</p>
                    <p class="gi-stat-value">
                        <CurrencyDisplay monto={data.total_pendiente} moneda={"DOP".to_string()} />
                    </p>
                </div>
                <div class="gi-dashboard-secondary">
                    <p class="gi-stat-label">{"Pagados este mes"}</p>
                    <p class="gi-stat-value">
                        <CurrencyDisplay monto={data.mes_actual} moneda={"DOP".to_string()} />
                    </p>
                </div>
                <div class={if data.gastos_vencidos > 0 { "gi-dashboard-secondary gi-dashboard-alert" } else { "gi-dashboard-secondary" }}>
                    <p class="gi-stat-label">{"Gastos vencidos"}</p>
                    <p class="gi-stat-value">{data.gastos_vencidos}</p>
                </div>
            </div>
        </div>
    }
}
