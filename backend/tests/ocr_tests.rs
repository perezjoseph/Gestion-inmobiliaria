use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;

use realestate_backend::models::importacion::ImportFormat;
use realestate_backend::models::ocr::{OcrLine, OcrResult, PreviewField};
use realestate_backend::services::ocr_client::OcrClient;
use realestate_backend::services::ocr_mapping::{
    map_deposito, map_gasto, parse_dr_currency, parse_dr_date,
};
use realestate_backend::services::ocr_preview::PreviewStore;

#[test]
fn parse_dr_date_dd_slash_mm_slash_yyyy() {
    assert_eq!(
        parse_dr_date("15/03/2025").unwrap(),
        NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()
    );
}

#[test]
fn parse_dr_date_dd_dash_mm_dash_yyyy() {
    assert_eq!(
        parse_dr_date("01-12-1999").unwrap(),
        NaiveDate::from_ymd_opt(1999, 12, 1).unwrap()
    );
}

#[test]
fn parse_dr_date_yyyy_mm_dd() {
    assert_eq!(
        parse_dr_date("2025-06-30").unwrap(),
        NaiveDate::from_ymd_opt(2025, 6, 30).unwrap()
    );
}

#[test]
fn parse_dr_date_dd_mm_yy_2000s() {
    assert_eq!(
        parse_dr_date("15-03-25").unwrap(),
        NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()
    );
}

#[test]
fn parse_dr_date_dd_mm_yy_1900s() {
    assert_eq!(
        parse_dr_date("01-06-99").unwrap(),
        NaiveDate::from_ymd_opt(1999, 6, 1).unwrap()
    );
}

#[test]
fn parse_dr_date_invalid() {
    assert!(parse_dr_date("not-a-date").is_err());
    assert!(parse_dr_date("").is_err());
}

#[test]
fn parse_dr_currency_rd_prefix() {
    let (amount, currency) = parse_dr_currency("RD$50,000.00").unwrap();
    assert_eq!(amount, Decimal::from_str("50000.00").unwrap());
    assert_eq!(currency, "DOP");
}

#[test]
fn parse_dr_currency_us_prefix() {
    let (amount, currency) = parse_dr_currency("US$1,500.50").unwrap();
    assert_eq!(amount, Decimal::from_str("1500.50").unwrap());
    assert_eq!(currency, "USD");
}

#[test]
fn parse_dr_currency_no_prefix_defaults_dop() {
    let (amount, currency) = parse_dr_currency("25000").unwrap();
    assert_eq!(amount, Decimal::from_str("25000").unwrap());
    assert_eq!(currency, "DOP");
}

#[test]
fn parse_dr_currency_with_commas() {
    let (amount, currency) = parse_dr_currency("1,234,567.89").unwrap();
    assert_eq!(amount, Decimal::from_str("1234567.89").unwrap());
    assert_eq!(currency, "DOP");
}

#[test]
fn parse_dr_currency_invalid() {
    assert!(parse_dr_currency("RD$abc").is_err());
    assert!(parse_dr_currency("").is_err());
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

#[test]
fn detect_format_jpg() {
    assert_eq!(detect_format_test("receipt.jpg"), Some(ImportFormat::Image));
}

#[test]
fn detect_format_jpeg() {
    assert_eq!(detect_format_test("photo.jpeg"), Some(ImportFormat::Image));
}

#[test]
fn detect_format_png() {
    assert_eq!(detect_format_test("scan.png"), Some(ImportFormat::Image));
}

#[test]
fn detect_format_pdf() {
    assert_eq!(detect_format_test("doc.pdf"), Some(ImportFormat::Image));
}

#[test]
fn detect_format_csv() {
    assert_eq!(detect_format_test("data.csv"), Some(ImportFormat::Csv));
}

#[test]
fn detect_format_xlsx() {
    assert_eq!(detect_format_test("sheet.xlsx"), Some(ImportFormat::Xlsx));
}

#[test]
fn detect_format_unsupported() {
    assert_eq!(detect_format_test("file.txt"), None);
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

#[test]
fn map_deposito_constant_fields() {
    let fields = HashMap::from([("monto".to_string(), "1000".to_string())]);
    let result = make_ocr_result("deposito_bancario", fields);
    let preview = map_deposito(&result).unwrap();

    let get =
        |name: &str| -> &PreviewField { preview.fields.iter().find(|f| f.name == name).unwrap() };

    assert_eq!(get("metodo_pago").value, "deposito_bancario");
    assert_eq!(get("metodo_pago").confidence, 1.0);
    assert_eq!(get("estado").value, "pagado");
    assert_eq!(get("estado").confidence, 1.0);
}

#[test]
fn map_gasto_missing_monto_error() {
    let fields = HashMap::from([("proveedor".to_string(), "FERRETERIA".to_string())]);
    let result = make_ocr_result("recibo_gasto", fields);
    let err = map_gasto(&result).unwrap_err();
    assert!(err.to_string().contains("monto no detectado"));
}

#[test]
fn ocr_client_new_succeeds() {
    unsafe { std::env::set_var("OCR_SERVICE_URL", "http://localhost:9999") };
    let client = OcrClient::new();
    assert!(client.is_ok());
}

#[tokio::test]
async fn ocr_client_extract_unreachable_returns_error() {
    unsafe { std::env::set_var("OCR_SERVICE_URL", "http://127.0.0.1:19999") };
    let client = OcrClient::new().unwrap();
    let result = client
        .extract(b"fake image data", "test.jpg", "image/jpeg")
        .await;
    assert!(result.is_err());
}

#[test]
fn preview_store_insert_get_roundtrip() {
    let store = PreviewStore::new();
    let preview = realestate_backend::models::ocr::ImportPreview {
        preview_id: uuid::Uuid::new_v4(),
        document_type: "deposito_bancario".to_string(),
        fields: vec![PreviewField {
            name: "monto".to_string(),
            value: "1000".to_string(),
            confidence: 0.95,
            label: "Monto".to_string(),
        }],
    };
    let id = store.insert(preview.clone());
    let retrieved = store.get(&id).unwrap();
    assert_eq!(retrieved.preview_id, id);
    assert_eq!(retrieved.document_type, "deposito_bancario");
    assert_eq!(retrieved.fields.len(), 1);
    assert_eq!(retrieved.fields[0].name, "monto");
}

#[test]
fn preview_store_remove_deletes() {
    let store = PreviewStore::new();
    let preview = realestate_backend::models::ocr::ImportPreview {
        preview_id: uuid::Uuid::new_v4(),
        document_type: "test".to_string(),
        fields: vec![],
    };
    let id = store.insert(preview);
    let removed = store.remove(&id);
    assert!(removed.is_some());
    assert!(store.get(&id).is_none());
}

#[test]
fn preview_store_get_after_remove_returns_none() {
    let store = PreviewStore::new();
    let preview = realestate_backend::models::ocr::ImportPreview {
        preview_id: uuid::Uuid::new_v4(),
        document_type: "test".to_string(),
        fields: vec![],
    };
    let id = store.insert(preview);
    store.remove(&id);
    assert!(store.get(&id).is_none());
}
