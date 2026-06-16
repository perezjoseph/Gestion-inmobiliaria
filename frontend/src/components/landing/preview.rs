use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;

#[component]
pub fn LandingPreview() -> Html {
    html! {
        <section class="gi-l-preview">
            <div class="gi-l-container">
                    <h2 class="gi-l-section-title">{"Así se ve por dentro"}</h2>
                    <p class="gi-l-section-sub">{"Un panel claro con tus propiedades, contratos y pagos al día."}</p>
                <div class="gi-l-preview-frame" aria-hidden="true">
                    <div class="gi-l-preview-bar">
                        <span class="gi-l-preview-dot"></span>
                        <span class="gi-l-preview-dot"></span>
                        <span class="gi-l-preview-dot"></span>
                        <span class="gi-l-preview-bar-title">{"Panel · Gestión Inmobiliaria"}</span>
                    </div>
                    <div class="gi-l-preview-body">
                        <div class="gi-l-preview-tiles">
                            <div class="gi-l-preview-tile">
                                <span class="gi-l-preview-tile-value">{"24"}</span>
                                <span class="gi-l-preview-tile-label">{"Propiedades"}</span>
                            </div>
                            <div class="gi-l-preview-tile">
                                <span class="gi-l-preview-tile-value">{"21"}</span>
                                <span class="gi-l-preview-tile-label">{"Contratos activos"}</span>
                            </div>
                            <div class="gi-l-preview-tile">
                                <span class="gi-l-preview-tile-value gi-l-preview-tile-value--accent">{"RD$485,200.00"}</span>
                                <span class="gi-l-preview-tile-label">{"Ingresos del mes"}</span>
                            </div>
                            <div class="gi-l-preview-tile">
                                <span class="gi-l-preview-tile-value gi-l-preview-tile-value--warn">{"3"}</span>
                                <span class="gi-l-preview-tile-label">{"Pagos atrasados"}</span>
                            </div>
                        </div>
                        <div class="gi-l-preview-rows">
                            <div class="gi-l-preview-row-head">
                                <span>{"Inmueble"}</span>
                                <span>{"Estado"}</span>
                            </div>
                            <div class="gi-l-preview-row">
                                <span class="gi-l-preview-row-info">
                                    <span class="gi-l-preview-row-name">{"Apartamento Naco 3B"}</span>
                                    <span class="gi-l-preview-row-meta">{"Vence 05/06/2026 · RD$32,000.00"}</span>
                                </span>
                                <span class="gi-l-tag gi-l-tag--ok">{"Al día"}</span>
                            </div>
                            <div class="gi-l-preview-row">
                                <span class="gi-l-preview-row-info">
                                    <span class="gi-l-preview-row-name">{"Local Piantini"}</span>
                                    <span class="gi-l-preview-row-meta">{"Venció 01/06/2026 · RD$78,500.00"}</span>
                                </span>
                                <span class="gi-l-tag gi-l-tag--late">{"Atrasado"}</span>
                            </div>
                            <div class="gi-l-preview-row">
                                <span class="gi-l-preview-row-info">
                                    <span class="gi-l-preview-row-name">{"Villa Punta Cana"}</span>
                                    <span class="gi-l-preview-row-meta">{"Vence 10/06/2026 · US$1,250.00"}</span>
                                </span>
                                <span class="gi-l-tag gi-l-tag--pending">{"Pendiente"}</span>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="gi-l-preview-cta">
                    <Link<Route>
                        to={Route::Registro}
                        classes="gi-l-btn gi-l-btn-primary"
                    >
                        {"Crear cuenta gratis"}
                    </Link<Route>>
                </div>
            </div>
        </section>
    }
}
