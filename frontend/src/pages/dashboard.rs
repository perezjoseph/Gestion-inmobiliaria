use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;
use crate::components::common::currency_display::CurrencyDisplay;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::DashboardSkeleton;
use crate::services::api::api_get;
use crate::types::DashboardStats;
use crate::types::dashboard_extra::IngresoComparacion;
use crate::types::notificacion::PagoVencido;

#[component]
pub fn Dashboard() -> Html {
    let stats = use_state(|| Option::<DashboardStats>::None);
    let overdue_pagos = use_state(Vec::<PagoVencido>::new);
    let ingreso_comp = use_state(|| Option::<IngresoComparacion>::None);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);

    {
        let stats = stats.clone();
        let overdue_pagos = overdue_pagos.clone();
        let ingreso_comp = ingreso_comp.clone();
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

    html! {
        <div>
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
                <OverdueSection pagos={(*overdue_pagos).clone()} />
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
                    <span class="gi-text-xs gi-mt-1">
                        <Link<Route> to={Route::Pagos} classes="gi-btn-text">{"Ver detalles →"}</Link<Route>>
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
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct OverdueSectionProps {
    pagos: Vec<PagoVencido>,
}

#[component]
fn OverdueSection(props: &OverdueSectionProps) -> Html {
    html! {
        <div class="gi-card-section">
            <div class="gi-section-header">
                <h2 class="gi-section-header-title">{"Pagos Atrasados"}</h2>
                <Link<Route> to={Route::Pagos} classes="gi-btn-text gi-text-xs">{"Ver todos →"}</Link<Route>>
            </div>
            if props.pagos.is_empty() {
                <p class="gi-text-sm gi-text-tertiary gi-py-3">{"Sin pagos atrasados. ¡Todo al día!"}</p>
            } else {
                <div class="gi-flex-col gi-gap-2">
                    { for props.pagos.iter().take(5).map(|p| html! {
                        <div class="gi-overdue-row">
                            <div class="gi-min-w-0 gi-flex-1">
                                <div class="gi-overdue-row-title">{&p.propiedad_titulo}</div>
                                <div class="gi-overdue-row-detail">
                                    {format!("{} {} — ", p.inquilino_nombre, p.inquilino_apellido)}<CurrencyDisplay monto={p.monto} moneda={p.moneda.clone()} />
                                </div>
                            </div>
                            <div class="flex items-center gi-gap-2 gi-flex-shrink-0">
                                <Link<Route> to={Route::Pagos} classes="gi-btn gi-btn-primary gi-btn-sm">
                                    {"Registrar Pago"}
                                </Link<Route>>
                                <span class="gi-badge gi-badge-error">{format!("{} días", p.dias_vencido)}</span>
                            </div>
                        </div>
                    })}
                </div>
            }
        </div>
    }
}
