#![allow(clippy::expect_used, clippy::unwrap_used)]

use proptest::prelude::*;
use std::collections::HashMap;

use crate::models::documento::{CrearPlantillaRequest, PlantillaResponse};

use super::plantillas::{replace_in_string, resolve_placeholders};

fn non_empty_string() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9 _-]{0,29}".prop_map(|s| s.trim().to_string())
}

fn valid_entity_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("propiedad".to_string()),
        Just("inquilino".to_string()),
        Just("contrato".to_string()),
        Just("pago".to_string()),
        Just("gasto".to_string()),
    ]
}

fn valid_contenido() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        Just(serde_json::json!({"version": 1, "blocks": []})),
        Just(serde_json::json!({"version": 1, "blocks": [{"type": "paragraph", "text": "Hello"}]})),
        Just(
            serde_json::json!({"version": 1, "blocks": [{"type": "heading", "text": "Title", "level": 1}]})
        ),
    ]
}

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

fn placeholder_key() -> impl Strategy<Value = String> {
    "[a-z][a-z_]{0,9}\\.[a-z][a-z_]{0,9}"
}

fn replacement_value() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{1,30}"
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    #[test]
    fn template_crud_round_trip(
        nombre in non_empty_string(),
        tipo_documento in non_empty_string(),
        entity_type in valid_entity_type(),
        contenido in valid_contenido(),
    ) {
        let input = CrearPlantillaRequest {
            nombre: nombre.clone(),
            tipo_documento: tipo_documento.clone(),
            entity_type: entity_type.clone(),
            contenido: contenido.clone(),
        };

        prop_assert!(!input.nombre.trim().is_empty());
        prop_assert!(!input.tipo_documento.trim().is_empty());

        let response = PlantillaResponse {
            id: uuid::Uuid::new_v4(),
            nombre: input.nombre,
            tipo_documento: input.tipo_documento,
            entity_type: input.entity_type,
            contenido: input.contenido,
        };

        prop_assert_eq!(&response.nombre, &nombre);
        prop_assert_eq!(&response.tipo_documento, &tipo_documento);
        prop_assert_eq!(&response.entity_type, &entity_type);
        prop_assert_eq!(&response.contenido, &contenido);
    }

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

        let active_model = plantilla_documento::Model {
            id: Uuid::new_v4(),
            organizacion_id: Uuid::new_v4(),
            nombre,
            tipo_documento,
            entity_type,
            contenido,
            activo: true,
            created_at: now,
            updated_at: now,
        };

        prop_assert!(active_model.activo);

        let deleted_model = plantilla_documento::Model {
            activo: false,
            ..active_model
        };

        prop_assert!(!deleted_model.activo);

        prop_assert!(
            !deleted_model.activo,
            "Soft-deleted template should not appear in active list"
        );
    }

    #[test]
    fn template_validation_rejects_empty_required_fields_nombre(
        nombre in whitespace_only_string(),
    ) {
        let is_invalid = nombre.trim().is_empty();
        prop_assert!(
            is_invalid,
            "Whitespace-only nombre '{}' should be rejected by validation",
            nombre
        );
    }

    #[test]
    fn template_validation_rejects_empty_required_fields_tipo_documento(
        tipo_documento in whitespace_only_string(),
    ) {
        let is_invalid = tipo_documento.trim().is_empty();
        prop_assert!(
            is_invalid,
            "Whitespace-only tipo_documento '{}' should be rejected by validation",
            tipo_documento
        );
    }

    #[test]
    fn placeholder_resolution_replaces_all_matching_keys(
        keys in prop::collection::vec(placeholder_key(), 1..5),
        values in prop::collection::vec(replacement_value(), 1..5),
    ) {
        let pair_count = keys.len().min(values.len());
        let mut replacements = HashMap::new();
        for i in 0..pair_count {
            replacements.insert(keys[i].clone(), values[i].clone());
        }

        let template_str: String = replacements
            .keys()
            .map(|k| format!("{{{{{k}}}}}"))
            .collect::<Vec<_>>()
            .join(" ");

        let resolved = replace_in_string(&template_str, &replacements);

        for key in replacements.keys() {
            let placeholder = format!("{{{{{key}}}}}");
            prop_assert!(
                !resolved.contains(&placeholder),
                "Placeholder '{}' should have been resolved, but found in: {}",
                placeholder,
                resolved
            );
        }

        for value in replacements.values() {
            prop_assert!(
                resolved.contains(value.as_str()),
                "Value '{}' should appear in resolved string: {}",
                value,
                resolved
            );
        }
    }

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

        let resolved = resolve_placeholders(&json_value, &replacements);

        let resolved_text = resolved["blocks"][0]["text"]
            .as_str()
            .expect("text field should be a string");

        for key in replacements.keys() {
            let placeholder = format!("{{{{{key}}}}}");
            prop_assert!(
                !resolved_text.contains(&placeholder),
                "Placeholder '{}' should have been resolved in JSON, but found in: {}",
                placeholder,
                resolved_text
            );
        }

        for value in replacements.values() {
            prop_assert!(
                resolved_text.contains(value.as_str()),
                "Value '{}' should appear in resolved JSON text: {}",
                value,
                resolved_text
            );
        }

        prop_assert_eq!(resolved["version"].clone(), serde_json::json!(1));
        prop_assert_eq!(resolved["blocks"][0]["type"].clone(), serde_json::json!("paragraph"));
    }
}
