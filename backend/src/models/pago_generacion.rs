use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::pago::PagoResponse;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerarPagosRequest {
    pub dia_vencimiento: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPagosQuery {
    pub dia_vencimiento: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPagosResponse {
    pub contrato_id: Uuid,
    pub pagos: Vec<PagoPreview>,
    pub total_pagos: usize,
    pub monto_total: Decimal,
    pub pagos_existentes: usize,
    pub pagos_nuevos: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagoPreview {
    pub monto: Decimal,
    pub moneda: String,
    pub fecha_vencimiento: NaiveDate,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerarPagosResponse {
    pub contrato_id: Uuid,
    pub pagos_generados: usize,
    pub pagos: Vec<PagoResponse>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn generar_pagos_request_deserializes_camel_case() {
        let json = r#"{"diaVencimiento":15}"#;
        let req: GenerarPagosRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.dia_vencimiento, Some(15));
    }

    #[test]
    fn generar_pagos_request_deserializes_without_dia() {
        let json = r"{}";
        let req: GenerarPagosRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.dia_vencimiento, None);
    }

    #[test]
    fn preview_pagos_query_deserializes_with_dia() {
        let json = r#"{"diaVencimiento":28}"#;
        let q: PreviewPagosQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.dia_vencimiento, Some(28));
    }

    #[test]
    fn preview_pagos_query_deserializes_without_dia() {
        let json = r"{}";
        let q: PreviewPagosQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.dia_vencimiento, None);
    }

    #[test]
    fn preview_pagos_response_serializes_to_camel_case() {
        let resp = PreviewPagosResponse {
            contrato_id: Uuid::nil(),
            pagos: vec![PagoPreview {
                monto: Decimal::from_str("15000.00").unwrap(),
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            }],
            total_pagos: 1,
            monto_total: Decimal::from_str("15000.00").unwrap(),
            pagos_existentes: 0,
            pagos_nuevos: 1,
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.get("contratoId").is_some());
        assert!(json.get("totalPagos").is_some());
        assert!(json.get("montoTotal").is_some());
        assert!(json.get("pagosExistentes").is_some());
        assert!(json.get("pagosNuevos").is_some());
        assert!(json.get("contrato_id").is_none());
    }

    #[test]
    fn pago_preview_serializes_to_camel_case() {
        let preview = PagoPreview {
            monto: Decimal::from_str("25000.00").unwrap(),
            moneda: "USD".to_string(),
            fecha_vencimiento: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
        };

        let json = serde_json::to_value(&preview).unwrap();
        assert!(json.get("fechaVencimiento").is_some());
        assert!(json.get("fecha_vencimiento").is_none());
    }
}
