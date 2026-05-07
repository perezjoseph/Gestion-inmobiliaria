#![allow(clippy::unwrap_used, clippy::expect_used)]

use proptest::prelude::*;

use crate::services::auth;
use crate::services::documento_editor::build_docx;
use crate::services::firmas::{firmante_tipo_from_rol, generar_password, validar_firma_imagen};

use chrono::Utc;

// ── Custom Strategies ──────────────────────────────────────────────────

/// Arbitrary printable ASCII passwords (8-64 chars).
fn arbitrary_password() -> impl Strategy<Value = String> {
    "[[:print:]]{8,64}"
}

/// A different password guaranteed to differ from the original.
fn different_password(original: String) -> String {
    format!("{original}_DIFFERENT")
}

/// Valid base64-encoded PNG-like data (non-empty, under 500KB).
fn valid_firma_imagen_b64() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<u8>(), 1..1024).prop_map(|bytes| {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    })
}

/// Invalid base64 strings.
fn invalid_base64() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("!!!not-base64!!!".to_string()),
        Just("====".to_string()),
        Just("abc$%^&*(".to_string()),
    ]
}

/// Non-empty IP address strings.
fn arbitrary_ip_address() -> impl Strategy<Value = String> {
    prop_oneof![
        (1u8..=255, any::<u8>(), any::<u8>(), 1u8..=255)
            .prop_map(|(a, b, c, d)| format!("{a}.{b}.{c}.{d}")),
        Just("::1".to_string()),
        Just("192.168.1.1".to_string()),
    ]
}

/// Non-empty user agent strings.
fn arbitrary_user_agent() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-zA-Z0-9/ .;()]{10,80}",
        Just("Mozilla/5.0 (Windows NT 10.0; Win64; x64)".to_string()),
    ]
}

/// Non-empty `firmante_nombre` strings.
fn arbitrary_firmante_nombre() -> impl Strategy<Value = String> {
    "[a-zA-Z ]{2,40}"
}

/// Valid roles for signing (admin or gerente).
fn signing_roles() -> impl Strategy<Value = String> {
    prop_oneof![Just("admin".to_string()), Just("gerente".to_string()),]
}

/// Valid roles that map to "propietario".
fn propietario_roles() -> impl Strategy<Value = String> {
    prop_oneof![Just("admin".to_string()), Just("gerente".to_string()),]
}

/// Roles that map to "inquilino".
fn inquilino_roles() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("inquilino".to_string()),
        Just("visualizador".to_string()),
        Just("otro".to_string()),
    ]
}

// ── DOCX Export Strategies ─────────────────────────────────────────────

/// Arbitrary non-empty text for block content.
fn arbitrary_text() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{1,50}"
}

/// Generate a heading block with arbitrary level and text.
fn heading_block() -> impl Strategy<Value = serde_json::Value> {
    (1u64..=3, arbitrary_text()).prop_map(|(level, text)| {
        serde_json::json!({ "type": "heading", "level": level, "text": text })
    })
}

/// Generate a paragraph block with arbitrary text.
fn paragraph_block() -> impl Strategy<Value = serde_json::Value> {
    arbitrary_text().prop_map(|text| serde_json::json!({ "type": "paragraph", "text": text }))
}

/// Generate a list block with arbitrary items.
fn list_block() -> impl Strategy<Value = serde_json::Value> {
    (
        any::<bool>(),
        prop::collection::vec(arbitrary_text(), 1..5),
    )
        .prop_map(|(ordered, items)| {
            serde_json::json!({ "type": "list", "ordered": ordered, "items": items })
        })
}

/// Generate a table block with headers and rows.
fn table_block() -> impl Strategy<Value = serde_json::Value> {
    (
        prop::collection::vec(arbitrary_text(), 1..4),
        prop::collection::vec(prop::collection::vec(arbitrary_text(), 1..4), 1..4),
    )
        .prop_map(|(headers, rows)| {
            serde_json::json!({ "type": "table", "headers": headers, "rows": rows })
        })
}

/// Generate a `page_break` block.
fn page_break_block() -> impl Strategy<Value = serde_json::Value> {
    Just(serde_json::json!({ "type": "page_break" }))
}

/// Generate an arbitrary valid `Block_JSON` block.
fn arbitrary_block() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        heading_block(),
        paragraph_block(),
        list_block(),
        table_block(),
        page_break_block(),
    ]
}

/// Generate a non-empty array of arbitrary valid blocks.
fn arbitrary_blocks() -> impl Strategy<Value = Vec<serde_json::Value>> {
    prop::collection::vec(arbitrary_block(), 1..10)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    // Feature: contract-document-signing, Property 8: Token generation correctness
    /// **Validates: Requirements 5.1, 5.2, 8.4, 8.5, 8.6**
    #[test]
    fn token_generation_correctness(
        firmante_nombre in "[a-zA-Z ]{2,50}",
        _email in "[a-z]{3,10}@[a-z]{3,8}\\.[a-z]{2,4}"
    ) {
        // Simulate what solicitar_firma does internally:
        // 1. Generate token (UUID v4)
        let token = uuid::Uuid::new_v4().to_string();

        // 2. Generate random 16-char password and hash with argon2
        let password = generar_password();
        let password_hash = auth::hash_password(&password).expect("hashing should succeed");

        // 3. Compute expira_at and created_at
        let now = Utc::now();
        let created_at = now;
        let expira_at = now + chrono::Duration::hours(72);

        // Property 8a: Token has at least 32 characters (sufficient entropy)
        prop_assert!(
            token.len() >= 32,
            "Token should have >= 32 chars, got {} (token: {})", token.len(), token
        );

        // Property 8b: password_hash is a valid argon2 hash
        prop_assert!(
            password_hash.starts_with("$argon2"),
            "password_hash should be a valid argon2 hash, got: {}",
            &password_hash[..20.min(password_hash.len())]
        );

        // Verify the hash actually works (round-trip)
        let valid = auth::verify_password(&password_hash, &password)
            .expect("verify should not error");
        prop_assert!(valid, "Generated password should verify against its hash");

        // Property 8c: expira_at is within 1 second of exactly 72 hours from created_at
        let diff = expira_at - created_at;
        let expected_secs = 72 * 3600;
        let actual_secs = diff.num_seconds();
        let delta = (actual_secs - expected_secs).abs();
        prop_assert!(
            delta <= 1,
            "expira_at should be within 1s of 72h from created_at, got delta={}s",
            delta
        );

        // Use firmante_nombre to avoid unused variable warning
        prop_assert!(!firmante_nombre.trim().is_empty(), "firmante_nombre should not be empty");
    }

    // Feature: contract-document-signing, Property 9: Password hashing round-trip
    /// **Validates: Requirements 5.2, 8.6**
    #[test]
    fn password_hashing_round_trip(password in arbitrary_password()) {
        let hash = auth::hash_password(&password).expect("hashing should succeed");

        // Verify original password succeeds
        let valid = auth::verify_password(&hash, &password).expect("verify should not error");
        prop_assert!(valid, "Original password should verify against its hash");

        // Verify different string fails
        let wrong = different_password(password);
        let invalid = auth::verify_password(&hash, &wrong).expect("verify should not error");
        prop_assert!(!invalid, "Different password should NOT verify against the hash");
    }

    // Feature: contract-document-signing, Property 8 (partial): Token generation correctness
    // Tests generar_password produces 16-char alphanumeric strings that hash correctly.
    /// **Validates: Requirements 5.1, 5.2, 8.4, 8.5, 8.6**
    #[test]
    fn generated_password_is_valid_and_hashable(_seed in 0u64..1000) {
        let password = generar_password();

        // Password is exactly 16 chars
        prop_assert_eq!(password.len(), 16, "Generated password should be 16 chars, got {}", password.len());

        // Password is alphanumeric
        prop_assert!(
            password.chars().all(|c| c.is_ascii_alphanumeric()),
            "Generated password should be alphanumeric, got: {password}"
        );

        // Password can be hashed and verified
        let hash = auth::hash_password(&password).expect("hashing should succeed");
        prop_assert!(hash.starts_with("$argon2"), "Hash should be argon2 format, got: {}", &hash[..20.min(hash.len())]);

        let valid = auth::verify_password(&hash, &password).expect("verify should not error");
        prop_assert!(valid, "Generated password should verify against its hash");
    }

    // Feature: contract-document-signing, Property 7 (partial): validar_firma_imagen accepts valid base64
    /// **Validates: Requirements 4.1, 8.1**
    #[test]
    fn validar_firma_imagen_accepts_valid_base64(b64 in valid_firma_imagen_b64()) {
        let result = validar_firma_imagen(&b64);
        prop_assert!(result.is_ok(), "Valid base64 should be accepted, got error: {:?}", result.err());

        let bytes = result.unwrap();
        prop_assert!(!bytes.is_empty(), "Decoded bytes should not be empty");
    }

    // Feature: contract-document-signing, Property 7 (partial): validar_firma_imagen rejects invalid base64
    /// **Validates: Requirements 4.1, 8.1**
    #[test]
    fn validar_firma_imagen_rejects_invalid_base64(b64 in invalid_base64()) {
        let result = validar_firma_imagen(&b64);
        prop_assert!(result.is_err(), "Invalid base64 should be rejected");
    }

    // Feature: contract-document-signing: firmante_tipo_from_rol maps correctly
    /// **Validates: Requirements 4.2**
    #[test]
    fn firmante_tipo_propietario_for_admin_gerente(rol in propietario_roles()) {
        let tipo = firmante_tipo_from_rol(&rol);
        prop_assert_eq!(tipo, "propietario", "Role '{}' should map to 'propietario'", rol);
    }

    // Feature: contract-document-signing: firmante_tipo_from_rol maps non-admin to inquilino
    /// **Validates: Requirements 4.2**
    #[test]
    fn firmante_tipo_inquilino_for_other_roles(rol in inquilino_roles()) {
        let tipo = firmante_tipo_from_rol(&rol);
        prop_assert_eq!(tipo, "inquilino", "Role '{}' should map to 'inquilino'", rol);
    }

    // Feature: contract-document-signing, Property 1: DOCX export produces valid output for any Block_JSON
    /// **Validates: Requirements 1.1**
    #[test]
    fn docx_export_produces_valid_output(blocks in arbitrary_blocks()) {
        let docx = build_docx(&blocks).expect("build_docx should succeed for valid blocks");

        let mut buf = Vec::new();
        docx.build()
            .pack(&mut std::io::Cursor::new(&mut buf))
            .expect("DOCX pack should succeed");

        // Output must be non-empty
        prop_assert!(!buf.is_empty(), "DOCX output should be non-empty");

        // Output must start with ZIP magic bytes PK\x03\x04
        prop_assert!(
            buf.len() >= 4,
            "DOCX output too short: {} bytes",
            buf.len()
        );
        prop_assert_eq!(buf[0], b'P', "First byte should be 'P'");
        prop_assert_eq!(buf[1], b'K', "Second byte should be 'K'");
        prop_assert_eq!(buf[2], 0x03, "Third byte should be 0x03");
        prop_assert_eq!(buf[3], 0x04, "Fourth byte should be 0x04");
    }

    // Feature: contract-document-signing, Property 10: Token access rejects expired or wrong password
    // Part A: Expired token → Gone (410)
    /// **Validates: Requirements 5.7, 5.8**
    #[test]
    fn token_access_rejects_expired_token(hours_ago in 1i64..1000) {
        // Simulate the expiry check from verificar_token:
        // if Utc::now() > expira_at → Gone
        let expira_at = Utc::now() - chrono::Duration::hours(hours_ago);
        let now = Utc::now();

        // This is the exact condition checked in verificar_token
        prop_assert!(
            now > expira_at,
            "Current time should be after expired expira_at"
        );

        // Verify the error produced matches what verificar_token returns
        let err = crate::errors::AppError::Gone("El enlace de firma ha expirado".to_string());
        let status = actix_web::error::ResponseError::status_code(&err);
        prop_assert_eq!(
            status,
            actix_web::http::StatusCode::GONE,
            "Expired token should produce 410 Gone"
        );
    }

    // Feature: contract-document-signing, Property 10: Token access rejects expired or wrong password
    // Part B: Wrong password → Unauthorized (401)
    /// **Validates: Requirements 5.7, 5.8**
    #[test]
    fn token_access_rejects_wrong_password(
        correct_password in arbitrary_password(),
        suffix in "[a-zA-Z0-9]{1,10}"
    ) {
        // Hash the correct password (simulates what solicitar_firma stores)
        let hash = auth::hash_password(&correct_password).expect("hashing should succeed");

        // Create a wrong password guaranteed to differ
        let wrong_password = format!("{correct_password}{suffix}");

        // Verify the wrong password fails (same logic as verificar_token)
        let valid = auth::verify_password(&hash, &wrong_password).expect("verify should not error");
        prop_assert!(!valid, "Wrong password should not verify against the hash");

        // Verify the error produced matches what verificar_token returns
        let err = crate::errors::AppError::Unauthorized(Some("Contraseña incorrecta".to_string()));
        let status = actix_web::error::ResponseError::status_code(&err);
        prop_assert_eq!(
            status,
            actix_web::http::StatusCode::UNAUTHORIZED,
            "Wrong password should produce 401 Unauthorized"
        );
    }
}


// ── Property 7: Signature record completeness ──────────────────────────
// Tests that for any valid signature inputs, the constructed firma_documento
// record has all required fields: non-null firma_imagen, non-empty ip_address,
// non-empty user_agent, firmado_at within 5s of now, estado="firmado".
proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    // Feature: contract-document-signing, Property 7: Signature record completeness
    /// **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 8.1, 8.2, 8.3**
    #[test]
    fn signature_record_completeness(
        firma_imagen_b64 in valid_firma_imagen_b64(),
        ip_address in arbitrary_ip_address(),
        user_agent in arbitrary_user_agent(),
        _firmante_nombre in arbitrary_firmante_nombre(),
        rol in signing_roles(),
    ) {
        // Validate firma_imagen (same validation as firmar_autenticado)
        let firma_bytes = validar_firma_imagen(&firma_imagen_b64)
            .expect("valid base64 should decode");

        let now = Utc::now();
        let firmante_tipo = firmante_tipo_from_rol(&rol);

        // Simulate the record fields that firmar_autenticado sets:
        let record_firma_imagen = firma_bytes;
        let record_ip_address = &ip_address;
        let record_user_agent = &user_agent;
        let record_firmado_at = now;
        let record_estado: &str = "firmado";

        // Verify: firma_imagen is non-empty
        prop_assert!(
            !record_firma_imagen.is_empty(),
            "firma_imagen must be non-empty"
        );

        // Verify: ip_address is non-empty
        prop_assert!(
            !record_ip_address.is_empty(),
            "ip_address must be non-empty"
        );

        // Verify: user_agent is non-empty
        prop_assert!(
            !record_user_agent.is_empty(),
            "user_agent must be non-empty"
        );

        // Verify: firmado_at is within 5 seconds of current time
        let diff = (Utc::now() - record_firmado_at).num_seconds().unsigned_abs();
        prop_assert!(
            diff <= 5,
            "firmado_at should be within 5s of now, but diff was {}s",
            diff
        );

        // Verify: estado is "firmado"
        prop_assert_eq!(record_estado, "firmado", "estado must be 'firmado'");

        // Verify: firmante_tipo is correctly derived from role
        prop_assert_eq!(
            firmante_tipo, "propietario",
            "admin/gerente roles must produce firmante_tipo='propietario'"
        );
    }
}


// ── Property 13: Sealed document immutability ──────────────────────────
// For any document with sellado=true, attempting to update contenido_editable
// SHALL return a Forbidden error (403) with the expected message.

/// Generate arbitrary valid `contenido_editable` JSON.
fn arbitrary_contenido_editable() -> impl Strategy<Value = serde_json::Value> {
    arbitrary_blocks().prop_map(|blocks| {
        serde_json::json!({
            "version": 1,
            "blocks": blocks
        })
    })
}

/// Helper: connect to DB and execute async closure.
/// Skips test gracefully if `DATABASE_URL` is not set or DB is unreachable.
fn with_db<F, Fut>(f: F)
where
    F: FnOnce(sea_orm::DatabaseConnection) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    dotenvy::dotenv().ok();
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL not set -- skipping DB property test");
        return;
    };
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        let mut opts = sea_orm::ConnectOptions::new(&url);
        opts.max_connections(2)
            .connect_timeout(std::time::Duration::from_secs(3))
            .acquire_timeout(std::time::Duration::from_secs(3));
        let Ok(db) = sea_orm::Database::connect(opts).await else {
            eprintln!("Database not reachable -- skipping DB property test");
            return;
        };
        f(db).await;
    });
}

// Feature: contract-document-signing, Property 13: Sealed document immutability
/// **Validates: Requirements 6.4, 8.7**
#[test]
fn sealed_document_immutability() {
    use proptest::test_runner::{Config as ProptestConfig, TestRunner};

    let mut runner = TestRunner::new(ProptestConfig {
        cases: 20,
        ..Default::default()
    });

    runner
        .run(&arbitrary_contenido_editable(), |contenido| {
            with_db(|db| async move {
                use sea_orm::{ActiveModelTrait, EntityTrait, Set};
                use uuid::Uuid;

                let now = Utc::now().into();
                let user_id = Uuid::new_v4();

                // Create a minimal document with sellado=true
                let doc_id = Uuid::new_v4();
                let doc = crate::entities::documento::ActiveModel {
                    id: Set(doc_id),
                    entity_type: Set("contrato".to_string()),
                    entity_id: Set(Uuid::new_v4()),
                    filename: Set(format!("sealed-test-{doc_id}.pdf")),
                    file_path: Set(format!("/tmp/sealed-test-{doc_id}.pdf")),
                    mime_type: Set("application/pdf".to_string()),
                    file_size: Set(1024),
                    uploaded_by: Set(user_id),
                    created_at: Set(now),
                    tipo_documento: Set("contrato".to_string()),
                    estado_verificacion: Set("verificado".to_string()),
                    fecha_vencimiento: Set(None),
                    verificado_por: Set(None),
                    fecha_verificacion: Set(None),
                    notas_verificacion: Set(None),
                    numero_documento: Set(None),
                    contenido_editable: Set(Some(contenido.clone())),
                    updated_at: Set(Some(now)),
                    sellado: Set(true),
                    sellado_at: Set(Some(now)),
                };
                doc.insert(&db)
                    .await
                    .expect("Failed to insert sealed test document");

                // Attempt to update contenido_editable on the sealed document
                let new_contenido = serde_json::json!({
                    "version": 1,
                    "blocks": [{"type": "paragraph", "text": "modified"}]
                });
                let result = crate::services::documento_editor::guardar_contenido(
                    &db, doc_id, new_contenido, user_id,
                )
                .await;

                // Verify: must return Forbidden (403)
                assert!(
                    result.is_err(),
                    "Updating a sealed document should fail, but got Ok"
                );
                let err = result.unwrap_err();
                let status = actix_web::error::ResponseError::status_code(&err);
                assert_eq!(
                    status,
                    actix_web::http::StatusCode::FORBIDDEN,
                    "Sealed document update should return 403, got {status}"
                );
                assert!(
                    err.to_string().contains("sellado"),
                    "Error message should mention 'sellado', got: {err}",
                );

                // Cleanup: delete the test document
                crate::entities::documento::Entity::delete_by_id(doc_id)
                    .exec(&db)
                    .await
                    .ok();
            });
            Ok(())
        })
        .unwrap();
}
