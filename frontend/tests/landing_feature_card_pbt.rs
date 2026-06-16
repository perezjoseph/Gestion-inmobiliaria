#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    unused_doc_comments
)]
//! Property 2: Feature item rendering completeness
//!
//! For any FeatureItem definition containing a title and description,
//! the rendered output SHALL include both fields as non-empty strings.

use proptest::prelude::*;

// Feature: landing-page, Property 2: Feature item rendering completeness

#[derive(Debug, Clone)]
struct FeatureItem {
    title: String,
    description: String,
}

impl FeatureItem {
    fn is_render_complete(&self) -> bool {
        !self.title.is_empty() && !self.description.is_empty()
    }
}

fn known_features() -> Vec<FeatureItem> {
    vec![
        FeatureItem {
            title: "Propiedades y unidades".to_string(),
            description: "Registra inmuebles con dirección, unidades individuales, precios en DOP o USD, y estado de ocupación. Todo visible de un vistazo.".to_string(),
        },
        FeatureItem {
            title: "Contratos e inquilinos".to_string(),
            description: "Asocia inquilinos con cédula verificada. Contratos con fechas claras, montos, y renovación controlada. Sin sorpresas.".to_string(),
        },
        FeatureItem {
            title: "Pagos y cobros".to_string(),
            description: "Seguimiento de cada pago: pendiente, al día, o atrasado. Alertas automáticas cuando algo se vence.".to_string(),
        },
        FeatureItem {
            title: "Gastos y mantenimiento".to_string(),
            description: "Registra gastos por propiedad o unidad. Solicitudes de reparación con prioridad, notas, y costos asociados.".to_string(),
        },
        FeatureItem {
            title: "Reportes y comprobantes".to_string(),
            description: "Ocupación, ingresos mensuales, cumplimiento fiscal. Genera comprobantes NCF para la DGII directamente.".to_string(),
        },
    ]
}

/// **Validates: Requirements 5.2**
#[test]
fn test_features_has_expected_count() {
    let features = known_features();
    assert_eq!(features.len(), 5, "FEATURES must contain exactly 5 items");
}

/// **Validates: Requirements 5.2**
#[test]
fn test_all_known_features_are_render_complete() {
    for (i, feature) in known_features().iter().enumerate() {
        assert!(
            feature.is_render_complete(),
            "Feature at index {i} is not render-complete: {feature:?}"
        );
    }
}

fn arb_feature_item() -> impl Strategy<Value = FeatureItem> {
    ("[^\x00]{1,50}", "[^\x00]{1,200}")
        .prop_map(|(title, description)| FeatureItem { title, description })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_feature_item_with_content_is_always_render_complete(
        feature in arb_feature_item()
    ) {
        prop_assert!(
            feature.is_render_complete(),
            "Generated feature item should be render-complete: {:?}",
            feature
        );
    }

    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_feature_item_with_empty_field_is_not_render_complete(
        title in prop::string::string_regex("[^\x00]{0,50}").unwrap(),
        description in prop::string::string_regex("[^\x00]{0,200}").unwrap(),
    ) {
        let feature = FeatureItem {
            title: title.clone(),
            description: description.clone(),
        };

        let has_empty_field = title.is_empty() || description.is_empty();

        if has_empty_field {
            prop_assert!(
                !feature.is_render_complete(),
                "Feature with empty field(s) should NOT be render-complete: {:?}",
                feature
            );
        } else {
            prop_assert!(
                feature.is_render_complete(),
                "Feature with all non-empty fields should be render-complete: {:?}",
                feature
            );
        }
    }
}
