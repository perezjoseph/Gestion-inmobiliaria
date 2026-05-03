use std::panic::AssertUnwindSafe;
use std::time::{Duration, Instant};

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::{ejecucion_tarea, organizacion};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::background_jobs::{EjecucionTareaResponse, HistorialQuery};

pub const TAREAS_VALIDAS: &[&str] = &[
    "marcar_pagos_atrasados",
    "marcar_contratos_vencidos",
    "marcar_documentos_vencidos",
    "generar_notificaciones",
];

const INTERVALO_POR_DEFECTO_SECS: u64 = 86_400;

pub fn iniciar_scheduler(db: DatabaseConnection) {
    for &tarea in TAREAS_VALIDAS {
        let db = db.clone();
        let nombre = tarea.to_string();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(INTERVALO_POR_DEFECTO_SECS));
            // The first tick completes immediately — skip it so the first
            // execution happens after one full interval.
            interval.tick().await;
            loop {
                interval.tick().await;
                let db_ref = db.clone();
                let nombre_ref = nombre.clone();
                let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    Box::pin(ejecutar_tarea_con_registro(db_ref, nombre_ref))
                }));
                match result {
                    Ok(fut) => {
                        if let Err(e) = fut.await {
                            tracing::error!(tarea = %nombre, error = %e, "Error ejecutando tarea de fondo");
                        }
                    }
                    Err(_) => {
                        tracing::error!(tarea = %nombre, "Panic capturado en tarea de fondo");
                    }
                }
            }
        });
    }
}

pub async fn ejecutar_tarea_por_nombre(
    db: &DatabaseConnection,
    nombre: &str,
) -> Result<EjecucionTareaResponse, AppError> {
    if !TAREAS_VALIDAS.contains(&nombre) {
        return Err(AppError::NotFound(format!("Tarea no encontrada: {nombre}")));
    }
    ejecutar_tarea_con_registro(db.clone(), nombre.to_string()).await
}

pub async fn historial(
    db: &DatabaseConnection,
    query: HistorialQuery,
) -> Result<PaginatedResponse<EjecucionTareaResponse>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let mut select = ejecucion_tarea::Entity::find();

    if let Some(ref nombre_tarea) = query.nombre_tarea {
        select = select.filter(ejecucion_tarea::Column::NombreTarea.eq(nombre_tarea));
    }
    if let Some(exitosa) = query.exitosa {
        select = select.filter(ejecucion_tarea::Column::Exitosa.eq(exitosa));
    }

    let paginator = select
        .order_by_desc(ejecucion_tarea::Column::IniciadoEn)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records
            .into_iter()
            .map(EjecucionTareaResponse::from)
            .collect(),
        total,
        page,
        per_page,
    })
}

async fn ejecutar_tarea_con_registro(
    db: DatabaseConnection,
    nombre: String,
) -> Result<EjecucionTareaResponse, AppError> {
    let inicio = Instant::now();

    let resultado = match nombre.as_str() {
        "marcar_pagos_atrasados" => ejecutar_marcar_pagos_atrasados(&db).await,
        "marcar_contratos_vencidos" => ejecutar_marcar_contratos_vencidos(&db).await,
        "marcar_documentos_vencidos" => ejecutar_marcar_documentos_vencidos(&db).await,
        "generar_notificaciones" => ejecutar_generar_notificaciones(&db).await,
        _ => {
            return Err(AppError::NotFound(format!("Tarea no encontrada: {nombre}")));
        }
    };

    let duracion_ms = inicio.elapsed().as_millis() as i64;

    match resultado {
        Ok(registros_afectados) => {
            registrar_ejecucion(&db, &nombre, duracion_ms, true, registros_afectados, None).await
        }
        Err(e) => {
            let mensaje_error = e.to_string();
            registrar_ejecucion(&db, &nombre, duracion_ms, false, 0, Some(mensaje_error)).await
        }
    }
}

async fn ejecutar_marcar_pagos_atrasados(db: &DatabaseConnection) -> Result<i64, AppError> {
    let rows = super::pagos::mark_overdue(db).await?;
    Ok(i64::try_from(rows).unwrap_or(i64::MAX))
}

async fn ejecutar_marcar_contratos_vencidos(db: &DatabaseConnection) -> Result<i64, AppError> {
    let rows = super::contratos::marcar_vencidos(db).await?;
    Ok(i64::try_from(rows).unwrap_or(i64::MAX))
}

async fn ejecutar_marcar_documentos_vencidos(db: &DatabaseConnection) -> Result<i64, AppError> {
    let rows = super::documentos::marcar_vencidos(db).await?;
    Ok(i64::try_from(rows).unwrap_or(i64::MAX))
}

async fn ejecutar_generar_notificaciones(db: &DatabaseConnection) -> Result<i64, AppError> {
    let organizaciones = organizacion::Entity::find().all(db).await?;

    let mut total: i64 = 0;
    for org in &organizaciones {
        let resp = super::notificaciones::generar_notificaciones(db, org.id).await?;
        total += i64::try_from(resp.total).unwrap_or(i64::MAX);
    }

    Ok(total)
}

async fn registrar_ejecucion(
    db: &DatabaseConnection,
    nombre: &str,
    duracion_ms: i64,
    exitosa: bool,
    registros_afectados: i64,
    mensaje_error: Option<String>,
) -> Result<EjecucionTareaResponse, AppError> {
    let now = Utc::now().fixed_offset();

    let model = ejecucion_tarea::ActiveModel {
        id: Set(Uuid::new_v4()),
        nombre_tarea: Set(nombre.to_string()),
        iniciado_en: Set(now),
        duracion_ms: Set(duracion_ms),
        exitosa: Set(exitosa),
        registros_afectados: Set(registros_afectados),
        mensaje_error: Set(mensaje_error),
    };

    let record = model.insert(db).await?;
    Ok(EjecucionTareaResponse::from(record))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn tareas_validas_contains_expected_names() {
        assert_eq!(TAREAS_VALIDAS.len(), 4);
        assert!(TAREAS_VALIDAS.contains(&"marcar_pagos_atrasados"));
        assert!(TAREAS_VALIDAS.contains(&"marcar_contratos_vencidos"));
        assert!(TAREAS_VALIDAS.contains(&"marcar_documentos_vencidos"));
        assert!(TAREAS_VALIDAS.contains(&"generar_notificaciones"));
    }

    #[test]
    fn tareas_validas_rejects_invalid_names() {
        assert!(!TAREAS_VALIDAS.contains(&"invalid_task"));
        assert!(!TAREAS_VALIDAS.contains(&""));
        assert!(!TAREAS_VALIDAS.contains(&"MARCAR_PAGOS_ATRASADOS"));
    }

    #[test]
    fn intervalo_por_defecto_is_24_hours() {
        assert_eq!(INTERVALO_POR_DEFECTO_SECS, 86_400);
    }
}
