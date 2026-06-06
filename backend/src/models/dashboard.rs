use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// --- Dashboard Comparativo DTOs ---

/// Per-property comparison row in the comparative dashboard.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyComparison {
    pub propiedad_id: Uuid,
    pub titulo: String,
    pub tipo_propiedad: String,
    pub moneda: String,
    /// Total income within the date range (None when no date range applied)
    pub ingresos: Option<Decimal>,
    /// Total expenses within the date range (None when no date range applied)
    pub gastos: Option<Decimal>,
    /// Total condominium fees within the date range (None when no date range applied)
    pub cuotas_condominio: Option<Decimal>,
    /// Net profitability percentage (None when no date range applied)
    pub rentabilidad_neta: Option<Decimal>,
    /// Occupancy rate percentage (None when no date range applied)
    pub tasa_ocupacion: Option<f64>,
    /// Delinquency percentage (None when no date range applied)
    pub morosidad_pct: Option<Decimal>,
    /// Total ITBIS collected (None if org is informal or property is not commercial)
    pub itbis_total: Option<Decimal>,
    /// Cadastral value for IPI visibility
    pub valor_catastral: Option<Decimal>,
    /// Whether rentabilidad calculation is unreliable (`valor_catastral` below 100,000 DOP)
    pub rentabilidad_unreliable: bool,
}

/// Response for the comparative dashboard endpoint.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardComparativoResponse {
    pub propiedades: Vec<PropertyComparison>,
    pub moneda_display: String,
}

/// Query parameters for the comparative dashboard.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardComparativoQuery {
    pub fecha_desde: Option<NaiveDate>,
    pub fecha_hasta: Option<NaiveDate>,
    pub tipo_propiedad: Option<String>,
    pub moneda_display: Option<String>,
}

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
    pub total_pendiente: Decimal,
    pub gastos_vencidos: u64,
}
