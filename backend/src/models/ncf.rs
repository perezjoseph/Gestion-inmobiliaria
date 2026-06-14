use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoNCF {
    B01,
    B02,
    B14,
    B15,
}

impl fmt::Display for TipoNCF {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::B01 => write!(f, "B01"),
            Self::B02 => write!(f, "B02"),
            Self::B14 => write!(f, "B14"),
            Self::B15 => write!(f, "B15"),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurarRangoRequest {
    pub tipo_ncf: TipoNCF,
    pub prefijo: char,
    pub rango_desde: i32,
    pub rango_hasta: i32,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AlertaRango {
    pub tipo_ncf: TipoNCF,
    pub consumo_porcentaje: f64,
    pub restantes: i32,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SecuenciaNcfResponse {
    pub id: Uuid,
    pub tipo_ncf: TipoNCF,
    pub prefijo: String,
    pub siguiente_numero: i32,
    pub rango_desde: i32,
    pub rango_hasta: i32,
    pub is_active: bool,
    pub is_ecf: bool,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn tipo_ncf_serializes_as_variant_name() {
        assert_eq!(serde_json::to_string(&TipoNCF::B01).unwrap(), "\"B01\"");
        assert_eq!(serde_json::to_string(&TipoNCF::B02).unwrap(), "\"B02\"");
        assert_eq!(serde_json::to_string(&TipoNCF::B14).unwrap(), "\"B14\"");
        assert_eq!(serde_json::to_string(&TipoNCF::B15).unwrap(), "\"B15\"");
    }

    #[test]
    fn tipo_ncf_deserializes_from_variant_name() {
        let b01: TipoNCF = serde_json::from_str("\"B01\"").unwrap();
        let b02: TipoNCF = serde_json::from_str("\"B02\"").unwrap();
        let b14: TipoNCF = serde_json::from_str("\"B14\"").unwrap();
        let b15: TipoNCF = serde_json::from_str("\"B15\"").unwrap();

        assert_eq!(b01, TipoNCF::B01);
        assert_eq!(b02, TipoNCF::B02);
        assert_eq!(b14, TipoNCF::B14);
        assert_eq!(b15, TipoNCF::B15);
    }

    #[test]
    fn tipo_ncf_display() {
        assert_eq!(TipoNCF::B01.to_string(), "B01");
        assert_eq!(TipoNCF::B02.to_string(), "B02");
        assert_eq!(TipoNCF::B14.to_string(), "B14");
        assert_eq!(TipoNCF::B15.to_string(), "B15");
    }

    #[test]
    fn configurar_rango_request_deserializes() {
        let json = serde_json::json!({
            "tipoNcf": "B01",
            "prefijo": "B",
            "rangoDesde": 1,
            "rangoHasta": 50000
        });
        let req: ConfigurarRangoRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.tipo_ncf, TipoNCF::B01);
        assert_eq!(req.prefijo, 'B');
        assert_eq!(req.rango_desde, 1);
        assert_eq!(req.rango_hasta, 50000);
    }

    #[test]
    fn alerta_rango_serializes_camel_case() {
        let alerta = AlertaRango {
            tipo_ncf: TipoNCF::B02,
            consumo_porcentaje: 85.5,
            restantes: 725,
        };
        let json = serde_json::to_value(&alerta).unwrap();
        assert_eq!(json["tipoNcf"], "B02");
        assert_eq!(json["consumoPorcentaje"], 85.5);
        assert_eq!(json["restantes"], 725);
    }

    #[test]
    fn secuencia_ncf_response_serializes_camel_case() {
        let resp = SecuenciaNcfResponse {
            id: Uuid::nil(),
            tipo_ncf: TipoNCF::B15,
            prefijo: "B".to_string(),
            siguiente_numero: 42,
            rango_desde: 1,
            rango_hasta: 10000,
            is_active: true,
            is_ecf: false,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["tipoNcf"], "B15");
        assert_eq!(json["prefijo"], "B");
        assert_eq!(json["siguienteNumero"], 42);
        assert_eq!(json["rangoDesde"], 1);
        assert_eq!(json["rangoHasta"], 10000);
        assert_eq!(json["isActive"], true);
        assert_eq!(json["isEcf"], false);
    }
}
