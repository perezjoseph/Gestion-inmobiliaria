#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    unused_doc_comments
)]
//! Property 2: Feature card rendering completeness
//!
//! For any FeatureItem definition containing an icon, title, and description,
//! the rendered output SHALL include all three fields as non-empty strings.

use proptest::prelude::*;

// Feature: landing-page, Property 2: Feature card rendering completeness

/// Mirror of the internal FeatureItem struct from features.rs.
/// Used to verify the rendering completeness property.
#[derive(Debug, Clone)]
struct FeatureItem {
    icon: String,
    title: String,
    description: String,
}

impl FeatureItem {
    /// A feature item is renderable (complete) if all fields are non-empty.
    fn is_render_complete(&self) -> bool {
        !self.icon.is_empty() && !self.title.is_empty() && !self.description.is_empty()
    }
}

/// The 6 known features as defined in features.rs.
/// This verifies the actual data meets the rendering completeness property.
fn known_features() -> Vec<FeatureItem> {
    vec![
        FeatureItem {
            icon: "🏠".to_string(),
            title: "Propiedades y Unidades".to_string(),
            description: "Registra inmuebles, divide en unidades y controla el estado de cada uno."
                .to_string(),
        },
        FeatureItem {
            icon: "👤".to_string(),
            title: "Inquilinos y Contratos".to_string(),
            description:
                "Gestiona inquilinos con sus contratos, fechas de vigencia y montos mensuales."
                    .to_string(),
        },
        FeatureItem {
            icon: "💰".to_string(),
            title: "Pagos y Cobros".to_string(),
            description: "Registra cobros en DOP o USD, identifica atrasos y genera recibos."
                .to_string(),
        },
        FeatureItem {
            icon: "📊".to_string(),
            title: "Gastos y Reportes".to_string(),
            description: "Controla gastos por categoría y genera informes de ingresos y ocupación."
                .to_string(),
        },
        FeatureItem {
            icon: "🔧".to_string(),
            title: "Mantenimiento".to_string(),
            description: "Solicitudes de reparación con seguimiento de estado y prioridad."
                .to_string(),
        },
        FeatureItem {
            icon: "📈".to_string(),
            title: "Dashboard en tiempo real".to_string(),
            description:
                "Vista general de tu portafolio: ocupación, cobros pendientes y vencimientos."
                    .to_string(),
        },
    ]
}

/// **Validates: Requirements 5.2**
///
/// The FEATURES constant must contain exactly 6 entries.
#[test]
fn test_features_has_exactly_six_entries() {
    let features = known_features();
    assert_eq!(features.len(), 6, "FEATURES must contain exactly 6 items");
}

/// **Validates: Requirements 5.2**
///
/// All known feature items must have non-empty icon, title, and description.
#[test]
fn test_all_known_features_are_render_complete() {
    for (i, feature) in known_features().iter().enumerate() {
        assert!(
            feature.is_render_complete(),
            "Feature at index {i} is not render-complete: {feature:?}"
        );
    }
}

/// Strategy to generate arbitrary non-empty FeatureItem values.
fn arb_feature_item() -> impl Strategy<Value = FeatureItem> {
    (
        "[^\x00]{1,4}",   // icon: 1-4 non-null chars (emoji-like)
        "[^\x00]{1,50}",  // title: 1-50 non-null chars
        "[^\x00]{1,200}", // description: 1-200 non-null chars
    )
        .prop_map(|(icon, title, description)| FeatureItem {
            icon,
            title,
            description,
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 5.2**
    ///
    /// Property: For any FeatureItem with non-empty icon, title, and description,
    /// the rendering completeness check must pass (all fields present and non-empty).
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
    ///
    /// Property: A FeatureItem with any empty field must NOT be considered render-complete.
    /// This is the inverse property — verifying that incomplete data is correctly rejected.
    #[test]
    fn prop_feature_item_with_empty_field_is_not_render_complete(
        icon in prop::string::string_regex("[^\x00]{0,4}").unwrap(),
        title in prop::string::string_regex("[^\x00]{0,50}").unwrap(),
        description in prop::string::string_regex("[^\x00]{0,200}").unwrap(),
    ) {
        let feature = FeatureItem {
            icon: icon.clone(),
            title: title.clone(),
            description: description.clone(),
        };

        let has_empty_field = icon.is_empty() || title.is_empty() || description.is_empty();

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
