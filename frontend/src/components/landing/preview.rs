use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;

#[component]
pub fn LandingPreview() -> Html {
    html! {
        <section class="gi-l-preview">
            <div class="gi-l-container">
                <div class="gi-l-section-head">
                    <h2 class="gi-l-section-title">{"AsÃ se ve por dentro"}</h2>
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
                                <span class="gi-l-preview-tile-placeholder"></span>
                                <span class="gi-l-preview-tile-label">{"Propiedades"}</span>
                            </div>
                            <div class="gi-l-preview-tile">
                                <span class="gi-l-preview-tile-placeholder"></span>
                                <span class="gi-l-preview-tile-label">{"Contratos"}</span>
                            </div>
                            <div class="gi-l-preview-tile">
                                <span class="gi-l-preview-tile-placeholder gi-l-preview-tile-placeholder--accent"></span>
                                <span class="gi-l-preview-tile-label">{"Ingresos"}</span>
                            </div>
                            <div class="gi-l-preview-tile">
                                <span class="gi-l-preview-tile-placeholder gi-l-preview-tile-placeholder--warn"></span>
                                <span class="gi-l-preview-tile-label">{"Alertas"}</span>
                            </div>
                        </div>
                        <div class="gi-l-preview-rows">
                            <div class="gi-l-preview-row">
                                <span class="gi-l-preview-row-placeholder"></span>
                                <span class="gi-l-preview-badge gi-l-preview-badge--ok"></span>
                            </div>
                            <div class="gi-l-preview-row">
                                <span class="gi-l-preview-row-placeholder gi-l-preview-row-placeholder--short"></span>
                                <span class="gi-l-preview-badge gi-l-preview-badge--warn"></span>
                            </div>
                            <div class="gi-l-preview-row">
                                <span class="gi-l-preview-row-placeholder"></span>
                                <span class="gi-l-preview-badge gi-l-preview-badge--ok"></span>
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
