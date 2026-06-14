use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IpiLiabilityResponse {
    pub valor_total: Decimal,
    pub umbral: Decimal,
    pub exceso: Decimal,
    pub ipi_anual: Decimal,
    pub pago_semestral: Decimal,
    pub proxima_fecha: NaiveDate,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CopropietarioResponse {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub nombre: String,
    pub cedula_rnc: String,
    pub porcentaje_propiedad: Decimal,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguracionIpiRequest {
    pub umbral_ipi: Decimal,
    pub anio: i32,
    pub fecha_pago_1: NaiveDate,
    pub fecha_pago_2: NaiveDate,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn ipi_liability_response_serializes_camel_case() {
        let response = IpiLiabilityResponse {
            valor_total: Decimal::new(1_500_000_000, 2),
            umbral: Decimal::new(1_069_549_400, 2),
            exceso: Decimal::new(430_450_600, 2),
            ipi_anual: Decimal::new(4_304_506, 2),
            pago_semestral: Decimal::new(2_152_253, 2),
            proxima_fecha: NaiveDate::from_ymd_opt(2026, 9, 11).unwrap(),
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json.get("valorTotal").is_some());
        assert!(json.get("umbral").is_some());
        assert!(json.get("exceso").is_some());
        assert!(json.get("ipiAnual").is_some());
        assert!(json.get("pagoSemestral").is_some());
        assert!(json.get("proximaFecha").is_some());
    }

    #[test]
    fn copropietario_response_serializes_camel_case() {
        let response = CopropietarioResponse {
            id: Uuid::new_v4(),
            propiedad_id: Uuid::new_v4(),
            nombre: "Juan Pérez".to_string(),
            cedula_rnc: "00112345678".to_string(),
            porcentaje_propiedad: Decimal::new(50_00, 2),
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json.get("id").is_some());
        assert!(json.get("propiedadId").is_some());
        assert!(json.get("nombre").is_some());
        assert!(json.get("cedulaRnc").is_some());
        assert!(json.get("porcentajePropiedad").is_some());
    }

    #[test]
    fn configuracion_ipi_request_deserializes_from_camel_case() {
        let json = serde_json::json!({
            "umbralIpi": "10695494.00",
            "anio": 2026,
            "fechaPago1": "2026-03-11",
            "fechaPago2": "2026-09-11"
        });
        let request: ConfiguracionIpiRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.umbral_ipi, Decimal::new(1_069_549_400, 2));
        assert_eq!(request.anio, 2026);
        assert_eq!(
            request.fecha_pago_1,
            NaiveDate::from_ymd_opt(2026, 3, 11).unwrap()
        );
        assert_eq!(
            request.fecha_pago_2,
            NaiveDate::from_ymd_opt(2026, 9, 11).unwrap()
        );
    }
}
