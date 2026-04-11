pub mod auditoria;
pub mod contrato;
pub mod dashboard;
pub mod documento;
pub mod gasto;
pub mod importacion;
pub mod inquilino;
pub mod mantenimiento;
pub mod notificacion;
pub mod pago;
pub mod propiedad;
pub mod reporte;
pub mod usuario;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}
