#![allow(clippy::needless_return)]
use actix_web::http::StatusCode;
use actix_web::web;
use chrono::{Duration, Utc};
use realestate_backend::app::create_app;
use realestate_backend::config::AppConfig;
use realestate_backend::entities::{documento, firma_documento, organizacion, usuario};
use realestate_backend::services::auth::{Claims, encode_jwt};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::{Value, json};
use std::net::SocketAddr;
use uuid::Uuid;

use crate::common::{JWT_SECRET, test_app_config, with_db};

fn make_config() -> AppConfig {
    test_app_config("")
}

/// Peer address for test requests (required by rate limiter middleware).
fn test_peer() -> SocketAddr {
    "127.0.0.1:8080".parse().unwrap()
}

fn make_token(user_id: Uuid, rol: &str, org_id: Uuid) -> String {
    let claims = Claims {
        sub: user_id,
        email: format!("{rol}@test.com"),
        rol: rol.to_string(),
        organizacion_id: org_id,
        exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
    };
    encode_jwt(&claims, JWT_SECRET).unwrap()
}

async fn create_test_organizacion(db: &DatabaseConnection) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    organizacion::ActiveModel {
        id: Set(id),
        tipo: Set("persona_fisica".to_string()),
        nombre: Set(format!("Org Test {id}")),
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
    .expect("Failed to create test organizacion");
    id
}

async fn create_test_usuario(db: &DatabaseConnection, rol: &str, org_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    usuario::ActiveModel {
        id: Set(id),
        nombre: Set(format!("Test {rol}")),
        email: Set(format!("{rol}+{id}@test.com")),
        password_hash: Set("not_used".to_string()),
        rol: Set(rol.to_string()),
        activo: Set(true),
        organizacion_id: Set(org_id),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .expect("Failed to create test usuario");
    id
}

/// Create a test document with editable content for signing tests.
/// Also creates propiedad + inquilino + contrato so the document's entity_id
/// references a real entity in the org (required by multi-tenancy check).
async fn create_test_documento(db: &DatabaseConnection, user_id: Uuid, org_id: Uuid) -> Uuid {
    use chrono::NaiveDate;
    use realestate_backend::entities::{contrato, inquilino, propiedad};
    use rust_decimal::Decimal;

    let id = Uuid::new_v4();
    let now = Utc::now().into();

    let propiedad_id = Uuid::new_v4();
    propiedad::ActiveModel {
        id: Set(propiedad_id),
        titulo: Set("Propiedad Firmas Test".to_string()),
        descripcion: Set(None),
        direccion: Set("Calle Test 1".to_string()),
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
    .expect("Failed to create test propiedad");

    let inquilino_id = Uuid::new_v4();
    inquilino::ActiveModel {
        id: Set(inquilino_id),
        nombre: Set("Inquilino".to_string()),
        apellido: Set("Firmas".to_string()),
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
    .expect("Failed to create test inquilino");

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
    .expect("Failed to create test contrato");

    documento::ActiveModel {
        id: Set(id),
        entity_type: Set("contrato".to_string()),
        entity_id: Set(contrato_id),
        filename: Set("test-doc.pdf".to_string()),
        file_path: Set("/tmp/test-doc.pdf".to_string()),
        mime_type: Set("application/pdf".to_string()),
        file_size: Set(1024),
        uploaded_by: Set(user_id),
        created_at: Set(now),
        tipo_documento: Set("contrato".to_string()),
        estado_verificacion: Set("pendiente".to_string()),
        fecha_vencimiento: Set(None),
        verificado_por: Set(None),
        fecha_verificacion: Set(None),
        notas_verificacion: Set(None),
        numero_documento: Set(None),
        contenido_editable: Set(Some(json!({
            "version": 1,
            "blocks": [
                {"type": "heading", "level": 1, "text": "Contrato de Arrendamiento"},
                {"type": "paragraph", "text": "Este es un documento de prueba."}
            ]
        }))),
        updated_at: Set(Some(now)),
        sellado: Set(false),
        sellado_at: Set(None),
        documento_origen_id: Set(None),
    }
    .insert(db)
    .await
    .expect("Failed to create test documento");
    id
}

/// Helper: set up org + admin user + document for firmas tests.
async fn setup_firmas_data(db: &DatabaseConnection) -> (Uuid, Uuid, Uuid) {
    let org_id = create_test_organizacion(db).await;
    let admin_id = create_test_usuario(db, "admin", org_id).await;
    let doc_id = create_test_documento(db, admin_id, org_id).await;
    (org_id, admin_id, doc_id)
}

fn preview_store() -> web::Data<realestate_backend::services::ocr_preview::PreviewStore> {
    web::Data::new(realestate_backend::services::ocr_preview::PreviewStore::new())
}

/// A small valid PNG (1x1 pixel) encoded as base64 for firma_imagen.
fn valid_firma_imagen_b64() -> String {
    use base64::Engine;
    // Minimal 1x1 white PNG
    let png_bytes: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    base64::engine::general_purpose::STANDARD.encode(png_bytes)
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Test: Authenticated signing flow end-to-end
// Requirements: 4.1
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[test]
fn test_authenticated_signing_flow_end_to_end() {
    with_db(|db| async move {
        let (org_id, admin_id, doc_id) = setup_firmas_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/documentos/{doc_id}/firmar"))
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .insert_header(("User-Agent", "TestAgent/1.0"))
            .set_json(json!({ "firmaImagen": valid_firma_imagen_b64() }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Expected 200 for authenticated signing"
        );

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["firmanteTipo"], "propietario");
        assert_eq!(body["estado"], "firmado");
        assert!(body["firmadoAt"].is_string());
        assert_eq!(body["documentoId"], doc_id.to_string());
    });
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Test: solicitar-firma creates pending record with valid token
// Requirements: 5.1
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[test]
fn test_solicitar_firma_creates_pending_record() {
    with_db(|db| async move {
        let (org_id, admin_id, doc_id) = setup_firmas_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/documentos/{doc_id}/solicitar-firma"))
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "firmanteNombre": "Juan PÃ©rez",
                "email": "juan@example.com"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::CREATED,
            "Expected 201 for solicitar-firma"
        );

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert!(body["firmaId"].is_string());
        assert!(body["token"].is_string());
        assert!(body["expiraAt"].is_string());

        // Verify the token is long enough (UUID = 36 chars with hyphens, 32 without)
        let firma_token = body["token"].as_str().unwrap();
        assert!(
            firma_token.len() >= 32,
            "Token should have sufficient entropy"
        );

        // Verify the record in DB is pendiente
        let firma_id: Uuid = body["firmaId"].as_str().unwrap().parse().unwrap();
        let firma = firma_documento::Entity::find_by_id(firma_id)
            .one(&db)
            .await
            .unwrap()
            .expect("Firma record should exist");
        assert_eq!(firma.estado, "pendiente");
        assert!(firma.token.is_some());
        assert!(firma.password_hash.is_some());
        assert!(firma.expira_at.is_some());
    });
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Test: Public token verification (valid, expired, wrong password)
// Requirements: 5.7, 5.8
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[test]
fn test_public_token_verification_valid() {
    with_db(|db| async move {
        let (_org_id, _admin_id, doc_id) = setup_firmas_data(&db).await;
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        // Create a firma with a known password directly for verification testing
        use argon2::password_hash::SaltString;
        use argon2::password_hash::rand_core::OsRng;
        use argon2::{Argon2, PasswordHasher};

        let known_password = "test_password_123";
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(known_password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        // Create a firma with known password directly
        let known_token = Uuid::new_v4().to_string();
        let now = Utc::now();
        firma_documento::ActiveModel {
            id: Set(Uuid::new_v4()),
            documento_id: Set(doc_id),
            firmante_tipo: Set("inquilino".to_string()),
            firmante_nombre: Set("Test Inquilino".to_string()),
            firma_imagen: Set(None),
            ip_address: Set(None),
            user_agent: Set(None),
            firmado_at: Set(None),
            token: Set(Some(known_token.clone())),
            password_hash: Set(Some(hash)),
            expira_at: Set(Some((now + Duration::hours(72)).into())),
            estado: Set("pendiente".to_string()),
            created_at: Set(now.into()),
        }
        .insert(&db)
        .await
        .unwrap();

        // Verify with correct password
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/firmas/{known_token}/verificar"))
            .peer_addr(test_peer())
            .set_json(json!({ "password": known_password }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Valid token + password should return 200"
        );

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["documentoId"], doc_id.to_string());
        assert!(body["contenido"].is_object());
        assert_eq!(body["firmanteNombre"], "Test Inquilino");
    });
}

#[test]
fn test_public_token_verification_expired() {
    with_db(|db| async move {
        let (_org_id, _admin_id, doc_id) = setup_firmas_data(&db).await;
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        // Create a firma with expired token
        use argon2::password_hash::SaltString;
        use argon2::password_hash::rand_core::OsRng;
        use argon2::{Argon2, PasswordHasher};

        let password = "expired_test_pass";
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        let expired_token = Uuid::new_v4().to_string();
        let now = Utc::now();
        firma_documento::ActiveModel {
            id: Set(Uuid::new_v4()),
            documento_id: Set(doc_id),
            firmante_tipo: Set("inquilino".to_string()),
            firmante_nombre: Set("Expired Inquilino".to_string()),
            firma_imagen: Set(None),
            ip_address: Set(None),
            user_agent: Set(None),
            firmado_at: Set(None),
            token: Set(Some(expired_token.clone())),
            password_hash: Set(Some(hash)),
            expira_at: Set(Some((now - Duration::hours(1)).into())), // expired
            estado: Set("pendiente".to_string()),
            created_at: Set((now - Duration::hours(73)).into()),
        }
        .insert(&db)
        .await
        .unwrap();

        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/firmas/{expired_token}/verificar"))
            .peer_addr(test_peer())
            .set_json(json!({ "password": password }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::GONE,
            "Expired token should return 410"
        );
    });
}

#[test]
fn test_public_token_verification_wrong_password() {
    with_db(|db| async move {
        let (_org_id, _admin_id, doc_id) = setup_firmas_data(&db).await;
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        use argon2::password_hash::SaltString;
        use argon2::password_hash::rand_core::OsRng;
        use argon2::{Argon2, PasswordHasher};

        let correct_password = "correct_password";
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(correct_password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        let token_str = Uuid::new_v4().to_string();
        let now = Utc::now();
        firma_documento::ActiveModel {
            id: Set(Uuid::new_v4()),
            documento_id: Set(doc_id),
            firmante_tipo: Set("inquilino".to_string()),
            firmante_nombre: Set("Wrong Pass Inquilino".to_string()),
            firma_imagen: Set(None),
            ip_address: Set(None),
            user_agent: Set(None),
            firmado_at: Set(None),
            token: Set(Some(token_str.clone())),
            password_hash: Set(Some(hash)),
            expira_at: Set(Some((now + Duration::hours(72)).into())),
            estado: Set("pendiente".to_string()),
            created_at: Set(now.into()),
        }
        .insert(&db)
        .await
        .unwrap();

        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/firmas/{token_str}/verificar"))
            .peer_addr(test_peer())
            .set_json(json!({ "password": "wrong_password" }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "Wrong password should return 401"
        );
    });
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Test: Public signing (success, already signed conflict)
// Requirements: 5.10
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[test]
fn test_public_signing_success() {
    with_db(|db| async move {
        let (_org_id, _admin_id, doc_id) = setup_firmas_data(&db).await;
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        use argon2::password_hash::SaltString;
        use argon2::password_hash::rand_core::OsRng;
        use argon2::{Argon2, PasswordHasher};

        let password = "sign_test_password";
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        let token_str = Uuid::new_v4().to_string();
        let now = Utc::now();
        firma_documento::ActiveModel {
            id: Set(Uuid::new_v4()),
            documento_id: Set(doc_id),
            firmante_tipo: Set("inquilino".to_string()),
            firmante_nombre: Set("Signing Inquilino".to_string()),
            firma_imagen: Set(None),
            ip_address: Set(None),
            user_agent: Set(None),
            firmado_at: Set(None),
            token: Set(Some(token_str.clone())),
            password_hash: Set(Some(hash)),
            expira_at: Set(Some((now + Duration::hours(72)).into())),
            estado: Set("pendiente".to_string()),
            created_at: Set(now.into()),
        }
        .insert(&db)
        .await
        .unwrap();

        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/firmas/{token_str}/firmar"))
            .peer_addr(test_peer())
            .insert_header(("User-Agent", "TenantBrowser/1.0"))
            .set_json(json!({
                "password": password,
                "firmaImagen": valid_firma_imagen_b64()
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Public signing should return 200"
        );

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["estado"], "firmado");
        assert_eq!(body["firmanteTipo"], "inquilino");
        assert!(body["firmadoAt"].is_string());
    });
}

#[test]
fn test_public_signing_already_signed_conflict() {
    with_db(|db| async move {
        let (_org_id, _admin_id, doc_id) = setup_firmas_data(&db).await;
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        use argon2::password_hash::SaltString;
        use argon2::password_hash::rand_core::OsRng;
        use argon2::{Argon2, PasswordHasher};

        let password = "conflict_test_pass";
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        let token_str = Uuid::new_v4().to_string();
        let now = Utc::now();
        // Create a firma that is already signed (estado = "firmado")
        firma_documento::ActiveModel {
            id: Set(Uuid::new_v4()),
            documento_id: Set(doc_id),
            firmante_tipo: Set("inquilino".to_string()),
            firmante_nombre: Set("Already Signed".to_string()),
            firma_imagen: Set(Some(vec![1, 2, 3])),
            ip_address: Set(Some("127.0.0.1".to_string())),
            user_agent: Set(Some("Test".to_string())),
            firmado_at: Set(Some(now.into())),
            token: Set(Some(token_str.clone())),
            password_hash: Set(Some(hash)),
            expira_at: Set(Some((now + Duration::hours(72)).into())),
            estado: Set("firmado".to_string()), // already signed
            created_at: Set(now.into()),
        }
        .insert(&db)
        .await
        .unwrap();

        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/firmas/{token_str}/firmar"))
            .peer_addr(test_peer())
            .insert_header(("User-Agent", "TenantBrowser/1.0"))
            .set_json(json!({
                "password": password,
                "firmaImagen": valid_firma_imagen_b64()
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::CONFLICT,
            "Already signed should return 409"
        );
    });
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Test: Document sealing after both parties sign
// Requirements: 6.1
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[test]
fn test_document_sealing_after_both_parties_sign() {
    with_db(|db| async move {
        let (org_id, admin_id, doc_id) = setup_firmas_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        // Step 1: Manager signs (authenticated)
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/documentos/{doc_id}/firmar"))
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .insert_header(("User-Agent", "ManagerApp/1.0"))
            .set_json(json!({ "firmaImagen": valid_firma_imagen_b64() }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Step 2: Create tenant firma with known password and sign via public endpoint
        use argon2::password_hash::SaltString;
        use argon2::password_hash::rand_core::OsRng;
        use argon2::{Argon2, PasswordHasher};

        let password = "seal_test_password";
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        let tenant_token = Uuid::new_v4().to_string();
        let now = Utc::now();
        firma_documento::ActiveModel {
            id: Set(Uuid::new_v4()),
            documento_id: Set(doc_id),
            firmante_tipo: Set("inquilino".to_string()),
            firmante_nombre: Set("Tenant Sealer".to_string()),
            firma_imagen: Set(None),
            ip_address: Set(None),
            user_agent: Set(None),
            firmado_at: Set(None),
            token: Set(Some(tenant_token.clone())),
            password_hash: Set(Some(hash)),
            expira_at: Set(Some((now + Duration::hours(72)).into())),
            estado: Set("pendiente".to_string()),
            created_at: Set(now.into()),
        }
        .insert(&db)
        .await
        .unwrap();

        // Tenant signs
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/firmas/{tenant_token}/firmar"))
            .peer_addr(test_peer())
            .insert_header(("User-Agent", "TenantBrowser/1.0"))
            .set_json(json!({
                "password": password,
                "firmaImagen": valid_firma_imagen_b64()
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify document is now sealed
        let doc = documento::Entity::find_by_id(doc_id)
            .one(&db)
            .await
            .unwrap()
            .expect("Document should exist");
        assert!(
            doc.sellado,
            "Document should be sealed after both parties sign"
        );
        assert!(doc.sellado_at.is_some(), "sellado_at should be set");
    });
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Test: Sealed document rejects content edits (403)
// Requirements: 6.4
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[test]
fn test_sealed_document_rejects_content_edits() {
    with_db(|db| async move {
        let (org_id, admin_id, doc_id) = setup_firmas_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        // Manually seal the document
        use sea_orm::IntoActiveModel;
        let doc = documento::Entity::find_by_id(doc_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        let mut active = doc.into_active_model();
        active.sellado = Set(true);
        active.sellado_at = Set(Some(Utc::now().into()));
        active.update(&db).await.unwrap();

        // Try to update content
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/documentos/{doc_id}/contenido"))
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "contenidoEditable": {
                    "version": 1,
                    "blocks": [{"type": "paragraph", "text": "Modified content"}]
                }
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "Sealed document should reject edits with 403"
        );

        let body: Value = actix_web::test::read_body_json(resp).await;
        let msg = body["message"].as_str().unwrap_or("");
        assert!(
            msg.contains("sellado"),
            "Error message should mention 'sellado', got: {msg}"
        );
    });
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Test: DOCX export returns valid response with correct headers
// Requirements: 1.1
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[test]
fn test_docx_export_returns_valid_response() {
    with_db(|db| async move {
        let (org_id, admin_id, doc_id) = setup_firmas_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        let req = actix_web::test::TestRequest::get()
            .uri(&format!("/api/v1/documentos/{doc_id}/exportar-docx"))
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "DOCX export should return 200"
        );

        // Check content type
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.contains(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            ),
            "Content-Type should be DOCX MIME type, got: {content_type}"
        );

        // Check content disposition
        let disposition = resp
            .headers()
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            disposition.contains(&format!("documento-{doc_id}.docx")),
            "Content-Disposition should contain filename, got: {disposition}"
        );

        // Check body starts with ZIP magic bytes (PK\x03\x04)
        let body = actix_web::test::read_body(resp).await;
        assert!(body.len() > 4, "DOCX body should not be empty");
        assert_eq!(
            &body[..4],
            b"PK\x03\x04",
            "DOCX should start with ZIP magic bytes"
        );
    });
}

#[test]
fn test_docx_export_no_content_returns_400() {
    with_db(|db| async move {
        use chrono::NaiveDate;
        use realestate_backend::entities::{contrato, inquilino, propiedad};
        use rust_decimal::Decimal;

        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);

        // Create propiedad + inquilino + contrato so the documento's entity_id
        // references a real entity (required by multi-tenancy check).
        let now = Utc::now().into();
        let propiedad_id = Uuid::new_v4();
        propiedad::ActiveModel {
            id: Set(propiedad_id),
            titulo: Set("Propiedad DocX Test".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle Test 1".to_string()),
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
        .unwrap();

        let inquilino_id = Uuid::new_v4();
        inquilino::ActiveModel {
            id: Set(inquilino_id),
            nombre: Set("Inquilino".to_string()),
            apellido: Set("DocX".to_string()),
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
        .insert(&db)
        .await
        .unwrap();

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
        .insert(&db)
        .await
        .unwrap();

        // Create a document without contenido_editable
        let doc_id = Uuid::new_v4();
        documento::ActiveModel {
            id: Set(doc_id),
            entity_type: Set("contrato".to_string()),
            entity_id: Set(contrato_id),
            filename: Set("empty-doc.pdf".to_string()),
            file_path: Set("/tmp/empty-doc.pdf".to_string()),
            mime_type: Set("application/pdf".to_string()),
            file_size: Set(512),
            uploaded_by: Set(admin_id),
            created_at: Set(now),
            tipo_documento: Set("contrato".to_string()),
            estado_verificacion: Set("pendiente".to_string()),
            fecha_vencimiento: Set(None),
            verificado_por: Set(None),
            fecha_verificacion: Set(None),
            notas_verificacion: Set(None),
            numero_documento: Set(None),
            contenido_editable: Set(None), // No content
            updated_at: Set(Some(now)),
            sellado: Set(false),
            sellado_at: Set(None),
            documento_origen_id: Set(None),
        }
        .insert(&db)
        .await
        .unwrap();

        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        let req = actix_web::test::TestRequest::get()
            .uri(&format!("/api/v1/documentos/{doc_id}/exportar-docx"))
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "No content should return 400"
        );
    });
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Test: Template CRUD operations (create, read, update, soft-delete)
// Requirements: 2.1, 2.3
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[test]
fn test_template_crud_create_and_read() {
    with_db(|db| async move {
        let (org_id, admin_id, _doc_id) = setup_firmas_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        // Create template
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/documentos/plantillas")
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "nombre": "Contrato EstÃ¡ndar",
                "tipoDocumento": "contrato",
                "entityType": "contrato",
                "contenido": {
                    "version": 1,
                    "blocks": [
                        {"type": "heading", "level": 1, "text": "Contrato de Arrendamiento"},
                        {"type": "paragraph", "text": "Entre {{inquilino.nombre}} y el propietario."}
                    ]
                }
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::CREATED,
            "Template creation should return 201"
        );

        let body: Value = actix_web::test::read_body_json(resp).await;
        let plantilla_id = body["id"].as_str().unwrap();
        assert_eq!(body["nombre"], "Contrato EstÃ¡ndar");
        assert_eq!(body["tipoDocumento"], "contrato");
        assert_eq!(body["entityType"], "contrato");

        // Read template by ID
        let req = actix_web::test::TestRequest::get()
            .uri(&format!("/api/v1/documentos/plantillas/{plantilla_id}"))
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Template read should return 200"
        );

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["nombre"], "Contrato EstÃ¡ndar");
        assert_eq!(body["tipoDocumento"], "contrato");
    });
}

#[test]
fn test_template_crud_update() {
    with_db(|db| async move {
        let (org_id, admin_id, _doc_id) = setup_firmas_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        // Create template first
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/documentos/plantillas")
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "nombre": "Template Original",
                "tipoDocumento": "contrato",
                "entityType": "contrato",
                "contenido": {"version": 1, "blocks": []}
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let plantilla_id = body["id"].as_str().unwrap().to_string();

        // Update template
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/documentos/plantillas/{plantilla_id}"))
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "nombre": "Template Actualizado"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Template update should return 200"
        );

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["nombre"], "Template Actualizado");
    });
}

#[test]
fn test_template_crud_soft_delete() {
    with_db(|db| async move {
        let (org_id, admin_id, _doc_id) = setup_firmas_data(&db).await;
        let token = make_token(admin_id, "admin", org_id);
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        // Create template
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/documentos/plantillas")
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "nombre": "Template Para Borrar",
                "tipoDocumento": "contrato",
                "entityType": "contrato",
                "contenido": {"version": 1, "blocks": []}
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let plantilla_id = body["id"].as_str().unwrap().to_string();

        // Soft-delete template
        let req = actix_web::test::TestRequest::delete()
            .uri(&format!("/api/v1/documentos/plantillas/{plantilla_id}"))
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "Soft-delete should return 204"
        );

        // Verify it no longer appears in the active list
        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/documentos/plantillas")
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = actix_web::test::read_body_json(resp).await;
        let templates = body.as_array().unwrap();
        let found = templates
            .iter()
            .any(|t| t["id"].as_str() == Some(&plantilla_id));
        assert!(
            !found,
            "Soft-deleted template should not appear in active list"
        );
    });
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Test: RBAC enforcement (WriteAccess for write endpoints)
// Requirements: 4.1, 2.1
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[test]
fn test_rbac_firmar_rejects_unauthenticated() {
    with_db(|db| async move {
        let (_org_id, _admin_id, doc_id) = setup_firmas_data(&db).await;
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/documentos/{doc_id}/firmar"))
            .peer_addr(test_peer())
            .set_json(json!({ "firmaImagen": valid_firma_imagen_b64() }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "Unauthenticated should get 401"
        );
    });
}

#[test]
fn test_rbac_firmar_rejects_visualizador() {
    with_db(|db| async move {
        let (org_id, _admin_id, doc_id) = setup_firmas_data(&db).await;
        let viewer_id = create_test_usuario(&db, "visualizador", org_id).await;
        let token = make_token(viewer_id, "visualizador", org_id);
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/documentos/{doc_id}/firmar"))
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "firmaImagen": valid_firma_imagen_b64() }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "Visualizador should get 403"
        );
    });
}

#[test]
fn test_rbac_solicitar_firma_rejects_visualizador() {
    with_db(|db| async move {
        let (org_id, _admin_id, doc_id) = setup_firmas_data(&db).await;
        let viewer_id = create_test_usuario(&db, "visualizador", org_id).await;
        let token = make_token(viewer_id, "visualizador", org_id);
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/documentos/{doc_id}/solicitar-firma"))
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "firmanteNombre": "Test",
                "email": "test@example.com"
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "Visualizador should get 403 on solicitar-firma"
        );
    });
}

#[test]
fn test_rbac_plantilla_create_rejects_visualizador() {
    with_db(|db| async move {
        let (org_id, _admin_id, _doc_id) = setup_firmas_data(&db).await;
        let viewer_id = create_test_usuario(&db, "visualizador", org_id).await;
        let token = make_token(viewer_id, "visualizador", org_id);
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/documentos/plantillas")
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "nombre": "Forbidden Template",
                "tipoDocumento": "contrato",
                "entityType": "contrato",
                "contenido": {"version": 1, "blocks": []}
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "Visualizador should get 403 on template create"
        );
    });
}

#[test]
fn test_rbac_plantilla_delete_rejects_visualizador() {
    with_db(|db| async move {
        let (org_id, admin_id, _doc_id) = setup_firmas_data(&db).await;
        let admin_token = make_token(admin_id, "admin", org_id);
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        // Create a template as admin
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/documentos/plantillas")
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {admin_token}")))
            .set_json(json!({
                "nombre": "RBAC Delete Test",
                "tipoDocumento": "contrato",
                "entityType": "contrato",
                "contenido": {"version": 1, "blocks": []}
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let plantilla_id = body["id"].as_str().unwrap().to_string();

        // Try to delete as visualizador
        let viewer_id = create_test_usuario(&db, "visualizador", org_id).await;
        let viewer_token = make_token(viewer_id, "visualizador", org_id);

        let req = actix_web::test::TestRequest::delete()
            .uri(&format!("/api/v1/documentos/plantillas/{plantilla_id}"))
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {viewer_token}")))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "Visualizador should get 403 on template delete"
        );
    });
}

#[test]
fn test_rbac_gerente_can_sign() {
    with_db(|db| async move {
        let (org_id, _admin_id, doc_id) = setup_firmas_data(&db).await;
        let gerente_id = create_test_usuario(&db, "gerente", org_id).await;
        let token = make_token(gerente_id, "gerente", org_id);
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/documentos/{doc_id}/firmar"))
            .peer_addr(test_peer())
            .insert_header(("Authorization", format!("Bearer {token}")))
            .insert_header(("User-Agent", "GerenteApp/1.0"))
            .set_json(json!({ "firmaImagen": valid_firma_imagen_b64() }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Gerente should be able to sign"
        );

        let body: Value = actix_web::test::read_body_json(resp).await;
        assert_eq!(body["firmanteTipo"], "propietario");
        assert_eq!(body["estado"], "firmado");
    });
}

#[test]
fn test_rbac_docx_export_rejects_unauthenticated() {
    with_db(|db| async move {
        let (_org_id, _admin_id, doc_id) = setup_firmas_data(&db).await;
        let app =
            actix_web::test::init_service(create_app(db.clone(), make_config(), preview_store()))
                .await;

        let req = actix_web::test::TestRequest::get()
            .uri(&format!("/api/v1/documentos/{doc_id}/exportar-docx"))
            .peer_addr(test_peer())
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "Unauthenticated should get 401 on DOCX export"
        );
    });
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Test: Mail integration using file-transport (AsyncFileTransport)
// Requirements: 3.5, 3.6
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

/// A `MailClient` implementation backed by `lettre::AsyncFileTransport`.
/// Writes `.eml` files to a temp directory for test inspection.
struct FileMailClient {
    dir: std::path::PathBuf,
}

impl FileMailClient {
    const fn new(dir: std::path::PathBuf) -> Self {
        Self { dir }
    }
}

impl realestate_backend::services::mail::MailClient for FileMailClient {
    fn send(
        &self,
        msg: realestate_backend::services::mail::OutgoingMail,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), realestate_backend::errors::AppError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            use lettre::message::MultiPart;
            use lettre::{AsyncFileTransport, AsyncTransport, Message, Tokio1Executor};

            let email = Message::builder()
                .from("no-reply@myhomeva.us".parse().unwrap())
                .to(msg.to.parse().map_err(|e| {
                    realestate_backend::errors::AppError::Validation(format!(
                        "DirecciÃ³n de correo invÃ¡lida: {e}"
                    ))
                })?)
                .subject(&msg.subject)
                .multipart(MultiPart::alternative_plain_html(
                    msg.body_text,
                    msg.body_html,
                ))
                .map_err(|e| {
                    realestate_backend::errors::AppError::Internal(anyhow::anyhow!(
                        "Error construyendo correo: {e}"
                    ))
                })?;

            let transport = AsyncFileTransport::<Tokio1Executor>::new(&self.dir);
            transport.send(email).await.map_err(|e| {
                realestate_backend::errors::AppError::Internal(anyhow::anyhow!(
                    "Error escribiendo correo a archivo: {e}"
                ))
            })?;

            Ok(())
        })
    }
}

#[test]
fn test_mail_file_transport_signing_email() {
    dotenvy::dotenv().ok();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Create a temp directory for .eml files
        let mail_dir = std::env::temp_dir().join(format!("mail_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&mail_dir).unwrap();

        let mail_client = FileMailClient::new(mail_dir.clone());

        let recipient = "inquilino@example.com";
        let token = "abc123-test-token";
        let password =
            std::env::var("TEST_FIRMA_PASSWORD").unwrap_or_else(|_| "SecurePass99".to_string()); // test-only

        // Call the signing email function directly
        let result = realestate_backend::services::firmas::enviar_email_firma(
            &mail_client,
            recipient,
            token,
            &password,
        )
        .await;

        assert!(
            result.is_ok(),
            "enviar_email_firma should succeed: {result:?}"
        );

        // Find the .eml file written by the file transport
        let entries: Vec<_> = std::fs::read_dir(&mail_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "eml"))
            .collect();

        assert_eq!(
            entries.len(),
            1,
            "Expected exactly one .eml file, found {}",
            entries.len()
        );

        let eml_content = std::fs::read_to_string(entries[0].path()).unwrap();

        // Helper: decode MIME Q-encoded and base64 body parts for assertion.
        // lettre Q-encodes the Subject (spaces â†’ '_', non-ASCII â†’ =XX) and
        // may base64-encode multipart body parts.
        let decoded_subject = {
            use base64::Engine;
            // Extract Subject header line(s) and decode MIME encoded-words
            let mut subj = String::new();
            for line in eml_content.lines() {
                if line.starts_with("Subject:") {
                    subj = line.to_string();
                    // Continuation lines (start with whitespace)
                    continue;
                }
                if !subj.is_empty() && (line.starts_with(' ') || line.starts_with('\t')) {
                    subj.push_str(line.trim());
                } else if !subj.is_empty() {
                    break;
                }
            }
            // Decode =?utf-8?b?...?= (base64) and =?utf-8?q?...?= (quoted-printable) tokens
            let mut decoded = subj.clone();
            // Base64 encoded words
            while let Some(start) = decoded
                .find("=?utf-8?b?")
                .or_else(|| decoded.find("=?UTF-8?b?"))
            {
                let prefix_len = "=?utf-8?b?".len();
                if let Some(end) = decoded[start + prefix_len..].find("?=") {
                    let b64_data = &decoded[start + prefix_len..start + prefix_len + end];
                    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64_data) {
                        if let Ok(text) = String::from_utf8(bytes) {
                            decoded = format!(
                                "{}{}{}",
                                &decoded[..start],
                                text,
                                &decoded[start + prefix_len + end + 2..]
                            );
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            // Q-encoded words
            if decoded.contains("=?utf-8?q?") || decoded.contains("=?UTF-8?q?") {
                decoded = decoded
                    .replace('_', " ")
                    .replace("=C3=B3", "Ã³")
                    .replace("=C3=A9", "Ã©")
                    .replace("=C3=AD", "Ã­")
                    .replace("=?utf-8?q?", "")
                    .replace("=?UTF-8?q?", "")
                    .replace("?=", "");
            }
            decoded
        };

        // Decode base64 body parts to check content
        let decoded_body = {
            use base64::Engine;
            let mut body = String::new();
            let mut in_base64_part = false;
            let mut b64_buf = String::new();
            for line in eml_content.lines() {
                if line.starts_with("Content-Transfer-Encoding: base64") {
                    in_base64_part = true;
                    b64_buf.clear();
                    continue;
                }
                if in_base64_part {
                    if line.is_empty() && !b64_buf.is_empty() {
                        // Skip the blank line after the header
                        continue;
                    }
                    if line.starts_with("--") {
                        // MIME boundary â€” decode accumulated base64
                        if let Ok(bytes) =
                            base64::engine::general_purpose::STANDARD.decode(b64_buf.trim())
                        {
                            if let Ok(text) = String::from_utf8(bytes) {
                                body.push_str(&text);
                                body.push('\n');
                            }
                        }
                        b64_buf.clear();
                        in_base64_part = false;
                    } else {
                        b64_buf.push_str(line);
                    }
                }
            }
            // Decode any remaining base64 buffer
            if !b64_buf.is_empty() {
                if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64_buf.trim())
                {
                    if let Ok(text) = String::from_utf8(bytes) {
                        body.push_str(&text);
                    }
                }
            }
            // Also include raw eml_content as fallback (for 7bit/quoted-printable parts)
            format!("{body}\n{eml_content}")
        };

        // Assert Spanish subject
        assert!(
            decoded_subject.contains("Firma electr"),
            "Email should contain Spanish subject about firma electrÃ³nica. Got: {decoded_subject}"
        );
        assert!(
            decoded_subject.contains("contrato"),
            "Email subject should reference contrato. Got: {decoded_subject}"
        );

        // Assert recipient address
        assert!(
            eml_content.contains(recipient),
            "Email should be addressed to the recipient: {recipient}"
        );

        // Assert Spanish body content
        assert!(
            decoded_body.contains("inquilino") || decoded_body.contains("Estimado"),
            "Email body should contain Spanish text"
        );
        assert!(
            decoded_body.contains("firma electr") || decoded_body.contains("firma"),
            "Email body should mention firma"
        );

        // Assert signing link is present
        assert!(
            decoded_body.contains(&format!("/firmas/{token}")),
            "Email should contain the signing link with token"
        );

        // Assert from address
        assert!(
            eml_content.contains("no-reply@myhomeva.us"),
            "Email should be from no-reply@myhomeva.us"
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&mail_dir);
    });
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Test: Real Mailcow staging (gated, requires SMTP credentials)
// Requirements: 3.5, 3.6
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[test]
#[ignore = "Requires real Mailcow SMTP credentials (SMTP_HOST, SMTP_USER, SMTP_PASS)"]
fn test_mail_real_mailcow_staging() {
    dotenvy::dotenv().ok();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use realestate_backend::config::SmtpConfig;
        use realestate_backend::services::mail::SmtpMailClient;

        let smtp_cfg = SmtpConfig::from_env().expect("SMTP env vars must be set for staging test");
        let client = SmtpMailClient::from_config(&smtp_cfg)
            .expect("SmtpMailClient should build from valid config");

        let test_recipient = std::env::var("SMTP_TEST_RECIPIENT")
            .unwrap_or_else(|_| "staging-test@myhomeva.us".to_string());

        let test_password =
            std::env::var("TEST_FIRMA_PASSWORD").unwrap_or_else(|_| "StagingPass123".to_string()); // test-only

        let result = realestate_backend::services::firmas::enviar_email_firma(
            &client,
            &test_recipient,
            "staging-test-token-12345",
            &test_password,
        )
        .await;

        assert!(result.is_ok(), "Real SMTP send should succeed: {result:?}");
    });
}
