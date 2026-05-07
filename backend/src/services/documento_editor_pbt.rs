#![allow(clippy::expect_used, clippy::unwrap_used)]

use proptest::prelude::*;
use std::io::{Cursor, Read};

use super::documento_editor::build_docx;

// ── Strategies for generating valid Block_JSON ──────────────────

fn arb_text() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 áéíóúñÁÉÍÓÚÑ]{1,50}"
}

fn arb_heading_block() -> impl Strategy<Value = serde_json::Value> {
    (arb_text(), 1u64..=3).prop_map(|(text, level)| {
        serde_json::json!({
            "type": "heading",
            "text": text,
            "level": level
        })
    })
}

fn arb_paragraph_block() -> impl Strategy<Value = serde_json::Value> {
    arb_text().prop_map(|text| {
        serde_json::json!({
            "type": "paragraph",
            "text": text
        })
    })
}

fn arb_list_block() -> impl Strategy<Value = serde_json::Value> {
    (
        prop::collection::vec(arb_text(), 1..=5),
        any::<bool>(),
    )
        .prop_map(|(items, ordered)| {
            serde_json::json!({
                "type": "list",
                "items": items,
                "ordered": ordered
            })
        })
}

fn arb_table_block() -> impl Strategy<Value = serde_json::Value> {
    (
        prop::collection::vec(arb_text(), 2..=4),
        prop::collection::vec(prop::collection::vec(arb_text(), 2..=4), 1..=3),
    )
        .prop_map(|(headers, rows)| {
            serde_json::json!({
                "type": "table",
                "headers": headers,
                "rows": rows
            })
        })
}

fn arb_page_break_block() -> impl Strategy<Value = serde_json::Value> {
    Just(serde_json::json!({"type": "page_break"}))
}

fn arb_block() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        arb_heading_block(),
        arb_paragraph_block(),
        arb_list_block(),
        arb_table_block(),
        arb_page_break_block(),
    ]
}

fn arb_blocks() -> impl Strategy<Value = Vec<serde_json::Value>> {
    prop::collection::vec(arb_block(), 1..=10)
}

/// Blocks that always contain text (heading, paragraph, list only).
fn arb_text_blocks() -> impl Strategy<Value = Vec<serde_json::Value>> {
    prop::collection::vec(
        prop_oneof![
            arb_heading_block(),
            arb_paragraph_block(),
            arb_list_block(),
        ],
        1..=8,
    )
}

// ── Helper: pack Docx into bytes ────────────────────────────────

fn pack_docx_bytes(blocks: &[serde_json::Value]) -> Vec<u8> {
    let docx = build_docx(blocks).expect("build_docx should succeed for valid blocks");
    let mut buf = Vec::new();
    docx.build()
        .pack(&mut Cursor::new(&mut buf))
        .expect("pack should succeed");
    buf
}

/// Extract all XML content from a DOCX zip archive as a single string.
fn extract_docx_xml(bytes: &[u8]) -> String {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("DOCX should be a valid ZIP");
    let mut all_xml = String::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).expect("zip entry should be readable");
        if std::path::Path::new(file.name())
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"))
            || std::path::Path::new(file.name())
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("rels"))
        {
            let mut content = String::new();
            file.read_to_string(&mut content)
                .expect("xml file should be readable as UTF-8");
            all_xml.push_str(&content);
        }
    }
    all_xml
}

/// Extract text strings from blocks for verification.
fn extract_text_from_blocks(blocks: &[serde_json::Value]) -> Vec<String> {
    let mut texts = Vec::new();
    for block in blocks {
        let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match block_type {
            "heading" | "paragraph" => {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    if !text.is_empty() {
                        texts.push(text.to_string());
                    }
                }
            }
            "list" => {
                if let Some(items) = block.get("items").and_then(|i| i.as_array()) {
                    for item in items {
                        if let Some(text) = item.as_str() {
                            if !text.is_empty() {
                                texts.push(text.to_string());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    texts
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    // Feature: contract-document-signing, Property 1: DOCX export produces valid output for any Block_JSON
    /// **Validates: Requirements 1.1**
    #[test]
    fn docx_export_produces_valid_zip_output(blocks in arb_blocks()) {
        let bytes = pack_docx_bytes(&blocks);

        // Output must be non-empty
        prop_assert!(
            !bytes.is_empty(),
            "DOCX output must be non-empty"
        );

        // Output must start with ZIP magic bytes PK\x03\x04
        prop_assert!(
            bytes.len() >= 4,
            "DOCX output must be at least 4 bytes, got {}",
            bytes.len()
        );
        prop_assert_eq!(
            &bytes[0..4],
            &[0x50, 0x4B, 0x03, 0x04],
            "DOCX output must start with ZIP magic bytes PK\\x03\\x04, got {:?}",
            &bytes[0..4]
        );
    }

    // Feature: contract-document-signing, Property 2: DOCX export preserves all text content
    /// **Validates: Requirements 1.4**
    #[test]
    fn docx_export_preserves_all_text_content(blocks in arb_text_blocks()) {
        let bytes = pack_docx_bytes(&blocks);
        let xml_content = extract_docx_xml(&bytes);
        let expected_texts = extract_text_from_blocks(&blocks);

        for text in &expected_texts {
            prop_assert!(
                xml_content.contains(text),
                "DOCX XML should contain text '{}' but it was not found.\nXML length: {}",
                text,
                xml_content.len()
            );
        }
    }
}
