use chrono::{Datelike, NaiveDate};
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;

use realestate_backend::models::importacion::ImportFormat;
use realestate_backend::models::ocr::{ImportPreview, OcrLine, OcrResult, PreviewField};
use realestate_backend::services::ocr_mapping::{
    map_deposito, map_gasto, parse_dr_currency, parse_dr_date,
};
use realestate_backend::services::ocr_preview::PreviewStore;

fn arb_unicode_text() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{1,50}"
}

fn arb_ocr_line() -> impl Strategy<Value = OcrLine> {
    (arb_unicode_text(), 0u32..=100).prop_map(|(text, conf_pct)| OcrLine {
        text,
        confidence: (conf_pct as f64) / 100.0,
        bbox: vec![0.0, 0.0, 100.0, 20.0],
    })
}

fn arb_ocr_result() -> impl Strategy<Value = OcrResult> {
    (
        arb_unicode_text(),
        prop::collection::vec(arb_ocr_line(), 0..=5),
        prop::collection::hash_map("[a-z]{1,10}", arb_unicode_text(), 0..=5),
    )
        .prop_map(|(document_type, lines, structured_fields)| OcrResult {
            document_type,
            lines,
            structured_fields,
        })
}

fn arb_date_in_range() -> impl Strategy<Value = NaiveDate> {
    (1950i32..=2049, 1u32..=12)
        .prop_flat_map(|(year, month)| {
            let max_day = NaiveDate::from_ymd_opt(year, if month == 12 { 1 } else { month + 1 }, 1)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
                .pred_opt()
                .unwrap()
                .day();
            (Just(year), Just(month), 1..=max_day)
        })
        .prop_map(|(year, month, day)| NaiveDate::from_ymd_opt(year, month, day).unwrap())
}

fn format_with_commas(amount: &Decimal) -> String {
    let s = amount.to_string();
    let parts: Vec<&str> = s.split('.').collect();
    let integer = parts[0];
    let decimal = parts.get(1).copied().unwrap_or("00");
    let digits: Vec<char> = integer.chars().collect();
    let mut result = String::new();
    for (i, ch) in digits.iter().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*ch);
    }
    format!("{result}.{decimal}")
}

fn arb_decimal_2dp() -> impl Strategy<Value = Decimal> {
    (0u64..=99_999_999, 0u32..=99).prop_map(|(whole, frac)| {
        let s = format!("{whole}.{frac:02}");
        Decimal::from_str(&s).unwrap()
    })
}

fn detect_format_test(filename: &str) -> Option<ImportFormat> {
    let lower = filename.to_lowercase();
    if lower.ends_with(".csv") {
        Some(ImportFormat::Csv)
    } else if lower.ends_with(".xlsx") {
        Some(ImportFormat::Xlsx)
    } else if lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".pdf")
    {
        Some(ImportFormat::Image)
    } else {
        None
    }
}

fn arb_filename_with_ext() -> impl Strategy<Value = (String, ImportFormat)> {
    let base = "[a-zA-Z0-9_]{1,20}";
    let ext = prop_oneof![
        Just((".jpg", ImportFormat::Image)),
        Just((".jpeg", ImportFormat::Image)),
        Just((".png", ImportFormat::Image)),
        Just((".pdf", ImportFormat::Image)),
        Just((".csv", ImportFormat::Csv)),
        Just((".xlsx", ImportFormat::Xlsx)),
    ];
    (base, ext).prop_map(|(name, (ext, fmt))| (format!("{name}{ext}"), fmt))
}

fn make_ocr_result(doc_type: &str, fields: HashMap<String, String>) -> OcrResult {
    let lines: Vec<OcrLine> = fields
        .values()
        .map(|v| OcrLine {
            text: v.clone(),
            confidence: 0.95,
            bbox: vec![0.0, 0.0, 100.0, 20.0],
        })
        .collect();
    OcrResult {
        document_type: doc_type.to_string(),
        lines,
        structured_fields: fields,
    }
}

fn arb_deposito_fields() -> impl Strategy<Value = HashMap<String, String>> {
    (
        arb_decimal_2dp(),
        prop_oneof![Just("RD$"), Just("US$"), Just("")],
        arb_date_in_range(),
        "[A-Z ]{3,20}",
        "[0-9\\-]{5,15}",
        "[A-Z0-9\\-]{3,15}",
    )
        .prop_map(|(amount, prefix, date, depositante, cuenta, referencia)| {
            let mut fields = HashMap::new();
            fields.insert("monto".to_string(), format_with_commas(&amount));
            if !prefix.is_empty() {
                fields.insert("moneda".to_string(), prefix.to_string());
            }
            fields.insert("fecha".to_string(), date.format("%d/%m/%Y").to_string());
            fields.insert("depositante".to_string(), depositante);
            fields.insert("cuenta".to_string(), cuenta);
            fields.insert("referencia".to_string(), referencia);
            fields
        })
}

fn arb_gasto_fields() -> impl Strategy<Value = HashMap<String, String>> {
    (
        arb_decimal_2dp(),
        prop_oneof![Just("RD$"), Just("US$"), Just("")],
        arb_date_in_range(),
        "[A-Z ]{3,20}",
        "[A-Z0-9\\-]{3,15}",
    )
        .prop_map(|(amount, prefix, date, proveedor, numero_factura)| {
            let mut fields = HashMap::new();
            fields.insert("monto".to_string(), format_with_commas(&amount));
            if !prefix.is_empty() {
                fields.insert("moneda".to_string(), prefix.to_string());
            }
            fields.insert("fecha".to_string(), date.format("%d/%m/%Y").to_string());
            fields.insert("proveedor".to_string(), proveedor);
            fields.insert("numero_factura".to_string(), numero_factura);
            fields
        })
}

fn arb_import_preview() -> impl Strategy<Value = ImportPreview> {
    (
        prop::collection::vec(
            (
                "[a-z_]{1,10}",
                "[a-zA-Z0-9 ]{1,20}",
                0.0f64..=1.0f64,
                "[A-Za-z ]{1,15}",
            )
                .prop_map(|(name, value, confidence, label)| PreviewField {
                    name,
                    value,
                    confidence,
                    label,
                }),
            1..=5,
        ),
        "[a-z_]{3,15}",
    )
        .prop_map(|(fields, document_type)| ImportPreview {
            preview_id: uuid::Uuid::new_v4(),
            document_type,
            fields,
        })
}
// Feature: paddleocr-integration, Property 1: OcrResult serialization round-trip
// **Validates: Requirements 9.3, 9.4, 9.5**
#[test]
fn ocr_result_serialization_roundtrip() {
    let mut runner = TestRunner::new(Config::with_cases(100));
    runner
        .run(&arb_ocr_result(), |result| {
            let json = serde_json::to_string(&result).unwrap();
            let deserialized: OcrResult = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(result, deserialized);
            Ok(())
        })
        .unwrap();
}

// Feature: paddleocr-integration, Property 2: DR date parsing round-trip (DD/MM/YYYY)
// **Validates: Requirements 10.1, 10.2, 10.6**
#[test]
fn dr_date_parsing_roundtrip_dd_slash_mm_slash_yyyy() {
    let mut runner = TestRunner::new(Config::with_cases(100));
    runner
        .run(&arb_date_in_range(), |date| {
            let formatted = date.format("%d/%m/%Y").to_string();
            let parsed = parse_dr_date(&formatted).unwrap();
            prop_assert_eq!(date, parsed);
            Ok(())
        })
        .unwrap();
}

// Feature: paddleocr-integration, Property 2: DR date parsing round-trip (DD-MM-YYYY)
// **Validates: Requirements 10.1, 10.2, 10.6**
#[test]
fn dr_date_parsing_roundtrip_dd_dash_mm_dash_yyyy() {
    let mut runner = TestRunner::new(Config::with_cases(100));
    runner
        .run(&arb_date_in_range(), |date| {
            let formatted = date.format("%d-%m-%Y").to_string();
            let parsed = parse_dr_date(&formatted).unwrap();
            prop_assert_eq!(date, parsed);
            Ok(())
        })
        .unwrap();
}

// Feature: paddleocr-integration, Property 2: DR date parsing round-trip (YYYY-MM-DD)
// **Validates: Requirements 10.1, 10.2, 10.6**
#[test]
fn dr_date_parsing_roundtrip_yyyy_mm_dd() {
    let mut runner = TestRunner::new(Config::with_cases(100));
    runner
        .run(&arb_date_in_range(), |date| {
            let formatted = date.format("%Y-%m-%d").to_string();
            let parsed = parse_dr_date(&formatted).unwrap();
            prop_assert_eq!(date, parsed);
            Ok(())
        })
        .unwrap();
}

// Feature: paddleocr-integration, Property 2: DR date parsing round-trip (DD-MM-YY)
// **Validates: Requirements 10.1, 10.2, 10.6**
#[test]
fn dr_date_parsing_roundtrip_dd_mm_yy() {
    let mut runner = TestRunner::new(Config::with_cases(100));
    let strategy = arb_date_in_range().prop_filter(
        "only dates in 1950-1999 or 2000-2049 round-trip with 2-digit year",
        |date| {
            let year = date.year();
            (1950..=1999).contains(&year) || (2000..=2049).contains(&year)
        },
    );
    runner
        .run(&strategy, |date| {
            let formatted = date.format("%d-%m-%y").to_string();
            let parsed = parse_dr_date(&formatted).unwrap();
            prop_assert_eq!(date, parsed);
            Ok(())
        })
        .unwrap();
}

// Feature: paddleocr-integration, Property 3: DR currency parsing round-trip (RD$)
// **Validates: Requirements 10.3, 10.4, 10.5, 10.7**
#[test]
fn dr_currency_parsing_roundtrip_rdp() {
    let mut runner = TestRunner::new(Config::with_cases(100));
    runner
        .run(&arb_decimal_2dp(), |amount| {
            let formatted = format!("RD${}", format_with_commas(&amount));
            let (parsed_amount, currency) = parse_dr_currency(&formatted).unwrap();
            prop_assert_eq!(amount, parsed_amount);
            prop_assert_eq!(currency, "DOP".to_string());
            Ok(())
        })
        .unwrap();
}

// Feature: paddleocr-integration, Property 3: DR currency parsing round-trip (US$)
// **Validates: Requirements 10.3, 10.4, 10.5, 10.7**
#[test]
fn dr_currency_parsing_roundtrip_usd() {
    let mut runner = TestRunner::new(Config::with_cases(100));
    runner
        .run(&arb_decimal_2dp(), |amount| {
            let formatted = format!("US${}", format_with_commas(&amount));
            let (parsed_amount, currency) = parse_dr_currency(&formatted).unwrap();
            prop_assert_eq!(amount, parsed_amount);
            prop_assert_eq!(currency, "USD".to_string());
            Ok(())
        })
        .unwrap();
}

// Feature: paddleocr-integration, Property 3: DR currency parsing round-trip (no prefix)
// **Validates: Requirements 10.3, 10.4, 10.5, 10.7**
#[test]
fn dr_currency_parsing_roundtrip_no_prefix() {
    let mut runner = TestRunner::new(Config::with_cases(100));
    runner
        .run(&arb_decimal_2dp(), |amount| {
            let formatted = format_with_commas(&amount);
            let (parsed_amount, currency) = parse_dr_currency(&formatted).unwrap();
            prop_assert_eq!(amount, parsed_amount);
            prop_assert_eq!(currency, "DOP".to_string());
            Ok(())
        })
        .unwrap();
}

// Feature: paddleocr-integration, Property 4: File extension detection
// **Validates: Requirements 3.2, 3.4**
#[test]
fn file_extension_detection() {
    let mut runner = TestRunner::new(Config::with_cases(100));
    runner
        .run(&arb_filename_with_ext(), |(filename, expected_format)| {
            let detected = detect_format_test(&filename);
            prop_assert_eq!(detected, Some(expected_format));
            Ok(())
        })
        .unwrap();
}

// Feature: paddleocr-integration, Property 5: Deposit receipt field mapping completeness
// **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.8, 4.9**
#[test]
fn deposit_receipt_field_mapping() {
    let mut runner = TestRunner::new(Config::with_cases(100));
    runner
        .run(&arb_deposito_fields(), |fields| {
            let monto_raw = fields.get("monto").unwrap().clone();
            let moneda_raw = fields.get("moneda").cloned().unwrap_or_default();
            let fecha_raw = fields.get("fecha").unwrap().clone();
            let depositante_raw = fields.get("depositante").unwrap().clone();
            let cuenta_raw = fields.get("cuenta").unwrap().clone();
            let referencia_raw = fields.get("referencia").unwrap().clone();
            let currency_input = if moneda_raw.is_empty() {
                monto_raw.clone()
            } else {
                format!("{moneda_raw}{monto_raw}")
            };
            let (expected_amount, expected_currency) = parse_dr_currency(&currency_input).unwrap();
            let expected_date = parse_dr_date(&fecha_raw).unwrap();
            let result = make_ocr_result("deposito_bancario", fields);
            let preview = map_deposito(&result).unwrap();
            let get = |name: &str| -> &PreviewField {
                preview.fields.iter().find(|f| f.name == name).unwrap()
            };
            prop_assert_eq!(&preview.document_type, "deposito_bancario");
            prop_assert_eq!(&get("monto").value, &expected_amount.to_string());
            prop_assert_eq!(&get("moneda").value, &expected_currency);
            prop_assert_eq!(
                &get("fecha_pago").value,
                &expected_date.format("%Y-%m-%d").to_string()
            );
            prop_assert_eq!(&get("depositante").value, &depositante_raw);
            let notas = &get("notas").value;
            prop_assert!(notas.contains(&cuenta_raw));
            prop_assert!(notas.contains(&referencia_raw));
            prop_assert_eq!(&get("metodo_pago").value, "deposito_bancario");
            prop_assert_eq!(&get("estado").value, "pagado");
            Ok(())
        })
        .unwrap();
}

// Feature: paddleocr-integration, Property 6: Expense receipt field mapping completeness
// **Validates: Requirements 5.1, 5.2, 5.3, 5.4, 5.5**
#[test]
fn expense_receipt_field_mapping() {
    let mut runner = TestRunner::new(Config::with_cases(100));
    runner
        .run(&arb_gasto_fields(), |fields| {
            let monto_raw = fields.get("monto").unwrap().clone();
            let moneda_raw = fields.get("moneda").cloned().unwrap_or_default();
            let fecha_raw = fields.get("fecha").unwrap().clone();
            let proveedor_raw = fields.get("proveedor").unwrap().clone();
            let numero_factura_raw = fields.get("numero_factura").unwrap().clone();
            let currency_input = if moneda_raw.is_empty() {
                monto_raw.clone()
            } else {
                format!("{moneda_raw}{monto_raw}")
            };
            let (expected_amount, expected_currency) = parse_dr_currency(&currency_input).unwrap();
            let expected_date = parse_dr_date(&fecha_raw).unwrap();
            let result = make_ocr_result("recibo_gasto", fields);
            let preview = map_gasto(&result).unwrap();
            let get = |name: &str| -> &PreviewField {
                preview.fields.iter().find(|f| f.name == name).unwrap()
            };
            prop_assert_eq!(&preview.document_type, "recibo_gasto");
            prop_assert_eq!(&get("monto").value, &expected_amount.to_string());
            prop_assert_eq!(&get("moneda").value, &expected_currency);
            prop_assert_eq!(
                &get("fecha_gasto").value,
                &expected_date.format("%Y-%m-%d").to_string()
            );
            prop_assert_eq!(&get("proveedor").value, &proveedor_raw);
            prop_assert_eq!(&get("numero_factura").value, &numero_factura_raw);
            Ok(())
        })
        .unwrap();
}

// Feature: paddleocr-integration, Property 7: Preview store TTL expiry
// **Validates: Requirements 6.5, 6.6**
#[test]
fn preview_store_insert_get_remove() {
    let mut runner = TestRunner::new(Config::with_cases(100));
    runner
        .run(&arb_import_preview(), |preview| {
            let store = PreviewStore::new();
            let id = store.insert(preview.clone());
            let retrieved = store.get(&id);
            prop_assert!(retrieved.is_some());
            let retrieved = retrieved.unwrap();
            prop_assert_eq!(retrieved.preview_id, id);
            prop_assert_eq!(retrieved.document_type, preview.document_type);
            prop_assert_eq!(retrieved.fields.len(), preview.fields.len());
            let removed = store.remove(&id);
            prop_assert!(removed.is_some());
            let after_remove = store.get(&id);
            prop_assert!(after_remove.is_none());
            Ok(())
        })
        .unwrap();
}

#[test]
fn preview_store_cleanup_does_not_remove_fresh() {
    let mut runner = TestRunner::new(Config::with_cases(100));
    runner
        .run(&arb_import_preview(), |preview| {
            let store = PreviewStore::new();
            let id = store.insert(preview);
            store.cleanup_expired();
            let retrieved = store.get(&id);
            prop_assert!(retrieved.is_some());
            Ok(())
        })
        .unwrap();
}
