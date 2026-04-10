use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContratoListQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateContratoRequest {
    pub propiedad_id: Uuid,
    pub inquilino_id: Uuid,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin: NaiveDate,
    pub monto_mensual: Decimal,
    pub deposito: Option<Decimal>,
    pub moneda: Option<String>,
    pub recargo_porcentaje: Option<Decimal>,
    pub dias_gracia: Option<i32>,
    pub dia_vencimiento: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenovarContratoRequest {
    pub fecha_fin: NaiveDate,
    pub monto_mensual: Decimal,
    pub dia_vencimiento: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminarContratoRequest {
    pub fecha_terminacion: NaiveDate,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PorVencerQuery {
    pub dias: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateContratoRequest {
    pub fecha_fin: Option<NaiveDate>,
    pub monto_mensual: Option<Decimal>,
    pub deposito: Option<Decimal>,
    pub estado: Option<String>,
    pub recargo_porcentaje: Option<Decimal>,
    pub dias_gracia: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContratoResponse {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub inquilino_id: Uuid,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin: NaiveDate,
    pub monto_mensual: Decimal,
    pub deposito: Option<Decimal>,
    pub moneda: String,
    pub estado: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagos_generados: Option<usize>,
    pub estado_deposito: Option<String>,
    pub fecha_cobro_deposito: Option<DateTime<Utc>>,
    pub fecha_devolucion_deposito: Option<DateTime<Utc>>,
    pub monto_retenido: Option<Decimal>,
    pub motivo_retencion: Option<String>,
    pub recargo_porcentaje: Option<Decimal>,
    pub dias_gracia: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CambiarEstadoDepositoRequest {
    pub estado: String,
    pub monto_retenido: Option<Decimal>,
    pub motivo_retencion: Option<String>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn renovar_contrato_request_deserializes_camel_case() {
        let json = r#"{"fechaFin":"2026-12-31","montoMensual":"25000.00"}"#;
        let req: RenovarContratoRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req.fecha_fin,
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()
        );
        assert_eq!(req.monto_mensual, Decimal::from_str("25000.00").unwrap());
    }

    #[test]
    fn terminar_contrato_request_deserializes_camel_case() {
        let json = r#"{"fechaTerminacion":"2025-06-15"}"#;
        let req: TerminarContratoRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req.fecha_terminacion,
            NaiveDate::from_ymd_opt(2025, 6, 15).unwrap()
        );
    }

    #[test]
    fn por_vencer_query_deserializes_with_dias() {
        let json = r#"{"dias":30}"#;
        let q: PorVencerQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.dias, Some(30));
    }

    #[test]
    fn por_vencer_query_deserializes_without_dias() {
        let json = r"{}";
        let q: PorVencerQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.dias, None);
    }

    #[test]
    fn contrato_response_serializes_to_camel_case() {
        let id = Uuid::nil();
        let resp = ContratoResponse {
            id,
            propiedad_id: id,
            inquilino_id: id,
            fecha_inicio: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            fecha_fin: NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            monto_mensual: Decimal::from_str("15000").unwrap(),
            deposito: Some(Decimal::from_str("30000").unwrap()),
            moneda: "DOP".to_string(),
            estado: "activo".to_string(),
            created_at: DateTime::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap(),
            pagos_generados: None,
            estado_deposito: None,
            fecha_cobro_deposito: None,
            fecha_devolucion_deposito: None,
            monto_retenido: None,
            motivo_retencion: None,
            recargo_porcentaje: None,
            dias_gracia: None,
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.get("propiedadId").is_some());
        assert!(json.get("inquilinoId").is_some());
        assert!(json.get("fechaInicio").is_some());
        assert!(json.get("fechaFin").is_some());
        assert!(json.get("montoMensual").is_some());
        assert!(json.get("createdAt").is_some());
        assert!(json.get("updatedAt").is_some());
        assert!(json.get("propiedad_id").is_none());
    }

    #[test]
    fn cambiar_estado_deposito_request_deserializes_camel_case() {
        let json = r#"{"estado":"cobrado"}"#;
        let req: CambiarEstadoDepositoRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.estado, "cobrado");
        assert!(req.monto_retenido.is_none());
        assert!(req.motivo_retencion.is_none());
    }

    #[test]
    fn cambiar_estado_deposito_request_deserializes_with_retention_fields() {
        let json = r#"{"estado":"retenido","montoRetenido":"5000.00","motivoRetencion":"Daños en la propiedad"}"#;
        let req: CambiarEstadoDepositoRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.estado, "retenido");
        assert_eq!(
            req.monto_retenido,
            Some(Decimal::from_str("5000.00").unwrap())
        );
        assert_eq!(
            req.motivo_retencion,
            Some("Daños en la propiedad".to_string())
        );
    }

    #[test]
    fn contrato_response_serializes_deposit_fields() {
        let id = Uuid::nil();
        let cobro_date = DateTime::from_timestamp(1_718_000_000, 0).unwrap();
        let devolucion_date = DateTime::from_timestamp(1_718_100_000, 0).unwrap();
        let resp = ContratoResponse {
            id,
            propiedad_id: id,
            inquilino_id: id,
            fecha_inicio: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            fecha_fin: NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            monto_mensual: Decimal::from_str("15000").unwrap(),
            deposito: Some(Decimal::from_str("30000").unwrap()),
            moneda: "DOP".to_string(),
            estado: "activo".to_string(),
            created_at: DateTime::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap(),
            pagos_generados: None,
            estado_deposito: Some("retenido".to_string()),
            fecha_cobro_deposito: Some(cobro_date),
            fecha_devolucion_deposito: Some(devolucion_date),
            monto_retenido: Some(Decimal::from_str("5000.00").unwrap()),
            motivo_retencion: Some("Daños en la propiedad".to_string()),
            recargo_porcentaje: None,
            dias_gracia: None,
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["estadoDeposito"], "retenido");
        assert!(json.get("fechaCobroDeposito").is_some());
        assert!(json.get("fechaDevolucionDeposito").is_some());
        assert_eq!(json["montoRetenido"], "5000.00");
        assert_eq!(json["motivoRetencion"], "Daños en la propiedad");
    }
}
