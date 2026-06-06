pub mod auditoria;
pub mod background_jobs;
pub mod chatbot;
#[cfg(test)]
mod chatbot_pbt;
pub mod condominios;
pub mod contrato;
pub mod dashboard;
pub mod desahucio;
pub mod dgii;
pub mod documento;
pub mod firma;
pub mod fiscal;
pub mod gasto;
pub mod gasto_recurrente;
pub mod importacion;
pub mod indexacion;
pub mod inquilino;
pub mod invitacion;
pub mod ipc;
pub mod ipi;
pub mod itbis;
pub mod mantenimiento;
pub mod mantenimiento_programado;
pub mod ncf;
pub mod notificacion;
pub mod ocr;
pub mod organizacion;
pub mod pago;
pub mod pago_generacion;
pub mod propiedad;
pub mod reporte;
pub mod reportes_dgii;
pub mod responsabilidad_servicio;
pub mod unidad;
pub mod usuario;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}
