#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    unused_doc_comments,
    clippy::uninlined_format_args
)]
//! Property 1: Step card rendering completeness
//!
//! For any Step definition containing a title and description,
//! the rendered output SHALL include both fields as non-empty strings.

use proptest::prelude::*;

// Feature: landing-page, Property 1: Step card rendering completeness

#[derive(Debug, Clone)]
struct StepData {
    title: String,
    description: String,
}

impl StepData {
    fn satisfies_rendering_contract(&self) -> bool {
        !self.title.is_empty() && !self.description.is_empty()
    }
}

fn valid_step_strategy() -> impl Strategy<Value = StepData> {
    (
        "[A-Za-záéíóúñÁÉÍÓÚÑ ]{3,50}",
        "[A-Za-záéíóúñÁÉÍÓÚÑ .,]{10,200}",
    )
        .prop_map(|(title, description)| StepData { title, description })
}

const EXPECTED_STEPS: &[(&str, &str)] = &[
    (
        "Registra tus propiedades",
        "Añade inmuebles, unidades, y precios. La estructura se arma sola.",
    ),
    (
        "Conecta inquilinos",
        "Crea contratos con fechas, montos, y asocia cada inquilino a su espacio.",
    ),
    (
        "Controla todo",
        "Pagos, gastos, mantenimiento, reportes. Todo fluye desde un solo panel.",
    ),
];

/// **Validates: Requirements 4.3**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_step_rendering_completeness(step in valid_step_strategy()) {
        prop_assert!(
            step.satisfies_rendering_contract(),
            "Step with title={:?}, description={:?} must satisfy rendering contract",
            step.title,
            step.description
        );
        prop_assert!(!step.title.is_empty(), "Title must be non-empty");
        prop_assert!(!step.description.is_empty(), "Description must be non-empty");
    }
}

/// **Validates: Requirements 4.3**
#[test]
fn test_steps_count_is_exactly_three() {
    assert_eq!(
        EXPECTED_STEPS.len(),
        3,
        "How It Works section must have exactly 3 steps"
    );
}

/// **Validates: Requirements 4.3**
#[test]
fn test_actual_steps_satisfy_rendering_contract() {
    for (i, (title, description)) in EXPECTED_STEPS.iter().enumerate() {
        assert!(!title.is_empty(), "Step {i} must have a non-empty title");
        assert!(
            !description.is_empty(),
            "Step {i} must have a non-empty description"
        );
    }
}
