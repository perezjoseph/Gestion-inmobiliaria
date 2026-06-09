use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;

/// A breadcrumb segment.
#[derive(Clone, PartialEq, Eq)]
pub struct Crumb {
    pub label: AttrValue,
    pub route: Option<Route>,
}

/// Derives breadcrumbs from the current route.
/// Returns an empty vec for top-level routes (no breadcrumb needed).
fn breadcrumbs_for_route(route: &Route) -> Vec<Crumb> {
    match route {
        // Nested under Propiedades
        Route::Condominios { .. } => vec![
            Crumb {
                label: "Propiedades".into(),
                route: Some(Route::Propiedades),
            },
            Crumb {
                label: "Condominios".into(),
                route: None,
            },
        ],

        // Document editor nested routes
        Route::DocumentoEditor { .. } | Route::DocumentoEditorExisting { .. } => vec![
            Crumb {
                label: "Documentos".into(),
                route: Some(Route::DocumentosPorVencer),
            },
            Crumb {
                label: "Editor".into(),
                route: None,
            },
        ],

        // Dashboard comparativo
        Route::DashboardComparativo => vec![
            Crumb {
                label: "Panel de Control".into(),
                route: Some(Route::Dashboard),
            },
            Crumb {
                label: "Comparativo".into(),
                route: None,
            },
        ],

        // Configuración sub-routes
        Route::ConfiguracionChatbot => vec![
            Crumb {
                label: "Configuración".into(),
                route: Some(Route::Configuracion),
            },
            Crumb {
                label: "Chatbot".into(),
                route: None,
            },
        ],
        Route::ConfiguracionFiscal => vec![
            Crumb {
                label: "Configuración".into(),
                route: Some(Route::Configuracion),
            },
            Crumb {
                label: "Fiscal".into(),
                route: None,
            },
        ],

        // Categorías de gastos
        Route::CategoriasGastos => vec![
            Crumb {
                label: "Gastos".into(),
                route: Some(Route::Gastos),
            },
            Crumb {
                label: "Categorías".into(),
                route: None,
            },
        ],

        // Reportes DGII
        Route::ReportesDgii => vec![
            Crumb {
                label: "Reportes".into(),
                route: Some(Route::Reportes),
            },
            Crumb {
                label: "DGII".into(),
                route: None,
            },
        ],

        // Documentos por vencer (under a logical "Documentos" parent)
        Route::DocumentosPorVencer => vec![Crumb {
            label: "Documentos por Vencer".into(),
            route: None,
        }],

        // Top-level routes: no breadcrumb
        _ => vec![],
    }
}

#[component]
pub fn Breadcrumb() -> Html {
    let route = use_route::<Route>();
    let Some(current) = route else {
        return html! {};
    };

    let crumbs = breadcrumbs_for_route(&current);
    if crumbs.is_empty() {
        return html! {};
    }

    html! {
        <nav class="gi-breadcrumb" aria-label="Navegación de migas de pan">
            <ol class="gi-breadcrumb-list">
                { for crumbs.iter().enumerate().map(|(i, crumb)| {
                    let is_last = i == crumbs.len() - 1;
                    if is_last {
                        html! {
                            <li class="gi-breadcrumb-item gi-breadcrumb-current" aria-current="page">
                                {&crumb.label}
                            </li>
                        }
                    } else if let Some(ref route) = crumb.route {
                        html! {
                            <li class="gi-breadcrumb-item">
                                <Link<Route> to={route.clone()} classes="gi-breadcrumb-link">
                                    {&crumb.label}
                                </Link<Route>>
                                <span class="gi-breadcrumb-sep" aria-hidden="true">{"/"}</span>
                            </li>
                        }
                    } else {
                        html! {
                            <li class="gi-breadcrumb-item">
                                <span>{&crumb.label}</span>
                                <span class="gi-breadcrumb-sep" aria-hidden="true">{"/"}</span>
                            </li>
                        }
                    }
                })}
            </ol>
        </nav>
    }
}
