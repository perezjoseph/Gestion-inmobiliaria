use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::ocr::{ExtractField, ImportPreview, OcrResult, PreviewField};

fn field_confidence(result: &OcrResult, value: &str) -> f64 {
    result
        .lines
        .iter()
        .filter(|l| l.text.contains(value) || value.contains(&l.text))
        .map(|l| l.confidence)
        .fold(f64::NEG_INFINITY, f64::max)
        .max(0.0)
}

pub fn map_deposito(result: &OcrResult) -> Result<ImportPreview, AppError> {
    let fields = &result.structured_fields;

    let monto_raw = fields
        .get("monto")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("monto no detectado")))?;

    let moneda_raw = fields.get("moneda").map_or("", |s| s.as_str());
    let currency_input = if moneda_raw.is_empty() {
        monto_raw.clone()
    } else {
        format!("{moneda_raw}{monto_raw}")
    };

    let (amount, currency) =
        parse_dr_currency(&currency_input).map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    let mut preview_fields = vec![
        PreviewField {
            name: "monto".to_string(),
            value: amount.to_string(),
            confidence: field_confidence(result, monto_raw),
            label: "Monto".to_string(),
        },
        PreviewField {
            name: "moneda".to_string(),
            value: currency,
            confidence: field_confidence(result, moneda_raw),
            label: "Moneda".to_string(),
        },
    ];

    if let Some(fecha_raw) = fields.get("fecha") {
        match parse_dr_date(fecha_raw) {
            Ok(date) => preview_fields.push(PreviewField {
                name: "fecha_pago".to_string(),
                value: date.format("%Y-%m-%d").to_string(),
                confidence: field_confidence(result, fecha_raw),
                label: "Fecha de pago".to_string(),
            }),
            Err(_) => preview_fields.push(PreviewField {
                name: "fecha_pago".to_string(),
                value: fecha_raw.clone(),
                confidence: 0.0,
                label: "Fecha de pago".to_string(),
            }),
        }
    }

    if let Some(depositante) = fields.get("depositante") {
        preview_fields.push(PreviewField {
            name: "depositante".to_string(),
            value: depositante.clone(),
            confidence: field_confidence(result, depositante),
            label: "Depositante".to_string(),
        });
    }

    let cuenta = fields.get("cuenta").map_or("", |s| s.as_str());
    let referencia = fields.get("referencia").map_or("", |s| s.as_str());
    let notas = format!("{cuenta} {referencia}").trim().to_string();
    if !notas.is_empty() {
        let notas_confidence = [cuenta, referencia]
            .iter()
            .filter(|v| !v.is_empty())
            .map(|v| field_confidence(result, v))
            .fold(f64::NEG_INFINITY, f64::max)
            .max(0.0);
        preview_fields.push(PreviewField {
            name: "notas".to_string(),
            value: notas,
            confidence: notas_confidence,
            label: "Notas".to_string(),
        });
    }

    preview_fields.push(PreviewField {
        name: "metodo_pago".to_string(),
        value: "deposito_bancario".to_string(),
        confidence: 1.0,
        label: "Método de pago".to_string(),
    });

    preview_fields.push(PreviewField {
        name: "estado".to_string(),
        value: "pagado".to_string(),
        confidence: 1.0,
        label: "Estado".to_string(),
    });

    Ok(ImportPreview {
        preview_id: Uuid::new_v4(),
        document_type: "deposito_bancario".to_string(),
        fields: preview_fields,
    })
}

pub fn map_gasto(result: &OcrResult) -> Result<ImportPreview, AppError> {
    let fields = &result.structured_fields;

    let monto_raw = fields
        .get("monto")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("monto no detectado")))?;

    let moneda_raw = fields.get("moneda").map_or("", |s| s.as_str());
    let currency_input = if moneda_raw.is_empty() {
        monto_raw.clone()
    } else {
        format!("{moneda_raw}{monto_raw}")
    };

    let (amount, currency) =
        parse_dr_currency(&currency_input).map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    let mut preview_fields = vec![
        PreviewField {
            name: "monto".to_string(),
            value: amount.to_string(),
            confidence: field_confidence(result, monto_raw),
            label: "Monto".to_string(),
        },
        PreviewField {
            name: "moneda".to_string(),
            value: currency,
            confidence: field_confidence(result, moneda_raw),
            label: "Moneda".to_string(),
        },
    ];

    if let Some(proveedor) = fields.get("proveedor") {
        preview_fields.push(PreviewField {
            name: "proveedor".to_string(),
            value: proveedor.clone(),
            confidence: field_confidence(result, proveedor),
            label: "Proveedor".to_string(),
        });
    }

    if let Some(fecha_raw) = fields.get("fecha") {
        match parse_dr_date(fecha_raw) {
            Ok(date) => preview_fields.push(PreviewField {
                name: "fecha_gasto".to_string(),
                value: date.format("%Y-%m-%d").to_string(),
                confidence: field_confidence(result, fecha_raw),
                label: "Fecha de gasto".to_string(),
            }),
            Err(_) => preview_fields.push(PreviewField {
                name: "fecha_gasto".to_string(),
                value: fecha_raw.clone(),
                confidence: 0.0,
                label: "Fecha de gasto".to_string(),
            }),
        }
    }

    if let Some(numero_factura) = fields.get("numero_factura") {
        preview_fields.push(PreviewField {
            name: "numero_factura".to_string(),
            value: numero_factura.clone(),
            confidence: field_confidence(result, numero_factura),
            label: "Número de factura".to_string(),
        });
    }

    Ok(ImportPreview {
        preview_id: Uuid::new_v4(),
        document_type: "recibo_gasto".to_string(),
        fields: preview_fields,
    })
}

pub fn parse_dr_date(text: &str) -> Result<NaiveDate, String> {
    let text = text.trim();

    if text.contains('/')
        && let Ok(date) = NaiveDate::parse_from_str(text, "%d/%m/%Y")
    {
        return Ok(date);
    }

    if let Some(first_segment) = text.split('-').next()
        && first_segment.len() == 4
        && let Ok(date) = NaiveDate::parse_from_str(text, "%Y-%m-%d")
    {
        return Ok(date);
    }

    if let Some(date) = parse_two_digit_year(text) {
        return Ok(date);
    }

    if let Ok(date) = NaiveDate::parse_from_str(text, "%d-%m-%Y") {
        return Ok(date);
    }

    Err(format!("Formato de fecha no reconocido: '{text}'"))
}

fn parse_two_digit_year(text: &str) -> Option<NaiveDate> {
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 3 {
        return None;
    }

    let day: u32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let yy: i32 = parts[2].parse().ok()?;

    if !(0..=99).contains(&yy) || parts[2].len() != 2 {
        return None;
    }

    let year = if yy <= 49 { 2000 + yy } else { 1900 + yy };

    NaiveDate::from_ymd_opt(year, month, day)
}

pub fn parse_dr_currency(text: &str) -> Result<(Decimal, String), String> {
    let text = text.trim();

    let (amount_str, currency) = if let Some(rest) = text.strip_prefix("RD$") {
        (rest, "DOP")
    } else if let Some(rest) = text.strip_prefix("US$") {
        (rest, "USD")
    } else {
        (text, "DOP")
    };

    let amount_str = amount_str.replace(',', "");

    let amount =
        Decimal::from_str(&amount_str).map_err(|_| format!("Monto no válido: '{text}'"))?;

    Ok((amount, currency.to_string()))
}

pub fn normalize_cedula(raw: &str) -> String {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 11 {
        format!("{}-{}-{}", &digits[0..3], &digits[3..10], &digits[10..11])
    } else {
        digits
    }
}

pub fn map_cedula(result: &OcrResult) -> Result<Vec<ExtractField>, AppError> {
    let fields = &result.structured_fields;

    let cedula_raw = fields.get("cedula").map_or("", |s| s.as_str());
    let cedula_value = normalize_cedula(cedula_raw);

    let nombre = fields.get("nombre").map_or("", |s| s.as_str());
    let apellido = fields.get("apellido").map_or("", |s| s.as_str());

    Ok(vec![
        ExtractField {
            name: "cedula".to_string(),
            value: cedula_value,
            label: "Cédula".to_string(),
            confidence: field_confidence(result, cedula_raw),
        },
        ExtractField {
            name: "nombre".to_string(),
            value: nombre.to_string(),
            label: "Nombre".to_string(),
            confidence: field_confidence(result, nombre),
        },
        ExtractField {
            name: "apellido".to_string(),
            value: apellido.to_string(),
            label: "Apellido".to_string(),
            confidence: field_confidence(result, apellido),
        },
    ])
}

pub fn map_contrato(result: &OcrResult) -> Result<Vec<ExtractField>, AppError> {
    let fields = &result.structured_fields;

    let monto_raw = fields.get("monto_mensual").map_or("", |s| s.as_str());
    let moneda_raw = fields.get("moneda").map_or("", |s| s.as_str());

    let (monto_value, monto_confidence) = if monto_raw.is_empty() {
        (String::new(), 0.0)
    } else {
        let currency_input = if moneda_raw.is_empty() {
            monto_raw.to_string()
        } else {
            format!("{moneda_raw}{monto_raw}")
        };
        match parse_dr_currency(&currency_input) {
            Ok((amount, _)) => (amount.to_string(), field_confidence(result, monto_raw)),
            Err(_) => (monto_raw.to_string(), field_confidence(result, monto_raw)),
        }
    };

    let moneda_value = if !moneda_raw.is_empty() {
        let currency_input = format!("{moneda_raw}0");
        match parse_dr_currency(&currency_input) {
            Ok((_, currency)) => currency,
            Err(_) => moneda_raw.to_string(),
        }
    } else if !monto_raw.is_empty() {
        match parse_dr_currency(monto_raw) {
            Ok((_, currency)) => currency,
            Err(_) => String::new(),
        }
    } else {
        String::new()
    };

    let fecha_inicio_raw = fields.get("fecha_inicio").map_or("", |s| s.as_str());
    let fecha_inicio_value = if fecha_inicio_raw.is_empty() {
        String::new()
    } else {
        match parse_dr_date(fecha_inicio_raw) {
            Ok(date) => date.format("%Y-%m-%d").to_string(),
            Err(_) => fecha_inicio_raw.to_string(),
        }
    };

    let fecha_fin_raw = fields.get("fecha_fin").map_or("", |s| s.as_str());
    let fecha_fin_value = if fecha_fin_raw.is_empty() {
        String::new()
    } else {
        match parse_dr_date(fecha_fin_raw) {
            Ok(date) => date.format("%Y-%m-%d").to_string(),
            Err(_) => fecha_fin_raw.to_string(),
        }
    };

    let deposito_raw = fields.get("deposito").map_or("", |s| s.as_str());
    let deposito_value = if deposito_raw.is_empty() {
        String::new()
    } else {
        match parse_dr_currency(deposito_raw) {
            Ok((amount, _)) => amount.to_string(),
            Err(_) => deposito_raw.to_string(),
        }
    };

    Ok(vec![
        ExtractField {
            name: "monto_mensual".to_string(),
            value: monto_value,
            label: "Monto Mensual".to_string(),
            confidence: monto_confidence,
        },
        ExtractField {
            name: "moneda".to_string(),
            value: moneda_value,
            label: "Moneda".to_string(),
            confidence: field_confidence(result, moneda_raw),
        },
        ExtractField {
            name: "fecha_inicio".to_string(),
            value: fecha_inicio_value,
            label: "Fecha de Inicio".to_string(),
            confidence: field_confidence(result, fecha_inicio_raw),
        },
        ExtractField {
            name: "fecha_fin".to_string(),
            value: fecha_fin_value,
            label: "Fecha de Fin".to_string(),
            confidence: field_confidence(result, fecha_fin_raw),
        },
        ExtractField {
            name: "deposito".to_string(),
            value: deposito_value,
            label: "Depósito".to_string(),
            confidence: field_confidence(result, deposito_raw),
        },
    ])
}

pub fn map_deposito_extract(result: &OcrResult) -> Result<Vec<ExtractField>, AppError> {
    let preview = map_deposito(result)?;
    Ok(preview
        .fields
        .into_iter()
        .map(|f| ExtractField {
            name: f.name,
            value: f.value,
            label: f.label,
            confidence: f.confidence,
        })
        .collect())
}

pub fn map_gasto_extract(result: &OcrResult) -> Result<Vec<ExtractField>, AppError> {
    let preview = map_gasto(result)?;
    Ok(preview
        .fields
        .into_iter()
        .map(|f| ExtractField {
            name: f.name,
            value: f.value,
            label: f.label,
            confidence: f.confidence,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn parse_yyyy_mm_dd() {
        assert_eq!(
            parse_dr_date("2025-03-15").unwrap(),
            NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()
        );
    }

    #[test]
    fn parse_dd_slash_mm_slash_yyyy() {
        assert_eq!(
            parse_dr_date("15/03/2025").unwrap(),
            NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()
        );
    }

    #[test]
    fn parse_dd_dash_mm_dash_yyyy() {
        assert_eq!(
            parse_dr_date("15-03-2025").unwrap(),
            NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()
        );
    }

    #[test]
    fn parse_dd_mm_yy_low_year() {
        assert_eq!(
            parse_dr_date("15-03-25").unwrap(),
            NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()
        );
    }

    #[test]
    fn parse_dd_mm_yy_high_year() {
        assert_eq!(
            parse_dr_date("01-06-99").unwrap(),
            NaiveDate::from_ymd_opt(1999, 6, 1).unwrap()
        );
    }

    #[test]
    fn parse_dd_mm_yy_boundary_49() {
        assert_eq!(
            parse_dr_date("31-12-49").unwrap(),
            NaiveDate::from_ymd_opt(2049, 12, 31).unwrap()
        );
    }

    #[test]
    fn parse_dd_mm_yy_boundary_50() {
        assert_eq!(
            parse_dr_date("01-01-50").unwrap(),
            NaiveDate::from_ymd_opt(1950, 1, 1).unwrap()
        );
    }

    #[test]
    fn parse_dd_mm_yy_year_00() {
        assert_eq!(
            parse_dr_date("15-06-00").unwrap(),
            NaiveDate::from_ymd_opt(2000, 6, 15).unwrap()
        );
    }

    #[test]
    fn parse_trims_whitespace() {
        assert_eq!(
            parse_dr_date("  2025-03-15  ").unwrap(),
            NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()
        );
    }

    #[test]
    fn parse_invalid_returns_error() {
        assert!(parse_dr_date("not-a-date").is_err());
        assert!(parse_dr_date("").is_err());
        assert!(parse_dr_date("2025/03/15").is_err());
    }

    #[test]
    fn currency_rd_prefix() {
        let (amount, currency) = parse_dr_currency("RD$50,000.00").unwrap();
        assert_eq!(amount, Decimal::from_str("50000.00").unwrap());
        assert_eq!(currency, "DOP");
    }

    #[test]
    fn currency_us_prefix() {
        let (amount, currency) = parse_dr_currency("US$1,500.50").unwrap();
        assert_eq!(amount, Decimal::from_str("1500.50").unwrap());
        assert_eq!(currency, "USD");
    }

    #[test]
    fn currency_no_prefix_defaults_dop() {
        let (amount, currency) = parse_dr_currency("25000").unwrap();
        assert_eq!(amount, Decimal::from_str("25000").unwrap());
        assert_eq!(currency, "DOP");
    }

    #[test]
    fn currency_with_commas_no_prefix() {
        let (amount, currency) = parse_dr_currency("1,234.56").unwrap();
        assert_eq!(amount, Decimal::from_str("1234.56").unwrap());
        assert_eq!(currency, "DOP");
    }

    #[test]
    fn currency_trims_whitespace() {
        let (amount, currency) = parse_dr_currency("  RD$100.00  ").unwrap();
        assert_eq!(amount, Decimal::from_str("100.00").unwrap());
        assert_eq!(currency, "DOP");
    }

    #[test]
    fn currency_invalid_returns_error() {
        assert!(parse_dr_currency("RD$abc").is_err());
        assert!(parse_dr_currency("").is_err());
    }

    fn make_deposito_result(fields: HashMap<String, String>) -> OcrResult {
        use crate::models::ocr::OcrLine;
        let lines: Vec<OcrLine> = fields
            .values()
            .map(|v| OcrLine {
                text: v.clone(),
                confidence: 0.95,
                bbox: vec![0.0, 0.0, 100.0, 20.0],
            })
            .collect();
        OcrResult {
            document_type: "deposito_bancario".to_string(),
            lines,
            structured_fields: fields,
        }
    }

    #[test]
    fn map_deposito_full_fields() {
        let fields = HashMap::from([
            ("monto".to_string(), "50,000.00".to_string()),
            ("moneda".to_string(), "RD$".to_string()),
            ("fecha".to_string(), "15/03/2025".to_string()),
            ("depositante".to_string(), "JUAN PEREZ".to_string()),
            ("cuenta".to_string(), "123-456789-0".to_string()),
            ("referencia".to_string(), "DEP-2025-001".to_string()),
        ]);
        let result = make_deposito_result(fields);
        let preview = map_deposito(&result).unwrap();

        assert_eq!(preview.document_type, "deposito_bancario");

        let get = |name: &str| -> &PreviewField {
            preview.fields.iter().find(|f| f.name == name).unwrap()
        };

        assert_eq!(get("monto").value, "50000.00");
        assert_eq!(get("moneda").value, "DOP");
        assert_eq!(get("fecha_pago").value, "2025-03-15");
        assert_eq!(get("depositante").value, "JUAN PEREZ");
        assert!(get("notas").value.contains("123-456789-0"));
        assert!(get("notas").value.contains("DEP-2025-001"));
        assert_eq!(get("metodo_pago").value, "deposito_bancario");
        assert_eq!(get("estado").value, "pagado");
    }

    #[test]
    fn map_deposito_missing_monto_returns_error() {
        let fields = HashMap::from([
            ("moneda".to_string(), "RD$".to_string()),
            ("fecha".to_string(), "15/03/2025".to_string()),
        ]);
        let result = make_deposito_result(fields);
        assert!(map_deposito(&result).is_err());
    }

    #[test]
    fn map_deposito_metodo_pago_and_estado() {
        let fields = HashMap::from([("monto".to_string(), "1000".to_string())]);
        let result = make_deposito_result(fields);
        let preview = map_deposito(&result).unwrap();

        let get = |name: &str| -> &PreviewField {
            preview.fields.iter().find(|f| f.name == name).unwrap()
        };

        assert_eq!(get("metodo_pago").value, "deposito_bancario");
        assert_eq!(get("estado").value, "pagado");
    }

    #[test]
    fn map_deposito_usd_currency() {
        let fields = HashMap::from([
            ("monto".to_string(), "1,500.50".to_string()),
            ("moneda".to_string(), "US$".to_string()),
        ]);
        let result = make_deposito_result(fields);
        let preview = map_deposito(&result).unwrap();

        let get = |name: &str| -> &PreviewField {
            preview.fields.iter().find(|f| f.name == name).unwrap()
        };

        assert_eq!(get("monto").value, "1500.50");
        assert_eq!(get("moneda").value, "USD");
    }

    fn make_gasto_result(fields: HashMap<String, String>) -> OcrResult {
        use crate::models::ocr::OcrLine;
        let lines: Vec<OcrLine> = fields
            .values()
            .map(|v| OcrLine {
                text: v.clone(),
                confidence: 0.92,
                bbox: vec![0.0, 0.0, 100.0, 20.0],
            })
            .collect();
        OcrResult {
            document_type: "recibo_gasto".to_string(),
            lines,
            structured_fields: fields,
        }
    }

    #[test]
    fn map_gasto_full_fields() {
        let fields = HashMap::from([
            ("proveedor".to_string(), "FERRETERIA NACIONAL".to_string()),
            ("monto".to_string(), "15,000.00".to_string()),
            ("moneda".to_string(), "RD$".to_string()),
            ("fecha".to_string(), "20/04/2025".to_string()),
            ("numero_factura".to_string(), "FAC-2025-100".to_string()),
        ]);
        let result = make_gasto_result(fields);
        let preview = map_gasto(&result).unwrap();

        assert_eq!(preview.document_type, "recibo_gasto");

        let get = |name: &str| -> &PreviewField {
            preview.fields.iter().find(|f| f.name == name).unwrap()
        };

        assert_eq!(get("monto").value, "15000.00");
        assert_eq!(get("moneda").value, "DOP");
        assert_eq!(get("proveedor").value, "FERRETERIA NACIONAL");
        assert_eq!(get("fecha_gasto").value, "2025-04-20");
        assert_eq!(get("numero_factura").value, "FAC-2025-100");
    }

    #[test]
    fn map_gasto_missing_monto_returns_error() {
        let fields = HashMap::from([
            ("proveedor".to_string(), "FERRETERIA NACIONAL".to_string()),
            ("moneda".to_string(), "RD$".to_string()),
        ]);
        let result = make_gasto_result(fields);
        let err = map_gasto(&result).unwrap_err();
        assert!(err.to_string().contains("monto no detectado"));
    }

    #[test]
    fn map_gasto_usd_currency() {
        let fields = HashMap::from([
            ("monto".to_string(), "500.00".to_string()),
            ("moneda".to_string(), "US$".to_string()),
        ]);
        let result = make_gasto_result(fields);
        let preview = map_gasto(&result).unwrap();

        let get = |name: &str| -> &PreviewField {
            preview.fields.iter().find(|f| f.name == name).unwrap()
        };

        assert_eq!(get("monto").value, "500.00");
        assert_eq!(get("moneda").value, "USD");
    }

    #[test]
    fn map_gasto_minimal_fields() {
        let fields = HashMap::from([("monto".to_string(), "1000".to_string())]);
        let result = make_gasto_result(fields);
        let preview = map_gasto(&result).unwrap();

        assert_eq!(preview.document_type, "recibo_gasto");
        assert!(preview.fields.iter().any(|f| f.name == "monto"));
        assert!(preview.fields.iter().any(|f| f.name == "moneda"));
        assert!(!preview.fields.iter().any(|f| f.name == "proveedor"));
        assert!(!preview.fields.iter().any(|f| f.name == "fecha_gasto"));
        assert!(!preview.fields.iter().any(|f| f.name == "numero_factura"));
    }
}
