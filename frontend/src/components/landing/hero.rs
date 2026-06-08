use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;

#[component]
pub fn LandingHero() -> Html {
    html! {
        <section class="px-4 py-16 md:py-24 text-center max-w-4xl mx-auto">
            <h1
                class="text-3xl md:text-5xl font-bold mb-4"
                style="font-family: var(--font-display); color: var(--text-primary);"
            >
                {"Gestiona tus propiedades en República Dominicana — simple y gratis"}
            </h1>
            <p
                class="text-lg md:text-xl mb-8 max-w-2xl mx-auto"
                style="font-family: var(--font-body); color: var(--text-secondary);"
            >
                {"Una herramienta hecha para administradores que quieren tener todo en orden: propiedades, inquilinos, contratos, pagos y más — sin complicaciones."}
            </p>
            <div class="flex flex-col sm:flex-row gap-3 justify-center">
                <Link<Route>
                    to={Route::Registro}
                    classes="w-full sm:w-auto px-8 py-3.5 rounded-lg font-bold text-white text-center text-lg"
                    style="background-color: #3d8b8b;"
                >
                    {"Registrarse gratis"}
                </Link<Route>>
                <Link<Route>
                    to={Route::Login}
                    classes="w-full sm:w-auto px-6 py-3 rounded-lg font-semibold text-center"
                    style="border: 1px solid var(--border-default); color: var(--text-primary);"
                >
                    {"Ya tengo cuenta"}
                </Link<Route>>
            </div>
        </section>
    }
}
