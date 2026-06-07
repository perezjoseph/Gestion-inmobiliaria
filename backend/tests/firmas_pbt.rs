#![allow(clippy::needless_return)]
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

use crate::migrations;

// â”€â”€ Strategies â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Non-pendiente estado values that should trigger a 409 Conflict.
fn non_pendiente_estado() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("firmado".to_string()),
        Just("expirado".to_string()),
        Just("cancelado".to_string()),
    ]
}

/// Strategy for sealed-document variants: (sellado, has_documento_origen_id)
/// At least one must be true for the document to be considered sealed.
fn sealed_document_variant() -> impl Strategy<Value = (bool, bool)> {
    prop_oneof![
        Just((true, false)), // sellado=true, no origen
        Just((false, true)), // not sellado, but has documento_origen_id
        Just((true, true)),  // both sealed and has origen
    ]
}

/// Arbitrary firmante name (1-30 alphanumeric chars).
fn arbitrary_firmante_nombre() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z ]{0,29}".prop_map(|s| s.trim().to_string())
}

/// Valid base64-encoded firma imagen (small, non-empty).
fn valid_firma_b64() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<u8>(), 10..200).prop_map(|bytes| {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    })
}

/// Arbitrary IP address string.
fn arbitrary_ip() -> impl Strategy<Value = String> {
    (1u8..255, 0u8..255, 0u8..255, 1u8..255).prop_map(|(a, b, c, d)| format!("{a}.{b}.{c}.{d}"))
}

/// Arbitrary user agent string.
fn arbitrary_user_agent() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9/ .;()-]{10,50}"
}

// â”€â”€ Async helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

mod pbt_async {
    use chrono::{Duration, Utc};
    use realestate_backend::entities::{documento, firma_documento, organizacion, usuario};
    use realestate_backend::errors::AppError;
    use realestate_backend::services::{auth, firmas};
    use sea_orm::{
        ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait, Set,
    };
    use sea_orm_migration::MigratorTrait;
    use uuid::Uuid;

    fn shared_rt_and_db() -> Option<&'static (tokio::runtime::Runtime, DatabaseConnection)> {
        static SHARED: std::sync::OnceLock<
            Result<(tokio::runtime::Runtime, DatabaseConnection), String>,
        > = std::sync::OnceLock::new();
        SHARED
            .get_or_init(|| {
                dotenvy::dotenv().ok();
                let url = std::env::var("DATABASE_URL")
                    .map_err(|_| "DATABASE_URL not set".to_string())?;
                let rt =
                    tokio::runtime::Runtime::new().map_err(|e| format!("Runtime error: {e}"))?;
                let db = rt.block_on(async {
                    let mut opts = ConnectOptions::new(&url);
                    opts.max_connections(5)
                        .min_connections(1)
                        .connect_timeout(std::time::Duration::from_secs(30))
                        .idle_timeout(std::time::Duration::from_secs(60))
                        .acquire_timeout(std::time::Duration::from_secs(30));
                    let db = Database::connect(opts)
                        .await
                        .map_err(|e| format!("Failed to connect to database: {e}"))?;
                    super::migrations::Migrator::up(&db, None)
                        .await
                        .map_err(|e| format!("Failed to run migrations: {e}"))?;
                    Ok::<_, String>(db)
                })?;
                Ok((rt, db))
            })
            .as_ref()
            .ok()
    }

    fn with_db<F, Fut>(f: F)
    where
        F: FnOnce(DatabaseConnection) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        dotenvy::dotenv().ok();
        if std::env::var("DATABASE_URL").is_err() {
            eprintln!("âš  DATABASE_URL not set â€“ skipping PBT");
            return;
        }
        let _guard = crate::GLOBAL_DB_SERIAL
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some((rt, db)) = shared_rt_and_db() else {
            eprintln!("âš  DB not reachable â€“ skipping PBT");
            return;
        };
        rt.block_on(f(db.clone()));
    }

    async fn create_documento(db: &DatabaseConnection) -> (Uuid, Uuid) {
        use chrono::NaiveDate;
        use realestate_backend::entities::{contrato, inquilino, propiedad};
        use rust_decimal::Decimal;

        let id = Uuid::new_v4();
        let now = Utc::now().into();

        // Create org + user to satisfy FK constraint on uploaded_by
        let org_id = Uuid::new_v4();
        organizacion::ActiveModel {
            id: Set(org_id),
            tipo: Set("persona_fisica".to_string()),
            nombre: Set(format!("PBT Org {org_id}")),
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
        .insert(db)
        .await
        .expect("create org for PBT");

        let user_id = Uuid::new_v4();
        usuario::ActiveModel {
            id: Set(user_id),
            nombre: Set("PBT User".to_string()),
            email: Set(format!("pbt+{user_id}@test.com")),
            password_hash: Set("not_used".to_string()),
            rol: Set("admin".to_string()),
            activo: Set(true),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
            password_changed_at: Set(now),
        }
        .insert(db)
        .await
        .expect("create user for PBT");

        // Create propiedad + inquilino + contrato so the documento's entity_id
        // references a real entity in the org (required by multi-tenancy check).
        let propiedad_id = Uuid::new_v4();
        propiedad::ActiveModel {
            id: Set(propiedad_id),
            titulo: Set("Propiedad PBT".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle PBT 1".to_string()),
            ciudad: Set("Santo Domingo".to_string()),
            provincia: Set("Distrito Nacional".to_string()),
            tipo_propiedad: Set("apartamento".to_string()),
            habitaciones: Set(Some(2)),
            banos: Set(Some(1)),
            area_m2: Set(Some(Decimal::new(8000, 2))),
            precio: Set(Decimal::new(2500000, 2)),
            moneda: Set("DOP".to_string()),
            estado: Set("disponible".to_string()),
            imagenes: Set(None),
            organizacion_id: Set(org_id),
            valor_catastral: Set(None),
            exento_ipi: Set(false),
            motivo_exencion: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("create propiedad for PBT");

        let inquilino_id = Uuid::new_v4();
        inquilino::ActiveModel {
            id: Set(inquilino_id),
            nombre: Set("Inquilino".to_string()),
            apellido: Set("PBT".to_string()),
            email: Set(Some(format!("inq+{inquilino_id}@test.com"))),
            telefono: Set(None),
            cedula: Set(format!("C{}", &inquilino_id.simple().to_string()[..19])),
            contacto_emergencia: Set(None),
            notas: Set(None),
            documentos: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("create inquilino for PBT");

        let contrato_id = Uuid::new_v4();
        contrato::ActiveModel {
            id: Set(contrato_id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date")),
            fecha_fin: Set(NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date")),
            monto_mensual: Set(Decimal::new(25000, 0)),
            deposito: Set(None),
            moneda: Set("DOP".to_string()),
            estado: Set("activo".to_string()),
            documentos: Set(None),
            organizacion_id: Set(org_id),
            estado_deposito: Set(None),
            fecha_cobro_deposito: Set(None),
            fecha_devolucion_deposito: Set(None),
            monto_retenido: Set(None),
            motivo_retencion: Set(None),
            recargo_porcentaje: Set(None),
            dias_gracia: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("create contrato for PBT");

        documento::ActiveModel {
            id: Set(id),
            entity_type: Set("contrato".to_string()),
            entity_id: Set(contrato_id),
            filename: Set(format!("pbt-doc-{id}.pdf")),
            file_path: Set(format!("/tmp/pbt-{id}.pdf")),
            mime_type: Set("application/pdf".to_string()),
            file_size: Set(1024),
            uploaded_by: Set(user_id),
            created_at: Set(now),
            tipo_documento: Set("contrato_arrendamiento".to_string()),
            estado_verificacion: Set("pendiente".to_string()),
            fecha_vencimiento: Set(None),
            verificado_por: Set(None),
            fecha_verificacion: Set(None),
            notas_verificacion: Set(None),
            numero_documento: Set(None),
            contenido_editable: Set(Some(serde_json::json!({"version": 1, "blocks": []}))),
            updated_at: Set(Some(now)),
            sellado: Set(false),
            sellado_at: Set(None),
            documento_origen_id: Set(None),
        }
        .insert(db)
        .await
        .expect("create documento for PBT");
        (id, org_id)
    }

    async fn cleanup(db: &DatabaseConnection, firma_id: Uuid, documento_id: Uuid) {
        let _ = firma_documento::Entity::delete_by_id(firma_id)
            .exec(db)
            .await;
        let _ = documento::Entity::delete_by_id(documento_id).exec(db).await;
    }

    /// Property 3 (spec-gap-remediation): Sealed-document deletion is rejected.
    /// Creates a sealed document (sellado=true and/or documento_origen_id set),
    /// attempts deletion, asserts HTTP 403 Forbidden, row persists, file unchanged.
    pub fn p3_sealed_document_deletion_rejected(sellado: bool, has_origen_id: bool) {
        with_db(|db| async move {
            use realestate_backend::entities::{contrato, inquilino, propiedad};
            use realestate_backend::services::documentos;
            use rust_decimal::Decimal;
            use std::io::Write;

            let now = Utc::now().into();
            let org_id = Uuid::new_v4();

            // Create org
            organizacion::ActiveModel {
                id: Set(org_id),
                tipo: Set("persona_fisica".to_string()),
                nombre: Set(format!("PBT Org {org_id}")),
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
            .expect("create org for P3");

            // Create user
            let user_id = Uuid::new_v4();
            usuario::ActiveModel {
                id: Set(user_id),
                nombre: Set("PBT User P3".to_string()),
                email: Set(format!("pbt-p3+{user_id}@test.com")),
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
            .expect("create user for P3");

            // Create propiedad
            let propiedad_id = Uuid::new_v4();
            propiedad::ActiveModel {
                id: Set(propiedad_id),
                titulo: Set("Propiedad P3".to_string()),
                descripcion: Set(None),
                direccion: Set("Calle P3".to_string()),
                ciudad: Set("Santo Domingo".to_string()),
                provincia: Set("Distrito Nacional".to_string()),
                tipo_propiedad: Set("apartamento".to_string()),
                habitaciones: Set(Some(2)),
                banos: Set(Some(1)),
                area_m2: Set(Some(Decimal::new(8000, 2))),
                precio: Set(Decimal::new(2500000, 2)),
                moneda: Set("DOP".to_string()),
                estado: Set("disponible".to_string()),
                imagenes: Set(None),
                organizacion_id: Set(org_id),
                valor_catastral: Set(None),
                exento_ipi: Set(false),
                motivo_exencion: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(&db)
            .await
            .expect("create propiedad for P3");

            // Create inquilino
            let inquilino_id = Uuid::new_v4();
            inquilino::ActiveModel {
                id: Set(inquilino_id),
                nombre: Set("Inquilino".to_string()),
                apellido: Set("P3".to_string()),
                email: Set(Some(format!("inq-p3+{inquilino_id}@test.com"))),
                telefono: Set(None),
                cedula: Set(format!("P3{}", &inquilino_id.simple().to_string()[..17])),
                contacto_emergencia: Set(None),
                notas: Set(None),
                documentos: Set(None),
                organizacion_id: Set(org_id),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(&db)
            .await
            .expect("create inquilino for P3");

            // Create contrato (needed as entity_id for the documento)
            let contrato_id = Uuid::new_v4();
            contrato::ActiveModel {
                id: Set(contrato_id),
                propiedad_id: Set(propiedad_id),
                inquilino_id: Set(inquilino_id),
                fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
                fecha_fin: Set(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
                monto_mensual: Set(Decimal::new(25000, 0)),
                deposito: Set(None),
                moneda: Set("DOP".to_string()),
                estado: Set("activo".to_string()),
                documentos: Set(None),
                organizacion_id: Set(org_id),
                estado_deposito: Set(None),
                fecha_cobro_deposito: Set(None),
                fecha_devolucion_deposito: Set(None),
                monto_retenido: Set(None),
                motivo_retencion: Set(None),
                recargo_porcentaje: Set(None),
                dias_gracia: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(&db)
            .await
            .expect("create contrato for P3");

            // Create a temp file on disk to verify it remains after failed delete
            let doc_id = Uuid::new_v4();
            let upload_dir =
                std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
            let file_path = format!("pbt-sealed-{doc_id}.pdf");
            let full_path = format!("{upload_dir}/{file_path}");

            // Ensure upload dir exists
            std::fs::create_dir_all(&upload_dir).expect("create upload dir");
            let file_content = b"sealed-pdf-content-for-pbt";
            {
                let mut f = std::fs::File::create(&full_path).expect("create test file on disk");
                f.write_all(file_content).expect("write test file");
            }

            // Create sealed documento
            let documento_origen_id = if has_origen_id {
                Some(contrato_id)
            } else {
                None
            };

            documento::ActiveModel {
                id: Set(doc_id),
                entity_type: Set("contrato".to_string()),
                entity_id: Set(contrato_id),
                filename: Set(format!("sellado-{doc_id}.pdf")),
                file_path: Set(file_path.clone()),
                mime_type: Set("application/pdf".to_string()),
                #[allow(clippy::cast_possible_wrap)]
                file_size: Set(file_content.len() as i64),
                uploaded_by: Set(user_id),
                created_at: Set(now),
                tipo_documento: Set("contrato_arrendamiento".to_string()),
                estado_verificacion: Set("pendiente".to_string()),
                fecha_vencimiento: Set(None),
                verificado_por: Set(None),
                fecha_verificacion: Set(None),
                notas_verificacion: Set(None),
                numero_documento: Set(None),
                contenido_editable: Set(None),
                updated_at: Set(Some(now)),
                sellado: Set(sellado),
                sellado_at: Set(if sellado { Some(now) } else { None }),
                documento_origen_id: Set(documento_origen_id),
            }
            .insert(&db)
            .await
            .expect("create sealed documento for P3");

            // Attempt deletion â€” should fail with Forbidden (403)
            let result = documentos::eliminar(&db, doc_id, user_id, org_id).await;

            assert!(
                result.is_err(),
                "Delete of sealed document should fail (sellado={sellado}, has_origen={has_origen_id})"
            );
            let err = result.unwrap_err();
            assert!(
                matches!(err, AppError::Forbidden(_)),
                "Expected AppError::Forbidden (HTTP 403), got: {err:?}"
            );

            // Verify row still exists in DB
            let doc_after = documento::Entity::find_by_id(doc_id)
                .one(&db)
                .await
                .expect("DB query should succeed")
                .expect("Sealed document row should still exist after rejected delete");
            assert_eq!(doc_after.id, doc_id);
            assert_eq!(doc_after.sellado, sellado);
            assert_eq!(doc_after.documento_origen_id, documento_origen_id);

            // Verify file on disk is unchanged
            let disk_content = std::fs::read(&full_path).expect("File should still exist on disk");
            assert_eq!(
                disk_content, file_content,
                "File content should be unchanged after rejected delete"
            );

            // Cleanup: remove test file and DB rows
            let _ = std::fs::remove_file(&full_path);
            let _ = documento::Entity::delete_by_id(doc_id).exec(&db).await;
        });
    }

    /// Property 12: Document sealing triggers on complete signatures.
    /// Creates a document, signs as propietario then inquilino, verifies sealing.
    pub fn p12_document_sealing(
        propietario_nombre: String,
        inquilino_nombre: String,
        firma_b64_prop: String,
        firma_b64_inq: String,
        ip_prop: String,
        ip_inq: String,
        ua_prop: String,
        ua_inq: String,
    ) {
        with_db(|db| async move {
            let (documento_id, org_id) = create_documento(&db).await;

            // Sign as propietario (admin role)
            let result = firmas::firmar_autenticado(
                &db,
                documento_id,
                &propietario_nombre,
                "admin",
                &firma_b64_prop,
                ip_prop,
                ua_prop,
                org_id,
            )
            .await;
            assert!(
                result.is_ok(),
                "Propietario signing should succeed: {:?}",
                result.err()
            );
            let prop_firma = result.unwrap();
            assert_eq!(prop_firma.firmante_tipo, "propietario");
            assert_eq!(prop_firma.estado, "firmado");

            // Document should NOT be sealed yet (only propietario signed)
            let doc = documento::Entity::find_by_id(documento_id)
                .one(&db)
                .await
                .expect("DB query should succeed")
                .expect("Document should exist");
            assert!(
                !doc.sellado,
                "Document should NOT be sealed with only propietario signature"
            );
            assert!(
                doc.sellado_at.is_none(),
                "sellado_at should be None before sealing"
            );

            // Sign as inquilino
            let result = firmas::firmar_autenticado(
                &db,
                documento_id,
                &inquilino_nombre,
                "inquilino",
                &firma_b64_inq,
                ip_inq,
                ua_inq,
                org_id,
            )
            .await;
            assert!(
                result.is_ok(),
                "Inquilino signing should succeed: {:?}",
                result.err()
            );
            let inq_firma = result.unwrap();
            assert_eq!(inq_firma.firmante_tipo, "inquilino");
            assert_eq!(inq_firma.estado, "firmado");

            // Document SHOULD be sealed now (both parties signed)
            let doc = documento::Entity::find_by_id(documento_id)
                .one(&db)
                .await
                .expect("DB query should succeed")
                .expect("Document should exist");
            assert!(
                doc.sellado,
                "Document should be sealed after both propietario and inquilino sign"
            );
            assert!(
                doc.sellado_at.is_some(),
                "sellado_at should be set after sealing"
            );
        });
    }

    /// Property 14: Signing order independence.
    /// Creates two documents, signs in different orders, verifies both end up sealed identically.
    pub fn p14_signing_order_independence(
        propietario_nombre: String,
        inquilino_nombre: String,
        firma_b64_prop: String,
        firma_b64_inq: String,
        ip_prop: String,
        ip_inq: String,
        ua_prop: String,
        ua_inq: String,
    ) {
        with_db(|db| async move {
            // Create two documents with identical content
            let (doc_a_id, org_a_id) = create_documento(&db).await;
            let (doc_b_id, org_b_id) = create_documento(&db).await;

            // Document A: propietario first, then inquilino
            let result = firmas::firmar_autenticado(
                &db,
                doc_a_id,
                &propietario_nombre,
                "admin",
                &firma_b64_prop,
                ip_prop.clone(),
                ua_prop.clone(),
                org_a_id,
            )
            .await;
            assert!(
                result.is_ok(),
                "Doc A propietario signing failed: {:?}",
                result.err()
            );

            let result = firmas::firmar_autenticado(
                &db,
                doc_a_id,
                &inquilino_nombre,
                "inquilino",
                &firma_b64_inq,
                ip_inq.clone(),
                ua_inq.clone(),
                org_a_id,
            )
            .await;
            assert!(
                result.is_ok(),
                "Doc A inquilino signing failed: {:?}",
                result.err()
            );

            // Document B: inquilino first, then propietario (reversed order)
            let result = firmas::firmar_autenticado(
                &db,
                doc_b_id,
                &inquilino_nombre,
                "inquilino",
                &firma_b64_inq,
                ip_inq,
                ua_inq,
                org_b_id,
            )
            .await;
            assert!(
                result.is_ok(),
                "Doc B inquilino signing failed: {:?}",
                result.err()
            );

            let result = firmas::firmar_autenticado(
                &db,
                doc_b_id,
                &propietario_nombre,
                "admin",
                &firma_b64_prop,
                ip_prop,
                ua_prop,
                org_b_id,
            )
            .await;
            assert!(
                result.is_ok(),
                "Doc B propietario signing failed: {:?}",
                result.err()
            );

            // Verify both documents are sealed identically
            let doc_a = documento::Entity::find_by_id(doc_a_id)
                .one(&db)
                .await
                .expect("DB query failed")
                .expect("Document A should exist");
            let doc_b = documento::Entity::find_by_id(doc_b_id)
                .one(&db)
                .await
                .expect("DB query failed")
                .expect("Document B should exist");

            // Both should be sealed
            assert!(
                doc_a.sellado,
                "Document A (propietario first) should be sealed"
            );
            assert!(
                doc_b.sellado,
                "Document B (inquilino first) should be sealed"
            );

            // Both should have sellado_at set
            assert!(
                doc_a.sellado_at.is_some(),
                "Document A sellado_at should be set"
            );
            assert!(
                doc_b.sellado_at.is_some(),
                "Document B sellado_at should be set"
            );

            // Final sealed state is identical regardless of order
            assert_eq!(
                doc_a.sellado, doc_b.sellado,
                "Sealed state should be identical regardless of signing order"
            );
        });
    }

    /// Property 11: Tenant signing state guard
    /// For firma with estado != "pendiente", verify signing attempt returns 409.
    pub fn p11_signing_state_guard(estado: String) {
        with_db(|db| async move {
            let (documento_id, _org_id) = create_documento(&db).await;

            // Generate a token and password for the firma.
            // Randomize the password per run to avoid a hard-coded cryptographic
            // value (CodeQL rust/hard-coded-cryptographic-value).
            let token = Uuid::new_v4().to_string();
            let password = Uuid::new_v4().to_string();
            let password_hash = auth::hash_password(&password).expect("hash password");

            let now = Utc::now();
            let expira_at = now + Duration::hours(72);

            // Create firma_documento with non-pendiente estado
            let firma_id = Uuid::new_v4();
            let firma = firma_documento::ActiveModel {
                id: Set(firma_id),
                documento_id: Set(documento_id),
                firmante_tipo: Set("inquilino".to_string()),
                firmante_nombre: Set("Inquilino PBT".to_string()),
                firma_imagen: Set(None),
                ip_address: Set(None),
                user_agent: Set(None),
                firmado_at: Set(None),
                token: Set(Some(token.clone())),
                password_hash: Set(Some(password_hash)),
                expira_at: Set(Some(expira_at.into())),
                estado: Set(estado.clone()),
                created_at: Set(now.into()),
            };
            firma.insert(&db).await.expect("insert firma for PBT");

            // Attempt to sign â€” should return Conflict (409)
            use base64::Engine;
            let firma_imagen_b64 =
                base64::engine::general_purpose::STANDARD.encode(b"fake-png-data");

            let result = firmas::firmar_con_token(
                &db,
                &token,
                &password,
                &firma_imagen_b64,
                "127.0.0.1".to_string(),
                "PBT-Agent/1.0".to_string(),
            )
            .await;

            assert!(
                result.is_err(),
                "Signing with estado='{estado}' should fail, but got Ok",
            );

            let err = result.unwrap_err();
            assert!(
                matches!(err, AppError::Conflict(_)),
                "Expected AppError::Conflict for estado='{estado}', got: {err:?}",
            );

            cleanup(&db, firma_id, documento_id).await;
        });
    }
}

// Feature: contract-document-signing, Property 11: Tenant signing state guard
// **Validates: Requirements 5.10**
#[test]
fn test_signing_state_guard_rejects_non_pendiente() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&non_pendiente_estado(), |estado| {
            pbt_async::p11_signing_state_guard(estado);
            Ok(())
        })
        .unwrap();
}

// Feature: contract-document-signing, Property 12: Document sealing triggers on complete signatures
// **Validates: Requirements 6.1, 6.5**
#[test]
fn test_document_sealing_triggers_on_complete_signatures() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(
                arbitrary_firmante_nombre(),
                arbitrary_firmante_nombre(),
                valid_firma_b64(),
                valid_firma_b64(),
                arbitrary_ip(),
                arbitrary_ip(),
                arbitrary_user_agent(),
                arbitrary_user_agent(),
            ),
            |(
                propietario_nombre,
                inquilino_nombre,
                firma_b64_prop,
                firma_b64_inq,
                ip_prop,
                ip_inq,
                ua_prop,
                ua_inq,
            )| {
                pbt_async::p12_document_sealing(
                    propietario_nombre,
                    inquilino_nombre,
                    firma_b64_prop,
                    firma_b64_inq,
                    ip_prop,
                    ip_inq,
                    ua_prop,
                    ua_inq,
                );
                Ok(())
            },
        )
        .unwrap();
}

// Feature: contract-document-signing, Property 14: Signing order independence
// **Validates: Requirements 6.6**
#[test]
fn test_signing_order_independence() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(
            &(
                arbitrary_firmante_nombre(),
                arbitrary_firmante_nombre(),
                valid_firma_b64(),
                valid_firma_b64(),
                arbitrary_ip(),
                arbitrary_ip(),
                arbitrary_user_agent(),
                arbitrary_user_agent(),
            ),
            |(
                propietario_nombre,
                inquilino_nombre,
                firma_b64_prop,
                firma_b64_inq,
                ip_prop,
                ip_inq,
                ua_prop,
                ua_inq,
            )| {
                pbt_async::p14_signing_order_independence(
                    propietario_nombre,
                    inquilino_nombre,
                    firma_b64_prop,
                    firma_b64_inq,
                    ip_prop,
                    ip_inq,
                    ua_prop,
                    ua_inq,
                );
                Ok(())
            },
        )
        .unwrap();
}

// Feature: spec-gap-remediation, Property 3: Sealed-document deletion is rejected
// **Validates: Requirements 3.2, 3.3**
#[test]
fn test_sealed_document_deletion_is_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&sealed_document_variant(), |(sellado, has_origen_id)| {
            pbt_async::p3_sealed_document_deletion_rejected(sellado, has_origen_id);
            Ok(())
        })
        .unwrap();
}
