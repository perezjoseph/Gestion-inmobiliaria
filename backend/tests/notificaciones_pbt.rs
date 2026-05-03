#![allow(clippy::needless_return)]
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

use crate::migrations;

// ── Custom Strategies ──────────────────────────────────────────────────

fn valid_tipo() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("pago_vencido".to_string()),
        Just("contrato_por_vencer".to_string()),
        Just("documento_vencido".to_string()),
        Just("mantenimiento_actualizado".to_string()),
    ]
}

fn valid_entity_type_for_tipo(tipo: &str) -> &'static str {
    match tipo {
        "pago_vencido" => "pago",
        "contrato_por_vencer" => "contrato",
        "documento_vencido" => "documento",
        "mantenimiento_actualizado" => "solicitud_mantenimiento",
        _ => "unknown",
    }
}

fn random_bool() -> impl Strategy<Value = bool> {
    prop_oneof![Just(true), Just(false)]
}

fn notification_count() -> impl Strategy<Value = usize> {
    2usize..6usize
}

fn read_unread_mix_count() -> impl Strategy<Value = (usize, usize)> {
    (1usize..4usize, 1usize..4usize)
}

// ── Async helpers module ───────────────────────────────────────────────

mod pbt_async {
    use chrono::Utc;
    use realestate_backend::entities::notificacion;
    use realestate_backend::models::notificacion::NotificacionListQuery;
    use realestate_backend::services::notificaciones;
    use sea_orm::{
        ActiveModelTrait, ColumnTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait,
        QueryFilter, Set,
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
                        .map_err(|e| format!("Failed to connect: {e}"))?;
                    super::migrations::Migrator::up(&db, None)
                        .await
                        .map_err(|e| format!("Failed migrations: {e}"))?;
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
            eprintln!("⚠ DATABASE_URL not set – skipping PBT");
            return;
        }
        let _guard = crate::GLOBAL_DB_SERIAL
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some((rt, db)) = shared_rt_and_db() else {
            eprintln!("⚠ DB not reachable – skipping PBT");
            return;
        };
        rt.block_on(f(db.clone()));
    }

    async fn create_org(db: &DatabaseConnection) -> Uuid {
        use realestate_backend::entities::organizacion;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        organizacion::ActiveModel {
            id: Set(id),
            tipo: Set("persona_fisica".to_string()),
            nombre: Set(format!("PBT Org {id}")),
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
        .expect("create org");
        id
    }

    async fn create_user(db: &DatabaseConnection, rol: &str, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::usuario;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        usuario::ActiveModel {
            id: Set(id),
            nombre: Set(format!("PBT {rol}")),
            email: Set(format!("{rol}+{id}@pbt.com")),
            password_hash: Set("not_used".to_string()),
            rol: Set(rol.to_string()),
            activo: Set(true),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("create user");
        id
    }

    async fn insert_notificacion(
        db: &DatabaseConnection,
        tipo: &str,
        entity_type: &str,
        usuario_id: Uuid,
        org_id: Uuid,
        leida: bool,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        notificacion::ActiveModel {
            id: Set(id),
            tipo: Set(tipo.to_string()),
            titulo: Set(format!("Titulo {tipo} {id}")),
            mensaje: Set(format!("Mensaje para {tipo}")),
            leida: Set(leida),
            entity_type: Set(entity_type.to_string()),
            entity_id: Set(Uuid::new_v4()),
            usuario_id: Set(usuario_id),
            organizacion_id: Set(org_id),
            created_at: Set(now),
        }
        .insert(db)
        .await
        .expect("insert notificacion");
        id
    }

    async fn cleanup(db: &DatabaseConnection, usuario_id: Uuid) {
        let _ = notificacion::Entity::delete_many()
            .filter(notificacion::Column::UsuarioId.eq(usuario_id))
            .exec(db)
            .await;
    }

    // ── P1: Listing returns only user's notifications ──────────────────
    pub fn p1(tipo: String, count: usize) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_a = create_user(&db, "admin", org_id).await;
            let user_b = create_user(&db, "gerente", org_id).await;
            let et = super::valid_entity_type_for_tipo(&tipo);

            for _ in 0..count {
                insert_notificacion(&db, &tipo, et, user_a, org_id, false).await;
                insert_notificacion(&db, &tipo, et, user_b, org_id, false).await;
            }

            let query = NotificacionListQuery {
                leida: None,
                tipo: None,
                page: Some(1),
                per_page: Some(100),
            };
            let result = notificaciones::listar(&db, user_a, query).await.unwrap();
            for n in &result.data {
                assert_eq!(
                    n.usuario_id, user_a,
                    "Listing returned notification for wrong user"
                );
            }

            cleanup(&db, user_a).await;
            cleanup(&db, user_b).await;
        });
    }

    // ── P2: List ordering invariant ────────────────────────────────────
    pub fn p2(count: usize) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_id = create_user(&db, "admin", org_id).await;

            for _ in 0..count {
                insert_notificacion(&db, "pago_vencido", "pago", user_id, org_id, false).await;
            }

            let query = NotificacionListQuery {
                leida: None,
                tipo: None,
                page: Some(1),
                per_page: Some(100),
            };
            let result = notificaciones::listar(&db, user_id, query).await.unwrap();
            for window in result.data.windows(2) {
                assert!(
                    window[0].created_at >= window[1].created_at,
                    "List not in descending order: {} < {}",
                    window[0].created_at,
                    window[1].created_at
                );
            }

            cleanup(&db, user_id).await;
        });
    }

    // ── P3: Filtering returns only matching records ────────────────────
    pub fn p3(tipo: String, filter_leida: bool) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_id = create_user(&db, "admin", org_id).await;
            let et = super::valid_entity_type_for_tipo(&tipo);

            // Insert mix of types and read states
            insert_notificacion(&db, &tipo, et, user_id, org_id, true).await;
            insert_notificacion(&db, &tipo, et, user_id, org_id, false).await;
            insert_notificacion(&db, "pago_vencido", "pago", user_id, org_id, true).await;
            insert_notificacion(
                &db,
                "contrato_por_vencer",
                "contrato",
                user_id,
                org_id,
                false,
            )
            .await;

            // Filter by tipo
            let query_tipo = NotificacionListQuery {
                leida: None,
                tipo: Some(tipo.clone()),
                page: Some(1),
                per_page: Some(100),
            };
            let result = notificaciones::listar(&db, user_id, query_tipo)
                .await
                .unwrap();
            for n in &result.data {
                assert_eq!(
                    n.tipo, tipo,
                    "Filter by tipo returned wrong tipo: {}",
                    n.tipo
                );
            }

            // Filter by leida
            let query_leida = NotificacionListQuery {
                leida: Some(filter_leida),
                tipo: None,
                page: Some(1),
                per_page: Some(100),
            };
            let result = notificaciones::listar(&db, user_id, query_leida)
                .await
                .unwrap();
            for n in &result.data {
                assert_eq!(
                    n.leida, filter_leida,
                    "Filter by leida returned wrong state: {}",
                    n.leida
                );
            }

            cleanup(&db, user_id).await;
        });
    }

    // ── P4: Unread count consistency ───────────────────────────────────
    pub fn p4(num_read: usize, num_unread: usize) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_id = create_user(&db, "admin", org_id).await;

            let mut unread_ids = Vec::new();
            for _ in 0..num_read {
                insert_notificacion(&db, "pago_vencido", "pago", user_id, org_id, true).await;
            }
            for _ in 0..num_unread {
                let id =
                    insert_notificacion(&db, "pago_vencido", "pago", user_id, org_id, false).await;
                unread_ids.push(id);
            }

            // Verify initial count
            let count = notificaciones::conteo_no_leidas(&db, user_id)
                .await
                .unwrap();
            assert_eq!(
                count, num_unread as u64,
                "Initial unread count mismatch: expected {num_unread}, got {count}"
            );

            // Mark one as read if there are unread
            if !unread_ids.is_empty() {
                let _ = notificaciones::marcar_leida(&db, unread_ids[0], user_id)
                    .await
                    .unwrap();
                let count = notificaciones::conteo_no_leidas(&db, user_id)
                    .await
                    .unwrap();
                assert_eq!(
                    count,
                    (num_unread - 1) as u64,
                    "After marking one, count should be {}",
                    num_unread - 1
                );
            }

            // Mark all as read
            let _ = notificaciones::marcar_todas_leidas(&db, user_id)
                .await
                .unwrap();
            let count = notificaciones::conteo_no_leidas(&db, user_id)
                .await
                .unwrap();
            assert_eq!(count, 0, "After mark all, count should be 0");

            cleanup(&db, user_id).await;
        });
    }

    // ── P5: Mark as read is idempotent ─────────────────────────────────
    pub fn p5(tipo: String) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_id = create_user(&db, "admin", org_id).await;
            let et = super::valid_entity_type_for_tipo(&tipo);
            let notif_id = insert_notificacion(&db, &tipo, et, user_id, org_id, false).await;

            // First mark
            let resp1 = notificaciones::marcar_leida(&db, notif_id, user_id)
                .await
                .unwrap();
            assert!(resp1.leida, "First mark should set leida=true");
            let count1 = notificaciones::conteo_no_leidas(&db, user_id)
                .await
                .unwrap();

            // Second mark (idempotent)
            let resp2 = notificaciones::marcar_leida(&db, notif_id, user_id)
                .await
                .unwrap();
            assert!(resp2.leida, "Second mark should still be leida=true");
            let count2 = notificaciones::conteo_no_leidas(&db, user_id)
                .await
                .unwrap();

            assert_eq!(
                count1, count2,
                "Idempotent mark should not change count: {count1} vs {count2}"
            );

            cleanup(&db, user_id).await;
        });
    }

    // ── P6: Mark all updates only unread ───────────────────────────────
    pub fn p6(num_read: usize, num_unread: usize) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_id = create_user(&db, "admin", org_id).await;

            for _ in 0..num_read {
                insert_notificacion(&db, "pago_vencido", "pago", user_id, org_id, true).await;
            }
            for _ in 0..num_unread {
                insert_notificacion(&db, "pago_vencido", "pago", user_id, org_id, false).await;
            }

            let updated = notificaciones::marcar_todas_leidas(&db, user_id)
                .await
                .unwrap();
            assert_eq!(
                updated, num_unread as u64,
                "mark_all should return count of previously unread: expected {num_unread}, got {updated}"
            );

            let count = notificaciones::conteo_no_leidas(&db, user_id)
                .await
                .unwrap();
            assert_eq!(count, 0, "After mark all, unread count should be 0");

            cleanup(&db, user_id).await;
        });
    }

    // ── P7: Notification deduplication ─────────────────────────────────
    pub fn p7() {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_id = create_user(&db, "admin", org_id).await;

            // Create overdue pago data for the org
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;
            let contrato_id =
                create_contrato(&db, propiedad_id, inquilino_id, org_id, "activo", -1).await;
            create_pago_vencido(&db, contrato_id, org_id).await;

            // First generation
            let resp1 = notificaciones::generar_notificaciones(&db, org_id)
                .await
                .unwrap();
            let first_total = resp1.total;

            // Second generation — should produce zero new
            let resp2 = notificaciones::generar_notificaciones(&db, org_id)
                .await
                .unwrap();
            assert_eq!(
                resp2.total, 0,
                "Second generation should produce 0 new, got {}",
                resp2.total
            );

            // Total count should be same as first
            let query = NotificacionListQuery {
                leida: None,
                tipo: None,
                page: Some(1),
                per_page: Some(100),
            };
            let list = notificaciones::listar(&db, user_id, query).await.unwrap();
            assert_eq!(
                list.total, first_total,
                "Total notifications should remain {first_total} after dedup"
            );

            cleanup(&db, user_id).await;
        });
    }

    // ── P8: Generated notifications have correct fields ────────────────
    pub fn p8() {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_id = create_user(&db, "admin", org_id).await;

            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;
            let contrato_id =
                create_contrato(&db, propiedad_id, inquilino_id, org_id, "activo", -1).await;
            create_pago_vencido(&db, contrato_id, org_id).await;

            let _ = notificaciones::generar_notificaciones(&db, org_id)
                .await
                .unwrap();

            let query = NotificacionListQuery {
                leida: None,
                tipo: None,
                page: Some(1),
                per_page: Some(100),
            };
            let result = notificaciones::listar(&db, user_id, query).await.unwrap();

            let valid_tipos = [
                "pago_vencido",
                "contrato_por_vencer",
                "documento_vencido",
                "mantenimiento_actualizado",
            ];

            for n in &result.data {
                assert!(
                    valid_tipos.contains(&n.tipo.as_str()),
                    "Invalid tipo: {}",
                    n.tipo
                );

                let expected_et = super::valid_entity_type_for_tipo(&n.tipo);
                assert_eq!(
                    n.entity_type, expected_et,
                    "entity_type mismatch for tipo {}: expected {expected_et}, got {}",
                    n.tipo, n.entity_type
                );

                assert!(!n.titulo.is_empty(), "titulo should not be empty");
                assert!(!n.mensaje.is_empty(), "mensaje should not be empty");
            }

            cleanup(&db, user_id).await;
        });
    }

    // ── P9: Cross-user isolation on mark operations ────────────────────
    pub fn p9(count: usize) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_a = create_user(&db, "admin", org_id).await;
            let user_b = create_user(&db, "gerente", org_id).await;

            let mut a_ids = Vec::new();
            for _ in 0..count {
                let id =
                    insert_notificacion(&db, "pago_vencido", "pago", user_a, org_id, false).await;
                a_ids.push(id);
                insert_notificacion(&db, "pago_vencido", "pago", user_b, org_id, false).await;
            }

            let b_count_before = notificaciones::conteo_no_leidas(&db, user_b).await.unwrap();

            // Mark all of user A's as read
            for id in &a_ids {
                let _ = notificaciones::marcar_leida(&db, *id, user_a)
                    .await
                    .unwrap();
            }

            // User B's count should be unchanged
            let b_count_after = notificaciones::conteo_no_leidas(&db, user_b).await.unwrap();
            assert_eq!(
                b_count_before, b_count_after,
                "User B count changed after marking A's: {b_count_before} vs {b_count_after}"
            );

            // User A trying to mark user B's notification should fail
            let b_query = NotificacionListQuery {
                leida: None,
                tipo: None,
                page: Some(1),
                per_page: Some(100),
            };
            let b_list = notificaciones::listar(&db, user_b, b_query).await.unwrap();
            if let Some(b_notif) = b_list.data.first() {
                let result = notificaciones::marcar_leida(&db, b_notif.id, user_a).await;
                assert!(
                    result.is_err(),
                    "User A should not be able to mark B's notification"
                );
            }

            cleanup(&db, user_a).await;
            cleanup(&db, user_b).await;
        });
    }

    // ── P10: New notifications default to unread ───────────────────────
    pub fn p10() {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let user_id = create_user(&db, "admin", org_id).await;

            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;
            let contrato_id =
                create_contrato(&db, propiedad_id, inquilino_id, org_id, "activo", -1).await;
            create_pago_vencido(&db, contrato_id, org_id).await;

            let _ = notificaciones::generar_notificaciones(&db, org_id)
                .await
                .unwrap();

            let query = NotificacionListQuery {
                leida: None,
                tipo: None,
                page: Some(1),
                per_page: Some(100),
            };
            let result = notificaciones::listar(&db, user_id, query).await.unwrap();

            for n in &result.data {
                assert!(
                    !n.leida,
                    "New notification should default to unread (leida=false)"
                );
            }

            cleanup(&db, user_id).await;
        });
    }

    // ── Data helpers for generator tests ───────────────────────────────

    async fn create_propiedad(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::propiedad;
        use rust_decimal::Decimal;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        propiedad::ActiveModel {
            id: Set(id),
            titulo: Set("Propiedad PBT Notif".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle PBT 456".to_string()),
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
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("create propiedad");
        id
    }

    async fn create_inquilino(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::inquilino;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        inquilino::ActiveModel {
            id: Set(id),
            nombre: Set("PBT Inquilino".to_string()),
            apellido: Set("Test".to_string()),
            email: Set(Some(format!("inq+{id}@pbt.com"))),
            telefono: Set(None),
            cedula: Set(format!("CED-{id}")),
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

    async fn create_contrato(
        db: &DatabaseConnection,
        propiedad_id: Uuid,
        inquilino_id: Uuid,
        org_id: Uuid,
        estado: &str,
        days_offset: i64,
    ) -> Uuid {
        use realestate_backend::entities::contrato;
        use rust_decimal::Decimal;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        let fecha_fin = Utc::now().date_naive() + chrono::Duration::days(days_offset);
        contrato::ActiveModel {
            id: Set(id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            fecha_fin: Set(fecha_fin),
            monto_mensual: Set(Decimal::new(25000, 0)),
            deposito: Set(None),
            moneda: Set("DOP".to_string()),
            estado: Set(estado.to_string()),
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
        .expect("create contrato");
        id
    }

    async fn create_pago_vencido(db: &DatabaseConnection, contrato_id: Uuid, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::pago;
        use rust_decimal::Decimal;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        let overdue_date = Utc::now().date_naive() - chrono::Duration::days(5);
        pago::ActiveModel {
            id: Set(id),
            contrato_id: Set(contrato_id),
            monto: Set(Decimal::new(25000, 0)),
            moneda: Set("DOP".to_string()),
            fecha_pago: Set(None),
            fecha_vencimiento: Set(overdue_date),
            metodo_pago: Set(None),
            estado: Set("pendiente".to_string()),
            notas: Set(None),
            recargo: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("create pago");
        id
    }
} // end pbt_async

// ── Test functions ─────────────────────────────────────────────────────

// Feature: notification-system, Property 1: Listing returns only user's notifications
// **Validates: Requirements 2.4, 3.2**
#[test]
fn test_listing_returns_only_users_notifications() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&(valid_tipo(), notification_count()), |(tipo, count)| {
            pbt_async::p1(tipo, count);
            Ok(())
        })
        .unwrap();
}

// Feature: notification-system, Property 2: List ordering invariant
// **Validates: Requirements 2.1**
#[test]
fn test_list_ordering_invariant() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&notification_count(), |count| {
            pbt_async::p2(count);
            Ok(())
        })
        .unwrap();
}

// Feature: notification-system, Property 3: Filtering returns only matching records
// **Validates: Requirements 2.2, 2.3**
#[test]
fn test_filtering_returns_matching() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&(valid_tipo(), random_bool()), |(tipo, leida)| {
            pbt_async::p3(tipo, leida);
            Ok(())
        })
        .unwrap();
}

// Feature: notification-system, Property 4: Unread count consistency
// **Validates: Requirements 3.1, 4.1, 5.1, 5.3**
#[test]
fn test_unread_count_consistency() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&read_unread_mix_count(), |(num_read, num_unread)| {
            pbt_async::p4(num_read, num_unread);
            Ok(())
        })
        .unwrap();
}

// Feature: notification-system, Property 5: Mark as read is idempotent
// **Validates: Requirements 4.4**
#[test]
fn test_mark_read_idempotent() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&valid_tipo(), |tipo| {
            pbt_async::p5(tipo);
            Ok(())
        })
        .unwrap();
}

// Feature: notification-system, Property 6: Mark all as read updates only unread
// **Validates: Requirements 5.1, 5.2, 5.3**
#[test]
fn test_mark_all_updates_only_unread() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&read_unread_mix_count(), |(num_read, num_unread)| {
            pbt_async::p6(num_read, num_unread);
            Ok(())
        })
        .unwrap();
}

// Feature: notification-system, Property 7: Notification deduplication
// **Validates: Requirements 6.3, 7.3, 8.3**
#[test]
fn test_deduplication() {
    // This test uses real data generation, so we run it fewer times
    // but still validate the deduplication property
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&Just(()), |()| {
            pbt_async::p7();
            Ok(())
        })
        .unwrap();
}

// Feature: notification-system, Property 8: Generated notifications have correct fields
// **Validates: Requirements 1.2, 6.2, 7.2, 8.2**
#[test]
fn test_generated_notifications_correct_fields() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&Just(()), |()| {
            pbt_async::p8();
            Ok(())
        })
        .unwrap();
}

// Feature: notification-system, Property 9: Cross-user isolation on mark operations
// **Validates: Requirements 4.3**
#[test]
fn test_cross_user_isolation() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&notification_count(), |count| {
            pbt_async::p9(count);
            Ok(())
        })
        .unwrap();
}

// Feature: notification-system, Property 10: New notifications default to unread
// **Validates: Requirements 1.3**
#[test]
fn test_new_notifications_default_unread() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&Just(()), |()| {
            pbt_async::p10();
            Ok(())
        })
        .unwrap();
}
