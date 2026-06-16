use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;

#[component]
pub fn LandingHero() -> Html {
    html! {
        <section class="gi-l-hero">
            <div class="gi-l-container gi-l-hero-grid">
                <div class="gi-l-hero-content">
                    <h1 class="gi-l-hero-title">
                        {"Tus propiedades,"}
                        <br />
                        <span class="gi-l-hero-title-accent">{"bajo control."}</span>
                    </h1>
                    <p class="gi-l-hero-lead">
                        {"Contratos, pagos, inquilinos, mantenimiento — todo en un solo lugar. Hecho para administradores en República Dominicana."}
                    </p>
                    <div class="gi-l-cta-row">
                        <Link<Route>
                            to={Route::Registro}
                            classes="gi-l-btn gi-l-btn-primary"
                        >
                            {"Empezar gratis"}
                        </Link<Route>>
                        <Link<Route>
                            to={Route::Login}
                            classes="gi-l-btn gi-l-btn-secondary"
                        >
                            {"Ya tengo cuenta"}
                        </Link<Route>>
                    </div>
                </div>
                <div class="gi-l-hero-visual" aria-hidden="true">
                    <div class="gi-l-hero-card">
                        <div class="gi-l-hero-card-header">
                            <span class="gi-l-hero-card-type">{"Contrato"}</span>
                            <span class="gi-l-tag gi-l-tag--ok">{"Activo"}</span>
                        </div>
                        <div class="gi-l-hero-card-body">
                            <p class="gi-l-hero-card-property">{"Apartamento Naco 3B"}</p>
                            <p class="gi-l-hero-card-tenant">{"María Fernández · Cédula 001-1234567-8"}</p>
                        </div>
                        <div class="gi-l-hero-card-details">
                            <div class="gi-l-hero-card-detail">
                                <span class="gi-l-hero-card-detail-label">{"Renta mensual"}</span>
                                <span class="gi-l-hero-card-detail-value">{"RD$32,000.00"}</span>
                            </div>
                            <div class="gi-l-hero-card-detail">
                                <span class="gi-l-hero-card-detail-label">{"Vencimiento"}</span>
                                <span class="gi-l-hero-card-detail-value">{"05/07/2026"}</span>
                            </div>
                            <div class="gi-l-hero-card-detail">
                                <span class="gi-l-hero-card-detail-label">{"Próximo pago"}</span>
                                <span class="gi-l-hero-card-detail-value gi-l-hero-card-detail-value--ok">{"Al día"}</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    }
}
