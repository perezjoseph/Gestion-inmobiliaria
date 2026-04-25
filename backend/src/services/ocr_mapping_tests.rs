#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use crate::models::ocr::{OcrLine, OcrResult};
    use crate::services::ocr_mapping::{
        map_cedula, map_contrato, map_deposito_extract, map_gasto_extract, normalize_cedula,
    };
    use std::collections::HashMap;

    fn make_result(doc_type: &str, fields: HashMap<String, String>) -> OcrResult {
        let lines: Vec<OcrLine> = fields
            .values()
            .map(|v| OcrLine {
                text: v.clone(),
                confidence: 0.90,
                bbox: vec![0.0, 0.0, 100.0, 20.0],
            })
            .collect();
        OcrResult {
            document_type: doc_type.to_string(),
            lines,
            structured_fields: fields,
        }
    }

    // ── normalize_cedula ──

    #[test]
    fn normalize_cedula_empty_string() {
        assert_eq!(normalize_cedula(""), "");
    }

    #[test]
    fn normalize_cedula_wrong_length() {
        assert_eq!(normalize_cedula("12345"), "12345");
    }

    #[test]
    fn normalize_cedula_already_formatted() {
        assert_eq!(normalize_cedula("001-1234567-8"), "001-1234567-8");
    }

    #[test]
    fn normalize_cedula_no_digits() {
        assert_eq!(normalize_cedula("abc-def"), "");
    }

    #[test]
    fn normalize_cedula_eleven_digits_no_dashes() {
        assert_eq!(normalize_cedula("00112345678"), "001-1234567-8");
    }

    #[test]
    fn normalize_cedula_eleven_digits_with_dots() {
        assert_eq!(normalize_cedula("001.1234567.8"), "001-1234567-8");
    }

    // ── map_cedula ──

    #[test]
    fn map_cedula_full_fields() {
        let fields = HashMap::from([
            ("cedula".into(), "00112345678".into()),
            ("nombre".into(), "JUAN".into()),
            ("apellido".into(), "PEREZ".into()),
        ]);
        let result = make_result("cedula", fields);
        let extracted = map_cedula(&result).unwrap();

        assert_eq!(extracted.len(), 3);

        let get = |name: &str| extracted.iter().find(|f| f.name == name).unwrap();

        assert_eq!(get("cedula").value, "001-1234567-8");
        assert_eq!(get("cedula").label, "Cédula");

        assert_eq!(get("nombre").value, "JUAN");
        assert_eq!(get("nombre").label, "Nombre");

        assert_eq!(get("apellido").value, "PEREZ");
        assert_eq!(get("apellido").label, "Apellido");
    }

    #[test]
    fn map_cedula_missing_cedula_field() {
        let fields = HashMap::from([
            ("nombre".into(), "JUAN".into()),
            ("apellido".into(), "PEREZ".into()),
        ]);
        let result = make_result("cedula", fields);
        let extracted = map_cedula(&result).unwrap();

        let cedula_field = extracted.iter().find(|f| f.name == "cedula").unwrap();
        assert_eq!(cedula_field.value, "");
    }

    #[test]
    fn map_cedula_missing_nombre_apellido() {
        let fields = HashMap::from([("cedula".into(), "00112345678".into())]);
        let result = make_result("cedula", fields);
        let extracted = map_cedula(&result).unwrap();

        let nombre = extracted.iter().find(|f| f.name == "nombre").unwrap();
        let apellido = extracted.iter().find(|f| f.name == "apellido").unwrap();
        assert_eq!(nombre.value, "");
        assert_eq!(apellido.value, "");
    }

    #[test]
    fn map_cedula_malformed_cedula() {
        let fields = HashMap::from([
            ("cedula".into(), "12345".into()),
            ("nombre".into(), "ANA".into()),
            ("apellido".into(), "GOMEZ".into()),
        ]);
        let result = make_result("cedula", fields);
        let extracted = map_cedula(&result).unwrap();

        let cedula_field = extracted.iter().find(|f| f.name == "cedula").unwrap();
        assert_eq!(cedula_field.value, "12345");
    }

    // ── map_contrato ──

    #[test]
    fn map_contrato_full_fields() {
        let fields = HashMap::from([
            ("monto_mensual".into(), "25,000.00".into()),
            ("moneda".into(), "RD$".into()),
            ("fecha_inicio".into(), "01/01/2025".into()),
            ("fecha_fin".into(), "31/12/2025".into()),
            ("deposito".into(), "RD$50,000.00".into()),
        ]);
        let result = make_result("contrato", fields);
        let extracted = map_contrato(&result).unwrap();

        assert_eq!(extracted.len(), 5);

        let get = |name: &str| extracted.iter().find(|f| f.name == name).unwrap();

        assert_eq!(get("monto_mensual").value, "25000.00");
        assert_eq!(get("monto_mensual").label, "Monto Mensual");

        assert_eq!(get("moneda").value, "DOP");
        assert_eq!(get("moneda").label, "Moneda");

        assert_eq!(get("fecha_inicio").value, "2025-01-01");
        assert_eq!(get("fecha_inicio").label, "Fecha de Inicio");

        assert_eq!(get("fecha_fin").value, "2025-12-31");
        assert_eq!(get("fecha_fin").label, "Fecha de Fin");

        assert_eq!(get("deposito").value, "50000.00");
        assert_eq!(get("deposito").label, "Depósito");
    }

    #[test]
    fn map_contrato_missing_monto_mensual() {
        let fields = HashMap::from([
            ("moneda".into(), "RD$".into()),
            ("fecha_inicio".into(), "01/01/2025".into()),
            ("fecha_fin".into(), "31/12/2025".into()),
            ("deposito".into(), "RD$50,000.00".into()),
        ]);
        let result = make_result("contrato", fields);
        let extracted = map_contrato(&result).unwrap();

        let monto = extracted
            .iter()
            .find(|f| f.name == "monto_mensual")
            .unwrap();
        assert_eq!(monto.value, "");
        assert_eq!(monto.confidence, 0.0);
    }

    #[test]
    fn map_contrato_unparseable_dates() {
        let fields = HashMap::from([
            ("monto_mensual".into(), "25000".into()),
            ("fecha_inicio".into(), "enero 2025".into()),
            ("fecha_fin".into(), "diciembre 2025".into()),
        ]);
        let result = make_result("contrato", fields);
        let extracted = map_contrato(&result).unwrap();

        let get = |name: &str| extracted.iter().find(|f| f.name == name).unwrap();

        assert_eq!(get("fecha_inicio").value, "enero 2025");
        assert_eq!(get("fecha_fin").value, "diciembre 2025");
    }

    #[test]
    fn map_contrato_missing_all_fields() {
        let fields = HashMap::new();
        let result = make_result("contrato", fields);
        let extracted = map_contrato(&result).unwrap();

        assert_eq!(extracted.len(), 5);

        for field in &extracted {
            assert_eq!(field.value, "");
        }
    }

    // ── map_deposito_extract ──

    #[test]
    fn map_deposito_extract_full_fields() {
        let fields = HashMap::from([
            ("monto".into(), "50,000.00".into()),
            ("moneda".into(), "RD$".into()),
            ("fecha".into(), "15/03/2025".into()),
            ("depositante".into(), "JUAN PEREZ".into()),
            ("cuenta".into(), "123-456789-0".into()),
            ("referencia".into(), "DEP-2025-001".into()),
        ]);
        let result = make_result("deposito_bancario", fields);
        let extracted = map_deposito_extract(&result).unwrap();

        let names: Vec<&str> = extracted.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"monto"));
        assert!(names.contains(&"moneda"));
        assert!(names.contains(&"fecha_pago"));
        assert!(names.contains(&"depositante"));
        assert!(names.contains(&"notas"));
        assert!(names.contains(&"metodo_pago"));
        assert!(names.contains(&"estado"));

        let get = |name: &str| extracted.iter().find(|f| f.name == name).unwrap();
        assert_eq!(get("monto").value, "50000.00");
        assert_eq!(get("moneda").value, "DOP");
        assert_eq!(get("fecha_pago").value, "2025-03-15");
    }

    // ── map_gasto_extract ──

    #[test]
    fn map_gasto_extract_full_fields() {
        let fields = HashMap::from([
            ("monto".into(), "15,000.00".into()),
            ("moneda".into(), "RD$".into()),
            ("proveedor".into(), "FERRETERIA NACIONAL".into()),
            ("fecha".into(), "20/04/2025".into()),
            ("numero_factura".into(), "FAC-2025-100".into()),
        ]);
        let result = make_result("recibo_gasto", fields);
        let extracted = map_gasto_extract(&result).unwrap();

        let names: Vec<&str> = extracted.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"monto"));
        assert!(names.contains(&"moneda"));
        assert!(names.contains(&"proveedor"));
        assert!(names.contains(&"fecha_gasto"));
        assert!(names.contains(&"numero_factura"));

        let get = |name: &str| extracted.iter().find(|f| f.name == name).unwrap();
        assert_eq!(get("monto").value, "15000.00");
        assert_eq!(get("moneda").value, "DOP");
        assert_eq!(get("proveedor").value, "FERRETERIA NACIONAL");
        assert_eq!(get("fecha_gasto").value, "2025-04-20");
        assert_eq!(get("numero_factura").value, "FAC-2025-100");
    }
}
