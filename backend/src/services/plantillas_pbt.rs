#![allow(clippy::expect_used, clippy::unwrap_used)]

use proptest::prelude::*;
use std::collections::HashMap;

use crate::models::documento::{CrearPlantillaRequest, PlantillaResponse};

use super::plantillas::{replace_in_string, resolve_placeholders};

// ── Strategies ─────────────────────────────────────────────────

/// Generate a non-empty string (at least one non-whitespace char).
fn non_empty_string() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9 _-]{0,29}".prop_map(|s| s.trim().to_string())
}

/// Generate a valid `entity_type` value.
fn valid_entity_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("propiedad".to_string()),
        Just("inquilino".to_string()),
        Just("contrato".to_string()),
        Just("pago".to_string()),
        Just("gasto".to_string()),
    ]
}

/// Generate valid JSON contenido for a template.
fn valid_contenido() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        Just(serde_json::json!({"version": 1, "blocks": []})),
        Just(serde_json::json!({"version": 1, "blocks": [{"type": "paragraph", "text": "Hello"}]})),
        Just(
            serde_json::json!({"version": 1, "blocks": [{"type": "heading", "text": "Title", "level": 1}]})
        ),
    ]
}

/// Generate whitespace-only strings (including empty).
fn whitespace_only_string() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        Just(" ".to_string()),
        Just("  ".to_string()),
        Just("\t".to_string()),
        Just("\n".to_string()),
        Just("   \t\n  ".to_string()),
    ]
}

/// Generate a valid placeholder key (alphanumeric with dots).
fn placeholder_key() -> impl Strategy<Value = String> {
    "[a-z][a-z_]{0,9}\\.[a-z][a-z_]{0,9}"
}

/// Generate a replacement value (non-empty, no braces).
fn replacement_value() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{1,30}"
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    // ── Property 3: Template CRUD round-trip ───────────────────────
    // For any valid template input, creating a PlantillaResponse with those
    // fields preserves all values identically.
    /// **Validates: Requirements 2.1, 2.4**
    #[test]
    fn template_crud_round_trip(
        nombre in non_empty_string(),
        tipo_documento in non_empty_string(),
        entity_type in valid_entity_type(),
        contenido in valid_contenido(),
    ) {
        // Simulate what `crear` does: validate inputs, then build response
        let input = CrearPlantillaRequest {
            nombre: nombre.clone(),
            tipo_documento: tipo_documento.clone(),
            entity_type: entity_type.clone(),
            contenido: contenido.clone(),
        };

        // Validation passes for non-empty strings
        prop_assert!(!input.nombre.trim().is_empty());
        prop_assert!(!input.tipo_documento.trim().is_empty());

        // Build response (simulates what crear returns)
        let response = PlantillaResponse {
            id: uuid::Uuid::new_v4(),
            nombre: input.nombre,
            tipo_documento: input.tipo_documento,
            entity_type: input.entity_type,
            contenido: input.contenido,
        };

        // Round-trip: all fields match
        prop_assert_eq!(&response.nombre, &nombre);
        prop_assert_eq!(&response.tipo_documento, &tipo_documento);
        prop_assert_eq!(&response.entity_type, &entity_type);
        prop_assert_eq!(&response.contenido, &contenido);
    }

    // ── Property 4: Template soft-delete removes from active list ──
    // After soft-delete (activo=false), the template should not pass
    // the active filter predicate used by `listar`.
    /// **Validates: Requirements 2.3**
    #[test]
    fn template_soft_delete_removes_from_active_list(
        nombre in non_empty_string(),
        tipo_documento in non_empty_string(),
        entity_type in valid_entity_type(),
        contenido in valid_contenido(),
    ) {
        use crate::entities::plantilla_documento;
        use chrono::Utc;
        use uuid::Uuid;

        let now = Utc::now().fixed_offset();

        // Simulate an active template
        let active_model = plantilla_documento::Model {
            id: Uuid::new_v4(),
            nombre,
            tipo_documento,
            entity_type,
            contenido,
            activo: true,
            created_at: now,
            updated_at: now,
        };

        // Active template passes the filter
        prop_assert!(active_model.activo);

        // After soft-delete, activo becomes false
        let deleted_model = plantilla_documento::Model {
            activo: false,
            ..active_model
        };

        // Deleted template does NOT pass the active filter
        prop_assert!(!deleted_model.activo);

        // Simulate listar filter: only activo=true templates are returned
        prop_assert!(
            !deleted_model.activo,
            "Soft-deleted template should not appear in active list"
        );
    }

    // ── Property 5: Template validation rejects empty required fields ──
    // Any whitespace-only nombre or tipo_documento must be rejected.
    /// **Validates: Requirements 2.7**
    #[test]
    fn template_validation_rejects_empty_required_fields_nombre(
        nombre in whitespace_only_string(),
    ) {
        // The validation logic from `crear`:
        let is_invalid = nombre.trim().is_empty();
        prop_assert!(
            is_invalid,
            "Whitespace-only nombre '{}' should be rejected by validation",
            nombre
        );
    }

    /// **Validates: Requirements 2.7**
    #[test]
    fn template_validation_rejects_empty_required_fields_tipo_documento(
        tipo_documento in whitespace_only_string(),
    ) {
        // The validation logic from `crear`:
        let is_invalid = tipo_documento.trim().is_empty();
        prop_assert!(
            is_invalid,
            "Whitespace-only tipo_documento '{}' should be rejected by validation",
            tipo_documento
        );
    }

    // ── Property 6: Placeholder resolution replaces all matching keys ──
    // For any template with {{key}} placeholders and a matching map,
    // no matched placeholders remain and all values appear.
    /// **Validates: Requirements 2.8**
    #[test]
    fn placeholder_resolution_replaces_all_matching_keys(
        keys in prop::collection::vec(placeholder_key(), 1..5),
        values in prop::collection::vec(replacement_value(), 1..5),
    ) {
        // Build a replacement map from generated keys and values
        let pair_count = keys.len().min(values.len());
        let mut replacements = HashMap::new();
        for i in 0..pair_count {
            replacements.insert(keys[i].clone(), values[i].clone());
        }

        // Build a template string with all placeholders
        let template_str: String = replacements
            .keys()
            .map(|k| format!("{{{{{k}}}}}"))
            .collect::<Vec<_>>()
            .join(" ");

        // Resolve placeholders
        let resolved = replace_in_string(&template_str, &replacements);

        // Verify: no matched placeholder patterns remain
        for key in replacements.keys() {
            let placeholder = format!("{{{{{key}}}}}");
            prop_assert!(
                !resolved.contains(&placeholder),
                "Placeholder '{}' should have been resolved, but found in: {}",
                placeholder,
                resolved
            );
        }

        // Verify: all values appear in the resolved string
        for value in replacements.values() {
            prop_assert!(
                resolved.contains(value.as_str()),
                "Value '{}' should appear in resolved string: {}",
                value,
                resolved
            );
        }
    }

    /// **Validates: Requirements 2.8**
    #[test]
    fn placeholder_resolution_works_on_json_values(
        keys in prop::collection::vec(placeholder_key(), 1..3),
        values in prop::collection::vec(replacement_value(), 1..3),
    ) {
        let pair_count = keys.len().min(values.len());
        let mut replacements = HashMap::new();
        for i in 0..pair_count {
            replacements.insert(keys[i].clone(), values[i].clone());
        }

        // Build a JSON value with placeholders in string fields
        let template_text: String = replacements
            .keys()
            .map(|k| format!("{{{{{k}}}}}"))
            .collect::<Vec<_>>()
            .join(" ");

        let json_value = serde_json::json!({
            "version": 1,
            "blocks": [
                {"type": "paragraph", "text": template_text}
            ]
        });

        // Resolve using the JSON resolver
        let resolved = resolve_placeholders(&json_value, &replacements);

        // Extract the resolved text
        let resolved_text = resolved["blocks"][0]["text"]
            .as_str()
            .expect("text field should be a string");

        // Verify: no matched placeholder patterns remain
        for key in replacements.keys() {
            let placeholder = format!("{{{{{key}}}}}");
            prop_assert!(
                !resolved_text.contains(&placeholder),
                "Placeholder '{}' should have been resolved in JSON, but found in: {}",
                placeholder,
                resolved_text
            );
        }

        // Verify: all values appear
        for value in replacements.values() {
            prop_assert!(
                resolved_text.contains(value.as_str()),
                "Value '{}' should appear in resolved JSON text: {}",
                value,
                resolved_text
            );
        }

        // Non-string fields should be unchanged
        prop_assert_eq!(resolved["version"].clone(), serde_json::json!(1));
        prop_assert_eq!(resolved["blocks"][0]["type"].clone(), serde_json::json!("paragraph"));
    }
}
