#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    unused_doc_comments
)]
//! Property 2: Preservation — Existing Routes and Sidebar Links Unchanged
//!
//! These tests observe and capture the current routing behavior on UNFIXED code.
//! All existing route paths must continue resolving to their expected variants,
//! and unrecognized paths must continue returning `None` (`NotFound` behavior).

use proptest::prelude::*;
use realestate_frontend::app::Route;
use yew_router::Routable;

/// Helper: returns all existing route paths paired with their expected variant.
fn existing_routes() -> Vec<(&'static str, Route)> {
    vec![
        ("/", Route::Landing),
        ("/login", Route::Login),
        ("/dashboard", Route::Dashboard),
        ("/propiedades", Route::Propiedades),
        ("/inquilinos", Route::Inquilinos),
        ("/contratos", Route::Contratos),
        ("/pagos", Route::Pagos),
        ("/gastos", Route::Gastos),
        ("/categorias-gastos", Route::CategoriasGastos),
        ("/registro", Route::Registro),
        ("/reportes", Route::Reportes),
        ("/usuarios", Route::UsuariosPage),
        ("/perfil", Route::Perfil),
        ("/auditoria", Route::AuditoriaPage),
        ("/importar", Route::Importar),
        ("/indexacion", Route::Indexacion),
        ("/mantenimiento", Route::Mantenimiento),
        ("/notificaciones", Route::Notificaciones),
        ("/configuracion", Route::Configuracion),
        ("/configuracion/chatbot", Route::ConfiguracionChatbot),
        ("/configuracion/fiscal", Route::ConfiguracionFiscal),
        ("/recibos-informales", Route::RecibosInformales),
        ("/dashboard/comparativo", Route::DashboardComparativo),
        ("/reportes-dgii", Route::ReportesDgii),
        ("/ipi", Route::Ipi),
        ("/plantillas", Route::Plantillas),
        ("/calendario", Route::Calendario),
        ("/documentos/por-vencer", Route::DocumentosPorVencer),
    ]
}

// Feature: missing-frontend-wiring, Property 2: Preservation

/// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9**
///
/// Verify all existing route paths continue to resolve to their expected variants.
#[test]
fn test_existing_routes_resolve_correctly() {
    for (path, expected) in existing_routes() {
        let result = Route::recognize(path);
        assert_eq!(
            result,
            Some(expected.clone()),
            "Route::recognize({path:?}) should return Some({expected:?})"
        );
    }
}

/// **Validates: Requirements 3.1**
#[test]
fn test_dashboard_route_preserved() {
    assert_eq!(Route::recognize("/dashboard"), Some(Route::Dashboard));
}

/// **Validates: Requirements 3.2**
#[test]
fn test_propiedades_route_preserved() {
    assert_eq!(Route::recognize("/propiedades"), Some(Route::Propiedades));
}

/// **Validates: Requirements 3.3**
#[test]
fn test_inquilinos_route_preserved() {
    assert_eq!(Route::recognize("/inquilinos"), Some(Route::Inquilinos));
}

/// **Validates: Requirements 3.4**
#[test]
fn test_contratos_route_preserved() {
    assert_eq!(Route::recognize("/contratos"), Some(Route::Contratos));
}

/// **Validates: Requirements 3.5**
#[test]
fn test_pagos_route_preserved() {
    assert_eq!(Route::recognize("/pagos"), Some(Route::Pagos));
}

/// **Validates: Requirements 3.6**
#[test]
fn test_gastos_route_preserved() {
    assert_eq!(Route::recognize("/gastos"), Some(Route::Gastos));
}

/// **Validates: Requirements 3.7**
#[test]
fn test_mantenimiento_route_preserved() {
    assert_eq!(
        Route::recognize("/mantenimiento"),
        Some(Route::Mantenimiento)
    );
}

/// **Validates: Requirements 3.8**
#[test]
fn test_configuracion_route_preserved() {
    assert_eq!(
        Route::recognize("/configuracion"),
        Some(Route::Configuracion)
    );
}

// Feature: missing-frontend-wiring, Property 2: Preservation (NotFound behavior)

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    // Validates: Requirements 3.11, 3.12
    // Property-based test: arbitrary unregistered paths must resolve to Route::NotFound
    // (the #[not_found] variant catches all unmatched paths).
    #[test]
    fn prop_unregistered_paths_return_not_found(
        suffix in "[a-z]{4,12}"
    ) {
        // Build a path that does not match any registered route
        let path = format!("/test-unknown-{suffix}");
        let result = Route::recognize(&path);
        prop_assert_eq!(
            result, Some(Route::NotFound),
            "Unregistered path {} should resolve to Some(Route::NotFound)",
            path
        );
    }
}

/// **Validates: Requirements 3.11**
///
/// Specific example: `/foobar` must resolve to Route::NotFound.
#[test]
fn test_foobar_returns_not_found() {
    assert_eq!(Route::recognize("/foobar"), Some(Route::NotFound));
}
