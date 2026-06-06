#![allow(clippy::derive_partial_eq_without_eq)]

pub mod cache_dgii;
pub mod chatbot_config;
pub mod chatbot_conversation;
#[cfg(feature = "evals")]
pub mod chatbot_eval_run;
#[cfg(feature = "evals")]
pub mod chatbot_eval_suite;
pub mod chatbot_receipt_extraction;
#[allow(dead_code)]
pub mod configuracion;
pub mod configuracion_ipi;
pub mod contrato;
pub mod copropietario;
pub mod cuota_condominio;
pub mod desahucio;
#[allow(dead_code)]
pub mod documento;
pub mod ejecucion_tarea;
pub mod firma_documento;
pub mod gasto;
pub mod gasto_recurrente;
pub mod inquilino;
pub mod invitacion;
pub mod mantenimiento_programado;
pub mod nota_mantenimiento;
pub mod notificacion;
pub mod organizacion;
pub mod pago;
#[allow(dead_code)]
pub mod plantilla_documento;
pub mod prelude;
pub mod preview_index;
pub mod propiedad;
pub mod recibo_informal;
pub mod registro_auditoria;
pub mod reporte_dgii;
pub mod responsabilidad_servicio;
pub mod secuencia_ncf;
pub mod solicitud_mantenimiento;
pub mod unidad;
pub mod usuario;
