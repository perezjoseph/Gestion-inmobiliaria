pub mod auditoria;
pub mod background_jobs;
pub mod contrato;
pub mod dashboard;
pub mod documento;
pub mod gasto;
pub mod importacion;
pub mod inquilino;
pub mod invitacion;
pub mod mantenimiento;
pub mod notificacion;
pub mod ocr;
pub mod organizacion;
pub mod pago;
pub mod pago_generacion;
pub mod propiedad;
pub mod reporte;
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
