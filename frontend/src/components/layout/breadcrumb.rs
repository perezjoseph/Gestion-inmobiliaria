use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;

#[derive(Clone, PartialEq, Eq)]
pub struct Crumb {
    pub label: AttrValue,
    pub route: Option<Route>,
}

fn breadcrumbs_for_route(route: &Route) -> Vec<Crumb> {
    match route {
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

        Route::ConfiguracionChatbot => vec![
            Crumb {
                label: "ConfiguraciÃ³n".into(),
                route: Some(Route::Configuracion),
            },
            Crumb {
                label: "Chatbot".into(),
                route: None,
            },
        ],
        Route::ConfiguracionFiscal => vec![
            Crumb {
                label: "ConfiguraciÃ³n".into(),
                route: Some(Route::Configuracion),
            },
            Crumb {
                label: "Fiscal".into(),
                route: None,
            },
        ],

        Route::CategoriasGastos => vec![
            Crumb {
                label: "Gastos".into(),
                route: Some(Route::Gastos),
            },
            Crumb {
                label: "CategorÃ­as".into(),
                route: None,
            },
        ],

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

        Route::DocumentosPorVencer => vec![Crumb {
            label: "Documentos por Vencer".into(),
            route: None,
        }],

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
        <nav class="gi-breadcrumb" aria-label="NavegaciÃ³n de migas de pan">
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
