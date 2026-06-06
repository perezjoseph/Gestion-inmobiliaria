use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Resultado del cálculo de ITBIS sobre un monto base.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItbisResult {
    pub monto_base: Decimal,
    pub monto_itbis: Decimal,
    pub monto_total: Decimal,
    pub tasa: Decimal,
}

/// Resultado del cálculo de retención de ITBIS (30% retenido por inquilino persona jurídica).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetencionResult {
    pub monto_retenido: Decimal,
    pub monto_neto: Decimal,
}
