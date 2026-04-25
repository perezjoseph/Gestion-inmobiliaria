use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentoResponse {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub filename: String,
    pub file_path: String,
    pub mime_type: String,
    pub file_size: i64,
    pub uploaded_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn documento_response_serializes_to_camel_case() {
        let id = Uuid::new_v4();
        let entity_id = Uuid::new_v4();
        let uploaded_by = Uuid::new_v4();
        let now = Utc::now();
        let resp = DocumentoResponse {
            id,
            entity_type: "propiedad".to_string(),
            entity_id,
            filename: "foto.jpg".to_string(),
            file_path: "/uploads/propiedad/abc/foto.jpg".to_string(),
            mime_type: "image/jpeg".to_string(),
            file_size: 1024,
            uploaded_by,
            created_at: now,
        };
        let serialized = serde_json::to_value(&resp).unwrap();
        assert!(serialized.get("entityType").is_some());
        assert!(serialized.get("entityId").is_some());
        assert!(serialized.get("filePath").is_some());
        assert!(serialized.get("mimeType").is_some());
        assert!(serialized.get("fileSize").is_some());
        assert!(serialized.get("uploadedBy").is_some());
        assert!(serialized.get("createdAt").is_some());
        assert!(serialized.get("entity_type").is_none());
    }
}
