use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItbisResult {
    pub monto_base: Decimal,
    pub monto_itbis: Decimal,
    pub monto_total: Decimal,
    pub tasa: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetencionResult {
    pub monto_retenido: Decimal,
    pub monto_neto: Decimal,
}
