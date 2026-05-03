use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUnidadRequest {
    pub numero_unidad: String,
    pub piso: Option<i32>,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    pub area_m2: Option<Decimal>,
    pub precio: Decimal,
    pub moneda: Option<String>,
    pub estado: Option<String>,
    pub descripcion: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUnidadRequest {
    pub numero_unidad: Option<String>,
    pub piso: Option<i32>,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    pub area_m2: Option<Decimal>,
    pub precio: Option<Decimal>,
    pub moneda: Option<String>,
    pub estado: Option<String>,
    pub descripcion: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnidadListQuery {
    pub estado: Option<String>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnidadResponse {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub numero_unidad: String,
    pub piso: Option<i32>,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    pub area_m2: Option<Decimal>,
    pub precio: Decimal,
    pub moneda: String,
    pub estado: String,
    pub descripcion: Option<String>,
    pub gastos_count: Option<u64>,
    pub mantenimiento_count: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcupacionResumen {
    pub total_unidades: u64,
    pub unidades_ocupadas: u64,
    pub tasa_ocupacion: f64,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn create_unidad_request_deserializes_camel_case() {
        let json = r#"{
            "numeroUnidad": "A-101",
            "piso": 1,
            "habitaciones": 3,
            "banos": 2,
            "areaM2": "85.50",
            "precio": "25000.00",
            "moneda": "DOP",
            "estado": "disponible",
            "descripcion": "Apartamento con vista al mar"
        }"#;
        let req: CreateUnidadRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.numero_unidad, "A-101");
        assert_eq!(req.piso, Some(1));
        assert_eq!(req.habitaciones, Some(3));
        assert_eq!(req.banos, Some(2));
        assert_eq!(req.area_m2, Some(Decimal::new(8550, 2)));
        assert_eq!(req.precio, Decimal::new(2_500_000, 2));
        assert_eq!(req.moneda.as_deref(), Some("DOP"));
        assert_eq!(req.estado.as_deref(), Some("disponible"));
        assert_eq!(
            req.descripcion.as_deref(),
            Some("Apartamento con vista al mar")
        );
    }

    #[test]
    fn create_unidad_request_deserializes_required_only() {
        let json = r#"{
            "numeroUnidad": "B-202",
            "precio": "15000"
        }"#;
        let req: CreateUnidadRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.numero_unidad, "B-202");
        assert_eq!(req.precio, Decimal::new(15000, 0));
        assert!(req.piso.is_none());
        assert!(req.habitaciones.is_none());
        assert!(req.banos.is_none());
        assert!(req.area_m2.is_none());
        assert!(req.moneda.is_none());
        assert!(req.estado.is_none());
        assert!(req.descripcion.is_none());
    }

    #[test]
    fn update_unidad_request_deserializes_partial() {
        let json = r#"{"estado": "ocupada", "precio": "30000"}"#;
        let req: UpdateUnidadRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.estado.as_deref(), Some("ocupada"));
        assert_eq!(req.precio, Some(Decimal::new(30000, 0)));
        assert!(req.numero_unidad.is_none());
        assert!(req.piso.is_none());
        assert!(req.habitaciones.is_none());
        assert!(req.banos.is_none());
        assert!(req.area_m2.is_none());
        assert!(req.moneda.is_none());
        assert!(req.descripcion.is_none());
    }

    #[test]
    fn unidad_list_query_with_filters() {
        let json = serde_json::json!({
            "estado": "disponible",
            "page": 1,
            "perPage": 20
        });
        let query: UnidadListQuery = serde_json::from_value(json).unwrap();
        assert_eq!(query.estado.as_deref(), Some("disponible"));
        assert_eq!(query.page, Some(1));
        assert_eq!(query.per_page, Some(20));
    }

    #[test]
    fn unidad_list_query_empty() {
        let json = serde_json::json!({});
        let query: UnidadListQuery = serde_json::from_value(json).unwrap();
        assert!(query.estado.is_none());
        assert!(query.page.is_none());
        assert!(query.per_page.is_none());
    }

    #[test]
    fn unidad_response_serializes_to_camel_case() {
        let response = UnidadResponse {
            id: Uuid::new_v4(),
            propiedad_id: Uuid::new_v4(),
            numero_unidad: "A-101".into(),
            piso: Some(1),
            habitaciones: Some(3),
            banos: Some(2),
            area_m2: Some(Decimal::new(8550, 2)),
            precio: Decimal::new(2_500_000, 2),
            moneda: "DOP".into(),
            estado: "disponible".into(),
            descripcion: Some("Apartamento".into()),
            gastos_count: Some(5),
            mantenimiento_count: Some(2),
            created_at: Utc.with_ymd_and_hms(2025, 4, 1, 10, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2025, 4, 1, 10, 0, 0).unwrap(),
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json.get("propiedadId").is_some());
        assert!(json.get("numeroUnidad").is_some());
        assert!(json.get("areaM2").is_some());
        assert!(json.get("gastosCount").is_some());
        assert!(json.get("mantenimientoCount").is_some());
        assert!(json.get("createdAt").is_some());
        assert!(json.get("updatedAt").is_some());
        assert_eq!(json["estado"], "disponible");
        assert_eq!(json["numeroUnidad"], "A-101");
    }

    #[test]
    fn ocupacion_resumen_serializes_to_camel_case() {
        let resumen = OcupacionResumen {
            total_unidades: 10,
            unidades_ocupadas: 7,
            tasa_ocupacion: 70.0,
        };
        let json = serde_json::to_value(&resumen).unwrap();
        assert_eq!(json["totalUnidades"], 10);
        assert_eq!(json["unidadesOcupadas"], 7);
        assert_eq!(json["tasaOcupacion"], 70.0);
    }
}
