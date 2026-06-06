use serde::{Deserialize, Serialize};

/// Tipo fiscal de la organización
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoFiscal {
    PersonaJuridica,
    PersonaFisica,
    Informal,
}

impl TipoFiscal {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::PersonaJuridica => "Persona Jurídica",
            Self::PersonaFisica => "Persona Física",
            Self::Informal => "Informal",
        }
    }

    pub const fn is_registered(&self) -> bool {
        matches!(self, Self::PersonaJuridica | Self::PersonaFisica)
    }
}

impl std::fmt::Display for TipoFiscal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::PersonaJuridica => "persona_juridica",
            Self::PersonaFisica => "persona_fisica",
            Self::Informal => "informal",
        };
        f.write_str(s)
    }
}

/// Response from GET /organizacion/fiscal/estado
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EstadoFiscalResponse {
    pub tipo_fiscal: TipoFiscal,
    pub rnc: Option<String>,
    pub cedula_rnc: Option<String>,
    pub razon_social: Option<String>,
    pub is_ecf: bool,
}

/// Request for PUT /organizacion/fiscal/tipo-fiscal
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActualizarTipoFiscalRequest {
    pub tipo_fiscal: TipoFiscal,
    pub identificador: Option<String>,
}

/// NCF sequence info
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecuenciaNcfResponse {
    pub id: String,
    pub tipo_ncf: String,
    pub prefijo: String,
    pub siguiente_numero: i32,
    pub rango_desde: i32,
    pub rango_hasta: i32,
    pub is_active: bool,
    pub is_ecf: bool,
}

impl SecuenciaNcfResponse {
    pub fn consumo_porcentaje(&self) -> f64 {
        let total = f64::from(self.rango_hasta - self.rango_desde + 1);
        if total <= 0.0 {
            return 100.0;
        }
        let usados = f64::from(self.siguiente_numero - self.rango_desde);
        (usados / total * 100.0).min(100.0)
    }

    pub fn restantes(&self) -> i32 {
        (self.rango_hasta - self.siguiente_numero + 1).max(0)
    }
}

/// Request for POST /ncf/configurar-rango
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurarRangoRequest {
    pub tipo_ncf: String,
    pub prefijo: String,
    pub rango_desde: i32,
    pub rango_hasta: i32,
}

/// Alert when NCF range is nearing exhaustion
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AlertaRango {
    pub tipo_ncf: String,
    pub consumo_porcentaje: f64,
    pub restantes: i32,
}
