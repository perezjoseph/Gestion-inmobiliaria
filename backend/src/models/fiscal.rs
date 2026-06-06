use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Fiscal classification of an Organizacion per DR tax law.
///
/// - `PersonaJuridica`: Company with 9-digit RNC (SRL, SAS, etc.)
/// - `PersonaFisica`: Registered individual (cédula serves as RNC)
/// - `Informal`: Unregistered, no fiscal obligations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoFiscal {
    PersonaJuridica,
    PersonaFisica,
    Informal,
}

impl fmt::Display for TipoFiscal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PersonaJuridica => write!(f, "persona_juridica"),
            Self::PersonaFisica => write!(f, "persona_fisica"),
            Self::Informal => write!(f, "informal"),
        }
    }
}

/// Request to update the fiscal type of an Organizacion.
///
/// When transitioning to `PersonaJuridica`, `identificador` must be a valid 9-digit RNC.
/// When transitioning to `PersonaFisica`, `identificador` must be a valid 11-digit cédula.
/// When transitioning to `Informal`, `identificador` is optional.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActualizarTipoFiscalRequest {
    pub tipo_fiscal: TipoFiscal,
    pub identificador: Option<String>,
}

/// Response containing the current fiscal state of an Organizacion.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EstadoFiscalResponse {
    pub tipo_fiscal: TipoFiscal,
    pub rnc: Option<String>,
    pub cedula_rnc: Option<String>,
    pub razon_social: Option<String>,
    pub regimen_pagos: Option<String>,
    pub fecha_inicio_operaciones: Option<NaiveDate>,
    pub is_ecf_certificado: bool,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn tipo_fiscal_serializes_to_snake_case() {
        let pj = serde_json::to_string(&TipoFiscal::PersonaJuridica).unwrap();
        let pf = serde_json::to_string(&TipoFiscal::PersonaFisica).unwrap();
        let inf = serde_json::to_string(&TipoFiscal::Informal).unwrap();

        assert_eq!(pj, "\"persona_juridica\"");
        assert_eq!(pf, "\"persona_fisica\"");
        assert_eq!(inf, "\"informal\"");
    }

    #[test]
    fn tipo_fiscal_deserializes_from_snake_case() {
        let pj: TipoFiscal = serde_json::from_str("\"persona_juridica\"").unwrap();
        let pf: TipoFiscal = serde_json::from_str("\"persona_fisica\"").unwrap();
        let inf: TipoFiscal = serde_json::from_str("\"informal\"").unwrap();

        assert_eq!(pj, TipoFiscal::PersonaJuridica);
        assert_eq!(pf, TipoFiscal::PersonaFisica);
        assert_eq!(inf, TipoFiscal::Informal);
    }

    #[test]
    fn tipo_fiscal_display() {
        assert_eq!(TipoFiscal::PersonaJuridica.to_string(), "persona_juridica");
        assert_eq!(TipoFiscal::PersonaFisica.to_string(), "persona_fisica");
        assert_eq!(TipoFiscal::Informal.to_string(), "informal");
    }

    #[test]
    fn actualizar_request_deserializes_with_identificador() {
        let json = serde_json::json!({
            "tipoFiscal": "persona_juridica",
            "identificador": "123456789"
        });
        let req: ActualizarTipoFiscalRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.tipo_fiscal, TipoFiscal::PersonaJuridica);
        assert_eq!(req.identificador.as_deref(), Some("123456789"));
    }

    #[test]
    fn actualizar_request_deserializes_without_identificador() {
        let json = serde_json::json!({
            "tipoFiscal": "informal"
        });
        let req: ActualizarTipoFiscalRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.tipo_fiscal, TipoFiscal::Informal);
        assert!(req.identificador.is_none());
    }

    #[test]
    fn estado_fiscal_response_serializes_camel_case() {
        let resp = EstadoFiscalResponse {
            tipo_fiscal: TipoFiscal::PersonaJuridica,
            rnc: Some("123456789".to_string()),
            cedula_rnc: None,
            razon_social: Some("Mi Empresa SRL".to_string()),
            regimen_pagos: Some("mensual".to_string()),
            fecha_inicio_operaciones: Some(NaiveDate::from_ymd_opt(2020, 1, 15).unwrap()),
            is_ecf_certificado: true,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["tipoFiscal"], "persona_juridica");
        assert_eq!(json["rnc"], "123456789");
        assert_eq!(json["cedulaRnc"], serde_json::Value::Null);
        assert_eq!(json["razonSocial"], "Mi Empresa SRL");
        assert_eq!(json["regimenPagos"], "mensual");
        assert_eq!(json["fechaInicioOperaciones"], "2020-01-15");
        assert_eq!(json["isEcfCertificado"], true);
    }
}
