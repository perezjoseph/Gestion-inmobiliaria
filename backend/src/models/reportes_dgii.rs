use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Registro607 {
    pub rnc_cliente: String,
    pub tipo_ncf: String,
    pub ncf: String,
    pub fecha_comprobante: NaiveDate,
    pub fecha_pago: NaiveDate,
    pub monto_servicios: Decimal,
    pub monto_bienes: Decimal,
    pub itbis_facturado: Decimal,
    pub itbis_retenido: Decimal,
    pub forma_pago: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Registro606 {
    pub rnc_proveedor: String,
    pub tipo_ncf: String,
    pub ncf_proveedor: String,
    pub fecha_comprobante: NaiveDate,
    pub fecha_pago: NaiveDate,
    pub monto_servicios: Decimal,
    pub monto_bienes: Decimal,
    pub itbis_facturado: Decimal,
    pub itbis_retenido: Decimal,
    pub itbis_al_costo: Decimal,
    pub forma_pago: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReporteGenerado {
    pub contenido: String,
    pub preview: Vec<RegistroPreview>,
    pub excluidos: Vec<RegistroExcluido>,
    pub cantidad_registros: u32,
    pub monto_total: Decimal,
    pub itbis_total: Decimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RegistroExcluido {
    pub razon: String,
    pub referencia: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RegistroPreview {
    pub campos: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItbisNetoResult {
    pub itbis_cobrado: Decimal,
    pub itbis_pagado: Decimal,
    pub itbis_neto: Decimal,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn registro_607_serializes_camel_case() {
        let registro = Registro607 {
            rnc_cliente: "123456789".to_string(),
            tipo_ncf: "B01".to_string(),
            ncf: "B0100000001".to_string(),
            fecha_comprobante: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            fecha_pago: NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            monto_servicios: Decimal::new(50_000, 2),
            monto_bienes: Decimal::ZERO,
            itbis_facturado: Decimal::new(9_000, 2),
            itbis_retenido: Decimal::ZERO,
            forma_pago: "transferencia".to_string(),
        };
        let json = serde_json::to_value(&registro).unwrap();
        assert!(json.get("rncCliente").is_some());
        assert!(json.get("tipoNcf").is_some());
        assert!(json.get("fechaComprobante").is_some());
        assert!(json.get("montoServicios").is_some());
        assert!(json.get("itbisFacturado").is_some());
        assert!(json.get("formaPago").is_some());
    }

    #[test]
    fn registro_606_serializes_camel_case() {
        let registro = Registro606 {
            rnc_proveedor: "987654321".to_string(),
            tipo_ncf: "B01".to_string(),
            ncf_proveedor: "B0100000005".to_string(),
            fecha_comprobante: NaiveDate::from_ymd_opt(2026, 2, 10).unwrap(),
            fecha_pago: NaiveDate::from_ymd_opt(2026, 2, 15).unwrap(),
            monto_servicios: Decimal::new(30_000, 2),
            monto_bienes: Decimal::new(10_000, 2),
            itbis_facturado: Decimal::new(7_200, 2),
            itbis_retenido: Decimal::ZERO,
            itbis_al_costo: Decimal::ZERO,
            forma_pago: "cheque".to_string(),
        };
        let json = serde_json::to_value(&registro).unwrap();
        assert!(json.get("rncProveedor").is_some());
        assert!(json.get("ncfProveedor").is_some());
        assert!(json.get("itbisAlCosto").is_some());
    }

    #[test]
    fn reporte_generado_serializes_correctly() {
        let reporte = ReporteGenerado {
            contenido: "607|123456789|202601|1|500.00".to_string(),
            preview: vec![RegistroPreview {
                campos: vec!["123456789".to_string(), "B01".to_string()],
            }],
            excluidos: vec![RegistroExcluido {
                razon: "Falta RNC".to_string(),
                referencia: "PAG-001".to_string(),
            }],
            cantidad_registros: 1,
            monto_total: Decimal::new(50_000, 2),
            itbis_total: Decimal::new(9_000, 2),
        };
        let json = serde_json::to_value(&reporte).unwrap();
        assert!(json.get("cantidadRegistros").is_some());
        assert!(json.get("montoTotal").is_some());
        assert!(json.get("itbisTotal").is_some());
        assert!(json.get("excluidos").is_some());
        assert!(json.get("preview").is_some());
    }

    #[test]
    fn itbis_neto_result_serializes_correctly() {
        let result = ItbisNetoResult {
            itbis_cobrado: Decimal::new(18_000, 2),
            itbis_pagado: Decimal::new(7_200, 2),
            itbis_neto: Decimal::new(10_800, 2),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert!(json.get("itbisCobrado").is_some());
        assert!(json.get("itbisPagado").is_some());
        assert!(json.get("itbisNeto").is_some());
    }

    #[test]
    fn registro_607_deserializes_from_camel_case() {
        let json = serde_json::json!({
            "rncCliente": "123456789",
            "tipoNcf": "B02",
            "ncf": "B0200000001",
            "fechaComprobante": "2026-03-01",
            "fechaPago": "2026-03-05",
            "montoServicios": "1000.00",
            "montoBienes": "0.00",
            "itbisFacturado": "180.00",
            "itbisRetenido": "0.00",
            "formaPago": "efectivo"
        });
        let registro: Registro607 = serde_json::from_value(json).unwrap();
        assert_eq!(registro.rnc_cliente, "123456789");
        assert_eq!(registro.tipo_ncf, "B02");
        assert_eq!(registro.monto_servicios, Decimal::new(100_000, 2));
    }

    #[test]
    fn registro_excluido_serializes_correctly() {
        let excluido = RegistroExcluido {
            razon: "Falta fecha_comprobante".to_string(),
            referencia: "PAG-042".to_string(),
        };
        let json = serde_json::to_value(&excluido).unwrap();
        assert_eq!(json["razon"], "Falta fecha_comprobante");
        assert_eq!(json["referencia"], "PAG-042");
    }
}
