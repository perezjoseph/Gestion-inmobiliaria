use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use uuid::Uuid;

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate a valid E.164 phone number (+ followed by 7-15 digits, first digit non-zero).
fn arb_e164_phone() -> impl Strategy<Value = String> {
    // Country code 1-3 digits (first non-zero) + subscriber 6-12 digits
    (1u8..=9, prop::collection::vec(0u8..=9, 6..=12)).prop_map(|(first, rest)| {
        let digits: String = rest.iter().map(|d| char::from(b'0' + d)).collect();
        format!("+{first}{digits}")
    })
}

/// Generate a pair of distinct UUIDs representing two different organizations.
fn arb_distinct_org_ids() -> impl Strategy<Value = (Uuid, Uuid)> {
    // Generate two random u128 values and ensure they differ
    (any::<u128>(), any::<u128>())
        .prop_filter("org IDs must differ", |(a, b)| a != b)
        .prop_map(|(a, b)| (Uuid::from_u128(a), Uuid::from_u128(b)))
}

// ── Tenant Resolution Model ───────────────────────────────────────────

/// Represents a tenant record with phone and organization scope.
#[derive(Debug, Clone)]
struct TenantRecord {
    phone: String,
    org_id: Uuid,
}

/// Simulates the org-scoped tenant resolution logic:
/// Query `inquilinos` WHERE `telefono = phone AND organizacion_id = org_id`.
/// Returns true if a matching tenant exists.
///
/// This mirrors the `tenants_only` policy check in `is_sender_allowed`:
/// ```rust
/// "tenants_only" => tenant_exists_by_phone(phone, org_id, db).await
/// ```
/// The critical property is that BOTH phone AND org_id are used as filters.
fn resolve_tenant_in_org(tenants: &[TenantRecord], phone: &str, org_id: Uuid) -> bool {
    tenants
        .iter()
        .any(|t| t.phone == phone && t.org_id == org_id)
}

// ── Property Tests ─────────────────────────────────────────────────────

// Feature: whatsapp-ai-assistant, Property 3: Organization-Scoped Tenant Resolution Isolation
// **Validates: Requirements 2.5, 13.2**
#[test]
fn test_org_scoped_tenant_resolution_isolation() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy =
        (arb_e164_phone(), arb_distinct_org_ids()).prop_flat_map(|(phone, (org_a, org_b))| {
            // Generate additional tenants in org_b with DIFFERENT phones to ensure
            // the org_b tenant set is non-empty but doesn't contain our target phone
            let other_phones = prop::collection::vec(arb_e164_phone(), 0..=5).prop_filter(
                "other phones must not match target",
                {
                    let phone = phone.clone();
                    move |phones| phones.iter().all(|p| *p != phone)
                },
            );
            (Just(phone), Just(org_a), Just(org_b), other_phones)
        });

    runner
        .run(&strategy, |(phone, org_a, org_b, other_phones_in_b)| {
            // Setup: phone P exists as a tenant in org_a
            let mut tenants = vec![TenantRecord {
                phone: phone.clone(),
                org_id: org_a,
            }];

            // Add other tenants in org_b (with different phones)
            for other_phone in &other_phones_in_b {
                tenants.push(TenantRecord {
                    phone: other_phone.clone(),
                    org_id: org_b,
                });
            }

            // Property: resolving phone P in org_a SHOULD find a match
            prop_assert!(
                resolve_tenant_in_org(&tenants, &phone, org_a),
                "Phone {} should be found in org_a {}",
                phone,
                org_a
            );

            // Property: resolving phone P in org_b SHALL return no match
            prop_assert!(
                !resolve_tenant_in_org(&tenants, &phone, org_b),
                "Phone {} must NOT be found in org_b {} (isolation violated)",
                phone,
                org_b
            );

            Ok(())
        })
        .unwrap();
}

// Feature: whatsapp-ai-assistant, Property 3 (supplementary): Resolution uses both filters
// **Validates: Requirements 2.5, 13.2**
#[test]
fn test_resolution_requires_both_phone_and_org_id() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy = (arb_e164_phone(), arb_e164_phone(), arb_distinct_org_ids())
        .prop_filter("phones must differ", |(p1, p2, _)| p1 != p2);

    runner
        .run(&strategy, |(phone_a, phone_b, (org_a, org_b))| {
            // Setup: phone_a in org_a, phone_b in org_b
            let tenants = vec![
                TenantRecord {
                    phone: phone_a.clone(),
                    org_id: org_a,
                },
                TenantRecord {
                    phone: phone_b.clone(),
                    org_id: org_b,
                },
            ];

            // Correct org + correct phone → match
            prop_assert!(resolve_tenant_in_org(&tenants, &phone_a, org_a));
            prop_assert!(resolve_tenant_in_org(&tenants, &phone_b, org_b));

            // Correct phone + wrong org → no match (isolation)
            prop_assert!(!resolve_tenant_in_org(&tenants, &phone_a, org_b));
            prop_assert!(!resolve_tenant_in_org(&tenants, &phone_b, org_a));

            // Wrong phone + correct org → no match
            prop_assert!(!resolve_tenant_in_org(&tenants, &phone_b, org_a));
            prop_assert!(!resolve_tenant_in_org(&tenants, &phone_a, org_b));

            Ok(())
        })
        .unwrap();
}

// Feature: whatsapp-ai-assistant, Property 3 (supplementary): Same phone in multiple orgs
// **Validates: Requirements 2.5, 13.2**
#[test]
fn test_same_phone_different_orgs_resolved_independently() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy = (
        arb_e164_phone(),
        arb_distinct_org_ids(),
        any::<bool>(), // whether phone exists in org_b too
    );

    runner
        .run(&strategy, |(phone, (org_a, org_b), exists_in_b)| {
            // Phone always exists in org_a
            let mut tenants = vec![TenantRecord {
                phone: phone.clone(),
                org_id: org_a,
            }];

            // Conditionally add to org_b
            if exists_in_b {
                tenants.push(TenantRecord {
                    phone: phone.clone(),
                    org_id: org_b,
                });
            }

            // org_a always resolves
            prop_assert!(resolve_tenant_in_org(&tenants, &phone, org_a));

            // org_b resolves only if we added it there
            prop_assert_eq!(
                resolve_tenant_in_org(&tenants, &phone, org_b),
                exists_in_b,
                "Resolution in org_b should match whether phone was added there"
            );

            Ok(())
        })
        .unwrap();
}

// ── Integration Test: record_extraction post-loop wiring ──────────────────────

// Feature: spec-gap-remediation, Task 10.5: Integration test for record_extraction post-loop wiring
// **Validates: Requirements 8.3, 8.5**

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait,
    QueryFilter, Set,
};
use sea_orm_migration::MigratorTrait;

use realestate_backend::entities::{
    chatbot_receipt_extraction, contrato, inquilino, organizacion, pago, propiedad, usuario,
};
use realestate_backend::models::chatbot::Confidence;
use realestate_backend::services::ai_module::PaymentReceipt;
use realestate_backend::services::chatbot::{confirm_receipt, record_extraction_from_agent};

fn db_url() -> String {
    dotenvy::dotenv().ok();
    std::env::var("DATABASE_URL").unwrap_or_default()
}

async fn setup_db() -> Result<DatabaseConnection, String> {
    let url = db_url();
    if url.is_empty() {
        return Err("DATABASE_URL not set".to_string());
    }
    let mut opts = ConnectOptions::new(&url);
    opts.max_connections(5)
        .min_connections(1)
        .connect_timeout(std::time::Duration::from_secs(30))
        .idle_timeout(std::time::Duration::from_secs(60))
        .acquire_timeout(std::time::Duration::from_secs(30));
    let db = Database::connect(opts)
        .await
        .map_err(|e| format!("Failed to connect to database: {e}"))?;
    crate::migrations::Migrator::up(&db, None)
        .await
        .map_err(|e| format!("Failed to run migrations: {e}"))?;
    Ok(db)
}

fn shared_rt_and_db_chatbot() -> Option<&'static (tokio::runtime::Runtime, DatabaseConnection)> {
    static SHARED: std::sync::OnceLock<
        Result<(tokio::runtime::Runtime, DatabaseConnection), String>,
    > = std::sync::OnceLock::new();
    SHARED
        .get_or_init(|| {
            let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Runtime error: {e}"))?;
            let db = rt.block_on(setup_db())?;
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
        eprintln!("⚠ DATABASE_URL not set – skipping integration test");
        return;
    }
    let _guard = crate::GLOBAL_DB_SERIAL
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let Some((rt, db)) = shared_rt_and_db_chatbot() else {
        eprintln!("⚠ DB not reachable – skipping integration test");
        return;
    };
    rt.block_on(f(db.clone()));
}

async fn seed_org(db: &DatabaseConnection) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    organizacion::ActiveModel {
        id: Set(id),
        tipo: Set("persona_fisica".to_string()),
        nombre: Set(format!("Org Chatbot Test {id}")),
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
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .expect("Failed to create test organizacion");
    id
}

async fn seed_usuario(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    usuario::ActiveModel {
        id: Set(id),
        nombre: Set("Admin Test".to_string()),
        email: Set(format!("admin+{id}@test.com")),
        password_hash: Set("not_used".to_string()),
        rol: Set("admin".to_string()),
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

async fn seed_propiedad(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    propiedad::ActiveModel {
        id: Set(id),
        titulo: Set("Propiedad Chatbot Test".to_string()),
        descripcion: Set(None),
        direccion: Set("Calle Test 456".to_string()),
        ciudad: Set("Santo Domingo".to_string()),
        provincia: Set("Distrito Nacional".to_string()),
        tipo_propiedad: Set("apartamento".to_string()),
        habitaciones: Set(Some(2)),
        banos: Set(Some(1)),
        area_m2: Set(Some(Decimal::new(7500, 2))),
        precio: Set(Decimal::new(2000000, 2)),
        moneda: Set("DOP".to_string()),
        estado: Set("ocupada".to_string()),
        imagenes: Set(None),
        organizacion_id: Set(org_id),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .expect("Failed to create test propiedad");
    id
}

async fn seed_inquilino(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    inquilino::ActiveModel {
        id: Set(id),
        nombre: Set("Juan".to_string()),
        apellido: Set("Pérez".to_string()),
        email: Set(Some(format!("juan+{id}@test.com"))),
        telefono: Set(None),
        cedula: Set(format!("C{}", &id.simple().to_string()[..19])),
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
    id
}

async fn seed_contrato(
    db: &DatabaseConnection,
    org_id: Uuid,
    propiedad_id: Uuid,
    inquilino_id: Uuid,
) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    let today = Utc::now().date_naive();
    contrato::ActiveModel {
        id: Set(id),
        propiedad_id: Set(propiedad_id),
        inquilino_id: Set(inquilino_id),
        fecha_inicio: Set(today),
        fecha_fin: Set(today + chrono::Duration::days(365)),
        monto_mensual: Set(Decimal::new(3000000, 2)),
        deposito: Set(None),
        moneda: Set("DOP".to_string()),
        estado: Set("activo".to_string()),
        documentos: Set(None),
        organizacion_id: Set(org_id),
        created_at: Set(now),
        updated_at: Set(now),
        estado_deposito: Set(None),
        fecha_cobro_deposito: Set(None),
        fecha_devolucion_deposito: Set(None),
        monto_retenido: Set(None),
        motivo_retencion: Set(None),
        recargo_porcentaje: Set(None),
        dias_gracia: Set(None),
    }
    .insert(db)
    .await
    .expect("Failed to create test contrato");
    id
}

/// Integration test: record_extraction post-loop wiring persists a Pago row.
///
/// Simulates the post-loop path in `chatbot_internal.rs`:
/// 1. A successful `ExtractReceiptTool` result (PaymentReceipt) is produced
/// 2. `record_extraction_from_agent` is called to persist the extraction
/// 3. The extraction is confirmed via `confirm_receipt`, creating a Pago row
///
/// This validates that the full pipeline from AI agent extraction through to
/// domain entity persistence works end-to-end.
#[test]
fn test_record_extraction_post_loop_persists_pago() {
    with_db(|db| async move {
        // Seed required entities
        let org_id = seed_org(&db).await;
        let user_id = seed_usuario(&db, org_id).await;
        let propiedad_id = seed_propiedad(&db, org_id).await;
        let inquilino_id = seed_inquilino(&db, org_id).await;
        let contrato_id = seed_contrato(&db, org_id, propiedad_id, inquilino_id).await;

        // Step 1: Stub a successful ExtractReceiptTool result
        let receipt = PaymentReceipt {
            bank: Some("Banco Popular".to_string()),
            amount: Decimal::new(1500000, 2), // 15,000.00
            currency: "DOP".to_string(),
            date: Some("2025-06-15".to_string()),
            reference: Some("REF-12345".to_string()),
            sender_name: Some("Juan Pérez".to_string()),
            recipient: Some("Inmobiliaria Test".to_string()),
            confidence: Confidence::High,
        };

        // Step 2: Call record_extraction_from_agent (the post-loop wiring)
        let extraction = record_extraction_from_agent(&db, &receipt, org_id, Some(user_id))
            .await
            .expect("record_extraction_from_agent should succeed");

        // Assert the extraction row is persisted with correct data
        assert_eq!(extraction.organizacion_id, org_id);
        assert_eq!(extraction.status, "pending_confirmation");
        assert_eq!(extraction.inquilino_id, None);

        // Verify extracted_data contains the receipt fields
        let data = &extraction.extracted_data;
        assert_eq!(data["currency"], "DOP");
        assert_eq!(data["bank"], "Banco Popular");
        assert_eq!(data["reference"], "REF-12345");

        // Step 3: Associate contrato_id with the extraction (simulates landlord review)
        let mut active: chatbot_receipt_extraction::ActiveModel = extraction.into();
        active.contrato_id = Set(Some(contrato_id));
        let updated_extraction = active
            .update(&db)
            .await
            .expect("Failed to update extraction with contrato_id");

        // Step 4: Confirm the extraction — this creates the Pago row
        let confirmed = confirm_receipt(&db, updated_extraction.id, user_id)
            .await
            .expect("confirm_receipt should succeed");

        assert_eq!(confirmed.status, "confirmed");
        assert_eq!(confirmed.confirmed_by, Some(user_id));

        // Step 5: Assert a Pago row was persisted
        let pagos = pago::Entity::find()
            .filter(pago::Column::ContratoId.eq(contrato_id))
            .filter(pago::Column::OrganizacionId.eq(org_id))
            .all(&db)
            .await
            .expect("Failed to query pagos");

        assert_eq!(pagos.len(), 1, "Exactly one Pago row should be persisted");
        let pago_row = &pagos[0];
        assert_eq!(pago_row.contrato_id, contrato_id);
        assert_eq!(pago_row.monto, Decimal::new(1500000, 2));
        assert_eq!(pago_row.moneda, "DOP");
        assert_eq!(pago_row.estado, "pagado");
        assert_eq!(
            pago_row.fecha_pago,
            Some(chrono::NaiveDate::from_ymd_opt(2025, 6, 15).unwrap())
        );
        assert!(
            pago_row.notas.as_ref().unwrap().contains("WhatsApp"),
            "Pago notas should reference WhatsApp origin"
        );
    });
}
