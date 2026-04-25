use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorialPagosQuery {
    pub fecha_desde: NaiveDate,
    pub fecha_hasta: NaiveDate,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngresoReportQuery {
    pub mes: u32,
    pub anio: i32,
    pub propiedad_id: Option<Uuid>,
    pub inquilino_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngresoReportRow {
    pub propiedad_titulo: String,
    pub inquilino_nombre: String,
    pub monto: Decimal,
    pub moneda: String,
    pub estado: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngresoReportSummary {
    pub rows: Vec<IngresoReportRow>,
    pub total_pagado: Decimal,
    pub total_pendiente: Decimal,
    pub total_atrasado: Decimal,
    pub tasa_ocupacion: f64,
    pub generated_at: DateTime<Utc>,
    pub generated_by: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorialPagoEntry {
    pub contrato_id: Uuid,
    pub monto: Decimal,
    pub moneda: String,
    pub fecha_vencimiento: NaiveDate,
    pub fecha_pago: Option<NaiveDate>,
    pub estado: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RentabilidadReportQuery {
    pub mes: u32,
    pub anio: i32,
    pub propiedad_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RentabilidadReportRow {
    pub propiedad_id: Uuid,
    pub propiedad_titulo: String,
    pub total_ingresos: Decimal,
    pub total_gastos: Decimal,
    pub ingreso_neto: Decimal,
    pub moneda: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RentabilidadReportSummary {
    pub rows: Vec<RentabilidadReportRow>,
    pub total_ingresos: Decimal,
    pub total_gastos: Decimal,
    pub total_neto: Decimal,
    pub mes: u32,
    pub anio: i32,
    pub generated_at: DateTime<Utc>,
    pub generated_by: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn ingreso_report_query_deserializes_camel_case_all_fields() {
        let json = r#"{
            "mes": 4,
            "anio": 2025,
            "propiedadId": "550e8400-e29b-41d4-a716-446655440000",
            "inquilinoId": "660e8400-e29b-41d4-a716-446655440000"
        }"#;
        let query: IngresoReportQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.mes, 4);
        assert_eq!(query.anio, 2025);
        assert!(query.propiedad_id.is_some());
        assert!(query.inquilino_id.is_some());
    }

    #[test]
    fn ingreso_report_query_deserializes_partial_fields() {
        let json = r#"{"mes": 1, "anio": 2024}"#;
        let query: IngresoReportQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.mes, 1);
        assert_eq!(query.anio, 2024);
        assert!(query.propiedad_id.is_none());
        assert!(query.inquilino_id.is_none());
    }

    #[test]
    fn historial_pagos_query_deserializes_camel_case() {
        let json = r#"{"fechaDesde": "2025-01-01", "fechaHasta": "2025-03-31"}"#;
        let query: HistorialPagosQuery = serde_json::from_str(json).unwrap();
        assert_eq!(
            query.fecha_desde,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()
        );
        assert_eq!(
            query.fecha_hasta,
            NaiveDate::from_ymd_opt(2025, 3, 31).unwrap()
        );
    }

    #[test]
    fn ingreso_report_summary_serializes_to_camel_case() {
        let summary = IngresoReportSummary {
            rows: vec![],
            total_pagado: Decimal::new(5000, 0),
            total_pendiente: Decimal::new(2000, 0),
            total_atrasado: Decimal::new(1000, 0),
            tasa_ocupacion: 75.5,
            generated_at: Utc.with_ymd_and_hms(2025, 4, 10, 12, 0, 0).unwrap(),
            generated_by: "admin@test.com".into(),
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert!(json.get("totalPagado").is_some());
        assert!(json.get("totalPendiente").is_some());
        assert!(json.get("totalAtrasado").is_some());
        assert!(json.get("tasaOcupacion").is_some());
        assert!(json.get("generatedAt").is_some());
        assert!(json.get("generatedBy").is_some());
        assert_eq!(json["tasaOcupacion"], 75.5);
    }

    #[test]
    fn ingreso_report_row_serializes_to_camel_case() {
        let row = IngresoReportRow {
            propiedad_titulo: "Apartamento Centro".into(),
            inquilino_nombre: "Juan Perez".into(),
            monto: Decimal::new(25000, 0),
            moneda: "DOP".into(),
            estado: "pagado".into(),
        };
        let json = serde_json::to_value(&row).unwrap();
        assert!(json.get("propiedadTitulo").is_some());
        assert!(json.get("inquilinoNombre").is_some());
        assert_eq!(json["estado"], "pagado");
    }

    #[test]
    fn historial_pago_entry_serializes_to_camel_case() {
        let entry = HistorialPagoEntry {
            contrato_id: Uuid::new_v4(),
            monto: Decimal::new(15000, 0),
            moneda: "DOP".into(),
            fecha_vencimiento: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            fecha_pago: Some(NaiveDate::from_ymd_opt(2025, 4, 3).unwrap()),
            estado: "pagado".into(),
        };
        let json = serde_json::to_value(&entry).unwrap();
        assert!(json.get("contratoId").is_some());
        assert!(json.get("fechaVencimiento").is_some());
        assert!(json.get("fechaPago").is_some());
    }

    #[test]
    fn historial_pago_entry_serializes_null_fecha_pago() {
        let entry = HistorialPagoEntry {
            contrato_id: Uuid::new_v4(),
            monto: Decimal::new(10000, 0),
            moneda: "USD".into(),
            fecha_vencimiento: NaiveDate::from_ymd_opt(2025, 5, 1).unwrap(),
            fecha_pago: None,
            estado: "pendiente".into(),
        };
        let json = serde_json::to_value(&entry).unwrap();
        assert!(json["fechaPago"].is_null());
    }

    #[test]
    fn rentabilidad_report_query_deserializes_all_fields() {
        let json = r#"{
            "mes": 4,
            "anio": 2025,
            "propiedadId": "550e8400-e29b-41d4-a716-446655440000"
        }"#;
        let query: RentabilidadReportQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.mes, 4);
        assert_eq!(query.anio, 2025);
        assert!(query.propiedad_id.is_some());
    }

    #[test]
    fn rentabilidad_report_query_deserializes_required_only() {
        let json = r#"{"mes": 1, "anio": 2024}"#;
        let query: RentabilidadReportQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.mes, 1);
        assert_eq!(query.anio, 2024);
        assert!(query.propiedad_id.is_none());
    }

    #[test]
    fn rentabilidad_report_row_serializes_to_camel_case() {
        let row = RentabilidadReportRow {
            propiedad_id: Uuid::new_v4(),
            propiedad_titulo: "Apartamento Centro".into(),
            total_ingresos: Decimal::new(50000, 0),
            total_gastos: Decimal::new(15000, 0),
            ingreso_neto: Decimal::new(35000, 0),
            moneda: "DOP".into(),
        };
        let json = serde_json::to_value(&row).unwrap();
        assert!(json.get("propiedadId").is_some());
        assert!(json.get("propiedadTitulo").is_some());
        assert!(json.get("totalIngresos").is_some());
        assert!(json.get("totalGastos").is_some());
        assert!(json.get("ingresoNeto").is_some());
        assert_eq!(json["moneda"], "DOP");
    }

    #[test]
    fn rentabilidad_report_summary_serializes_to_camel_case() {
        let summary = RentabilidadReportSummary {
            rows: vec![],
            total_ingresos: Decimal::new(100_000, 0),
            total_gastos: Decimal::new(30_000, 0),
            total_neto: Decimal::new(70_000, 0),
            mes: 4,
            anio: 2025,
            generated_at: Utc.with_ymd_and_hms(2025, 4, 10, 12, 0, 0).unwrap(),
            generated_by: "admin@test.com".into(),
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert!(json.get("totalIngresos").is_some());
        assert!(json.get("totalGastos").is_some());
        assert!(json.get("totalNeto").is_some());
        assert!(json.get("generatedAt").is_some());
        assert!(json.get("generatedBy").is_some());
        assert_eq!(json["mes"], 4);
        assert_eq!(json["anio"], 2025);
    }
}
