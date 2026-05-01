use crate::errors::AppError;

/// Strip all non-digit characters from input.
fn solo_digitos(input: &str) -> String {
    input.chars().filter(char::is_ascii_digit).collect()
}

/// Validate a Dominican Republic RNC (Registro Nacional del Contribuyente).
///
/// The RNC must be exactly 9 digits. The check digit (9th digit) is validated
/// using the DGII weighted modulus algorithm:
/// - Weights: [7, 9, 8, 6, 5, 4, 3, 2] applied to the first 8 digits
/// - check = sum(weight\[i\] * digit\[i\]) % 11
/// - `check_digit` = (10 - check) % 9 + 1
pub fn validar_rnc(rnc: &str) -> Result<(), AppError> {
    let solo = solo_digitos(rnc);

    if solo.len() != 9 {
        return Err(AppError::Validation(
            "RNC inválido: formato o dígito verificador incorrecto".to_string(),
        ));
    }

    let cifras: Vec<u32> = solo
        .chars()
        .map(|c| c.to_digit(10).unwrap_or(0))
        .collect();

    let weights: [u32; 8] = [7, 9, 8, 6, 5, 4, 3, 2];

    let sum: u32 = weights
        .iter()
        .zip(cifras.iter())
        .map(|(w, d)| w * d)
        .sum();

    let check = sum % 11;
    let expected = (10 - check) % 9 + 1;

    if cifras[8] != expected {
        return Err(AppError::Validation(
            "RNC inválido: formato o dígito verificador incorrecto".to_string(),
        ));
    }

    Ok(())
}

/// Validate a Dominican Republic cédula (national identity number).
///
/// The cédula must be exactly 11 digits. The check digit (11th digit) is
/// validated using the Luhn algorithm:
/// - Alternating weights [1, 2, 1, 2, ...] applied left-to-right to first 10 digits
/// - If product > 9, sum the two digits (e.g., 14 → 1 + 4 = 5)
/// - `check_digit` = (10 - (sum % 10)) % 10
pub fn validar_cedula(cedula: &str) -> Result<(), AppError> {
    let solo = solo_digitos(cedula);

    if solo.len() != 11 {
        return Err(AppError::Validation(
            "Cédula inválida: formato o dígito verificador incorrecto".to_string(),
        ));
    }

    let cifras: Vec<u32> = solo
        .chars()
        .map(|c| c.to_digit(10).unwrap_or(0))
        .collect();

    let weights: [u32; 10] = [1, 2, 1, 2, 1, 2, 1, 2, 1, 2];

    let sum: u32 = weights
        .iter()
        .zip(cifras.iter())
        .map(|(w, d)| {
            let product = w * d;
            if product > 9 {
                // Sum the two digits of the product (e.g., 14 → 1 + 4 = 5)
                product / 10 + product % 10
            } else {
                product
            }
        })
        .sum();

    let expected = (10 - (sum % 10)) % 10;

    if cifras[10] != expected {
        return Err(AppError::Validation(
            "Cédula inválida: formato o dígito verificador incorrecto".to_string(),
        ));
    }

    Ok(())
}

/// Format a raw 9-digit RNC string into the pattern `X-XX-XXXXX-X`.
///
/// Expects a 9-digit string. Non-digit characters are stripped first.
pub fn formato_rnc(rnc: &str) -> String {
    let d = solo_digitos(rnc);
    if d.len() != 9 {
        return d;
    }
    format!("{}-{}-{}-{}", &d[0..1], &d[1..3], &d[3..8], &d[8..9])
}

/// Format a raw 11-digit cédula string into the pattern `XXX-XXXXXXX-X`.
///
/// Expects an 11-digit string. Non-digit characters are stripped first.
pub fn formato_cedula(cedula: &str) -> String {
    let d = solo_digitos(cedula);
    if d.len() != 11 {
        return d;
    }
    format!("{}-{}-{}", &d[0..3], &d[3..10], &d[10..11])
}

/// Parse a formatted RNC (e.g., `"1-31-24679-6"`) back to raw digits (`"131246796"`).
///
/// Strips all non-digit characters.
pub fn parse_rnc(formatted: &str) -> String {
    solo_digitos(formatted)
}

/// Parse a formatted cédula (e.g., `"224-0002211-1"`) back to raw digits (`"22400022111"`).
///
/// Strips all non-digit characters.
pub fn parse_cedula(formatted: &str) -> String {
    solo_digitos(formatted)
}

/// Validate a Dominican Republic NCF (Número de Comprobante Fiscal).
///
/// The NCF must be a single uppercase letter followed by exactly 10 digits
/// (e.g., `B0100000001`). Total length: 11 characters.
pub fn validar_ncf(ncf: &str) -> Result<(), AppError> {
    let bytes = ncf.as_bytes();
    if bytes.len() == 11
        && bytes[0].is_ascii_uppercase()
        && bytes[1..].iter().all(u8::is_ascii_digit)
    {
        Ok(())
    } else {
        Err(AppError::Validation(
            "NCF inválido: debe ser una letra mayúscula seguida de 10 dígitos (ej: B0100000001)"
                .to_string(),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use actix_web::error::ResponseError;
    use actix_web::http::StatusCode;

    // ── RNC tests ──────────────────────────────────────────────

    #[test]
    fn validar_rnc_acepta_rnc_valido() {
        // 131246796: sum = 7*1+9*3+8*1+6*2+5*4+4*6+3*7+2*9 = 137, 137%11=5, (10-5)%9+1=6
        assert!(validar_rnc("131246796").is_ok());
    }

    #[test]
    fn validar_rnc_rechaza_digito_verificador_incorrecto() {
        // Change last digit from 6 to 5
        let result = validar_rnc("131246795");
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    #[test]
    fn validar_rnc_rechaza_longitud_corta() {
        let result = validar_rnc("12345678");
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    #[test]
    fn validar_rnc_rechaza_longitud_larga() {
        let result = validar_rnc("1312467960");
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    #[test]
    fn validar_rnc_acepta_formato_con_guiones() {
        // "1-31-24679-6" strips to "131246796"
        assert!(validar_rnc("1-31-24679-6").is_ok());
    }

    // ── Cédula tests ───────────────────────────────────────────

    #[test]
    fn validar_cedula_acepta_cedula_valida() {
        // 22400022111: weights [1,2,1,2,1,2,1,2,1,2] on first 10 digits
        // products: 2,4,4,0,0,0,2,4,1,2 → sum=19, (10-9)%10=1
        assert!(validar_cedula("22400022111").is_ok());
    }

    #[test]
    fn validar_cedula_rechaza_digito_verificador_incorrecto() {
        // Change last digit from 1 to 0
        let result = validar_cedula("22400022110");
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    #[test]
    fn validar_cedula_rechaza_longitud_corta() {
        let result = validar_cedula("2240002211");
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    #[test]
    fn validar_cedula_rechaza_longitud_larga() {
        let result = validar_cedula("224000221110");
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    #[test]
    fn validar_cedula_acepta_formato_con_guiones() {
        // "224-0002211-1" strips to "22400022111"
        assert!(validar_cedula("224-0002211-1").is_ok());
    }

    // ── NCF tests ──────────────────────────────────────────────

    #[test]
    fn validar_ncf_acepta_ncf_valido() {
        assert!(validar_ncf("B0100000001").is_ok());
    }

    #[test]
    fn validar_ncf_acepta_diferentes_prefijos() {
        assert!(validar_ncf("A0000000001").is_ok());
        assert!(validar_ncf("E0100000001").is_ok());
        assert!(validar_ncf("Z9999999999").is_ok());
    }

    #[test]
    fn validar_ncf_rechaza_cadena_vacia() {
        let result = validar_ncf("");
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    #[test]
    fn validar_ncf_rechaza_prefijo_minuscula() {
        let result = validar_ncf("b0100000001");
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    #[test]
    fn validar_ncf_rechaza_sin_prefijo_letra() {
        let result = validar_ncf("10100000001");
        assert!(result.is_err());
    }

    #[test]
    fn validar_ncf_rechaza_pocos_digitos() {
        // 9 digits instead of 10
        let result = validar_ncf("B010000001");
        assert!(result.is_err());
    }

    #[test]
    fn validar_ncf_rechaza_muchos_digitos() {
        // 11 digits instead of 10
        let result = validar_ncf("B01000000011");
        assert!(result.is_err());
    }

    #[test]
    fn validar_ncf_rechaza_letras_en_digitos() {
        let result = validar_ncf("B010000000A");
        assert!(result.is_err());
    }

    #[test]
    fn validar_ncf_rechaza_solo_letras() {
        let result = validar_ncf("ABCDEFGHIJK");
        assert!(result.is_err());
    }

    #[test]
    fn validar_ncf_rechaza_con_espacios() {
        let result = validar_ncf("B 010000001");
        assert!(result.is_err());
    }

    #[test]
    fn validar_ncf_rechaza_con_guiones() {
        let result = validar_ncf("B-010000001");
        assert!(result.is_err());
    }

    // ── Round-trip tests ───────────────────────────────────────

    #[test]
    fn formato_rnc_parse_rnc_roundtrip() {
        let raw = "131246796";
        let formatted = formato_rnc(raw);
        assert_eq!(formatted, "1-31-24679-6");

        let parsed = parse_rnc(&formatted);
        assert_eq!(parsed, raw);

        // Second format produces the same output
        assert_eq!(formato_rnc(&parsed), formatted);
    }

    #[test]
    fn formato_cedula_parse_cedula_roundtrip() {
        let raw = "22400022111";
        let formatted = formato_cedula(raw);
        assert_eq!(formatted, "224-0002211-1");

        let parsed = parse_cedula(&formatted);
        assert_eq!(parsed, raw);

        // Second format produces the same output
        assert_eq!(formato_cedula(&parsed), formatted);
    }
}
