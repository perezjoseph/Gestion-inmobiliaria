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

const TOPE_LEGAL_PORCENTAJE: Decimal = Decimal::TEN;

const IPC_STALE_DAYS: i64 = 90;

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

    let ipc_data = ipc::obtener_ipc_actual(db, contrato_record.organizacion_id)
        .await?
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Datos de IPC no disponibles. Actualice manualmente"
            ))
        })?;

    let now = Utc::now();
    let days_since_fetch = (now - ipc_data.ultimo_fetch_exitoso).num_days();
    let datos_stale = days_since_fetch > IPC_STALE_DAYS;

    let ipc_porcentaje = ipc_data.valor_ipc;
    let monto_actual = contrato_record.monto_mensual;

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

    let max_allowed =
        original.monto_mensual * (Decimal::ONE + TOPE_LEGAL_PORCENTAJE / Decimal::from(100));

    if monto_aprobado > max_allowed {
        return Err(AppError::Validation(format!(
            "El monto aprobado ({monto_aprobado}) excede el máximo legal ({max_allowed})"
        )));
    }

    if monto_aprobado < original.monto_mensual {
        return Err(AppError::Validation(
            "El monto aprobado no puede ser menor al monto actual".to_string(),
        ));
    }

    let ipc_data = ipc::obtener_ipc_actual(db, original.organizacion_id).await?;
    let ipc_porcentaje = ipc_data.as_ref().map_or(Decimal::ZERO, |d| d.valor_ipc);

    let porcentaje_aplicado = if original.monto_mensual > Decimal::ZERO {
        ((monto_aprobado - original.monto_mensual) / original.monto_mensual) * Decimal::from(100)
    } else {
        Decimal::ZERO
    };

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

    let mut original_active: contrato::ActiveModel = original.clone().into();
    original_active.estado = Set("finalizado".to_string());
    original_active.updated_at = Set(Utc::now().into());
    original_active.update(&txn).await?;

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
