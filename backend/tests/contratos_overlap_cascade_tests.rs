use chrono::Utc;
use realestate_backend::app::create_app;
use realestate_backend::config::AppConfig;
use realestate_backend::entities::propiedad;
use realestate_backend::services::auth::{Claims, encode_jwt};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::common::{JWT_SECRET, test_app_config, with_db};

fn make_config() -> AppConfig {
    test_app_config("")
}

fn make_token(user_id: Uuid, rol: &str, org_id: Uuid) -> String {
    let claims = Claims {
        sub: user_id,
        email: format!("{rol}@test.com"),
        rol: rol.to_string(),
        organizacion_id: org_id,
        jti: Uuid::new_v4(),
        iat: Utc::now().timestamp(),
        exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
    };
    encode_jwt(&claims, JWT_SECRET).unwrap()
}

async fn create_test_organizacion(db: &DatabaseConnection) -> Uuid {
    use realestate_backend::entities::organizacion;
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    organizacion::ActiveModel {
        id: Set(id),
        tipo: Set("persona_fisica".to_string()),
        nombre: Set(format!("Org Overlap Test {id}")),
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
    .expect("create org");
    id
}

async fn create_test_usuario(db: &DatabaseConnection, rol: &str, org_id: Uuid) -> Uuid {
    use realestate_backend::entities::usuario;
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
        password_changed_at: Set(now),
    }
    .insert(db)
    .await
    .expect("create usuario");
    id
}

async fn create_test_propiedad(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    propiedad::ActiveModel {
        id: Set(id),
        titulo: Set("Propiedad Overlap Test".to_string()),
        descripcion: Set(None),
        direccion: Set("Calle Test 123".to_string()),
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
    .expect("create propiedad");
    id
}

async fn create_test_inquilino(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
    use realestate_backend::entities::inquilino;
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    inquilino::ActiveModel {
        id: Set(id),
        nombre: Set("Test".to_string()),
        apellido: Set("Inquilino".to_string()),
        cedula: Set(format!("001-{}-0001", &Uuid::new_v4().to_string()[..7])),
        telefono: Set(None),
        email: Set(Some(format!("inq+{id}@test.com"))),
        contacto_emergencia: Set(None),
        notas: Set(None),
        documentos: Set(None),
        organizacion_id: Set(org_id),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .expect("create inquilino");
    id
}

fn contrato_body(propiedad_id: Uuid, inquilino_id: Uuid, inicio: &str, fin: &str) -> Value {
    json!({
        "propiedadId": propiedad_id,
        "inquilinoId": inquilino_id,
        "fechaInicio": inicio,
        "fechaFin": fin,
        "montoMensual": "20000",
        "moneda": "DOP",
        "diaCobro": 1
    })
}

// ──────────────────────────────────────────────────────────────────────────────

/// Overlapping active contracts on the same propiedad must be rejected with 409.
#[test]
fn create_contrato_overlapping_dates_returns_409() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_a = create_test_inquilino(&db, org_id).await;
        let inquilino_b = create_test_inquilino(&db, org_id).await;
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // First contract: 2027-01-01 to 2027-12-31
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(
                propiedad_id,
                inquilino_a,
                "2027-01-01",
                "2027-12-31",
            ))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        // Second contract overlapping: 2027-06-01 to 2028-06-01 → must be 409
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(
                propiedad_id,
                inquilino_b,
                "2027-06-01",
                "2028-06-01",
            ))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 409);
    });
}

/// A valid contrato creation returns 201 and sets propiedad estado to ocupada.
#[test]
fn create_contrato_valid_returns_201_and_sets_propiedad_ocupada() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(
                propiedad_id,
                inquilino_id,
                "2027-01-01",
                "2027-12-31",
            ))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        // Verify propiedad estado changed to ocupada
        let prop = propiedad::Entity::find_by_id(propiedad_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(prop.estado, "ocupada");
    });
}

/// Terminating the last active contrato cascades propiedad estado back to disponible.
#[test]
fn terminar_last_contrato_sets_propiedad_disponible() {
    with_db(|db| async move {
        let org_id = create_test_organizacion(&db).await;
        let admin_id = create_test_usuario(&db, "admin", org_id).await;
        let token = make_token(admin_id, "admin", org_id);
        let propiedad_id = create_test_propiedad(&db, org_id).await;
        let inquilino_id = create_test_inquilino(&db, org_id).await;
        let app = actix_web::test::init_service(create_app(
            db.clone(),
            make_config(),
            actix_web::web::Data::new(
                realestate_backend::services::ocr_preview::PreviewStore::new(),
            ),
        ))
        .await;

        // Create active contract
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/contratos")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(contrato_body(
                propiedad_id,
                inquilino_id,
                "2027-01-01",
                "2027-12-31",
            ))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
        let body: Value = actix_web::test::read_body_json(resp).await;
        let contrato_id = body["id"].as_str().unwrap();

        // Terminate the contract
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/contratos/{contrato_id}/terminar"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "fechaTerminacion": "2027-06-15" }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Verify propiedad estado reverted to disponible
        let prop = propiedad::Entity::find_by_id(propiedad_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(prop.estado, "disponible");
    });
}
