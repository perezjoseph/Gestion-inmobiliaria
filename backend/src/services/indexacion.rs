use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::entities::{contrato, inquilino, propiedad};
use crate::errors::AppError;
use crate::models::indexacion::{ContratoProximoVencer, PropuestaRenovacion};
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::ipc;

/// Maximum annual rent increase percentage allowed by Ley 85-25.
const TOPE_LEGAL_PORCENTAJE: Decimal = Decimal::TEN;

/// IPC data is considered stale if older than this many days.
const IPC_STALE_DAYS: i64 = 90;

/// Calculate a renewal proposal for a given contract based on IPC and Ley 85-25 cap.
///
/// Formula: `monto_actual * (1 + min(ipc_interanual, 10) / 100)`
/// Result never exceeds `monto_actual * 1.10` regardless of IPC value.
/// If IPC cache is > 90 days old, uses cached value but sets `datos_stale = true`.
pub async fn calcular_propuesta_renovacion(
    db: &DatabaseConnection,
    contrato_id: Uuid,
) -> Result<PropuestaRenovacion, AppError> {
    let contrato_record = contrato::Entity::find_by_id(contrato_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    if contrato_record.estado != "activo" {
        return Err(AppError::Validation(
            "Solo se puede calcular propuesta para contratos activos".to_string(),
        ));
    }

    let ipc_data = ipc::obtener_ipc_actual(db).await?.ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "Datos de IPC no disponibles. Actualice manualmente"
        ))
    })?;

    // Determine if IPC data is stale (> 90 days old)
    let now = Utc::now();
    let days_since_fetch = (now - ipc_data.ultimo_fetch_exitoso).num_days();
    let datos_stale = days_since_fetch > IPC_STALE_DAYS;

    let ipc_porcentaje = ipc_data.valor_ipc;
    let monto_actual = contrato_record.monto_mensual;

    // Apply formula: monto * (1 + min(ipc, 10%) / 100)
    let porcentaje_aplicable = if ipc_porcentaje > TOPE_LEGAL_PORCENTAJE {
        TOPE_LEGAL_PORCENTAJE
    } else {
        ipc_porcentaje
    };
    let tope_aplicado = ipc_porcentaje > TOPE_LEGAL_PORCENTAJE;

    let monto_maximo = monto_actual * (Decimal::ONE + porcentaje_aplicable / Decimal::from(100));

    Ok(PropuestaRenovacion {
        contrato_id,
        monto_actual,
        monto_maximo,
        ipc_porcentaje,
        tope_aplicado,
        datos_stale,
    })
}

/// Approve a renewal: verify calculation integrity, create renewed contrato, record audit trail.
///
/// The approved amount must not exceed `monto_actual * 1.10` (absolute legal cap).
/// Sets old contrato to "finalizado" and creates a new one with `estado="activo"`.
/// Supports custom escalation clause overrides (lower-than-IPC increases).
pub async fn aprobar_renovacion(
    db: &DatabaseConnection,
    contrato_id: Uuid,
    monto_aprobado: Decimal,
    admin_id: Uuid,
) -> Result<contrato::Model, AppError> {
    let txn = db.begin().await?;

    let original = contrato::Entity::find_by_id(contrato_id)
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    if original.estado != "activo" {
        return Err(AppError::Validation(
            "Solo se pueden renovar contratos activos".to_string(),
        ));
    }

    // Verify calculation integrity: approved amount must not exceed 10% legal cap
    let max_allowed =
        original.monto_mensual * (Decimal::ONE + TOPE_LEGAL_PORCENTAJE / Decimal::from(100));

    if monto_aprobado > max_allowed {
        return Err(AppError::Validation(format!(
            "El monto aprobado ({monto_aprobado}) excede el máximo legal ({max_allowed})"
        )));
    }

    // Approved amount must be at least the current amount (no decreases via this endpoint)
    if monto_aprobado < original.monto_mensual {
        return Err(AppError::Validation(
            "El monto aprobado no puede ser menor al monto actual".to_string(),
        ));
    }

    // Fetch IPC for audit trail
    let ipc_data = ipc::obtener_ipc_actual(db).await?;
    let ipc_porcentaje = ipc_data.as_ref().map_or(Decimal::ZERO, |d| d.valor_ipc);

    // Calculate the actual percentage applied
    let porcentaje_aplicado = if original.monto_mensual > Decimal::ZERO {
        ((monto_aprobado - original.monto_mensual) / original.monto_mensual) * Decimal::from(100)
    } else {
        Decimal::ZERO
    };

    // Create renewed contrato: new period starts the day after old one ends,
    // same duration as original contract (anniversary-based indexation).
    let duracion_dias = (original.fecha_fin - original.fecha_inicio).num_days();
    let new_fecha_inicio = original
        .fecha_fin
        .succ_opt()
        .ok_or_else(|| AppError::Validation("Fecha de fin inválida".to_string()))?;
    let new_fecha_fin = new_fecha_inicio + Duration::days(duracion_dias);

    let now = Utc::now().into();
    let new_id = Uuid::new_v4();

    let new_contrato = contrato::ActiveModel {
        id: Set(new_id),
        propiedad_id: Set(original.propiedad_id),
        inquilino_id: Set(original.inquilino_id),
        fecha_inicio: Set(new_fecha_inicio),
        fecha_fin: Set(new_fecha_fin),
        monto_mensual: Set(monto_aprobado),
        deposito: Set(original.deposito),
        moneda: Set(original.moneda.clone()),
        estado: Set("activo".to_string()),
        documentos: Set(None),
        organizacion_id: Set(original.organizacion_id),
        created_at: Set(now),
        updated_at: Set(now),
        estado_deposito: Set(None),
        fecha_cobro_deposito: Set(None),
        fecha_devolucion_deposito: Set(None),
        monto_retenido: Set(None),
        motivo_retencion: Set(None),
        recargo_porcentaje: Set(original.recargo_porcentaje),
        dias_gracia: Set(original.dias_gracia),
    };

    let new_record = new_contrato.insert(&txn).await?;

    // Set the original contrato to "finalizado"
    let mut original_active: contrato::ActiveModel = original.clone().into();
    original_active.estado = Set("finalizado".to_string());
    original_active.updated_at = Set(Utc::now().into());
    original_active.update(&txn).await?;

    // Record audit trail with all required information per Ley 85-25
    auditoria::registrar_best_effort(
        &txn,
        CreateAuditoriaEntry {
            usuario_id: admin_id,
            entity_type: "contrato".to_string(),
            entity_id: new_id,
            accion: "renovacion_indexacion".to_string(),
            cambios: json!({
                "contrato_original_id": contrato_id,
                "ipc_porcentaje": ipc_porcentaje.to_string(),
                "tope_legal": "10%",
                "porcentaje_aplicado": porcentaje_aplicado.to_string(),
                "monto_anterior": original.monto_mensual.to_string(),
                "monto_nuevo": monto_aprobado.to_string(),
            }),
        },
    )
    .await;

    txn.commit().await?;

    Ok(new_record)
}

/// Find active contracts expiring within the given number of days.
///
/// Returns contracts where `fecha_fin - today <= dias` AND `estado = "activo"`.
/// Includes property title and tenant name for display purposes.
pub async fn contratos_proximos_vencer(
    db: &DatabaseConnection,
    org_id: Uuid,
    dias: i32,
) -> Result<Vec<ContratoProximoVencer>, AppError> {
    let today = Utc::now().date_naive();
    let cutoff = today + Duration::days(i64::from(dias));

    let records = contrato::Entity::find()
        .filter(
            Condition::all()
                .add(contrato::Column::OrganizacionId.eq(org_id))
                .add(contrato::Column::Estado.eq("activo"))
                .add(contrato::Column::FechaFin.gte(today))
                .add(contrato::Column::FechaFin.lte(cutoff)),
        )
        .order_by_asc(contrato::Column::FechaFin)
        .all(db)
        .await?;

    let mut result = Vec::with_capacity(records.len());

    for record in &records {
        let prop = propiedad::Entity::find_by_id(record.propiedad_id)
            .one(db)
            .await?;
        let inq = inquilino::Entity::find_by_id(record.inquilino_id)
            .one(db)
            .await?;

        let propiedad_titulo =
            prop.map_or_else(|| "Propiedad desconocida".to_string(), |p| p.titulo);
        let inquilino_nombre = inq.map_or_else(
            || "Inquilino desconocido".to_string(),
            |i| format!("{} {}", i.nombre, i.apellido),
        );

        let dias_restantes = (record.fecha_fin - today).num_days() as i32;

        result.push(ContratoProximoVencer {
            contrato_id: record.id,
            propiedad_titulo,
            inquilino_nombre,
            fecha_fin: record.fecha_fin,
            monto_actual: record.monto_mensual,
            moneda: record.moneda.clone(),
            dias_restantes,
        });
    }

    Ok(result)
}
