use rig::completion::ToolDefinition;
use rig::tool::Tool;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::models::chatbot::Confidence;
use crate::services::ocr_client::OcrClient;

// =============================================================================
// ExtractReceiptTool — extracts structured payment data from a receipt image
// =============================================================================

/// Structured payment receipt data extracted via OCR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentReceipt {
    pub bank: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub date: Option<String>,
    pub reference: Option<String>,
    pub sender_name: Option<String>,
    pub recipient: Option<String>,
    pub confidence: Confidence,
}

/// Input provided by the LLM when invoking the `extract_receipt` tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractReceiptInput {
    pub image_base64: String,
    #[serde(default)]
    pub caption: Option<String>,
}

/// A media store that receives the image inline as base64 (no external fetch needed).
#[derive(Debug, Clone, Copy)]
pub struct InlineBase64MediaStore;

/// Tool that extracts structured receipt data from a base64-encoded image.
pub struct ExtractReceiptTool {
    pub media_store: InlineBase64MediaStore,
    pub ocr: OcrClient,
}

/// Error type specific to the receipt extraction tool.
#[derive(Debug, thiserror::Error)]
pub enum ExtractReceiptError {
    #[error("Error de OCR: {0}")]
    Ocr(String),
    #[error("Error de decodificación: {0}")]
    Decode(String),
    #[error("Error de parseo: {0}")]
    Parse(String),
}

impl Tool for ExtractReceiptTool {
    const NAME: &'static str = "extract_receipt";

    type Error = ExtractReceiptError;
    type Args = ExtractReceiptInput;
    type Output = PaymentReceipt;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name(),
            description: "Extrae datos estructurados de un comprobante de pago a partir de una imagen. Usa esta herramienta cuando el usuario envía una foto de un recibo o comprobante de transferencia.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "image_base64": {
                        "type": "string",
                        "description": "Imagen del recibo codificada en base64"
                    },
                    "caption": {
                        "type": "string",
                        "description": "Texto opcional que acompaña la imagen"
                    }
                },
                "required": ["image_base64"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        use crate::models::chatbot::map_confidence;
        use base64::Engine;

        let image_data = base64::engine::general_purpose::STANDARD
            .decode(&args.image_base64)
            .map_err(|e| ExtractReceiptError::Decode(e.to_string()))?;

        let content_type = infer::get(&image_data).map_or("image/jpeg", |t| t.mime_type());

        let ocr_result = self
            .ocr
            .extract(&image_data, "receipt.jpg", content_type, Some("receipt"))
            .await
            .map_err(|e| ExtractReceiptError::Ocr(e.to_string()))?;

        // Extract fields from OCR structured_fields
        let fields = &ocr_result.structured_fields;

        let amount: Decimal = fields
            .get("amount")
            .and_then(|v| v.parse().ok())
            .unwrap_or_default();

        let currency = fields
            .get("currency")
            .cloned()
            .unwrap_or_else(|| "DOP".to_string());

        let avg_confidence = if ocr_result.lines.is_empty() {
            0.0
        } else {
            ocr_result.lines.iter().map(|l| l.confidence).sum::<f64>()
                / ocr_result.lines.len() as f64
        };

        Ok(PaymentReceipt {
            bank: fields.get("bank").cloned(),
            amount,
            currency,
            date: fields.get("date").cloned(),
            reference: fields.get("reference").cloned(),
            sender_name: fields.get("sender_name").cloned(),
            recipient: fields.get("recipient").cloned(),
            confidence: map_confidence(avg_confidence),
        })
    }
}
