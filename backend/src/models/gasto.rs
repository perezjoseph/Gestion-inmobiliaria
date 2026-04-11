use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGastoRequest {
    pub propiedad_id: Uuid,
    pub unidad_id: Option<Uuid>,
    pub categoria: String,
    pub descripcion: String,
    pub monto: Decimal,
    pub moneda: String,
    pub fecha_gasto: NaiveDate,
    pub proveedor: Option<String>,
    pub numero_factura: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGastoRequest {
    pub categoria: Option<String>,
    pub descripcion: Option<String>,
    pub monto: Option<Decimal>,
    pub moneda: Option<String>,
    pub fecha_gasto: Option<NaiveDate>,
    pub unidad_id: Option<Uuid>,
    pub proveedor: Option<String>,
    pub numero_factura: Option<String>,
    pub estado: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GastoListQuery {
    pub propiedad_id: Option<Uuid>,
    pub unidad_id: Option<Uuid>,
    pub categoria: Option<String>,
    pub estado: Option<String>,
    pub fecha_desde: Option<NaiveDate>,
    pub fecha_hasta: Option<NaiveDate>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumenCategoriasQuery {
    pub propiedad_id: Uuid,
    pub fecha_desde: Option<NaiveDate>,
    pub fecha_hasta: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GastoResponse {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub unidad_id: Option<Uuid>,
    pub categoria: String,
    pub descripcion: String,
    pub monto: Decimal,
    pub moneda: String,
    pub fecha_gasto: NaiveDate,
    pub estado: String,
    pub proveedor: Option<String>,
    pub numero_factura: Option<String>,
    pub notas: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumenCategoriaRow {
    pub categoria: String,
    pub total: Decimal,
    pub cantidad: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn create_gasto_request_deserializes_camel_case() {
        let json = r#"{
            "propiedadId": "550e8400-e29b-41d4-a716-446655440000",
            "categoria": "mantenimiento",
            "descripcion": "Reparación de techo",
            "monto": "15000.50",
            "moneda": "DOP",
            "fechaGasto": "2025-04-01",
            "proveedor": "Constructora ABC",
            "numeroFactura": "FAC-001",
            "notas": "Urgente"
        }"#;
        let req: CreateGastoRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.categoria, "mantenimiento");
        assert_eq!(req.monto, Decimal::new(1500050, 2));
        assert_eq!(
            req.fecha_gasto,
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap()
        );
        assert!(req.unidad_id.is_none());
        assert_eq!(req.proveedor.as_deref(), Some("Constructora ABC"));
    }

    #[test]
    fn create_gasto_request_deserializes_required_only() {
        let json = r#"{
            "propiedadId": "550e8400-e29b-41d4-a716-446655440000",
            "categoria": "impuestos",
            "descripcion": "Impuesto predial",
            "monto": "5000",
            "moneda": "DOP",
            "fechaGasto": "2025-03-15"
        }"#;
        let req: CreateGastoRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.categoria, "impuestos");
        assert!(req.unidad_id.is_none());
        assert!(req.proveedor.is_none());
        assert!(req.numero_factura.is_none());
        assert!(req.notas.is_none());
    }

    #[test]
    fn update_gasto_request_deserializes_partial() {
        let json = r#"{"estado": "pagado", "monto": "20000"}"#;
        let req: UpdateGastoRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.estado.as_deref(), Some("pagado"));
        assert_eq!(req.monto, Some(Decimal::new(20000, 0)));
        assert!(req.categoria.is_none());
        assert!(req.descripcion.is_none());
    }

    #[test]
    fn gasto_list_query_with_filters() {
        let json = serde_json::json!({
            "propiedadId": "550e8400-e29b-41d4-a716-446655440000",
            "categoria": "seguros",
            "fechaDesde": "2025-01-01",
            "fechaHasta": "2025-06-30",
            "page": 1,
            "perPage": 20
        });
        let query: GastoListQuery = serde_json::from_value(json).unwrap();
        assert!(query.propiedad_id.is_some());
        assert_eq!(query.categoria.as_deref(), Some("seguros"));
        assert_eq!(
            query.fecha_desde,
            Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
        );
        assert_eq!(query.page, Some(1));
        assert_eq!(query.per_page, Some(20));
    }

    #[test]
    fn gasto_list_query_empty() {
        let json = serde_json::json!({});
        let query: GastoListQuery = serde_json::from_value(json).unwrap();
        assert!(query.propiedad_id.is_none());
        assert!(query.categoria.is_none());
        assert!(query.estado.is_none());
        assert!(query.fecha_desde.is_none());
        assert!(query.fecha_hasta.is_none());
        assert!(query.page.is_none());
        assert!(query.per_page.is_none());
    }

    #[test]
    fn resumen_categorias_query_deserializes() {
        let json = r#"{
            "propiedadId": "550e8400-e29b-41d4-a716-446655440000",
            "fechaDesde": "2025-01-01",
            "fechaHasta": "2025-12-31"
        }"#;
        let query: ResumenCategoriasQuery = serde_json::from_str(json).unwrap();
        assert_eq!(
            query.fecha_desde,
            Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
        );
        assert_eq!(
            query.fecha_hasta,
            Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap())
        );
    }

    #[test]
    fn gasto_response_serializes_to_camel_case() {
        let response = GastoResponse {
            id: Uuid::new_v4(),
            propiedad_id: Uuid::new_v4(),
            unidad_id: None,
            categoria: "mantenimiento".into(),
            descripcion: "Reparación".into(),
            monto: Decimal::new(15000, 0),
            moneda: "DOP".into(),
            fecha_gasto: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            estado: "pendiente".into(),
            proveedor: Some("ABC".into()),
            numero_factura: None,
            notas: None,
            created_at: Utc.with_ymd_and_hms(2025, 4, 1, 10, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2025, 4, 1, 10, 0, 0).unwrap(),
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json.get("propiedadId").is_some());
        assert!(json.get("unidadId").is_some());
        assert!(json.get("fechaGasto").is_some());
        assert!(json.get("createdAt").is_some());
        assert!(json.get("updatedAt").is_some());
        assert!(json.get("numeroFactura").is_some());
        assert_eq!(json["estado"], "pendiente");
    }

    #[test]
    fn resumen_categoria_row_serializes_to_camel_case() {
        let row = ResumenCategoriaRow {
            categoria: "mantenimiento".into(),
            total: Decimal::new(50000, 0),
            cantidad: 5,
        };
        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["categoria"], "mantenimiento");
        assert_eq!(json["cantidad"], 5);
        assert!(json.get("total").is_some());
    }
}
