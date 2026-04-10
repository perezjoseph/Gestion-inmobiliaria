use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::{contrato, pago};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::pago::{CreatePagoRequest, PagoListQuery, PagoResponse, UpdatePagoRequest};
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::{
    recargos,
    validation::{METODOS_PAGO, MONEDAS, validate_enum},
};

const ESTADOS_PAGO: &[&str] = &["pendiente", "pagado", "atrasado", "cancelado"];

impl From<pago::Model> for PagoResponse {
    fn from(m: pago::Model) -> Self {
        Self {
            id: m.id,
            contrato_id: m.contrato_id,
            monto: m.monto,
            moneda: m.moneda,
            fecha_pago: m.fecha_pago,
            fecha_vencimiento: m.fecha_vencimiento,
            metodo_pago: m.metodo_pago,
            estado: m.estado,
            notas: m.notas,
            recargo: m.recargo,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    input: CreatePagoRequest,
    usuario_id: Uuid,
    organizacion_id: Uuid,
) -> Result<PagoResponse, AppError> {
    if let Some(ref moneda) = input.moneda {
        validate_enum("moneda", moneda, MONEDAS)?;
    }
    if let Some(ref metodo_pago) = input.metodo_pago {
        validate_enum("metodo_pago", metodo_pago, METODOS_PAGO)?;
    }

    contrato::Entity::find_by_id(input.contrato_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    let now = Utc::now().into();
    let id = Uuid::new_v4();

    let model = pago::ActiveModel {
        id: Set(id),
        contrato_id: Set(input.contrato_id),
        monto: Set(input.monto),
        moneda: Set(input.moneda.unwrap_or_else(|| "DOP".to_string())),
        fecha_pago: Set(input.fecha_pago),
        fecha_vencimiento: Set(input.fecha_vencimiento),
        metodo_pago: Set(input.metodo_pago),
        estado: Set("pendiente".to_string()),
        notas: Set(input.notas),
        recargo: Set(None),
        organizacion_id: Set(organizacion_id),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = model.insert(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "pago".to_string(),
            entity_id: id,
            accion: "crear".to_string(),
            cambios: serde_json::json!(PagoResponse::from(record.clone())),
        },
    )
    .await;

    Ok(PagoResponse::from(record))
}

pub async fn get_by_id(
    db: &DatabaseConnection,
    org_id: Uuid,
    id: Uuid,
) -> Result<PagoResponse, AppError> {
    let record = pago::Entity::find_by_id(id)
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Pago no encontrado".to_string()))?;
    Ok(PagoResponse::from(record))
}

pub async fn list(
    db: &DatabaseConnection,
    org_id: Uuid,
    query: PagoListQuery,
) -> Result<PaginatedResponse<PagoResponse>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let mut select = pago::Entity::find().filter(pago::Column::OrganizacionId.eq(org_id));

    if let Some(ref contrato_id) = query.contrato_id {
        select = select.filter(pago::Column::ContratoId.eq(*contrato_id));
    }
    if let Some(ref estado) = query.estado {
        select = select.filter(pago::Column::Estado.eq(estado));
    }
    if let Some(fecha_desde) = query.fecha_desde {
        select = select.filter(pago::Column::FechaVencimiento.gte(fecha_desde));
    }
    if let Some(fecha_hasta) = query.fecha_hasta {
        select = select.filter(pago::Column::FechaVencimiento.lte(fecha_hasta));
    }

    let paginator = select
        .order_by_desc(pago::Column::FechaVencimiento)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records.into_iter().map(PagoResponse::from).collect(),
        total,
        page,
        per_page,
    })
}

pub async fn update<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    id: Uuid,
    input: UpdatePagoRequest,
    usuario_id: Uuid,
) -> Result<PagoResponse, AppError> {
    if let Some(ref estado) = input.estado {
        validate_enum("estado", estado, ESTADOS_PAGO)?;
    }
    if let Some(ref metodo_pago) = input.metodo_pago {
        validate_enum("metodo_pago", metodo_pago, METODOS_PAGO)?;
    }

    let existing = pago::Entity::find_by_id(id)
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Pago no encontrado".to_string()))?;

    let old_estado = existing.estado.clone();
    let contrato_id = existing.contrato_id;
    let pago_id = existing.id;

    let mut active: pago::ActiveModel = existing.into();

    if let Some(monto) = input.monto {
        active.monto = Set(monto);
    }
    if let Some(fecha_pago) = input.fecha_pago {
        active.fecha_pago = Set(Some(fecha_pago));
    }
    if let Some(metodo_pago) = input.metodo_pago {
        active.metodo_pago = Set(Some(metodo_pago));
    }
    if let Some(estado) = input.estado {
        active.estado = Set(estado);
    }
    if let Some(notas) = input.notas {
        active.notas = Set(Some(notas));
    }

    active.updated_at = Set(Utc::now().into());

    let mut updated = active.update(db).await?;

    // Recargo logic based on estado transitions
    let new_estado = &updated.estado;
    if new_estado == "atrasado" && old_estado != "atrasado" {
        // Transitioning to atrasado: calculate and apply recargo
        if let Some(contrato_model) = contrato::Entity::find_by_id(contrato_id).one(db).await? {
            recargos::aplicar_recargo(db, pago_id, &contrato_model).await?;
            // Re-fetch to get the updated recargo value
            updated = pago::Entity::find_by_id(pago_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Pago no encontrado".to_string()))?;
        }
    } else if old_estado == "atrasado" && new_estado != "atrasado" {
        // Transitioning from atrasado: clear recargo
        let mut clear_active: pago::ActiveModel = updated.into();
        clear_active.recargo = Set(None);
        updated = clear_active.update(db).await?;
    }

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "pago".to_string(),
            entity_id: id,
            accion: "actualizar".to_string(),
            cambios: serde_json::json!(PagoResponse::from(updated.clone())),
        },
    )
    .await;

    Ok(PagoResponse::from(updated))
}

pub async fn delete<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    id: Uuid,
    usuario_id: Uuid,
) -> Result<(), AppError> {
    let existing = pago::Entity::find_by_id(id)
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Pago no encontrado".to_string()))?;

    let active: pago::ActiveModel = existing.into();
    active.delete(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "pago".to_string(),
            entity_id: id,
            accion: "eliminar".to_string(),
            cambios: serde_json::json!({ "id": id }),
        },
    )
    .await;

    Ok(())
}

pub async fn mark_overdue(db: &DatabaseConnection) -> Result<u64, AppError> {
    let today = Utc::now().date_naive();

    let pending_pagos: Vec<(pago::Model, Option<contrato::Model>)> = pago::Entity::find()
        .filter(pago::Column::Estado.eq("pendiente"))
        .find_also_related(contrato::Entity)
        .all(db)
        .await?;

    let mut overdue_ids: Vec<Uuid> = Vec::new();
    let mut recargo_candidates: Vec<(Uuid, contrato::Model)> = Vec::new();

    for (pago_record, contrato_opt) in &pending_pagos {
        let dias_gracia = contrato_opt
            .as_ref()
            .and_then(|c| c.dias_gracia)
            .unwrap_or(0);

        let effective_due =
            pago_record.fecha_vencimiento + chrono::Duration::days(i64::from(dias_gracia));

        if today <= effective_due {
            continue;
        }

        overdue_ids.push(pago_record.id);

        if let Some(contrato_model) = contrato_opt {
            recargo_candidates.push((pago_record.id, contrato_model.clone()));
        }
    }

    if overdue_ids.is_empty() {
        return Ok(0);
    }

    let result = pago::Entity::update_many()
        .col_expr(
            pago::Column::Estado,
            sea_orm::sea_query::Expr::value("atrasado"),
        )
        .col_expr(
            pago::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(Utc::now().fixed_offset()),
        )
        .filter(pago::Column::Id.is_in(overdue_ids))
        .exec(db)
        .await?;

    let affected_count = result.rows_affected;

    let mut recargos_calculated: u64 = 0;
    for (pago_id, contrato_model) in &recargo_candidates {
        if (recargos::aplicar_recargo(db, *pago_id, contrato_model).await?).is_some() {
            recargos_calculated += 1;
        }
    }

    if affected_count > 0 {
        auditoria::registrar_best_effort(
            db,
            CreateAuditoriaEntry {
                usuario_id: Uuid::nil(),
                entity_type: "pago".to_string(),
                entity_id: Uuid::nil(),
                accion: "mark_overdue".to_string(),
                cambios: serde_json::json!({
                    "pagos_afectados": affected_count,
                    "recargos_calculados": recargos_calculated,
                }),
            },
        )
        .await;
    }

    Ok(affected_count)
}
