#![allow(clippy::needless_return)]
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};

use crate::migrations;

// ── Custom Strategies ──────────────────────────────────────────────────

/// Random pago estado: pendiente, pagado, or atrasado.
fn pago_estado() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("pendiente".to_string()),
        Just("pagado".to_string()),
        Just("atrasado".to_string()),
    ]
}

/// Days offset from today for fecha_vencimiento: negative = past, positive = future.
fn days_offset() -> impl Strategy<Value = i64> {
    -60i64..60i64
}

/// Random contrato estado: activo, vencido, cancelado, finalizado, terminado.
fn contrato_estado() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("activo".to_string()),
        Just("vencido".to_string()),
        Just("cancelado".to_string()),
        Just("finalizado".to_string()),
        Just("terminado".to_string()),
    ]
}

/// Random documento estado_verificacion: verificado, pendiente, rechazado, vencido.
fn documento_estado_verificacion() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("verificado".to_string()),
        Just("pendiente".to_string()),
        Just("rechazado".to_string()),
        Just("vencido".to_string()),
    ]
}

/// Random valid task name from TAREAS_VALIDAS.
fn tarea_valida() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("marcar_pagos_atrasados".to_string()),
        Just("marcar_contratos_vencidos".to_string()),
        Just("marcar_documentos_vencidos".to_string()),
        Just("generar_notificaciones".to_string()),
    ]
}

/// Random invalid task name: alphanumeric + underscore strings NOT in TAREAS_VALIDAS.
fn tarea_invalida() -> impl Strategy<Value = String> {
    "[a-z_]{1,30}".prop_filter("must not be a valid task name", |s| {
        !realestate_backend::services::background_jobs::TAREAS_VALIDAS.contains(&s.as_str())
    })
}

// ── Async helpers module ───────────────────────────────────────────────

mod pbt_async {
    use chrono::Utc;
    use realestate_backend::entities::pago;
    use realestate_backend::services::pagos;
    use rust_decimal::Decimal;
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

    async fn create_propiedad(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::propiedad;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        propiedad::ActiveModel {
            id: Set(id),
            titulo: Set("Propiedad PBT BgJobs".to_string()),
            descripcion: Set(None),
            direccion: Set("Calle PBT 789".to_string()),
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
            apellido: Set("BgJobs".to_string()),
            email: Set(Some(format!("inq+{id}@pbt.com"))),
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
        .expect("create inquilino");
        id
    }

    async fn create_contrato(
        db: &DatabaseConnection,
        propiedad_id: Uuid,
        inquilino_id: Uuid,
        org_id: Uuid,
    ) -> Uuid {
        use realestate_backend::entities::contrato;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        contrato::ActiveModel {
            id: Set(id),
            propiedad_id: Set(propiedad_id),
            inquilino_id: Set(inquilino_id),
            fecha_inicio: Set(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            fecha_fin: Set(chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
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
        .expect("create contrato");
        id
    }

    async fn insert_pago(
        db: &DatabaseConnection,
        contrato_id: Uuid,
        org_id: Uuid,
        estado: &str,
        days_offset: i64,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        let fecha_vencimiento = Utc::now().date_naive() + chrono::Duration::days(days_offset);
        pago::ActiveModel {
            id: Set(id),
            contrato_id: Set(contrato_id),
            monto: Set(Decimal::new(25000, 0)),
            moneda: Set("DOP".to_string()),
            fecha_pago: Set(None),
            fecha_vencimiento: Set(fecha_vencimiento),
            metodo_pago: Set(None),
            estado: Set(estado.to_string()),
            notas: Set(None),
            recargo: Set(None),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("insert pago");
        id
    }

    async fn cleanup_pagos(db: &DatabaseConnection, ids: &[Uuid]) {
        for id in ids {
            let _ = pago::Entity::delete_by_id(*id).exec(db).await;
        }
    }

    async fn create_contrato_pbt(
        db: &DatabaseConnection,
        propiedad_id: Uuid,
        inquilino_id: Uuid,
        org_id: Uuid,
        estado: &str,
        fecha_fin: chrono::NaiveDate,
    ) -> Uuid {
        use realestate_backend::entities::contrato;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
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
        .expect("create contrato pbt");
        id
    }

    async fn cleanup_contratos(db: &DatabaseConnection, ids: &[Uuid]) {
        use realestate_backend::entities::contrato;
        for id in ids {
            let _ = contrato::Entity::delete_by_id(*id).exec(db).await;
        }
    }

    async fn create_usuario_pbt(db: &DatabaseConnection, org_id: Uuid) -> Uuid {
        use realestate_backend::entities::usuario;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        usuario::ActiveModel {
            id: Set(id),
            nombre: Set("PBT Usuario".to_string()),
            email: Set(format!("usr+{id}@pbt.com")),
            password_hash: Set("hash_placeholder".to_string()),
            rol: Set("admin".to_string()),
            activo: Set(true),
            organizacion_id: Set(org_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("create usuario pbt");
        id
    }

    async fn create_documento_pbt(
        db: &DatabaseConnection,
        uploaded_by: Uuid,
        estado_verificacion: &str,
        fecha_vencimiento: Option<chrono::NaiveDate>,
    ) -> Uuid {
        use realestate_backend::entities::documento;
        let id = Uuid::new_v4();
        let now = Utc::now().into();
        documento::ActiveModel {
            id: Set(id),
            entity_type: Set("propiedad".to_string()),
            entity_id: Set(Uuid::new_v4()),
            filename: Set(format!("pbt_doc_{id}.pdf")),
            file_path: Set(format!("/tmp/pbt/{id}.pdf")),
            mime_type: Set("application/pdf".to_string()),
            file_size: Set(1024),
            uploaded_by: Set(uploaded_by),
            tipo_documento: Set("cedula".to_string()),
            estado_verificacion: Set(estado_verificacion.to_string()),
            fecha_vencimiento: Set(fecha_vencimiento),
            verificado_por: Set(None),
            fecha_verificacion: Set(None),
            notas_verificacion: Set(None),
            numero_documento: Set(None),
            contenido_editable: Set(None),
            created_at: Set(now),
            updated_at: Set(None),
        }
        .insert(db)
        .await
        .expect("create documento pbt");
        id
    }

    async fn cleanup_documentos(db: &DatabaseConnection, ids: &[Uuid]) {
        use realestate_backend::entities::documento;
        for id in ids {
            let _ = documento::Entity::delete_by_id(*id).exec(db).await;
        }
    }

    // ── P1: Idempotencia de marcar pagos atrasados ─────────────────────
    pub fn p1(estados: Vec<String>, offsets: Vec<i64>) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;
            let contrato_id = create_contrato(&db, propiedad_id, inquilino_id, org_id).await;

            // Insert pagos with random estados and dates
            let mut pago_ids = Vec::new();
            for (estado, offset) in estados.iter().zip(offsets.iter()) {
                let id = insert_pago(&db, contrato_id, org_id, estado, *offset).await;
                pago_ids.push(id);
            }

            // First execution: may update some pagos
            let _first = pagos::mark_overdue(&db).await.expect("first mark_overdue");

            // Second execution: must return 0 (idempotent)
            let second = pagos::mark_overdue(&db).await.expect("second mark_overdue");
            assert_eq!(
                second, 0,
                "Second mark_overdue should return 0 affected rows, got {second}"
            );

            // Cleanup
            cleanup_pagos(&db, &pago_ids).await;
        });
    }
    // ── P2: Idempotencia de marcar contratos vencidos ─────────────────
    pub fn p2(estados: Vec<String>, offsets: Vec<i64>) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;

            // Insert contratos with random estados and fecha_fin
            let mut contrato_ids = Vec::new();
            for (estado, offset) in estados.iter().zip(offsets.iter()) {
                let fecha_fin = Utc::now().date_naive() + chrono::Duration::days(*offset);
                let id =
                    create_contrato_pbt(&db, propiedad_id, inquilino_id, org_id, estado, fecha_fin)
                        .await;
                contrato_ids.push(id);
            }

            // First execution: may update some contratos
            let _first = realestate_backend::services::contratos::marcar_vencidos(&db)
                .await
                .expect("first marcar_vencidos");

            // Second execution: must return 0 (idempotent)
            let second = realestate_backend::services::contratos::marcar_vencidos(&db)
                .await
                .expect("second marcar_vencidos");
            assert_eq!(
                second, 0,
                "Second marcar_vencidos should return 0 affected rows, got {second}"
            );

            // Cleanup
            cleanup_contratos(&db, &contrato_ids).await;
        });
    }

    // ── P4: Registro de ejecución completo ───────────────────────────
    pub fn p4(nombre_tarea: String) {
        with_db(|db| async move {
            let result = realestate_backend::services::background_jobs::ejecutar_tarea_por_nombre(
                &db,
                &nombre_tarea,
            )
            .await
            .expect("ejecutar_tarea_por_nombre should succeed");

            // id is not nil UUID
            assert_ne!(
                result.id,
                Uuid::nil(),
                "Execution record id should not be nil UUID"
            );
            // nombre_tarea matches the input
            assert_eq!(
                result.nombre_tarea, nombre_tarea,
                "nombre_tarea should match the input"
            );
            // duracion_ms >= 0
            assert!(
                result.duracion_ms >= 0,
                "duracion_ms should be >= 0, got {}",
                result.duracion_ms
            );
            // exitosa is true (task should succeed on a valid DB)
            assert!(
                result.exitosa,
                "exitosa should be true for a valid task execution"
            );
            // registros_afectados >= 0
            assert!(
                result.registros_afectados >= 0,
                "registros_afectados should be >= 0, got {}",
                result.registros_afectados
            );
        });
    }

    // ── P3: Idempotencia de marcar documentos vencidos ────────────────
    pub fn p3(estados: Vec<String>, offsets: Vec<i64>) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let uploaded_by = create_usuario_pbt(&db, org_id).await;

            // Insert documentos with random estados and fecha_vencimiento
            let mut doc_ids = Vec::new();
            for (estado, offset) in estados.iter().zip(offsets.iter()) {
                let fecha_vencimiento = Utc::now().date_naive() + chrono::Duration::days(*offset);
                let id =
                    create_documento_pbt(&db, uploaded_by, estado, Some(fecha_vencimiento)).await;
                doc_ids.push(id);
            }

            // First execution: may update some documentos
            let _first = realestate_backend::services::documentos::marcar_vencidos(&db)
                .await
                .expect("first marcar_vencidos docs");

            // Second execution: must return 0 (idempotent)
            let second = realestate_backend::services::documentos::marcar_vencidos(&db)
                .await
                .expect("second marcar_vencidos docs");
            assert_eq!(
                second, 0,
                "Second marcar_vencidos (docs) should return 0 affected rows, got {second}"
            );

            // Cleanup
            cleanup_documentos(&db, &doc_ids).await;
        });
    }
    // ── P5: Nombre de tarea inválido retorna 404 ──────────────────────
    pub fn p5(nombre_invalido: String) {
        with_db(|db| async move {
            let result = realestate_backend::services::background_jobs::ejecutar_tarea_por_nombre(
                &db,
                &nombre_invalido,
            )
            .await;

            assert!(
                result.is_err(),
                "Expected error for invalid task name '{nombre_invalido}', got Ok"
            );
            let err = result.unwrap_err();
            assert!(
                matches!(err, realestate_backend::errors::AppError::NotFound(_)),
                "Expected NotFound for invalid task name '{nombre_invalido}', got: {err:?}"
            );
        });
    }

    // ── P6: Historial ordenado por fecha descendente ──────────────────
    pub fn p6(tareas: Vec<String>) {
        with_db(|db| async move {
            use realestate_backend::models::background_jobs::HistorialQuery;

            // Execute each task to create execution records
            for nombre in &tareas {
                let _ = realestate_backend::services::background_jobs::ejecutar_tarea_por_nombre(
                    &db, nombre,
                )
                .await
                .expect("ejecutar_tarea_por_nombre should succeed");
            }

            // Query historial with no filters, large per_page to get all records
            let query = HistorialQuery {
                nombre_tarea: None,
                exitosa: None,
                page: Some(1),
                per_page: Some(100),
            };
            let result = realestate_backend::services::background_jobs::historial(&db, query)
                .await
                .expect("historial should succeed");

            // Verify descending order by iniciado_en
            let items = &result.data;
            for i in 0..items.len().saturating_sub(1) {
                assert!(
                    items[i].iniciado_en >= items[i + 1].iniciado_en,
                    "Historial not in descending order: items[{i}].iniciado_en ({}) < items[{}].iniciado_en ({})",
                    items[i].iniciado_en,
                    i + 1,
                    items[i + 1].iniciado_en,
                );
            }
        });
    }

    // ── P7: Filtrado del historial retorna solo registros coincidentes ─
    pub fn p7(tareas: Vec<String>, filtro_nombre: String) {
        with_db(|db| async move {
            use realestate_backend::models::background_jobs::HistorialQuery;

            // Execute each task to create varied execution records
            for nombre in &tareas {
                let _ = realestate_backend::services::background_jobs::ejecutar_tarea_por_nombre(
                    &db, nombre,
                )
                .await
                .expect("ejecutar_tarea_por_nombre should succeed");
            }

            // Filter by nombre_tarea and verify all returned records match
            let query_nombre = HistorialQuery {
                nombre_tarea: Some(filtro_nombre.clone()),
                exitosa: None,
                page: Some(1),
                per_page: Some(100),
            };
            let result_nombre =
                realestate_backend::services::background_jobs::historial(&db, query_nombre)
                    .await
                    .expect("historial filtered by nombre_tarea should succeed");

            for item in &result_nombre.data {
                assert_eq!(
                    item.nombre_tarea, filtro_nombre,
                    "Filtered by nombre_tarea='{}', but got record with nombre_tarea='{}'",
                    filtro_nombre, item.nombre_tarea,
                );
            }

            // Filter by exitosa=true and verify all returned records match
            let query_exitosa = HistorialQuery {
                nombre_tarea: None,
                exitosa: Some(true),
                page: Some(1),
                per_page: Some(100),
            };
            let result_exitosa =
                realestate_backend::services::background_jobs::historial(&db, query_exitosa)
                    .await
                    .expect("historial filtered by exitosa should succeed");

            for item in &result_exitosa.data {
                assert!(
                    item.exitosa,
                    "Filtered by exitosa=true, but got record with exitosa=false (nombre_tarea='{}')",
                    item.nombre_tarea,
                );
            }
        });
    }
    // ── P8: Post-condición de marcar pagos atrasados ─────────────────
    pub fn p8(estados: Vec<String>, offsets: Vec<i64>) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;
            let contrato_id = create_contrato(&db, propiedad_id, inquilino_id, org_id).await;

            // Insert pagos with random estados and dates
            let mut pago_ids = Vec::new();
            for (estado, offset) in estados.iter().zip(offsets.iter()) {
                let id = insert_pago(&db, contrato_id, org_id, estado, *offset).await;
                pago_ids.push(id);
            }

            // Execute mark_overdue
            let _ = pagos::mark_overdue(&db).await.expect("mark_overdue");

            // Post-condition: no test-created pago should have estado="pendiente"
            // AND fecha_vencimiento < today
            let today = Utc::now().date_naive();
            let remaining = pago::Entity::find()
                .filter(pago::Column::Id.is_in(pago_ids.clone()))
                .filter(pago::Column::Estado.eq("pendiente"))
                .filter(pago::Column::FechaVencimiento.lt(today))
                .all(&db)
                .await
                .expect("query pagos post-condition");

            assert!(
                remaining.is_empty(),
                "After mark_overdue, found {} pagos still pendiente with fecha_vencimiento < today: {:?}",
                remaining.len(),
                remaining
                    .iter()
                    .map(|p| (p.id, &p.estado, p.fecha_vencimiento))
                    .collect::<Vec<_>>(),
            );

            // Cleanup
            cleanup_pagos(&db, &pago_ids).await;
        });
    }

    // ── P9: Post-condición de marcar contratos vencidos ──────────────
    pub fn p9(estados: Vec<String>, offsets: Vec<i64>) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let propiedad_id = create_propiedad(&db, org_id).await;
            let inquilino_id = create_inquilino(&db, org_id).await;

            // Insert contratos with random estados and fecha_fin
            let mut contrato_ids = Vec::new();
            for (estado, offset) in estados.iter().zip(offsets.iter()) {
                let fecha_fin = Utc::now().date_naive() + chrono::Duration::days(*offset);
                let id =
                    create_contrato_pbt(&db, propiedad_id, inquilino_id, org_id, estado, fecha_fin)
                        .await;
                contrato_ids.push(id);
            }

            // Execute marcar_vencidos
            let _ = realestate_backend::services::contratos::marcar_vencidos(&db)
                .await
                .expect("marcar_vencidos");

            // Post-condition: no test-created contrato should have estado="activo"
            // AND fecha_fin < today
            use realestate_backend::entities::contrato;
            let today = Utc::now().date_naive();
            let remaining = contrato::Entity::find()
                .filter(contrato::Column::Id.is_in(contrato_ids.clone()))
                .filter(contrato::Column::Estado.eq("activo"))
                .filter(contrato::Column::FechaFin.lt(today))
                .all(&db)
                .await
                .expect("query contratos post-condition");

            assert!(
                remaining.is_empty(),
                "After marcar_vencidos, found {} contratos still activo with fecha_fin < today: {:?}",
                remaining.len(),
                remaining
                    .iter()
                    .map(|c| (c.id, &c.estado, c.fecha_fin))
                    .collect::<Vec<_>>(),
            );

            // Cleanup
            cleanup_contratos(&db, &contrato_ids).await;
        });
    }

    // ── P10: Post-condición de marcar documentos vencidos ─────────────
    pub fn p10(estados: Vec<String>, offsets: Vec<i64>) {
        with_db(|db| async move {
            let org_id = create_org(&db).await;
            let uploaded_by = create_usuario_pbt(&db, org_id).await;

            // Insert documentos with random estados and fecha_vencimiento
            let mut doc_ids = Vec::new();
            for (estado, offset) in estados.iter().zip(offsets.iter()) {
                let fecha_vencimiento = Utc::now().date_naive() + chrono::Duration::days(*offset);
                let id =
                    create_documento_pbt(&db, uploaded_by, estado, Some(fecha_vencimiento)).await;
                doc_ids.push(id);
            }

            // Execute marcar_vencidos
            let _ = realestate_backend::services::documentos::marcar_vencidos(&db)
                .await
                .expect("marcar_vencidos docs");

            // Post-condition: no test-created documento should have
            // estado_verificacion="verificado" AND fecha_vencimiento < today
            use realestate_backend::entities::documento;
            let today = Utc::now().date_naive();
            let remaining = documento::Entity::find()
                .filter(documento::Column::Id.is_in(doc_ids.clone()))
                .filter(documento::Column::EstadoVerificacion.eq("verificado"))
                .filter(documento::Column::FechaVencimiento.lt(today))
                .all(&db)
                .await
                .expect("query documentos post-condition");

            assert!(
                remaining.is_empty(),
                "After marcar_vencidos, found {} documentos still verificado with fecha_vencimiento < today: {:?}",
                remaining.len(),
                remaining
                    .iter()
                    .map(|d| (d.id, &d.estado_verificacion, d.fecha_vencimiento))
                    .collect::<Vec<_>>(),
            );

            // Cleanup
            cleanup_documentos(&db, &doc_ids).await;
        });
    }
} // end pbt_async

// ── Test functions ─────────────────────────────────────────────────────

// Feature: background-jobs, Property 1: Idempotencia de marcar pagos atrasados
// **Validates: Requirements 1.1, 1.4**
#[test]
fn test_idempotencia_marcar_pagos() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(
            &(
                proptest::collection::vec(pago_estado(), 1..6),
                proptest::collection::vec(days_offset(), 1..6),
            ),
            |(estados, offsets)| {
                // Ensure both vecs have the same length by truncating to the shorter
                let len = estados.len().min(offsets.len());
                let estados = estados[..len].to_vec();
                let offsets = offsets[..len].to_vec();
                pbt_async::p1(estados, offsets);
                Ok(())
            },
        )
        .unwrap();
}

// Feature: background-jobs, Property 2: Idempotencia de marcar contratos vencidos
// **Validates: Requirements 2.1, 2.4**
#[test]
fn test_idempotencia_marcar_contratos() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(
            &(
                proptest::collection::vec(contrato_estado(), 1..6),
                proptest::collection::vec(days_offset(), 1..6),
            ),
            |(estados, offsets)| {
                // Ensure both vecs have the same length by truncating to the shorter
                let len = estados.len().min(offsets.len());
                let estados = estados[..len].to_vec();
                let offsets = offsets[..len].to_vec();
                pbt_async::p2(estados, offsets);
                Ok(())
            },
        )
        .unwrap();
}

// Feature: background-jobs, Property 3: Idempotencia de marcar documentos vencidos
// **Validates: Requirements 3.1, 3.4**
#[test]
fn test_idempotencia_marcar_documentos() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(
            &(
                proptest::collection::vec(documento_estado_verificacion(), 1..6),
                proptest::collection::vec(days_offset(), 1..6),
            ),
            |(estados, offsets)| {
                // Ensure both vecs have the same length by truncating to the shorter
                let len = estados.len().min(offsets.len());
                let estados = estados[..len].to_vec();
                let offsets = offsets[..len].to_vec();
                pbt_async::p3(estados, offsets);
                Ok(())
            },
        )
        .unwrap();
}

// Feature: background-jobs, Property 4: Registro de ejecución completo
// **Validates: Requirements 5.1**
#[test]
fn test_registro_ejecucion_completo() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&tarea_valida(), |nombre_tarea| {
            pbt_async::p4(nombre_tarea);
            Ok(())
        })
        .unwrap();
}

// Feature: background-jobs, Property 5: Nombre de tarea inválido retorna 404
// **Validates: Requirements 7.2**
#[test]
fn test_nombre_tarea_invalido_404() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&tarea_invalida(), |nombre_invalido| {
            pbt_async::p5(nombre_invalido);
            Ok(())
        })
        .unwrap();
}

// Feature: background-jobs, Property 6: Historial ordenado por fecha descendente
// **Validates: Requirements 8.1**
#[test]
fn test_historial_ordenado() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(&proptest::collection::vec(tarea_valida(), 1..5), |tareas| {
            pbt_async::p6(tareas);
            Ok(())
        })
        .unwrap();
}

// Feature: background-jobs, Property 7: Filtrado del historial retorna solo registros coincidentes
// **Validates: Requirements 8.2, 8.3**
#[test]
fn test_filtrado_historial() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(
            &(
                proptest::collection::vec(tarea_valida(), 1..5),
                tarea_valida(),
            ),
            |(tareas, filtro_nombre)| {
                pbt_async::p7(tareas, filtro_nombre);
                Ok(())
            },
        )
        .unwrap();
}

// Feature: background-jobs, Property 8: Post-condición de marcar pagos atrasados
// **Validates: Requirements 1.1**
#[test]
fn test_postcondicion_pagos_atrasados() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(
            &(
                proptest::collection::vec(pago_estado(), 1..6),
                proptest::collection::vec(days_offset(), 1..6),
            ),
            |(estados, offsets)| {
                // Ensure both vecs have the same length by truncating to the shorter
                let len = estados.len().min(offsets.len());
                let estados = estados[..len].to_vec();
                let offsets = offsets[..len].to_vec();
                pbt_async::p8(estados, offsets);
                Ok(())
            },
        )
        .unwrap();
}

// Feature: background-jobs, Property 9: Post-condición de marcar contratos vencidos
// **Validates: Requirements 2.1**
#[test]
fn test_postcondicion_contratos_vencidos() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(
            &(
                proptest::collection::vec(contrato_estado(), 1..6),
                proptest::collection::vec(days_offset(), 1..6),
            ),
            |(estados, offsets)| {
                // Ensure both vecs have the same length by truncating to the shorter
                let len = estados.len().min(offsets.len());
                let estados = estados[..len].to_vec();
                let offsets = offsets[..len].to_vec();
                pbt_async::p9(estados, offsets);
                Ok(())
            },
        )
        .unwrap();
}

// Feature: background-jobs, Property 10: Post-condición de marcar documentos vencidos
// **Validates: Requirements 3.1**
#[test]
fn test_postcondicion_documentos_vencidos() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });
    runner
        .run(
            &(
                proptest::collection::vec(documento_estado_verificacion(), 1..6),
                proptest::collection::vec(days_offset(), 1..6),
            ),
            |(estados, offsets)| {
                // Ensure both vecs have the same length by truncating to the shorter
                let len = estados.len().min(offsets.len());
                let estados = estados[..len].to_vec();
                let offsets = offsets[..len].to_vec();
                pbt_async::p10(estados, offsets);
                Ok(())
            },
        )
        .unwrap();
}
