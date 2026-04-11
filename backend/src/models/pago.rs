use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePagoRequest {
    pub contrato_id: Uuid,
    pub monto: Decimal,
    pub moneda: Option<String>,
    pub fecha_pago: Option<NaiveDate>,
    pub fecha_vencimiento: NaiveDate,
    pub metodo_pago: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePagoRequest {
    pub monto: Option<Decimal>,
    pub fecha_pago: Option<NaiveDate>,
    pub metodo_pago: Option<String>,
    pub estado: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagoListQuery {
    pub contrato_id: Option<Uuid>,
    pub estado: Option<String>,
    pub fecha_desde: Option<NaiveDate>,
    pub fecha_hasta: Option<NaiveDate>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagoResponse {
    pub id: Uuid,
    pub contrato_id: Uuid,
    pub monto: Decimal,
    pub moneda: String,
    pub fecha_pago: Option<NaiveDate>,
    pub fecha_vencimiento: NaiveDate,
    pub metodo_pago: Option<String>,
    pub estado: String,
    pub notas: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pago_list_query_with_date_range() {
        let json = serde_json::json!({
            "fechaDesde": "2025-01-01",
            "fechaHasta": "2025-01-31"
        });
        let query: PagoListQuery = serde_json::from_value(json).unwrap();
        assert_eq!(
            query.fecha_desde,
            Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
        );
        assert_eq!(
            query.fecha_hasta,
            Some(NaiveDate::from_ymd_opt(2025, 1, 31).unwrap())
        );
    }

    #[test]
    fn pago_list_query_without_date_range() {
        let json = serde_json::json!({
            "estado": "pendiente",
            "page": 1,
            "perPage": 20
        });
        let query: PagoListQuery = serde_json::from_value(json).unwrap();
        assert!(query.fecha_desde.is_none());
        assert!(query.fecha_hasta.is_none());
        assert_eq!(query.estado.as_deref(), Some("pendiente"));
        assert_eq!(query.page, Some(1));
        assert_eq!(query.per_page, Some(20));
    }

    #[test]
    fn pago_list_query_with_only_fecha_desde() {
        let json = serde_json::json!({
            "fechaDesde": "2025-03-15"
        });
        let query: PagoListQuery = serde_json::from_value(json).unwrap();
        assert_eq!(
            query.fecha_desde,
            Some(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap())
        );
        assert!(query.fecha_hasta.is_none());
    }

    #[test]
    fn pago_list_query_with_only_fecha_hasta() {
        let json = serde_json::json!({
            "fechaHasta": "2025-06-30"
        });
        let query: PagoListQuery = serde_json::from_value(json).unwrap();
        assert!(query.fecha_desde.is_none());
        assert_eq!(
            query.fecha_hasta,
            Some(NaiveDate::from_ymd_opt(2025, 6, 30).unwrap())
        );
    }
}
