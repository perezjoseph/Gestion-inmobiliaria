use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request to create a new condominium fee for a property.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrearCuotaRequest {
    pub propiedad_id: Uuid,
    pub monto: Decimal,
    pub moneda: String,
    pub frecuencia: String,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin: Option<NaiveDate>,
    pub es_passthrough: bool,
    pub contrato_id: Option<Uuid>,
}

/// Request to update an existing condominium fee. All fields optional for partial updates.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCuotaRequest {
    pub monto: Option<Decimal>,
    pub moneda: Option<String>,
    pub frecuencia: Option<String>,
    pub fecha_fin: Option<NaiveDate>,
    pub es_passthrough: Option<bool>,
    pub contrato_id: Option<Uuid>,
}

/// Response representing a condominium fee record.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CuotaResponse {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub monto: Decimal,
    pub moneda: String,
    pub frecuencia: String,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin: Option<NaiveDate>,
    pub es_passthrough: bool,
    pub contrato_id: Option<Uuid>,
}

/// Breakdown of billing amounts including base rent, condominium fee, and applicable ITBIS.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingDesglose {
    pub monto_base: Decimal,
    pub cuota_condominio: Decimal,
    pub itbis_base: Decimal,
    pub itbis_cuota: Decimal,
    pub total: Decimal,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
mod tests {
    use super::*;

    #[test]
    fn crear_cuota_request_deserializes_camel_case() {
        let json = serde_json::json!({
            "propiedadId": "550e8400-e29b-41d4-a716-446655440000",
            "monto": "5000.00",
            "moneda": "DOP",
            "frecuencia": "mensual",
            "fechaInicio": "2026-01-01",
            "fechaFin": null,
            "esPassthrough": true,
            "contratoId": "660e8400-e29b-41d4-a716-446655440001"
        });
        let req: CrearCuotaRequest = serde_json::from_value(json).unwrap();
        assert_eq!(
            req.propiedad_id,
            "550e8400-e29b-41d4-a716-446655440000"
                .parse::<Uuid>()
                .unwrap()
        );
        assert_eq!(req.monto, Decimal::new(500000, 2));
        assert_eq!(req.moneda, "DOP");
        assert_eq!(req.frecuencia, "mensual");
        assert_eq!(
            req.fecha_inicio,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        );
        assert!(req.fecha_fin.is_none());
        assert!(req.es_passthrough);
        assert!(req.contrato_id.is_some());
    }

    #[test]
    fn update_cuota_request_deserializes_partial() {
        let json = serde_json::json!({
            "monto": "7500.50",
            "esPassthrough": false
        });
        let req: UpdateCuotaRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.monto, Some(Decimal::new(750050, 2)));
        assert_eq!(req.es_passthrough, Some(false));
        assert!(req.moneda.is_none());
        assert!(req.frecuencia.is_none());
        assert!(req.fecha_fin.is_none());
        assert!(req.contrato_id.is_none());
    }

    #[test]
    fn cuota_response_serializes_camel_case() {
        let resp = CuotaResponse {
            id: Uuid::nil(),
            propiedad_id: Uuid::nil(),
            monto: Decimal::new(350000, 2),
            moneda: "USD".to_string(),
            frecuencia: "trimestral".to_string(),
            fecha_inicio: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            fecha_fin: Some(NaiveDate::from_ymd_opt(2027, 3, 1).unwrap()),
            es_passthrough: true,
            contrato_id: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["monto"], "3500.00");
        assert_eq!(json["moneda"], "USD");
        assert_eq!(json["frecuencia"], "trimestral");
        assert_eq!(json["fechaInicio"], "2026-03-01");
        assert_eq!(json["fechaFin"], "2027-03-01");
        assert_eq!(json["esPassthrough"], true);
        assert!(json["contratoId"].is_null());
    }

    #[test]
    fn billing_desglose_serializes_camel_case() {
        let desglose = BillingDesglose {
            monto_base: Decimal::new(2500000, 2),
            cuota_condominio: Decimal::new(500000, 2),
            itbis_base: Decimal::new(450000, 2),
            itbis_cuota: Decimal::new(90000, 2),
            total: Decimal::new(3540000, 2),
        };
        let json = serde_json::to_value(&desglose).unwrap();
        assert_eq!(json["montoBase"], "25000.00");
        assert_eq!(json["cuotaCondominio"], "5000.00");
        assert_eq!(json["itbisBase"], "4500.00");
        assert_eq!(json["itbisCuota"], "900.00");
        assert_eq!(json["total"], "35400.00");
    }
}
