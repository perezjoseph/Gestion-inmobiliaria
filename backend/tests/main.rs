// Single test binary entry point — avoids linking 18 separate binaries.
// Each module is compiled and linked once instead of independently.

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
