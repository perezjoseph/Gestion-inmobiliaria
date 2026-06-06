pub mod ai_module;
#[cfg(test)]
mod ai_module_pbt;
pub mod auditoria;
pub mod auth;
pub mod background_jobs;
pub mod baileys_client;
pub mod chatbot;
#[cfg(feature = "evals")]
pub mod chatbot_evals;
#[cfg(test)]
mod chatbot_pbt;
pub mod configuracion;
pub mod contratos;
pub mod crypto;
pub mod dashboard;
pub mod desahucios;
pub mod dgii;
pub mod documento_editor;
#[cfg(test)]
mod documento_editor_pbt;
pub mod documentos;
pub mod firmas;
#[cfg(test)]
mod firmas_pbt;
pub mod gastos;
pub mod gastos_recurrentes;
pub mod importacion;
pub mod inquilinos;
pub mod invitaciones;
pub mod ipc;
pub mod mail;
pub mod mantenimiento;
pub mod mantenimiento_programado;
pub mod notificaciones;
pub mod ocr_client;
pub mod ocr_mapping;
#[cfg(test)]
mod ocr_mapping_pbt;
#[cfg(test)]
mod ocr_mapping_tests;
pub mod ocr_preview;
pub mod organizaciones;
pub mod ovms_provider;
pub mod pago_generacion;
pub mod pagos;
pub mod perfil;
pub mod plantillas;
#[cfg(test)]
mod plantillas_pbt;
pub mod propiedades;
pub mod recargos;
pub mod recibos;
pub mod reportes;
pub mod servicios_publicos;
pub mod unidades;
pub mod usuarios;
pub mod validacion_fiscal;
pub mod validation;
