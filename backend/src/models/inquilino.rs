use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InquilinoSearchQuery {
    pub search: Option<String>,
    pub busqueda: Option<String>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInquilinoRequest {
    pub nombre: String,
    pub apellido: String,
    pub email: Option<String>,
    pub telefono: Option<String>,
    pub cedula: String,
    pub contacto_emergencia: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInquilinoRequest {
    pub nombre: Option<String>,
    pub apellido: Option<String>,
    pub email: Option<String>,
    pub telefono: Option<String>,
    pub cedula: Option<String>,
    pub contacto_emergencia: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InquilinoResponse {
    pub id: Uuid,
    pub nombre: String,
    pub apellido: String,
    pub email: Option<String>,
    pub telefono: Option<String>,
    pub cedula: String,
    pub contacto_emergencia: Option<String>,
    pub notas: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn search_query_with_busqueda() {
        let json = serde_json::json!({
            "busqueda": "García"
        });
        let query: InquilinoSearchQuery = serde_json::from_value(json).unwrap();
        assert_eq!(query.busqueda.as_deref(), Some("García"));
        assert!(query.search.is_none());
    }

    #[test]
    fn search_query_with_search_field_backward_compat() {
        let json = serde_json::json!({
            "search": "Juan"
        });
        let query: InquilinoSearchQuery = serde_json::from_value(json).unwrap();
        assert_eq!(query.search.as_deref(), Some("Juan"));
        assert!(query.busqueda.is_none());
    }

    #[test]
    fn search_query_empty_object() {
        let json = serde_json::json!({});
        let query: InquilinoSearchQuery = serde_json::from_value(json).unwrap();
        assert!(query.busqueda.is_none());
        assert!(query.search.is_none());
        assert!(query.page.is_none());
        assert!(query.per_page.is_none());
    }
}
