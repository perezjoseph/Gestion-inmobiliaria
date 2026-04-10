use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::loading::Loading;
use crate::services::api::api_get;
use crate::types::DashboardStats;
use crate::types::PaginatedResponse;
use crate::types::contrato::Contrato;
use crate::types::pago::Pago;
use crate::types::propiedad::Propiedad;
use crate::utils::{format_currency, format_date_display};

#[function_component]
pub fn Dashboard() -> Html {
    let stats = use_state(|| Option::<DashboardStats>::None);
    let overdue_pagos = use_state(Vec::<Pago>::new);
    let expiring_contratos = use_state(Vec::<Contrato>::new);
    let vacant_props = use_state(Vec::<Propiedad>::new);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);

    {
        let stats = stats.clone();
        let overdue_pagos = overdue_pagos.clone();
        let expiring_contratos = expiring_contratos.clone();
        let vacant_props = vacant_props.clone();
        let error = error.clone();
        let loading = loading.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                let (stats_res, pagos_res, contratos_res, props_res) = futures::join!(
                    api_get::<DashboardStats>("/dashboard/stats"),
                    api_get::<Vec<Pago>>("/pagos?estado=atrasado"),
                    api_get::<Vec<Contrato>>("/contratos"),
                    api_get::<PaginatedResponse<Propiedad>>(
                        "/propiedades?estado=disponible&page=1&perPage=5"
                    ),
                );

                match stats_res {
                    Ok(data) => stats.set(Some(data)),
                    Err(err) => error.set(Some(err)),
                }
                if let Ok(data) = pagos_res {
                    overdue_pagos.set(data);
                }
                if let Ok(data) = contratos_res {
                    let expiring: Vec<Contrato> = data
                        .into_iter()
                        .filter(|c| c.estado == "activo")
                        .take(5)
                        .collect();
                    expiring_contratos.set(expiring);
                }
                if let Ok(resp) = props_res {
                    vacant_props.set(resp.data);
                }
                loading.set(false);
            });
        });
    }

    if *loading {
        return html! { <Loading /> };
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

            if !has_data {
                // First-run onboarding
                <div class="gi-welcome-card">
                    <h2 class="text-display" style="font-size: var(--text-xl); font-weight: 700; color: var(--text-primary); margin-bottom: var(--space-2);">
                        {"Bienvenido a Gestión Inmobiliaria"}
                    </h2>
                    <p style="font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-6);">
                        {"Comience configurando su portafolio en tres pasos:"}
                    </p>
                    <div style="display: flex; flex-direction: column; gap: var(--space-3);">
                        <div class="gi-welcome-step">
                            <div class="gi-welcome-step-num">{"1"}</div>
                            <div>
                                <div style="font-weight: 600; font-size: var(--text-sm); color: var(--text-primary);">{"Agregue una propiedad"}</div>
                                <div style="font-size: var(--text-xs); color: var(--text-secondary);">{"Registre la dirección, tipo y precio de alquiler."}</div>
                            </div>
                        </div>
                        <div class="gi-welcome-step">
                            <div class="gi-welcome-step-num">{"2"}</div>
                            <div>
                                <div style="font-weight: 600; font-size: var(--text-sm); color: var(--text-primary);">{"Registre un inquilino"}</div>
                                <div style="font-size: var(--text-xs); color: var(--text-secondary);">{"Agregue los datos del arrendatario con su cédula."}</div>
                            </div>
                        </div>
                        <div class="gi-welcome-step">
                            <div class="gi-welcome-step-num">{"3"}</div>
                            <div>
                                <div style="font-weight: 600; font-size: var(--text-sm); color: var(--text-primary);">{"Cree un contrato"}</div>
                                <div style="font-size: var(--text-xs); color: var(--text-secondary);">{"Vincule propiedad e inquilino con fechas y monto mensual."}</div>
                            </div>
                        </div>
                    </div>
                    <div style="margin-top: var(--space-6);">
                        <Link<Route> to={Route::Propiedades} classes="gi-btn gi-btn-primary">
                            {"Agregar primera propiedad"}
                        </Link<Route>>
                    </div>
                </div>
            } else {
                // Stats row
                if let Some(s) = (*stats).as_ref() {
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--space-4); margin-bottom: var(--space-6);">
                        <div class="gi-stat">
                            <p style="font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-tertiary);">
                                {"Propiedades"}
                            </p>
                            <p class="tabular-nums text-display" style="font-size: var(--text-2xl); font-weight: 700; color: var(--text-primary); margin-top: var(--space-1);">
                                {s.total_propiedades}
                            </p>
                        </div>
                        <div class="gi-stat">
                            <p style="font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-tertiary);">
                                {"Ocupación"}
                            </p>
                            <p class="tabular-nums text-display" style="font-size: var(--text-2xl); font-weight: 700; color: var(--text-primary); margin-top: var(--space-1);">
                                {format!("{:.0}%", s.tasa_ocupacion)}
                            </p>
                            <div style="margin-top: var(--space-2); height: 4px; border-radius: 2px; background-color: var(--border-subtle); overflow: hidden;">
                                <div style={format!("width: {:.0}%; height: 100%; border-radius: 2px; background-color: var(--color-success); transition: width 0.6s var(--ease-out);", s.tasa_ocupacion)}></div>
                            </div>
                        </div>
                        <div class="gi-stat">
                            <p style="font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-tertiary);">
                                {"Ingreso Mensual"}
                            </p>
                            <p class="tabular-nums text-display" style="font-size: var(--text-xl); font-weight: 700; color: var(--text-primary); margin-top: var(--space-1);">
                                {format_currency("DOP", s.ingreso_mensual)}
                            </p>
                        </div>
                        <div class="gi-stat">
                            <p style="font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-tertiary);">
                                {"Pagos Atrasados"}
                            </p>
                            <p class="tabular-nums text-display" style={format!("font-size: var(--text-2xl); font-weight: 700; margin-top: var(--space-1); color: {};",
                                if s.pagos_atrasados > 0 { "var(--color-error)" } else { "var(--text-primary)" })}>
                                {s.pagos_atrasados}
                            </p>
                        </div>
                    </div>
                }

                // Actionable sections
                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr)); gap: var(--space-5);">
                    // Overdue payments
                    <div class="gi-card" style="padding: var(--space-5);">
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-4);">
                            <h2 class="text-display" style="font-size: var(--text-base); font-weight: 600; color: var(--text-primary);">
                                {"Pagos Atrasados"}
                            </h2>
                            <Link<Route> to={Route::Pagos} classes="gi-btn-text">
                                {"Ver todos →"}
                            </Link<Route>>
                        </div>
                        if (*overdue_pagos).is_empty() {
                            <p style="font-size: var(--text-sm); color: var(--text-tertiary); padding: var(--space-3) 0;">
                                {"Sin pagos atrasados. ¡Todo al día!"}
                            </p>
                        } else {
                            <div style="display: flex; flex-direction: column; gap: var(--space-2);">
                                { for (*overdue_pagos).iter().take(5).map(|p| {
                                    html! {
                                        <div style="display: flex; justify-content: space-between; align-items: center; padding: var(--space-2) var(--space-3); border-radius: 8px; background-color: var(--color-error-light);">
                                            <div>
                                                <span class="tabular-nums" style="font-size: var(--text-sm); font-weight: 600; color: var(--color-error-dark);">
                                                    {format_currency(&p.moneda, p.monto)}
                                                </span>
                                            </div>
                                            <span class="tabular-nums" style="font-size: var(--text-xs); color: var(--color-error);">
                                                {"Vence: "}{format_date_display(&p.fecha_vencimiento)}
                                            </span>
                                        </div>
                                    }
                                })}
                            </div>
                        }
                    </div>

                    // Active contracts
                    <div class="gi-card" style="padding: var(--space-5);">
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-4);">
                            <h2 class="text-display" style="font-size: var(--text-base); font-weight: 600; color: var(--text-primary);">
                                {"Contratos Activos"}
                            </h2>
                            <Link<Route> to={Route::Contratos} classes="gi-btn-text">
                                {"Ver todos →"}
                            </Link<Route>>
                        </div>
                        if (*expiring_contratos).is_empty() {
                            <p style="font-size: var(--text-sm); color: var(--text-tertiary); padding: var(--space-3) 0;">
                                {"No hay contratos activos registrados."}
                            </p>
                        } else {
                            <div style="display: flex; flex-direction: column; gap: var(--space-2);">
                                { for (*expiring_contratos).iter().map(|c| {
                                    html! {
                                        <div style="display: flex; justify-content: space-between; align-items: center; padding: var(--space-2) var(--space-3); border-radius: 8px; background-color: var(--surface-base);">
                                            <span class="tabular-nums" style="font-size: var(--text-sm); font-weight: 500; color: var(--text-primary);">
                                                {format_currency(&c.moneda, c.monto_mensual)}{" /mes"}
                                            </span>
                                            <span class="tabular-nums" style="font-size: var(--text-xs); color: var(--text-secondary);">
                                                {"Hasta: "}{format_date_display(&c.fecha_fin)}
                                            </span>
                                        </div>
                                    }
                                })}
                            </div>
                        }
                    </div>

                    // Vacant properties
                    <div class="gi-card" style="padding: var(--space-5);">
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-4);">
                            <h2 class="text-display" style="font-size: var(--text-base); font-weight: 600; color: var(--text-primary);">
                                {"Propiedades Disponibles"}
                            </h2>
                            <Link<Route> to={Route::Propiedades} classes="gi-btn-text">
                                {"Ver todas →"}
                            </Link<Route>>
                        </div>
                        if (*vacant_props).is_empty() {
                            <p style="font-size: var(--text-sm); color: var(--text-tertiary); padding: var(--space-3) 0;">
                                {"Todas las propiedades están ocupadas."}
                            </p>
                        } else {
                            <div style="display: flex; flex-direction: column; gap: var(--space-2);">
                                { for (*vacant_props).iter().map(|p| {
                                    html! {
                                        <div style="display: flex; justify-content: space-between; align-items: center; padding: var(--space-2) var(--space-3); border-radius: 8px; background-color: var(--surface-base);">
                                            <div>
                                                <div style="font-size: var(--text-sm); font-weight: 500; color: var(--text-primary);">{&p.titulo}</div>
                                                <div style="font-size: var(--text-xs); color: var(--text-secondary);">{&p.ciudad}{", "}{&p.provincia}</div>
                                            </div>
                                            <span class="tabular-nums" style="font-size: var(--text-sm); font-weight: 600; color: var(--color-primary-500);">
                                                {format_currency(&p.moneda, p.precio)}
                                            </span>
                                        </div>
                                    }
                                })}
                            </div>
                        }
                    </div>
                </div>
            }
        </div>
    }
}
