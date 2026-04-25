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
    unsafe_code
)]

#[path = "../migrations/mod.rs"]
mod migrations;

mod auditoria_tests;
mod auth_tests;
mod contratos_tests;
mod documentos_tests;
mod gastos_pbt;
mod gastos_tests;
mod inquilinos_tests;
mod mantenimiento_pbt;
mod mantenimiento_tests;
mod notificaciones_tests;
mod ocr_pbt;
mod ocr_tests;
mod pagos_tests;
mod perfil_tests;
mod propiedades_tests;
mod recibos_tests;
mod reportes_tests;
mod usuarios_tests;
