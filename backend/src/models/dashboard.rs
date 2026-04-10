use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcupacionMensual {
    pub mes: u32,
    pub anio: i32,
    pub tasa: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngresoComparacion {
    pub esperado: Decimal,
    pub cobrado: Decimal,
    pub diferencia: Decimal,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagoProximo {
    pub pago_id: Uuid,
    pub propiedad_titulo: String,
    pub inquilino_nombre: String,
    pub monto: Decimal,
    pub moneda: String,
    pub fecha_vencimiento: NaiveDate,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContratoCalendario {
    pub contrato_id: Uuid,
    pub propiedad_titulo: String,
    pub inquilino_nombre: String,
    pub fecha_fin: NaiveDate,
    pub dias_restantes: i64,
    pub color: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcupacionTendenciaQuery {
    pub meses: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagosProximosQuery {
    pub dias: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GastosComparacion {
    pub mes_actual: Decimal,
    pub mes_anterior: Decimal,
    pub porcentaje_cambio: f64,
}
