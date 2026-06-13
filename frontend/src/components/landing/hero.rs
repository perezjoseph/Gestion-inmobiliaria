use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;

#[component]
pub fn LandingHero() -> Html {
    html! {
        <section class="gi-l-hero">
            <div class="gi-l-container gi-l-hero-grid">
                <div class="gi-l-hero-content">
                    <p class="gi-l-eyebrow"><span class="gi-l-dot gi-l-dot--accent"></span>{"Simple y gratis"}</p>
                    <h1 class="gi-l-hero-title">
                        {"Gestiona tus propiedades en "}
                        <span class="gi-l-hero-title-accent">{"República Dominicana"}</span>
                        {"."}
                    </h1>
                    <p class="gi-l-hero-lead">
                        {"Una herramienta para administradores que quieren todo en orden: propiedades, inquilinos, contratos, pagos y más, sin complicaciones."}
                    </p>
                    <div class="gi-l-cta-row">
                        <Link<Route>
                            to={Route::Registro}
                            classes="gi-l-btn gi-l-btn-primary"
                        >
                            {"Registrarse gratis"}
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
                    <div class="gi-l-hero-panel">
                        <div class="gi-l-mini-stat">
                            <span class="gi-l-mini-stat-placeholder gi-l-mini-stat-placeholder--wide"></span>
                            <span class="gi-l-mini-stat-label">{"Propiedades"}</span>
                        </div>
                        <div class="gi-l-mini-stat">
                            <span class="gi-l-mini-stat-placeholder"></span>
                            <span class="gi-l-mini-stat-label">{"Ocupación"}</span>
                        </div>
                        <div class="gi-l-mini-stat">
                            <span class="gi-l-mini-stat-placeholder gi-l-mini-stat-placeholder--accent"></span>
                            <span class="gi-l-mini-stat-label">{"Ingresos"}</span>
                        </div>
                        <div class="gi-l-mini-stat">
                            <span class="gi-l-mini-stat-placeholder gi-l-mini-stat-placeholder--short"></span>
                            <span class="gi-l-mini-stat-label">{"Alertas"}</span>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    }
}
