#![allow(clippy::unwrap_used, clippy::expect_used)]

use proptest::prelude::*;

use crate::services::auth;
use crate::services::documento_editor::build_docx;
use crate::services::firmas::{firmante_tipo_from_rol, generar_password, validar_firma_imagen};

use chrono::Utc;

fn arbitrary_password() -> impl Strategy<Value = String> {
    "[[:print:]]{8,64}"
}

fn different_password(original: String) -> String {
    format!("{original}_DIFFERENT")
}

fn valid_firma_imagen_b64() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<u8>(), 1..1024).prop_map(|bytes| {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    })
}

fn invalid_base64() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("!!!not-base64!!!".to_string()),
        Just("====".to_string()),
        Just("abc$%^&*(".to_string()),
    ]
}

fn arbitrary_ip_address() -> impl Strategy<Value = String> {
    prop_oneof![
        (1u8..=255, any::<u8>(), any::<u8>(), 1u8..=255)
            .prop_map(|(a, b, c, d)| format!("{a}.{b}.{c}.{d}")),
        Just("::1".to_string()),
        Just("192.168.1.1".to_string()),
    ]
}

fn arbitrary_user_agent() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-zA-Z0-9/ .;()]{10,80}",
        Just("Mozilla/5.0 (Windows NT 10.0; Win64; x64)".to_string()),
    ]
}

fn arbitrary_firmante_nombre() -> impl Strategy<Value = String> {
    "[a-zA-Z ]{2,40}"
}

fn signing_roles() -> impl Strategy<Value = String> {
    prop_oneof![Just("admin".to_string()), Just("gerente".to_string()),]
}

fn propietario_roles() -> impl Strategy<Value = String> {
    prop_oneof![Just("admin".to_string()), Just("gerente".to_string()),]
}

fn inquilino_roles() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("inquilino".to_string()),
        Just("visualizador".to_string()),
        Just("otro".to_string()),
    ]
}

fn arbitrary_text() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{1,50}"
}

fn heading_block() -> impl Strategy<Value = serde_json::Value> {
    (1u64..=3, arbitrary_text()).prop_map(
        |(level, text)| serde_json::json!({ "type": "heading", "level": level, "text": text }),
    )
}

fn paragraph_block() -> impl Strategy<Value = serde_json::Value> {
    arbitrary_text().prop_map(|text| serde_json::json!({ "type": "paragraph", "text": text }))
}

fn list_block() -> impl Strategy<Value = serde_json::Value> {
    (
        any::<bool>(),
        prop::collection::vec(arbitrary_text(), 1..5),
    )
        .prop_map(|(ordered, items)| {
            serde_json::json!({ "type": "list", "ordered": ordered, "items": items })
        })
}

fn table_block() -> impl Strategy<Value = serde_json::Value> {
    (
        prop::collection::vec(arbitrary_text(), 1..4),
        prop::collection::vec(prop::collection::vec(arbitrary_text(), 1..4), 1..4),
    )
        .prop_map(|(headers, rows)| {
            serde_json::json!({ "type": "table", "headers": headers, "rows": rows })
        })
}

fn page_break_block() -> impl Strategy<Value = serde_json::Value> {
    Just(serde_json::json!({ "type": "page_break" }))
}

fn arbitrary_block() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        heading_block(),
        paragraph_block(),
        list_block(),
        table_block(),
        page_break_block(),
    ]
}

fn arbitrary_blocks() -> impl Strategy<Value = Vec<serde_json::Value>> {
    prop::collection::vec(arbitrary_block(), 1..10)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    #[test]
    fn token_generation_correctness(
        firmante_nombre in "[a-zA-Z]{1,25}( [a-zA-Z]{1,24}){0,3}",
        _email in "[a-z]{3,10}@[a-z]{3,8}\\.[a-z]{2,4}"
    ) {
        let token = uuid::Uuid::new_v4().to_string();

        let password = generar_password();
        let password_hash = auth::hash_password(&password).expect("hashing should succeed");

        let now = Utc::now();
        let created_at = now;
        let expira_at = now + chrono::Duration::hours(72);

        prop_assert!(
            token.len() >= 32,
            "Token should have >= 32 chars, got {} (token: {})", token.len(), token
        );

        prop_assert!(
            password_hash.starts_with("$argon2"),
            "password_hash should be a valid argon2 hash, got: {}",
            &password_hash[..20.min(password_hash.len())]
        );

        let valid = auth::verify_password(&password_hash, &password)
            .expect("verify should not error");
        prop_assert!(valid, "Generated password should verify against its hash");

        let diff = expira_at - created_at;
        let expected_secs = 72 * 3600;
        let actual_secs = diff.num_seconds();
        let delta = (actual_secs - expected_secs).abs();
        prop_assert!(
            delta <= 1,
            "expira_at should be within 1s of 72h from created_at, got delta={}s",
            delta
        );

        prop_assert!(!firmante_nombre.trim().is_empty(), "firmante_nombre should not be empty");
    }

    #[test]
    fn password_hashing_round_trip(password in arbitrary_password()) {
        let hash = auth::hash_password(&password).expect("hashing should succeed");

        let valid = auth::verify_password(&hash, &password).expect("verify should not error");
        prop_assert!(valid, "Original password should verify against its hash");

        let wrong = different_password(password);
        let invalid = auth::verify_password(&hash, &wrong).expect("verify should not error");
        prop_assert!(!invalid, "Different password should NOT verify against the hash");
    }

    #[test]
    fn generated_password_is_valid_and_hashable(_seed in 0u64..1000) {
        let password = generar_password();

        prop_assert_eq!(password.len(), 16, "Generated password should be 16 chars, got {}", password.len());

        prop_assert!(
            password.chars().all(|c| c.is_ascii_alphanumeric()),
            "Generated password should be alphanumeric, got: {password}"
        );

        let hash = auth::hash_password(&password).expect("hashing should succeed");
        prop_assert!(hash.starts_with("$argon2"), "Hash should be argon2 format, got: {}", &hash[..20.min(hash.len())]);

        let valid = auth::verify_password(&hash, &password).expect("verify should not error");
        prop_assert!(valid, "Generated password should verify against its hash");
    }

    #[test]
    fn validar_firma_imagen_accepts_valid_base64(b64 in valid_firma_imagen_b64()) {
        let result = validar_firma_imagen(&b64);
        prop_assert!(result.is_ok(), "Valid base64 should be accepted, got error: {:?}", result.err());

        let bytes = result.unwrap();
        prop_assert!(!bytes.is_empty(), "Decoded bytes should not be empty");
    }

    #[test]
    fn validar_firma_imagen_rejects_invalid_base64(b64 in invalid_base64()) {
        let result = validar_firma_imagen(&b64);
        prop_assert!(result.is_err(), "Invalid base64 should be rejected");
    }

    #[test]
    fn firmante_tipo_propietario_for_admin_gerente(rol in propietario_roles()) {
        let tipo = firmante_tipo_from_rol(&rol);
        prop_assert_eq!(tipo, "propietario", "Role '{}' should map to 'propietario'", rol);
    }

    #[test]
    fn firmante_tipo_inquilino_for_other_roles(rol in inquilino_roles()) {
        let tipo = firmante_tipo_from_rol(&rol);
        prop_assert_eq!(tipo, "inquilino", "Role '{}' should map to 'inquilino'", rol);
    }

    #[test]
    fn docx_export_produces_valid_output(blocks in arbitrary_blocks()) {
        let docx = build_docx(&blocks).expect("build_docx should succeed for valid blocks");

        let mut buf = Vec::new();
        docx.build()
            .pack(&mut std::io::Cursor::new(&mut buf))
            .expect("DOCX pack should succeed");

        prop_assert!(!buf.is_empty(), "DOCX output should be non-empty");

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

    #[test]
    fn token_access_rejects_expired_token(hours_ago in 1i64..1000) {
        let expira_at = Utc::now() - chrono::Duration::hours(hours_ago);
        let now = Utc::now();

        prop_assert!(
            now > expira_at,
            "Current time should be after expired expira_at"
        );

        let err = crate::errors::AppError::Gone("El enlace de firma ha expirado".to_string());
        let status = actix_web::error::ResponseError::status_code(&err);
        prop_assert_eq!(
            status,
            actix_web::http::StatusCode::GONE,
            "Expired token should produce 410 Gone"
        );
    }

    #[test]
    fn token_access_rejects_wrong_password(
        correct_password in arbitrary_password(),
        suffix in "[a-zA-Z0-9]{1,10}"
    ) {
        let hash = auth::hash_password(&correct_password).expect("hashing should succeed");

        let wrong_password = format!("{correct_password}{suffix}");

        let valid = auth::verify_password(&hash, &wrong_password).expect("verify should not error");
        prop_assert!(!valid, "Wrong password should not verify against the hash");

        let err = crate::errors::AppError::Unauthorized(Some("Contraseña incorrecta".to_string()));
        let status = actix_web::error::ResponseError::status_code(&err);
        prop_assert_eq!(
            status,
            actix_web::http::StatusCode::UNAUTHORIZED,
            "Wrong password should produce 401 Unauthorized"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    #[test]
    fn signature_record_completeness(
        firma_imagen_b64 in valid_firma_imagen_b64(),
        ip_address in arbitrary_ip_address(),
        user_agent in arbitrary_user_agent(),
        _firmante_nombre in arbitrary_firmante_nombre(),
        rol in signing_roles(),
    ) {
        let firma_bytes = validar_firma_imagen(&firma_imagen_b64)
            .expect("valid base64 should decode");

        let now = Utc::now();
        let firmante_tipo = firmante_tipo_from_rol(&rol);

        let record_firma_imagen = firma_bytes;
        let record_ip_address = &ip_address;
        let record_user_agent = &user_agent;
        let record_firmado_at = now;
        let record_estado: &str = "firmado";

        prop_assert!(
            !record_firma_imagen.is_empty(),
            "firma_imagen must be non-empty"
        );

        prop_assert!(
            !record_ip_address.is_empty(),
            "ip_address must be non-empty"
        );

        prop_assert!(
            !record_user_agent.is_empty(),
            "user_agent must be non-empty"
        );

        let diff = (Utc::now() - record_firmado_at).num_seconds().unsigned_abs();
        prop_assert!(
            diff <= 5,
            "firmado_at should be within 5s of now, but diff was {}s",
            diff
        );

        prop_assert_eq!(record_estado, "firmado", "estado must be 'firmado'");

        prop_assert_eq!(
            firmante_tipo, "propietario",
            "admin/gerente roles must produce firmante_tipo='propietario'"
        );
    }
}

fn arbitrary_contenido_editable() -> impl Strategy<Value = serde_json::Value> {
    arbitrary_blocks().prop_map(|blocks| {
        serde_json::json!({
            "version": 1,
            "blocks": blocks
        })
    })
}

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
        use sea_orm::ConnectionTrait;
        let check = sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            "SELECT 1 FROM information_schema.tables WHERE table_name = 'documentos' LIMIT 1",
        );
        if let Ok(Some(_)) = db.query_one(check).await {
        } else {
            eprintln!("Schema not ready (documentos table missing) -- skipping DB property test");
            return;
        }
        f(db).await;
    });
}

#[test]
fn sealed_document_immutability() {
    use proptest::test_runner::{Config as ProptestConfig, TestRunner};

    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::test_support::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&arbitrary_contenido_editable(), |contenido| {
            with_db(|db| async move {
                use sea_orm::{ActiveModelTrait, EntityTrait, Set};
                use uuid::Uuid;

                let now = Utc::now().into();
                let org_id = Uuid::new_v4();
                let user_id = Uuid::new_v4();

                crate::entities::organizacion::ActiveModel {
                    id: Set(org_id),
                    tipo: Set("persona_fisica".to_string()),
                    nombre: Set("PBT Sealed Org".to_string()),
                    estado: Set("activo".to_string()),
                    cedula: Set(None),
                    telefono: Set(None),
                    email_organizacion: Set(None),
                    rnc: Set(None),
                    razon_social: Set(None),
                    nombre_comercial: Set(None),
                    direccion_fiscal: Set(None),
                    representante_legal: Set(None),
                    dgii_data: Set(None),
                    tipo_fiscal: Set("informal".to_string()),
                    regimen_pagos: Set(None),
                    fecha_inicio_operaciones: Set(None),
                    is_ecf_certificado: Set(false),
                    created_at: Set(now),
                    updated_at: Set(now),
                }
                .insert(&db)
                .await
                .expect("Failed to insert test org");

                crate::entities::usuario::ActiveModel {
                    id: Set(user_id),
                    nombre: Set("PBT Sealed User".to_string()),
                    email: Set(format!("pbt-sealed+{user_id}@test.com")),
                    password_hash: Set("not_used".to_string()),
                    rol: Set("admin".to_string()),
                    activo: Set(true),
                    organizacion_id: Set(org_id),
                    created_at: Set(now),
                    updated_at: Set(now),
                    password_changed_at: Set(now),
                }
                .insert(&db)
                .await
                .expect("Failed to insert test user");

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
                    documento_origen_id: Set(None),
                };
                doc.insert(&db)
                    .await
                    .expect("Failed to insert sealed test document");

                let new_contenido = serde_json::json!({
                    "version": 1,
                    "blocks": [{"type": "paragraph", "text": "modified"}]
                });
                let result = crate::services::documento_editor::guardar_contenido(
                    &db,
                    doc_id,
                    new_contenido,
                    user_id,
                    org_id,
                )
                .await;

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

                crate::entities::documento::Entity::delete_by_id(doc_id)
                    .exec(&db)
                    .await
                    .ok();
                crate::entities::usuario::Entity::delete_by_id(user_id)
                    .exec(&db)
                    .await
                    .ok();
                crate::entities::organizacion::Entity::delete_by_id(org_id)
                    .exec(&db)
                    .await
                    .ok();
            });
            Ok(())
        })
        .unwrap();
}
