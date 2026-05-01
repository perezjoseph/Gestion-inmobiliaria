pub mod auditoria;
pub mod configuracion;
pub mod contrato;
pub mod dashboard_extra;
pub mod gasto;
#[allow(dead_code)]
pub mod documento;
pub mod importacion;
pub mod inquilino;
pub mod mantenimiento;
pub mod notificacion;
pub mod ocr;
pub mod pago;
pub mod propiedad;
pub mod reporte;
pub mod usuario;

use serde::{Deserialize, Serialize};

pub fn deserialize_f64_from_any<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct F64Visitor;

    impl de::Visitor<'_> for F64Visitor {
        type Value = f64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a number or numeric string")
        }

        fn visit_f64<E: de::Error>(self, v: f64) -> Result<f64, E> {
            Ok(v)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<f64, E> {
            Ok(v as f64)
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<f64, E> {
            Ok(v as f64)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<f64, E> {
            v.parse::<f64>().map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(F64Visitor)
}

pub fn deserialize_option_f64_from_any<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct OptionF64Visitor;

    impl<'de> de::Visitor<'de> for OptionF64Visitor {
        type Value = Option<f64>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a number, numeric string, or null")
        }

        fn visit_none<E: de::Error>(self) -> Result<Option<f64>, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Option<f64>, E> {
            Ok(None)
        }

        fn visit_some<D2: serde::Deserializer<'de>>(
            self,
            deserializer: D2,
        ) -> Result<Option<f64>, D2::Error> {
            deserialize_f64_from_any(deserializer).map(Some)
        }

        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Option<f64>, E> {
            Ok(Some(v))
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Option<f64>, E> {
            Ok(Some(v as f64))
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Option<f64>, E> {
            Ok(Some(v as f64))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Option<f64>, E> {
            v.parse::<f64>().map(Some).map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_option(OptionF64Visitor)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_propiedades: u64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub tasa_ocupacion: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub ingreso_mensual: f64,
    pub pagos_atrasados: u64,
}
