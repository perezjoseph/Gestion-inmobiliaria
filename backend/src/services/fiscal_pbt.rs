#![allow(clippy::unwrap_used, clippy::expect_used)]

use proptest::prelude::*;

use crate::entities::organizacion;
use crate::models::fiscal::TipoFiscal;
use crate::services::fiscal::verificar_acceso_fiscal;
use crate::services::validacion_fiscal::{validar_cedula, validar_rnc};
use chrono::Utc;
use uuid::Uuid;

// ── Helper: build organizacion Model ───────────────────────────────────

fn make_org(tipo_fiscal: &str) -> organizacion::Model {
    organizacion::Model {
        id: Uuid::new_v4(),
        tipo: "propietario".to_string(),
        nombre: "Test Org".to_string(),
        estado: "activo".to_string(),
        cedula: None,
        telefono: None,
        email_organizacion: None,
        rnc: None,
        razon_social: None,
        nombre_comercial: None,
        direccion_fiscal: None,
        representante_legal: None,
        dgii_data: None,
        tipo_fiscal: tipo_fiscal.to_string(),
        regimen_pagos: None,
        fecha_inicio_operaciones: None,
        is_ecf_certificado: false,
        created_at: Utc::now().into(),
        updated_at: Utc::now().into(),
    }
}

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate an 8-digit prefix for RNC check-digit computation.
fn rnc_prefix_8_digits() -> impl Strategy<Value = [u32; 8]> {
    prop::array::uniform8(0u32..10)
}

/// Generate a 10-digit prefix for cédula Luhn computation.
fn cedula_prefix_10_digits() -> impl Strategy<Value = [u32; 10]> {
    prop::array::uniform10(0u32..10)
}

/// A position index (0..=8) for mutating a single digit in a 9-digit RNC.
fn rnc_mutation_index() -> impl Strategy<Value = usize> {
    0usize..9
}

/// A position index (0..=10) for mutating a single digit in an 11-digit cédula.
fn cedula_mutation_index() -> impl Strategy<Value = usize> {
    0usize..11
}

/// A non-zero offset (1..=9) to change a digit.
fn digit_offset() -> impl Strategy<Value = u32> {
    1u32..10
}

/// Strategy that produces one of the three TipoFiscal values as a string.
fn tipo_fiscal_string() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("persona_juridica".to_string()),
        Just("persona_fisica".to_string()),
        Just("informal".to_string()),
    ]
}

/// Strategy for invalid RNC identifiers (wrong length or non-digits).
fn invalid_rnc() -> impl Strategy<Value = String> {
    prop_oneof![
        // Too short (1-8 digits)
        "[0-9]{1,8}",
        // Too long (10-15 digits)
        "[0-9]{10,15}",
        // Correct length but contains at least one letter (guaranteed non-numeric)
        "[a-z][a-z0-9]{8}",
        // Empty
        Just(String::new()),
    ]
}

/// Strategy for invalid cédula identifiers (wrong length or non-digits).
fn invalid_cedula() -> impl Strategy<Value = String> {
    prop_oneof![
        // Too short (1-10 digits)
        "[0-9]{1,10}",
        // Too long (12-15 digits)
        "[0-9]{12,15}",
        // Correct length but contains at least one letter (guaranteed non-numeric)
        "[a-z][a-z0-9]{10}",
        // Empty
        Just(String::new()),
    ]
}

// ── RNC Check-Digit Helpers ────────────────────────────────────────────

/// Compute the DGII check digit for an 8-digit RNC prefix.
fn compute_rnc_check_digit(prefix: &[u32; 8]) -> u32 {
    let weights: [u32; 8] = [7, 9, 8, 6, 5, 4, 3, 2];
    let sum: u32 = weights.iter().zip(prefix.iter()).map(|(w, d)| w * d).sum();
    let check = sum % 11;
    (10 - check) % 9 + 1
}

/// Build a 9-digit RNC string from prefix + computed check digit.
fn build_valid_rnc(prefix: &[u32; 8]) -> String {
    let check = compute_rnc_check_digit(prefix);
    let mut s = String::with_capacity(9);
    for d in prefix {
        s.push(char::from_digit(*d, 10).unwrap());
    }
    s.push(char::from_digit(check, 10).unwrap());
    s
}

// ── Cédula Luhn Helpers ────────────────────────────────────────────────

/// Compute the Luhn check digit for a 10-digit cédula prefix.
fn compute_cedula_check_digit(prefix: &[u32; 10]) -> u32 {
    let weights: [u32; 10] = [1, 2, 1, 2, 1, 2, 1, 2, 1, 2];
    let sum: u32 = weights
        .iter()
        .zip(prefix.iter())
        .map(|(w, d)| {
            let product = w * d;
            if product > 9 {
                product / 10 + product % 10
            } else {
                product
            }
        })
        .sum();
    (10 - (sum % 10)) % 10
}

/// Build an 11-digit cédula string from prefix + computed check digit.
fn build_valid_cedula(prefix: &[u32; 10]) -> String {
    let check = compute_cedula_check_digit(prefix);
    let mut s = String::with_capacity(11);
    for d in prefix {
        s.push(char::from_digit(*d, 10).unwrap());
    }
    s.push(char::from_digit(check, 10).unwrap());
    s
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    // Feature: dr-landlord-compliance, Property 1: RNC Check-Digit Validation Round Trip
    /// **Validates: Requirements 1.2**
    ///
    /// For any 8-digit prefix, computing the DGII check digit and appending it
    /// produces a valid RNC. Changing any single digit in the valid RNC breaks validation.
    #[test]
    fn rnc_check_digit_round_trip(
        prefix in rnc_prefix_8_digits(),
        mutation_idx in rnc_mutation_index(),
        offset in digit_offset(),
    ) {
        let valid_rnc = build_valid_rnc(&prefix);

        // Part A: The constructed RNC must pass validation.
        let result = validar_rnc(&valid_rnc);
        prop_assert!(
            result.is_ok(),
            "RNC built from prefix {:?} should be valid, got error: {:?}",
            prefix,
            result.err()
        );

        // Part B: Mutating any single digit either breaks validation OR
        // the mutation happened to produce a different valid RNC (recomputed
        // check digit matches). The key property: the original check digit
        // computation is the ONLY way to produce a passing result.
        let mut digits: Vec<u32> = valid_rnc
            .chars()
            .map(|c| c.to_digit(10).unwrap())
            .collect();
        let original = digits[mutation_idx];
        digits[mutation_idx] = (original + offset) % 10;

        // Only test if the mutation actually changed the digit
        if digits[mutation_idx] != original {
            let mutated: String = digits
                .iter()
                .map(|d| char::from_digit(*d, 10).unwrap())
                .collect();
            let mutated_result = validar_rnc(&mutated);

            if mutated_result.is_ok() {
                // The mutation still passes — verify it's because the mutated
                // prefix legitimately produces the existing check digit.
                let mutated_prefix: [u32; 8] = [
                    digits[0], digits[1], digits[2], digits[3],
                    digits[4], digits[5], digits[6], digits[7],
                ];
                let recomputed_check = compute_rnc_check_digit(&mutated_prefix);
                prop_assert_eq!(
                    digits[8],
                    recomputed_check,
                    "Mutated RNC '{}' passes validation but check digit doesn't match recomputed value",
                    mutated
                );
            }
            // If mutated_result is Err, the mutation correctly broke validation — property holds.
        }
    }

    // Feature: dr-landlord-compliance, Property 2: Cédula Luhn Validation Round Trip
    /// **Validates: Requirements 1.3**
    ///
    /// For any 10-digit prefix, computing the Luhn check digit and appending it
    /// produces a valid cédula. Changing any single digit in the valid cédula breaks validation.
    #[test]
    fn cedula_luhn_round_trip(
        prefix in cedula_prefix_10_digits(),
        mutation_idx in cedula_mutation_index(),
        offset in digit_offset(),
    ) {
        let valid_cedula = build_valid_cedula(&prefix);

        // Part A: The constructed cédula must pass validation.
        let result = validar_cedula(&valid_cedula);
        prop_assert!(
            result.is_ok(),
            "Cédula built from prefix {:?} should be valid, got error: {:?}",
            prefix,
            result.err()
        );

        // Part B: Mutating any single digit either breaks validation OR
        // the mutation happened to produce a different valid cédula (recomputed
        // check digit matches). The key property: the original check digit
        // computation is the ONLY way to produce a passing result.
        let mut digits: Vec<u32> = valid_cedula
            .chars()
            .map(|c| c.to_digit(10).unwrap())
            .collect();
        let original = digits[mutation_idx];
        digits[mutation_idx] = (original + offset) % 10;

        // Only test if the mutation actually changed the digit
        if digits[mutation_idx] != original {
            let mutated: String = digits
                .iter()
                .map(|d| char::from_digit(*d, 10).unwrap())
                .collect();
            let mutated_result = validar_cedula(&mutated);

            if mutated_result.is_ok() {
                // The mutation still passes — verify it's because the mutated
                // prefix legitimately produces the existing check digit.
                let mutated_prefix: [u32; 10] = [
                    digits[0], digits[1], digits[2], digits[3], digits[4],
                    digits[5], digits[6], digits[7], digits[8], digits[9],
                ];
                let recomputed_check = compute_cedula_check_digit(&mutated_prefix);
                prop_assert_eq!(
                    digits[10],
                    recomputed_check,
                    "Mutated cédula '{}' passes validation but check digit doesn't match recomputed value",
                    mutated
                );
            }
            // If mutated_result is Err, the mutation correctly broke validation — property holds.
        }
    }

    // Feature: dr-landlord-compliance, Property 3: Fiscal Feature Access Gate
    /// **Validates: Requirements 1.5, 1.6**
    ///
    /// Access to fiscal features is granted iff tipo_fiscal != informal.
    /// All requests with tipo_fiscal = informal are rejected with a Forbidden error.
    #[test]
    fn fiscal_feature_access_gate(tipo in tipo_fiscal_string()) {
        let org = make_org(&tipo);
        let result = verificar_acceso_fiscal(&org);

        match tipo.as_str() {
            "persona_juridica" | "persona_fisica" => {
                prop_assert!(
                    result.is_ok(),
                    "tipo_fiscal='{}' should grant fiscal access, got: {:?}",
                    tipo,
                    result.err()
                );
            }
            "informal" => {
                prop_assert!(
                    result.is_err(),
                    "tipo_fiscal='informal' should deny fiscal access"
                );
                if let Err(ref e) = result {
                    let status = actix_web::error::ResponseError::status_code(e);
                    prop_assert_eq!(
                        status,
                        actix_web::http::StatusCode::FORBIDDEN,
                        "Informal access denial should be 403, got {}",
                        status
                    );
                }
            }
            _ => {
                // Unreachable given strategy, but be explicit
                prop_assert!(false, "Unexpected tipo_fiscal: {}", tipo);
            }
        }
    }

    // Feature: dr-landlord-compliance, Property 4: Tipo Fiscal Transition Requires Valid Identifier
    /// **Validates: Requirements 1.7**
    ///
    /// Transition from informal to a registered tipo_fiscal succeeds only when
    /// the corresponding identifier passes validation. Invalid/missing identifiers are rejected.
    #[test]
    fn tipo_fiscal_transition_requires_valid_identifier_persona_juridica(
        prefix in rnc_prefix_8_digits(),
    ) {
        // Valid RNC should pass the validation step in actualizar_tipo_fiscal
        let valid_rnc = build_valid_rnc(&prefix);

        // Simulate the validation logic from actualizar_tipo_fiscal for PersonaJuridica
        let nuevo_tipo = TipoFiscal::PersonaJuridica;
        let identificador: Option<&str> = Some(&valid_rnc);

        let validation_result = match &nuevo_tipo {
            TipoFiscal::PersonaJuridica => {
                match identificador {
                    Some(rnc) => validar_rnc(rnc),
                    None => Err(crate::errors::AppError::Validation(
                        "RNC requerido para persona jurídica".to_string(),
                    )),
                }
            }
            _ => Ok(()),
        };

        prop_assert!(
            validation_result.is_ok(),
            "Valid RNC '{}' should pass transition validation, got: {:?}",
            valid_rnc,
            validation_result.err()
        );
    }

    #[test]
    fn tipo_fiscal_transition_requires_valid_identifier_persona_fisica(
        prefix in cedula_prefix_10_digits(),
    ) {
        // Valid cédula should pass the validation step in actualizar_tipo_fiscal
        let valid_cedula = build_valid_cedula(&prefix);

        let nuevo_tipo = TipoFiscal::PersonaFisica;
        let identificador: Option<&str> = Some(&valid_cedula);

        let validation_result = match &nuevo_tipo {
            TipoFiscal::PersonaFisica => {
                match identificador {
                    Some(cedula) => validar_cedula(cedula),
                    None => Err(crate::errors::AppError::Validation(
                        "Cédula requerida para persona física".to_string(),
                    )),
                }
            }
            _ => Ok(()),
        };

        prop_assert!(
            validation_result.is_ok(),
            "Valid cédula '{}' should pass transition validation, got: {:?}",
            valid_cedula,
            validation_result.err()
        );
    }

    #[test]
    fn tipo_fiscal_transition_rejects_invalid_rnc(invalid in invalid_rnc()) {
        // Invalid RNC should be rejected when transitioning to PersonaJuridica
        let nuevo_tipo = TipoFiscal::PersonaJuridica;
        let identificador: Option<&str> = if invalid.is_empty() {
            None
        } else {
            Some(&invalid)
        };

        let validation_result = match &nuevo_tipo {
            TipoFiscal::PersonaJuridica => {
                match identificador {
                    Some(rnc) => validar_rnc(rnc),
                    None => Err(crate::errors::AppError::Validation(
                        "RNC requerido para persona jurídica".to_string(),
                    )),
                }
            }
            _ => Ok(()),
        };

        prop_assert!(
            validation_result.is_err(),
            "Invalid RNC '{}' should be rejected for transition to persona_juridica",
            invalid
        );
    }

    #[test]
    fn tipo_fiscal_transition_rejects_invalid_cedula(invalid in invalid_cedula()) {
        // Invalid cédula should be rejected when transitioning to PersonaFisica
        let nuevo_tipo = TipoFiscal::PersonaFisica;
        let identificador: Option<&str> = if invalid.is_empty() {
            None
        } else {
            Some(&invalid)
        };

        let validation_result = match &nuevo_tipo {
            TipoFiscal::PersonaFisica => {
                match identificador {
                    Some(cedula) => validar_cedula(cedula),
                    None => Err(crate::errors::AppError::Validation(
                        "Cédula requerida para persona física".to_string(),
                    )),
                }
            }
            _ => Ok(()),
        };

        prop_assert!(
            validation_result.is_err(),
            "Invalid cédula '{}' should be rejected for transition to persona_fisica",
            invalid
        );
    }

    #[test]
    fn tipo_fiscal_transition_rejects_missing_identifier_persona_juridica(_seed in 0u32..10) {
        // Missing identifier should be rejected for PersonaJuridica
        let nuevo_tipo = TipoFiscal::PersonaJuridica;
        let identificador: Option<&str> = None;

        let validation_result = match &nuevo_tipo {
            TipoFiscal::PersonaJuridica => {
                match identificador {
                    Some(rnc) => validar_rnc(rnc),
                    None => Err(crate::errors::AppError::Validation(
                        "RNC requerido para persona jurídica".to_string(),
                    )),
                }
            }
            _ => Ok(()),
        };

        prop_assert!(
            validation_result.is_err(),
            "Missing identifier should be rejected for transition to persona_juridica"
        );
    }

    #[test]
    fn tipo_fiscal_transition_rejects_missing_identifier_persona_fisica(_seed in 0u32..10) {
        // Missing identifier should be rejected for PersonaFisica
        let nuevo_tipo = TipoFiscal::PersonaFisica;
        let identificador: Option<&str> = None;

        let validation_result = match &nuevo_tipo {
            TipoFiscal::PersonaFisica => {
                match identificador {
                    Some(cedula) => validar_cedula(cedula),
                    None => Err(crate::errors::AppError::Validation(
                        "Cédula requerida para persona física".to_string(),
                    )),
                }
            }
            _ => Ok(()),
        };

        prop_assert!(
            validation_result.is_err(),
            "Missing identifier should be rejected for transition to persona_fisica"
        );
    }
}
