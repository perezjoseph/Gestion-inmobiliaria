use chrono::Utc;
use numaelis_rckive_genpdf::Element as _;
use numaelis_rckive_genpdf::{elements, style};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use uuid::Uuid;

use crate::entities::{documento, plantilla_documento};
use crate::errors::AppError;
use crate::models::documento::{DigitalizarResponse, DocumentoResponse};
use crate::models::ocr::{OcrLine, OcrResult};
use crate::services::{auditoria, documentos, ocr_client::OcrClient};

// ── Vertical proximity threshold (in bbox units) for paragraph grouping ──
const PARAGRAPH_GAP_THRESHOLD: f64 = 15.0;

// ── Confidence threshold for flagging low-confidence fields ──
const LOW_CONFIDENCE_THRESHOLD: f64 = 0.80;

// ── Digitalizar: OCR scan → editable document ──────────────────

/// Upload a scanned file, run OCR, and convert the result to an editable
/// document in the editor JSON format.
pub async fn digitalizar(
    db: &DatabaseConnection,
    entity_type: &str,
    entity_id: Uuid,
    file_data: &[u8],
    filename: &str,
    mime_type: &str,
    uploaded_by: Uuid,
) -> Result<DigitalizarResponse, AppError> {
    // 1. Store the original file as a Documento with tipo_documento="otro"
    let doc_response = documentos::upload(
        db,
        entity_type,
        entity_id,
        file_data,
        filename,
        mime_type,
        uploaded_by,
        "otro",
        None, // fecha_vencimiento
        None, // numero_documento
        None, // notas_verificacion
    )
    .await?;

    // 2. Call OCR service to extract text
    let ocr_client = OcrClient::new()?;
    let ocr_result = ocr_client
        .extract(file_data, filename, mime_type, None)
        .await?;

    // 3. Convert OcrResult to editor JSON
    let (contenido_editable, campos_baja_confianza) =
        convertir_ocr_a_editor(db, &ocr_result).await?;

    // 4. Store the editor content on the original document
    let doc_model = documento::Entity::find_by_id(doc_response.id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Documento recién creado no encontrado: {}",
                doc_response.id
            ))
        })?;

    let mut active: documento::ActiveModel = doc_model.into_active_model();
    active.contenido_editable = Set(Some(contenido_editable.clone()));
    active.updated_at = Set(Some(Utc::now().into()));
    active.update(db).await?;

    Ok(DigitalizarResponse {
        document_type: ocr_result.document_type,
        contenido_editable,
        campos_baja_confianza,
        documento_original_id: doc_response.id,
    })
}

// ── OCR-to-Editor conversion ───────────────────────────────────

/// Convert an `OcrResult` into editor JSON format.
///
/// Groups OCR lines into paragraphs by vertical proximity (bbox y-coordinates),
/// sets confidence per block as the minimum confidence of constituent lines,
/// and attempts to match a Plantilla if the document type is known.
async fn convertir_ocr_a_editor(
    db: &DatabaseConnection,
    ocr_result: &OcrResult,
) -> Result<(serde_json::Value, Vec<String>), AppError> {
    let paragraphs = group_lines_into_paragraphs(&ocr_result.lines);

    let mut blocks = Vec::with_capacity(paragraphs.len());
    let mut campos_baja_confianza = Vec::new();

    for (idx, paragraph) in paragraphs.iter().enumerate() {
        let text: String = paragraph
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let confidence = paragraph
            .iter()
            .map(|line| line.confidence)
            .fold(f64::INFINITY, f64::min);

        let mut block = serde_json::json!({
            "type": "paragraph",
            "text": text,
            "confidence": confidence,
            "source": "ocr"
        });

        if confidence < LOW_CONFIDENCE_THRESHOLD {
            let field_name = format!("bloque_{}", idx + 1);
            campos_baja_confianza.push(field_name.clone());
            block["low_confidence"] = serde_json::Value::Bool(true);
        }

        blocks.push(block);
    }

    // If document_type is known (not "unknown"), try to match a Plantilla and merge fields
    if ocr_result.document_type != "unknown" && !ocr_result.document_type.is_empty() {
        if let Some(plantilla) = find_matching_plantilla(db, &ocr_result.document_type).await? {
            merge_plantilla_fields(
                &mut blocks,
                &plantilla.contenido,
                &ocr_result.structured_fields,
            );
        }
    }

    let contenido = serde_json::json!({
        "version": 1,
        "blocks": blocks
    });

    Ok((contenido, campos_baja_confianza))
}

/// Group OCR lines into paragraphs based on vertical proximity of bounding boxes.
/// Lines whose y-coordinate gap exceeds `PARAGRAPH_GAP_THRESHOLD` start a new paragraph.
fn group_lines_into_paragraphs(lines: &[OcrLine]) -> Vec<Vec<&OcrLine>> {
    if lines.is_empty() {
        return Vec::new();
    }

    let mut paragraphs: Vec<Vec<&OcrLine>> = Vec::new();
    let mut current_paragraph: Vec<&OcrLine> = vec![&lines[0]];

    for i in 1..lines.len() {
        let prev_line = lines[i - 1].bbox.get(3).copied().unwrap_or(0.0); // bottom y of previous
        let curr_line = lines[i].bbox.get(1).copied().unwrap_or(0.0); // top y of current

        let gap = (curr_line - prev_line).abs();

        if gap > PARAGRAPH_GAP_THRESHOLD {
            paragraphs.push(current_paragraph);
            current_paragraph = Vec::new();
        }

        current_paragraph.push(&lines[i]);
    }

    if !current_paragraph.is_empty() {
        paragraphs.push(current_paragraph);
    }

    paragraphs
}

/// Find a Plantilla matching the OCR-detected document type.
async fn find_matching_plantilla(
    db: &DatabaseConnection,
    document_type: &str,
) -> Result<Option<plantilla_documento::Model>, AppError> {
    let plantilla = plantilla_documento::Entity::find()
        .filter(plantilla_documento::Column::TipoDocumento.eq(document_type))
        .filter(plantilla_documento::Column::Activo.eq(true))
        .one(db)
        .await?;

    Ok(plantilla)
}

/// Merge structured OCR fields into the editor blocks using the template structure.
/// If the template has placeholder fields that match OCR `structured_fields`, inject them.
fn merge_plantilla_fields(
    blocks: &mut Vec<serde_json::Value>,
    plantilla_contenido: &serde_json::Value,
    structured_fields: &std::collections::HashMap<String, String>,
) {
    if structured_fields.is_empty() {
        return;
    }

    // Extract template blocks to understand the expected structure
    let template_blocks = plantilla_contenido.get("blocks").and_then(|b| b.as_array());

    if let Some(template_blocks) = template_blocks {
        // For each template block, check if it contains placeholders that match OCR fields
        for template_block in template_blocks {
            if let Some(text) = template_block.get("text").and_then(|t| t.as_str()) {
                let mut resolved_text = text.to_string();
                let mut has_replacements = false;

                for (key, value) in structured_fields {
                    let placeholder = format!("{{{{{key}}}}}");
                    if resolved_text.contains(&placeholder) {
                        resolved_text = resolved_text.replace(&placeholder, value);
                        has_replacements = true;
                    }
                }

                if has_replacements {
                    let block_type = template_block
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("paragraph");
                    let level = template_block.get("level");

                    let mut new_block = serde_json::json!({
                        "type": block_type,
                        "text": resolved_text,
                        "source": "template_ocr"
                    });

                    if let Some(level) = level {
                        new_block["level"] = level.clone();
                    }

                    blocks.push(new_block);
                }
            }
        }
    }
}

// ── Guardar contenido ──────────────────────────────────────────

/// Save editor content to a document's `contenido_editable` field.
pub async fn guardar_contenido(
    db: &DatabaseConnection,
    documento_id: Uuid,
    contenido: serde_json::Value,
    usuario_id: Uuid,
) -> Result<DocumentoResponse, AppError> {
    let doc = documento::Entity::find_by_id(documento_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Documento {documento_id} no encontrado")))?;

    let mut active: documento::ActiveModel = doc.into_active_model();
    active.contenido_editable = Set(Some(contenido.clone()));
    active.updated_at = Set(Some(Utc::now().into()));

    let updated = active.update(db).await?;

    // Audit trail (best-effort)
    auditoria::registrar_best_effort(
        db,
        auditoria::CreateAuditoriaEntry {
            usuario_id,
            entity_type: "documento".to_string(),
            entity_id: documento_id,
            accion: "guardar_editable".to_string(),
            cambios: serde_json::json!({
                "documento_id": documento_id.to_string(),
            }),
        },
    )
    .await;

    Ok(model_to_response(updated))
}

// ── Exportar PDF ───────────────────────────────────────────────

/// Export a document's `contenido_editable` as a PDF.
pub async fn exportar_pdf(
    db: &DatabaseConnection,
    documento_id: Uuid,
) -> Result<Vec<u8>, AppError> {
    let doc = documento::Entity::find_by_id(documento_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Documento {documento_id} no encontrado")))?;

    let contenido = doc.contenido_editable.ok_or_else(|| {
        AppError::Validation("El documento no tiene contenido editable para exportar".to_string())
    })?;

    let blocks = contenido
        .get("blocks")
        .and_then(|b| b.as_array())
        .ok_or_else(|| {
            AppError::Validation(
                "El contenido editable no tiene un formato válido (falta 'blocks')".to_string(),
            )
        })?;

    let font_family = load_font_family()?;
    let mut pdf_doc = numaelis_rckive_genpdf::Document::new(font_family);
    pdf_doc.set_title("Documento Exportado");
    pdf_doc.set_minimal_conformance();

    let mut decorator = numaelis_rckive_genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    pdf_doc.set_page_decorator(decorator);

    for block in blocks {
        render_block(&mut pdf_doc, block);
    }

    let mut buf: Vec<u8> = Vec::new();
    pdf_doc
        .render(&mut buf)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error renderizando PDF: {e}")))?;

    Ok(buf)
}

// ── PDF rendering helpers ──────────────────────────────────────

fn load_font_family() -> Result<
    numaelis_rckive_genpdf::fonts::FontFamily<numaelis_rckive_genpdf::fonts::FontData>,
    AppError,
> {
    let regular = numaelis_rckive_genpdf::fonts::FontData::new(
        include_bytes!("../../fonts/Arial-Regular.ttf").to_vec(),
        None,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error cargando fuente regular: {e}")))?;
    let bold = numaelis_rckive_genpdf::fonts::FontData::new(
        include_bytes!("../../fonts/Arial-Bold.ttf").to_vec(),
        None,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error cargando fuente bold: {e}")))?;
    let italic = numaelis_rckive_genpdf::fonts::FontData::new(
        include_bytes!("../../fonts/Arial-Italic.ttf").to_vec(),
        None,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error cargando fuente italic: {e}")))?;
    let bold_italic = numaelis_rckive_genpdf::fonts::FontData::new(
        include_bytes!("../../fonts/Arial-BoldItalic.ttf").to_vec(),
        None,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error cargando fuente bold-italic: {e}")))?;

    Ok(numaelis_rckive_genpdf::fonts::FontFamily {
        regular,
        bold,
        italic,
        bold_italic,
    })
}

/// Render a single editor block into the PDF document.
fn render_block(doc: &mut numaelis_rckive_genpdf::Document, block: &serde_json::Value) {
    let block_type = block
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("paragraph");

    match block_type {
        "heading" => render_heading(doc, block),
        "paragraph" => render_paragraph(doc, block),
        "list" => render_list(doc, block),
        "table" => render_table(doc, block),
        "page_break" => {
            doc.push(elements::PageBreak::new());
        }
        _ => {
            // Unknown block type: render as paragraph
            render_paragraph(doc, block);
        }
    }
}

fn render_heading(doc: &mut numaelis_rckive_genpdf::Document, block: &serde_json::Value) {
    let text = block
        .get("text")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let level = block
        .get("level")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1);

    let font_size = match level {
        1 => 18,
        2 => 15,
        3 => 13,
        _ => 12,
    };

    doc.push(elements::Break::new(0.5));
    doc.push(
        elements::Paragraph::new(text).styled(style::Style::new().bold().with_font_size(font_size)),
    );
    doc.push(elements::Break::new(0.3));
}

fn render_paragraph(doc: &mut numaelis_rckive_genpdf::Document, block: &serde_json::Value) {
    let text = block.get("text").and_then(|t| t.as_str()).unwrap_or("");

    if !text.is_empty() {
        doc.push(elements::Paragraph::new(text).styled(style::Style::new().with_font_size(11)));
        doc.push(elements::Break::new(0.2));
    }
}

fn render_list(doc: &mut numaelis_rckive_genpdf::Document, block: &serde_json::Value) {
    let _ordered = block
        .get("ordered")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let items = block.get("items").and_then(serde_json::Value::as_array);

    if let Some(items) = items {
        let mut list = elements::OrderedList::new();

        for item in items {
            let text = item.as_str().unwrap_or("");
            list.push(elements::Paragraph::new(text));
        }

        doc.push(list);
        doc.push(elements::Break::new(0.2));
    }
}

fn render_table(doc: &mut numaelis_rckive_genpdf::Document, block: &serde_json::Value) {
    let headers = block.get("headers").and_then(serde_json::Value::as_array);
    let rows = block.get("rows").and_then(serde_json::Value::as_array);

    let col_count = headers
        .map(Vec::len)
        .or_else(|| {
            rows.and_then(|r| r.first())
                .and_then(serde_json::Value::as_array)
                .map(Vec::len)
        })
        .unwrap_or(0);

    if col_count == 0 {
        return;
    }

    let weights: Vec<usize> = vec![1; col_count];
    let mut table = elements::TableLayout::new(weights);
    table.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));

    // Header row
    if let Some(headers) = headers {
        let mut row = table.row();
        for header in headers {
            let text = header.as_str().unwrap_or("");
            row.push_element(
                elements::Paragraph::new(text)
                    .styled(style::Style::new().bold().with_font_size(10)),
            );
        }
        row.push()
            .map_err(|e| {
                tracing::error!("Error adding table header row: {e}");
            })
            .ok();
    }

    // Data rows
    if let Some(rows) = rows {
        for row_data in rows {
            if let Some(cells) = row_data.as_array() {
                let mut row = table.row();
                for cell in cells {
                    let text = cell.as_str().unwrap_or("");
                    row.push_element(
                        elements::Paragraph::new(text)
                            .styled(style::Style::new().with_font_size(10)),
                    );
                }
                row.push()
                    .map_err(|e| {
                        tracing::error!("Error adding table data row: {e}");
                    })
                    .ok();
            }
        }
    }

    doc.push(table);
    doc.push(elements::Break::new(0.3));
}

// ── model_to_response (same pattern as documentos.rs) ──────────

fn model_to_response(d: documento::Model) -> DocumentoResponse {
    DocumentoResponse {
        id: d.id,
        entity_type: d.entity_type,
        entity_id: d.entity_id,
        filename: d.filename,
        file_path: d.file_path,
        mime_type: d.mime_type,
        file_size: d.file_size,
        uploaded_by: d.uploaded_by,
        created_at: d.created_at.into(),
        tipo_documento: d.tipo_documento,
        estado_verificacion: d.estado_verificacion,
        fecha_vencimiento: d.fecha_vencimiento,
        verificado_por: d.verificado_por,
        fecha_verificacion: d.fecha_verificacion.map(Into::into),
        notas_verificacion: d.notas_verificacion,
        numero_documento: d.numero_documento,
        contenido_editable: d.contenido_editable,
        updated_at: d.updated_at.map(Into::into),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::models::ocr::OcrLine;

    #[test]
    fn group_lines_empty_input() {
        let result = group_lines_into_paragraphs(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn group_lines_single_line() {
        let lines = vec![OcrLine {
            text: "Hello".to_string(),
            confidence: 0.95,
            bbox: vec![0.0, 0.0, 100.0, 10.0],
        }];
        let result = group_lines_into_paragraphs(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].text, "Hello");
    }

    #[test]
    fn group_lines_close_lines_same_paragraph() {
        let lines = vec![
            OcrLine {
                text: "Line 1".to_string(),
                confidence: 0.90,
                bbox: vec![0.0, 0.0, 100.0, 10.0],
            },
            OcrLine {
                text: "Line 2".to_string(),
                confidence: 0.85,
                bbox: vec![0.0, 12.0, 100.0, 22.0], // gap = |12 - 10| = 2 < 15
            },
        ];
        let result = group_lines_into_paragraphs(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 2);
    }

    #[test]
    fn group_lines_far_lines_different_paragraphs() {
        let lines = vec![
            OcrLine {
                text: "Paragraph 1".to_string(),
                confidence: 0.90,
                bbox: vec![0.0, 0.0, 100.0, 10.0],
            },
            OcrLine {
                text: "Paragraph 2".to_string(),
                confidence: 0.85,
                bbox: vec![0.0, 50.0, 100.0, 60.0], // gap = |50 - 10| = 40 > 15
            },
        ];
        let result = group_lines_into_paragraphs(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0][0].text, "Paragraph 1");
        assert_eq!(result[1][0].text, "Paragraph 2");
    }

    #[test]
    fn group_lines_confidence_below_threshold_flagged() {
        let lines = vec![
            OcrLine {
                text: "Clear text".to_string(),
                confidence: 0.95,
                bbox: vec![0.0, 0.0, 100.0, 10.0],
            },
            OcrLine {
                text: "Blurry text".to_string(),
                confidence: 0.60,
                bbox: vec![0.0, 50.0, 100.0, 60.0],
            },
        ];
        let paragraphs = group_lines_into_paragraphs(&lines);
        assert_eq!(paragraphs.len(), 2);

        // Check that the second paragraph has low confidence
        let min_conf = paragraphs[1]
            .iter()
            .map(|l| l.confidence)
            .fold(f64::INFINITY, f64::min);
        assert!(min_conf < LOW_CONFIDENCE_THRESHOLD);
    }

    #[test]
    fn merge_plantilla_fields_with_empty_structured_fields() {
        let mut blocks = vec![serde_json::json!({"type": "paragraph", "text": "Hello"})];
        let plantilla = serde_json::json!({"blocks": [{"type": "paragraph", "text": "{{name}}"}]});
        let fields = std::collections::HashMap::new();

        merge_plantilla_fields(&mut blocks, &plantilla, &fields);
        // No new blocks should be added
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn merge_plantilla_fields_resolves_placeholders() {
        let mut blocks = Vec::new();
        let plantilla = serde_json::json!({
            "blocks": [
                {"type": "heading", "level": 1, "text": "Contrato de {{inquilino.nombre}}"},
                {"type": "paragraph", "text": "Dirección: {{propiedad.direccion}}"}
            ]
        });
        let mut fields = std::collections::HashMap::new();
        fields.insert("inquilino.nombre".to_string(), "Juan Pérez".to_string());
        fields.insert(
            "propiedad.direccion".to_string(),
            "Calle Principal #1".to_string(),
        );

        merge_plantilla_fields(&mut blocks, &plantilla, &fields);
        assert_eq!(blocks.len(), 2);
        assert_eq!(
            blocks[0]["text"].as_str().unwrap(),
            "Contrato de Juan Pérez"
        );
        assert_eq!(
            blocks[1]["text"].as_str().unwrap(),
            "Dirección: Calle Principal #1"
        );
    }
}
