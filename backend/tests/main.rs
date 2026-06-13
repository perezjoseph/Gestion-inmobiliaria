// Single test binary entry point — avoids linking 18 separate binaries.
// Each module is compiled and linked once instead of independently.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unreadable_literal,
    clippy::similar_names,
    clippy::float_cmp,
    clippy::needless_collect,
    clippy::ignore_without_reason,
    clippy::redundant_clone,
    clippy::redundant_closure_for_method_calls,
    clippy::manual_string_new,
    clippy::cast_lossless,
    clippy::incompatible_msrv,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::suspicious_open_options,
    clippy::literal_string_with_formatting_args,
    clippy::doc_markdown,
    clippy::ignored_unit_patterns,
    unsafe_code
)]

#[path = "../migrations/mod.rs"]
mod migrations;

mod common;

/// Global mutex shared by all integration test modules.
/// Each module's `with_db` acquires this lock so tests that share the
/// database never run concurrently — even across modules.
pub static GLOBAL_DB_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Returns the number of PBT cases to run.
/// Reads `PROPTEST_CASES` from the environment (set lower in CI for speed).
/// Falls back to 100 for local development.
pub fn pbt_cases() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100)
}

mod agent_loop_bug_pbt;
mod ai_module_pbt;
mod architecture_tests;
mod auditoria_tests;
mod auth_pbt;
mod auth_tests;
mod background_jobs_pbt;
mod background_jobs_tests;
mod chatbot_pbt;
mod chatbot_self_message_pbt;
mod contratos_tests;
mod deposit_tracking_pbt;
mod deposit_tracking_tests;
mod desahucios_tests;
mod dgii_tests;
mod documentos_pbt;
mod documentos_tests;
mod dr_legal_compliance_pbt;
mod firmas_pbt;
mod firmas_tests;
mod gastos_pbt;
mod gastos_tests;
mod importacion_pbt;
mod inquilinos_tests;
mod ipc_tests;
mod late_fees_pbt;
mod late_fees_tests;
mod mantenimiento_pbt;
mod mantenimiento_tests;
mod notificaciones_pbt;
mod notificaciones_tests;
mod ocr_pbt;
mod ocr_tests;
mod organizaciones_tests;
mod pago_generacion_pbt;
mod pago_generacion_tests;
mod pagos_tests;
mod perfil_tests;
mod preservation_pbt;
mod propiedades_tests;
mod recibos_pbt;
mod recibos_tests;
mod reportes_tests;
mod servicios_publicos_tests;
mod unidades_pbt;
mod unidades_tests;
mod usuarios_tests;

mod dr_compliance_integration_tests;
mod ncf_bug_condition_pbt;
mod ncf_preservation_pbt;
mod security_audit_pbt;
mod security_preservation_pbt;

#[cfg(feature = "evals")]
mod chatbot_evals_tests;
