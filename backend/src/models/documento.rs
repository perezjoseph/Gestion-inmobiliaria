use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
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
    pub tipo_documento: String,
    pub estado_verificacion: String,
    pub fecha_vencimiento: Option<NaiveDate>,
    pub verificado_por: Option<Uuid>,
    pub fecha_verificacion: Option<DateTime<Utc>>,
    pub notas_verificacion: Option<String>,
    pub numero_documento: Option<String>,
    pub contenido_editable: Option<serde_json::Value>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentoListQuery {
    pub tipo_documento: Option<String>,
    pub estado_verificacion: Option<String>,
    pub fecha_vencimiento_desde: Option<NaiveDate>,
    pub fecha_vencimiento_hasta: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificarDocumentoRequest {
    pub estado_verificacion: String,
    pub notas_verificacion: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PorVencerQuery {
    pub dias: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CumplimientoResponse {
    pub entity_type: String,
    pub entity_id: Uuid,
    pub documentos: Vec<CumplimientoItem>,
    pub porcentaje: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CumplimientoItem {
    pub tipo_documento: String,
    pub nombre: String,
    pub requerido: bool,
    pub estado: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardarEditorRequest {
    pub contenido_editable: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantillaResponse {
    pub id: Uuid,
    pub nombre: String,
    pub tipo_documento: String,
    pub entity_type: String,
    pub contenido: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantillaRellenadaResponse {
    pub plantilla_id: Uuid,
    pub nombre: String,
    pub tipo_documento: String,
    pub contenido: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DigitalizarResponse {
    pub document_type: String,
    pub contenido_editable: serde_json::Value,
    pub campos_baja_confianza: Vec<String>,
    pub documento_original_id: Uuid,
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
            tipo_documento: "titulo_propiedad".to_string(),
            estado_verificacion: "pendiente".to_string(),
            fecha_vencimiento: None,
            verificado_por: None,
            fecha_verificacion: None,
            notas_verificacion: None,
            numero_documento: None,
            contenido_editable: None,
            updated_at: None,
        };
        let serialized = serde_json::to_value(&resp).unwrap();
        assert!(serialized.get("entityType").is_some());
        assert!(serialized.get("entityId").is_some());
        assert!(serialized.get("filePath").is_some());
        assert!(serialized.get("mimeType").is_some());
        assert!(serialized.get("fileSize").is_some());
        assert!(serialized.get("uploadedBy").is_some());
        assert!(serialized.get("createdAt").is_some());
        assert!(serialized.get("tipoDocumento").is_some());
        assert!(serialized.get("estadoVerificacion").is_some());
        assert!(serialized.get("fechaVencimiento").is_some());
        assert!(serialized.get("verificadoPor").is_some());
        assert!(serialized.get("fechaVerificacion").is_some());
        assert!(serialized.get("notasVerificacion").is_some());
        assert!(serialized.get("numeroDocumento").is_some());
        assert!(serialized.get("contenidoEditable").is_some());
        assert!(serialized.get("updatedAt").is_some());
        // Ensure snake_case keys are NOT present
        assert!(serialized.get("entity_type").is_none());
        assert!(serialized.get("tipo_documento").is_none());
        assert!(serialized.get("estado_verificacion").is_none());
    }

    #[test]
    fn documento_response_serializes_with_populated_optional_fields() {
        let id = Uuid::new_v4();
        let entity_id = Uuid::new_v4();
        let uploaded_by = Uuid::new_v4();
        let verificado_por = Uuid::new_v4();
        let now = Utc::now();
        let resp = DocumentoResponse {
            id,
            entity_type: "contrato".to_string(),
            entity_id,
            filename: "contrato.pdf".to_string(),
            file_path: "/uploads/contrato/abc/contrato.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            file_size: 2048,
            uploaded_by,
            created_at: now,
            tipo_documento: "contrato_arrendamiento".to_string(),
            estado_verificacion: "verificado".to_string(),
            fecha_vencimiento: Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
            verificado_por: Some(verificado_por),
            fecha_verificacion: Some(now),
            notas_verificacion: Some("Documento verificado correctamente".to_string()),
            numero_documento: Some("DOC-001".to_string()),
            contenido_editable: Some(serde_json::json!({"version": 1, "blocks": []})),
            updated_at: Some(now),
        };
        let serialized = serde_json::to_value(&resp).unwrap();
        assert_eq!(serialized["tipoDocumento"], "contrato_arrendamiento");
        assert_eq!(serialized["estadoVerificacion"], "verificado");
        assert_eq!(serialized["fechaVencimiento"], "2025-12-31");
        assert_eq!(serialized["verificadoPor"], verificado_por.to_string());
        assert!(serialized["contenidoEditable"].is_object());
    }

    #[test]
    fn cumplimiento_response_serializes_correctly() {
        let entity_id = Uuid::new_v4();
        let resp = CumplimientoResponse {
            entity_type: "inquilino".to_string(),
            entity_id,
            documentos: vec![
                CumplimientoItem {
                    tipo_documento: "cedula".to_string(),
                    nombre: "Cédula de Identidad".to_string(),
                    requerido: true,
                    estado: "presente".to_string(),
                },
                CumplimientoItem {
                    tipo_documento: "comprobante_ingresos".to_string(),
                    nombre: "Comprobante de Ingresos".to_string(),
                    requerido: true,
                    estado: "faltante".to_string(),
                },
            ],
            porcentaje: 50,
        };
        let serialized = serde_json::to_value(&resp).unwrap();
        assert_eq!(serialized["entityType"], "inquilino");
        assert_eq!(serialized["porcentaje"], 50);
        let docs = serialized["documentos"].as_array().unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0]["tipoDocumento"], "cedula");
        assert_eq!(docs[1]["estado"], "faltante");
    }

    #[test]
    fn plantilla_response_serializes_correctly() {
        let id = Uuid::new_v4();
        let resp = PlantillaResponse {
            id,
            nombre: "Contrato de Arrendamiento".to_string(),
            tipo_documento: "contrato_arrendamiento".to_string(),
            entity_type: "contrato".to_string(),
            contenido: serde_json::json!({"version": 1, "blocks": []}),
        };
        let serialized = serde_json::to_value(&resp).unwrap();
        assert_eq!(serialized["tipoDocumento"], "contrato_arrendamiento");
        assert_eq!(serialized["entityType"], "contrato");
        assert!(serialized["contenido"].is_object());
    }

    #[test]
    fn digitalizar_response_serializes_correctly() {
        let doc_id = Uuid::new_v4();
        let resp = DigitalizarResponse {
            document_type: "cedula".to_string(),
            contenido_editable: serde_json::json!({"version": 1, "blocks": []}),
            campos_baja_confianza: vec!["nombre".to_string(), "fecha".to_string()],
            documento_original_id: doc_id,
        };
        let serialized = serde_json::to_value(&resp).unwrap();
        assert_eq!(serialized["documentType"], "cedula");
        assert!(serialized["contenidoEditable"].is_object());
        let campos = serialized["camposBajaConfianza"].as_array().unwrap();
        assert_eq!(campos.len(), 2);
        assert_eq!(serialized["documentoOriginalId"], doc_id.to_string());
    }

    #[test]
    fn verificar_documento_request_deserializes() {
        let json = r#"{"estadoVerificacion":"verificado","notasVerificacion":"Todo correcto"}"#;
        let req: VerificarDocumentoRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.estado_verificacion, "verificado");
        assert_eq!(req.notas_verificacion.unwrap(), "Todo correcto");
    }

    #[test]
    fn documento_list_query_deserializes() {
        let json = r#"{"tipoDocumento":"cedula","estadoVerificacion":"pendiente"}"#;
        let query: DocumentoListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.tipo_documento.unwrap(), "cedula");
        assert_eq!(query.estado_verificacion.unwrap(), "pendiente");
        assert!(query.fecha_vencimiento_desde.is_none());
        assert!(query.fecha_vencimiento_hasta.is_none());
    }

    #[test]
    fn por_vencer_query_deserializes() {
        let json = r#"{"dias":60}"#;
        let query: PorVencerQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.dias.unwrap(), 60);
    }

    #[test]
    fn guardar_editor_request_deserializes() {
        let json = r#"{"contenidoEditable":{"version":1,"blocks":[]}}"#;
        let req: GuardarEditorRequest = serde_json::from_str(json).unwrap();
        assert!(req.contenido_editable.is_object());
    }

    #[test]
    fn plantilla_rellenada_response_serializes_correctly() {
        let id = Uuid::new_v4();
        let resp = PlantillaRellenadaResponse {
            plantilla_id: id,
            nombre: "Recibo de Pago".to_string(),
            tipo_documento: "recibo_pago".to_string(),
            contenido: serde_json::json!({"version": 1, "blocks": [{"type": "paragraph", "text": "Monto: RD$ 25,000.00"}]}),
        };
        let serialized = serde_json::to_value(&resp).unwrap();
        assert_eq!(serialized["plantillaId"], id.to_string());
        assert_eq!(serialized["tipoDocumento"], "recibo_pago");
        assert!(serialized["contenido"]["blocks"].is_array());
    }
}
