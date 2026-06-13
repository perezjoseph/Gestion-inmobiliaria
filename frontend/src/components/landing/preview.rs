use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;

#[component]
pub fn LandingPreview() -> Html {
    html! {
        <section class="gi-l-preview">
            <div class="gi-l-container">
                <div class="gi-l-section-head">
                    <h2 class="gi-l-section-title">{"Así se ve por dentro"}</h2>
                </div>
                <div class="gi-l-preview-frame" aria-hidden="true">
                    <div class="gi-l-preview-bar">
                        <span class="gi-l-preview-dot"></span>
                        <span class="gi-l-preview-dot"></span>
                        <span class="gi-l-preview-dot"></span>
                    </div>
                    <div class="gi-l-preview-body">
                        <div class="gi-l-preview-tiles">
                            <div class="gi-l-preview-tile">
                                <span class="gi-l-preview-tile-value">{"24"}</span>
                                <span class="gi-l-preview-tile-label">{"Propiedades"}</span>
                            </div>
                            <div class="gi-l-preview-tile">
                                <span class="gi-l-preview-tile-value">{"18"}</span>
                                <span class="gi-l-preview-tile-label">{"Contratos activos"}</span>
                            </div>
                            <div class="gi-l-preview-tile">
                                <span class="gi-l-preview-tile-value gi-l-preview-tile-value--accent">{"RD$ 410K"}</span>
                                <span class="gi-l-preview-tile-label">{"Cobrado este mes"}</span>
                            </div>
                            <div class="gi-l-preview-tile">
                                <span class="gi-l-preview-tile-value gi-l-preview-tile-value--warn">{"3"}</span>
                                <span class="gi-l-preview-tile-label">{"Pagos atrasados"}</span>
                            </div>
                        </div>
                        <div class="gi-l-preview-rows">
                            <div class="gi-l-preview-row">
                                <span>{"Apartamento Naco 2B"}</span>
                                <span class="gi-l-preview-badge gi-l-preview-badge--ok">{"Pagado"}</span>
                            </div>
                            <div class="gi-l-preview-row">
                                <span>{"Local Piantini 04"}</span>
                                <span class="gi-l-preview-badge gi-l-preview-badge--warn">{"Pendiente"}</span>
                            </div>
                            <div class="gi-l-preview-row">
                                <span>{"Casa Los Prados"}</span>
                                <span class="gi-l-preview-badge gi-l-preview-badge--ok">{"Pagado"}</span>
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
