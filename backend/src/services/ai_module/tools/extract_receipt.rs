use base64::Engine;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::models::chatbot::{Confidence, map_confidence};
use crate::services::ocr_client::OcrClient;

// =============================================================================
// MediaStore
// =============================================================================

/// Errors from media fetching.
#[derive(Debug, thiserror::Error)]
pub enum MediaStoreError {
    #[error("Error decodificando media: {0}")]
    Decode(String),
    #[error("Media no encontrada: {0}")]
    NotFound(String),
    #[error("Error de red: {0}")]
    Network(String),
}

/// Media store that decodes inline base64 data.
///
/// In the `WhatsApp` flow the `media_id` is currently the base64-encoded image
/// data itself. This struct can be replaced with a remote media store (e.g. S3,
/// `WhatsApp` Cloud API download) without changing the tool's public interface.
#[derive(Clone)]
pub struct InlineBase64MediaStore;

impl InlineBase64MediaStore {
    /// Fetch raw bytes for the given media identifier (base64-encoded data).
    #[allow(clippy::unused_async)]
    pub async fn fetch(&self, media_id: &str) -> Result<Vec<u8>, MediaStoreError> {
        base64::engine::general_purpose::STANDARD
            .decode(media_id)
            .map_err(|e| MediaStoreError::Decode(format!("Error decodificando base64: {e}")))
    }
}

// =============================================================================
// ExtractReceiptTool
// =============================================================================

/// Input the LLM provides when it wants to extract receipt data from an image.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct ExtractReceiptInput {
    /// Imagen del recibo codificada en base64
    pub image_base64: String,
    /// Texto opcional que acompaña la imagen
    pub caption: Option<String>,
}

/// Structured payment receipt data extracted from an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentReceipt {
    pub bank: Option<String>,
    pub amount: Decimal,
    /// `"DOP"` or `"USD"`.
    pub currency: String,
    /// ISO 8601 date string.
    pub date: Option<String>,
    /// Reference number (up to 50 characters).
    pub reference: Option<String>,
    pub sender_name: Option<String>,
    pub recipient: Option<String>,
    pub confidence: Confidence,
}

/// Rig tool that fetches media via [`InlineBase64MediaStore`], runs OCR via
/// [`OcrClient`] (targeting OVMS `/v3`), and returns a [`PaymentReceipt`].
pub struct ExtractReceiptTool {
    pub media_store: InlineBase64MediaStore,
    pub ocr: OcrClient,
}

/// Error type for the extract receipt tool.
#[derive(Debug, thiserror::Error)]
pub enum ExtractReceiptError {
    #[error("Error de servicio: {0}")]
    Service(String),
    #[error("Error de validación: {0}")]
    Validation(String),
}

impl Tool for ExtractReceiptTool {
    const NAME: &'static str = "extract_receipt";

    type Error = ExtractReceiptError;
    type Args = ExtractReceiptInput;
    type Output = PaymentReceipt;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name(),
            description: "Extrae datos estructurados de un comprobante de pago a partir de una \
                imagen. Usa esta herramienta cuando el usuario envía una foto de un recibo o \
                comprobante de transferencia."
                .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(Self::Args)).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Fetch media bytes via the media store (decodes base64 in the default impl)
        let bytes = self
            .media_store
            .fetch(&args.image_base64)
            .await
            .map_err(|e| ExtractReceiptError::Validation(e.to_string()))?;

        // Call OCR service (targets OVMS /v3)
        let ocr_result = self
            .ocr
            .extract(&bytes, "receipt.jpg", "image/jpeg", Some("receipt"))
            .await
            .map_err(|e| ExtractReceiptError::Service(format!("Error en servicio OCR: {e}")))?;

        // Map OcrResult structured_fields to PaymentReceipt
        let fields = &ocr_result.structured_fields;

        let amount_str = fields
            .get("amount")
            .or_else(|| fields.get("monto"))
            .cloned()
            .unwrap_or_default();
        let amount = amount_str
            .replace(',', "")
            .replace("RD$", "")
            .replace("US$", "")
            .trim()
            .parse::<Decimal>()
            .unwrap_or(Decimal::ZERO);

        let currency = fields
            .get("currency")
            .or_else(|| fields.get("moneda"))
            .cloned()
            .unwrap_or_else(|| "DOP".to_string());

        // Calculate average confidence from OCR lines
        let avg_confidence = if ocr_result.lines.is_empty() {
            0.0
        } else {
            ocr_result.lines.iter().map(|l| l.confidence).sum::<f64>()
                / ocr_result.lines.len() as f64
        };

        Ok(PaymentReceipt {
            bank: fields.get("bank").or_else(|| fields.get("banco")).cloned(),
            amount,
            currency,
            date: fields.get("date").or_else(|| fields.get("fecha")).cloned(),
            reference: fields
                .get("reference")
                .or_else(|| fields.get("referencia"))
                .cloned(),
            sender_name: fields
                .get("sender_name")
                .or_else(|| fields.get("remitente"))
                .cloned(),
            recipient: fields
                .get("recipient")
                .or_else(|| fields.get("destinatario"))
                .cloned(),
            confidence: map_confidence(avg_confidence),
        })
    }
}
