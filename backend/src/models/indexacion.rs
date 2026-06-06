use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Propuesta de renovación generada por el sistema basada en IPC y Ley 85-25.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropuestaRenovacion {
    pub contrato_id: Uuid,
    pub monto_actual: Decimal,
    pub monto_maximo: Decimal,
    pub ipc_porcentaje: Decimal,
    pub tope_aplicado: bool,
    pub datos_stale: bool,
}

/// Solicitud del admin para aprobar una renovación con un monto específico.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AprobarRenovacionRequest {
    pub monto_aprobado: Decimal,
}

/// Contrato próximo a vencer (dentro de 60 días) para revisión de indexación.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContratoProximoVencer {
    pub contrato_id: Uuid,
    pub propiedad_titulo: String,
    pub inquilino_nombre: String,
    pub fecha_fin: NaiveDate,
    pub monto_actual: Decimal,
    pub moneda: String,
    pub dias_restantes: i32,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn propuesta_renovacion_serializes_to_camel_case() {
        let propuesta = PropuestaRenovacion {
            contrato_id: Uuid::nil(),
            monto_actual: Decimal::from_str("25000.00").unwrap(),
            monto_maximo: Decimal::from_str("27500.00").unwrap(),
            ipc_porcentaje: Decimal::from_str("8.5").unwrap(),
            tope_aplicado: false,
            datos_stale: false,
        };

        let json = serde_json::to_value(&propuesta).unwrap();
        assert!(json.get("contratoId").is_some());
        assert!(json.get("montoActual").is_some());
        assert!(json.get("montoMaximo").is_some());
        assert!(json.get("ipcPorcentaje").is_some());
        assert!(json.get("topeAplicado").is_some());
        assert!(json.get("datosStale").is_some());
        assert!(json.get("contrato_id").is_none());
    }

    #[test]
    fn propuesta_renovacion_with_tope_aplicado() {
        let propuesta = PropuestaRenovacion {
            contrato_id: Uuid::nil(),
            monto_actual: Decimal::from_str("30000.00").unwrap(),
            monto_maximo: Decimal::from_str("33000.00").unwrap(),
            ipc_porcentaje: Decimal::from_str("12.3").unwrap(),
            tope_aplicado: true,
            datos_stale: false,
        };

        let json = serde_json::to_value(&propuesta).unwrap();
        assert_eq!(json["topeAplicado"], true);
        assert_eq!(json["montoMaximo"], "33000.00");
    }

    #[test]
    fn aprobar_renovacion_request_deserializes_camel_case() {
        let json = r#"{"montoAprobado":"27000.00"}"#;
        let req: AprobarRenovacionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.monto_aprobado, Decimal::from_str("27000.00").unwrap());
    }

    #[test]
    fn contrato_proximo_vencer_serializes_to_camel_case() {
        let contrato = ContratoProximoVencer {
            contrato_id: Uuid::nil(),
            propiedad_titulo: "Apartamento 301".to_string(),
            inquilino_nombre: "Juan Pérez".to_string(),
            fecha_fin: NaiveDate::from_ymd_opt(2026, 8, 15).unwrap(),
            monto_actual: Decimal::from_str("20000.00").unwrap(),
            moneda: "DOP".to_string(),
            dias_restantes: 45,
        };

        let json = serde_json::to_value(&contrato).unwrap();
        assert!(json.get("contratoId").is_some());
        assert!(json.get("propiedadTitulo").is_some());
        assert!(json.get("inquilinoNombre").is_some());
        assert!(json.get("fechaFin").is_some());
        assert!(json.get("montoActual").is_some());
        assert!(json.get("diasRestantes").is_some());
        assert_eq!(json["moneda"], "DOP");
        assert_eq!(json["diasRestantes"], 45);
        assert!(json.get("contrato_id").is_none());
    }
}
