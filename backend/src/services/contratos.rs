use chrono::{Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use uuid::Uuid;

use crate::entities::{contrato, inquilino, pago, propiedad};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::contrato::{
    CambiarEstadoDepositoRequest, ContratoResponse, CreateContratoRequest, RenovarContratoRequest,
    TerminarContratoRequest, UpdateContratoRequest,
};
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::pago_generacion::{PagoGenerado, calcular_pagos, validar_dia_vencimiento};
use crate::services::validation::{MONEDAS, validate_enum};

const ESTADOS_CONTRATO: &[&str] = &["activo", "vencido", "cancelado"];

fn validar_recargo_y_gracia(
    recargo_porcentaje: Option<Decimal>,
    dias_gracia: Option<i32>,
) -> Result<(), AppError> {
    if let Some(p) = recargo_porcentaje {
        if p < Decimal::ZERO || p > Decimal::from(100) {
            return Err(AppError::Validation(
                "El porcentaje de recargo debe estar entre 0 y 100".to_string(),
            ));
        }
    }
    if let Some(d) = dias_gracia {
        if d < 0 {
            return Err(AppError::Validation(
                "Los días de gracia deben ser mayor o igual a 0".to_string(),
            ));
        }
    }
    Ok(())
}

fn validar_retencion(
    monto_retenido: Option<Decimal>,
    motivo_retencion: Option<&str>,
    deposito: Decimal,
) -> Result<(), AppError> {
    let monto = monto_retenido.ok_or_else(|| {
        AppError::Validation("El monto retenido es requerido para retención".to_string())
    })?;

    if monto <= Decimal::ZERO {
        return Err(AppError::Validation(
            "El monto retenido debe ser mayor a cero".to_string(),
        ));
    }

    if monto > deposito {
        return Err(AppError::Validation(
            "El monto retenido no puede exceder el depósito".to_string(),
        ));
    }

    let motivo = motivo_retencion.unwrap_or("");
    if motivo.trim().is_empty() {
        return Err(AppError::Validation(
            "El motivo de retención es requerido".to_string(),
        ));
    }

    Ok(())
}

pub const ESTADOS_DEPOSITO: &[&str] = &["pendiente", "cobrado", "devuelto", "retenido"];

/// Pure validation of deposit state transitions.
///
/// Valid transitions:
///   pendiente → cobrado
///   cobrado   → devuelto
///   cobrado   → retenido
pub fn validar_transicion_deposito(
    estado_actual: &str,
    nuevo_estado: &str,
) -> Result<(), AppError> {
    match (estado_actual, nuevo_estado) {
        // Valid transitions
        ("pendiente", "cobrado") | ("cobrado", "devuelto" | "retenido") => Ok(()),

        // pendiente cannot skip to devuelto or retenido
        ("pendiente", "devuelto" | "retenido") => Err(AppError::Validation(
            "El depósito debe ser cobrado antes de ser devuelto o retenido".to_string(),
        )),

        // Terminal states cannot transition
        ("devuelto" | "retenido", _) => Err(AppError::Validation(
            "Los depósitos devueltos o retenidos no pueden cambiar de estado".to_string(),
        )),

        // cobrado cannot revert to pendiente
        ("cobrado", "pendiente") => Err(AppError::Validation(
            "No se puede revertir un depósito cobrado a pendiente".to_string(),
        )),

        // Any other combination (e.g. same state, unknown states)
        _ => Err(AppError::Validation(format!(
            "Transición de estado de depósito no válida: {estado_actual} → {nuevo_estado}"
        ))),
    }
}

impl From<contrato::Model> for ContratoResponse {
    fn from(m: contrato::Model) -> Self {
        Self {
            id: m.id,
            propiedad_id: m.propiedad_id,
            inquilino_id: m.inquilino_id,
            fecha_inicio: m.fecha_inicio,
            fecha_fin: m.fecha_fin,
            monto_mensual: m.monto_mensual,
            deposito: m.deposito,
            moneda: m.moneda,
            estado: m.estado,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
            pagos_generados: None,
            estado_deposito: m.estado_deposito,
            fecha_cobro_deposito: m.fecha_cobro_deposito.map(|dt| dt.with_timezone(&Utc)),
            fecha_devolucion_deposito: m.fecha_devolucion_deposito.map(|dt| dt.with_timezone(&Utc)),
            monto_retenido: m.monto_retenido,
            motivo_retencion: m.motivo_retencion,
            recargo_porcentaje: m.recargo_porcentaje,
            dias_gracia: m.dias_gracia,
        }
    }
}

async fn validate_no_overlap<C: ConnectionTrait>(
    db: &C,
    propiedad_id: Uuid,
    fecha_inicio: chrono::NaiveDate,
    fecha_fin: chrono::NaiveDate,
    exclude_id: Option<Uuid>,
) -> Result<(), AppError> {
    let mut condition = Condition::all()
        .add(contrato::Column::PropiedadId.eq(propiedad_id))
        .add(contrato::Column::Estado.eq("activo"))
        .add(contrato::Column::FechaInicio.lt(fecha_fin))
        .add(contrato::Column::FechaFin.gt(fecha_inicio));

    if let Some(id) = exclude_id {
        condition = condition.add(contrato::Column::Id.ne(id));
    }

    let overlapping = contrato::Entity::find().filter(condition).one(db).await?;

    if overlapping.is_some() {
        return Err(AppError::Conflict(
            "Ya existe un contrato activo para esta propiedad en el rango de fechas indicado"
                .to_string(),
        ));
    }

    Ok(())
}

pub(crate) async fn insertar_pagos_generados<C: ConnectionTrait>(
    db: &C,
    contrato_id: Uuid,
    organizacion_id: Uuid,
    pagos: &[PagoGenerado],
) -> Result<usize, AppError> {
    if pagos.is_empty() {
        return Ok(0);
    }

    let now = Utc::now().into();
    let models: Vec<pago::ActiveModel> = pagos
        .iter()
        .map(|p| pago::ActiveModel {
            id: Set(Uuid::new_v4()),
            contrato_id: Set(contrato_id),
            monto: Set(p.monto),
            moneda: Set(p.moneda.clone()),
            fecha_pago: Set(None),
            fecha_vencimiento: Set(p.fecha_vencimiento),
            metodo_pago: Set(None),
            estado: Set("pendiente".to_string()),
            notas: Set(None),
            recargo: Set(None),
            organizacion_id: Set(organizacion_id),
            created_at: Set(now),
            updated_at: Set(now),
        })
        .collect();

    let count = models.len();
    pago::Entity::insert_many(models).exec(db).await?;
    Ok(count)
}

struct PagoGeneracionParams<'a> {
    contrato_id: Uuid,
    organizacion_id: Uuid,
    fecha_inicio: NaiveDate,
    fecha_fin: NaiveDate,
    monto_mensual: Decimal,
    moneda: &'a str,
    dia_vencimiento: u32,
    usuario_id: Uuid,
}

async fn generar_pagos_para_contrato<C: ConnectionTrait>(
    db: &C,
    params: &PagoGeneracionParams<'_>,
) -> Result<usize, AppError> {
    let pagos = calcular_pagos(
        params.fecha_inicio,
        params.fecha_fin,
        params.monto_mensual,
        params.moneda,
        params.dia_vencimiento,
    );
    let count =
        insertar_pagos_generados(db, params.contrato_id, params.organizacion_id, &pagos).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id: params.usuario_id,
            entity_type: "contrato".to_string(),
            entity_id: params.contrato_id,
            accion: "generar_pagos_auto".to_string(),
            cambios: serde_json::json!({
                "pagos_generados": count,
            }),
        },
    )
    .await;

    Ok(count)
}

async fn cancelar_pagos_futuros<C: ConnectionTrait>(
    db: &C,
    contrato_id: Uuid,
    fecha_terminacion: NaiveDate,
) -> Result<usize, AppError> {
    let result = pago::Entity::update_many()
        .col_expr(
            pago::Column::Estado,
            sea_orm::sea_query::Expr::value("cancelado"),
        )
        .col_expr(
            pago::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(Utc::now().fixed_offset()),
        )
        .filter(
            Condition::all()
                .add(pago::Column::ContratoId.eq(contrato_id))
                .add(pago::Column::Estado.eq("pendiente"))
                .add(pago::Column::FechaVencimiento.gt(fecha_terminacion)),
        )
        .exec(db)
        .await?;

    Ok(result.rows_affected as usize)
}

pub async fn create(
    db: &DatabaseConnection,
    input: CreateContratoRequest,
    usuario_id: Uuid,
    organizacion_id: Uuid,
) -> Result<ContratoResponse, AppError> {
    if input.fecha_inicio >= input.fecha_fin {
        return Err(AppError::Validation(
            "La fecha de fin debe ser posterior a la fecha de inicio".to_string(),
        ));
    }

    validar_recargo_y_gracia(input.recargo_porcentaje, input.dias_gracia)?;

    let dia_vencimiento = input.dia_vencimiento.unwrap_or(1);
    if input.dia_vencimiento.is_some() {
        validar_dia_vencimiento(dia_vencimiento)?;
    }

    if let Some(ref moneda) = input.moneda {
        validate_enum("moneda", moneda, MONEDAS)?;
    }

    inquilino::Entity::find_by_id(input.inquilino_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Inquilino no encontrado".to_string()))?;

    let txn = db.begin().await?;

    // Single propiedad lookup — reused for both validation and status update
    let prop = propiedad::Entity::find_by_id(input.propiedad_id)
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    validate_no_overlap(
        &txn,
        input.propiedad_id,
        input.fecha_inicio,
        input.fecha_fin,
        None,
    )
    .await?;

    let now = Utc::now().into();
    let id = Uuid::new_v4();

    let estado_deposito = match input.deposito {
        Some(d) if d > rust_decimal::Decimal::ZERO => Some("pendiente".to_string()),
        _ => None,
    };

    let model = contrato::ActiveModel {
        id: Set(id),
        propiedad_id: Set(input.propiedad_id),
        inquilino_id: Set(input.inquilino_id),
        fecha_inicio: Set(input.fecha_inicio),
        fecha_fin: Set(input.fecha_fin),
        monto_mensual: Set(input.monto_mensual),
        deposito: Set(input.deposito),
        moneda: Set(input.moneda.unwrap_or_else(|| "DOP".to_string())),
        estado: Set("activo".to_string()),
        documentos: Set(None),
        organizacion_id: Set(organizacion_id),
        created_at: Set(now),
        updated_at: Set(now),
        estado_deposito: Set(estado_deposito),
        fecha_cobro_deposito: Set(None),
        fecha_devolucion_deposito: Set(None),
        monto_retenido: Set(None),
        motivo_retencion: Set(None),
        recargo_porcentaje: Set(input.recargo_porcentaje),
        dias_gracia: Set(input.dias_gracia),
    };

    let record = model.insert(&txn).await?;

    let mut prop_active: propiedad::ActiveModel = prop.into();
    prop_active.estado = Set("ocupada".to_string());
    prop_active.updated_at = Set(Utc::now().into());
    prop_active.update(&txn).await?;

    auditoria::registrar_best_effort(
        &txn,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "contrato".to_string(),
            entity_id: id,
            accion: "crear".to_string(),
            cambios: serde_json::json!(ContratoResponse::from(record.clone())),
        },
    )
    .await;

    let pagos_count = if record.estado == "activo" {
        let count = generar_pagos_para_contrato(
            &txn,
            &PagoGeneracionParams {
                contrato_id: id,
                organizacion_id,
                fecha_inicio: record.fecha_inicio,
                fecha_fin: record.fecha_fin,
                monto_mensual: record.monto_mensual,
                moneda: &record.moneda,
                dia_vencimiento,
                usuario_id,
            },
        )
        .await?;
        Some(count)
    } else {
        None
    };

    txn.commit().await?;

    let mut response = ContratoResponse::from(record);
    response.pagos_generados = pagos_count;
    Ok(response)
}

pub async fn get_by_id(
    db: &DatabaseConnection,
    org_id: Uuid,
    id: Uuid,
) -> Result<ContratoResponse, AppError> {
    let record = contrato::Entity::find_by_id(id)
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;
    Ok(ContratoResponse::from(record))
}

pub async fn list(
    db: &DatabaseConnection,
    org_id: Uuid,
    page: Option<u64>,
    per_page: Option<u64>,
) -> Result<PaginatedResponse<ContratoResponse>, AppError> {
    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(20).clamp(1, 100);

    let paginator = contrato::Entity::find()
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .order_by_desc(contrato::Column::CreatedAt)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records.into_iter().map(ContratoResponse::from).collect(),
        total,
        page,
        per_page,
    })
}

pub async fn update(
    db: &DatabaseConnection,
    org_id: Uuid,
    id: Uuid,
    input: UpdateContratoRequest,
    usuario_id: Uuid,
) -> Result<ContratoResponse, AppError> {
    if let Some(ref estado) = input.estado {
        validate_enum("estado", estado, ESTADOS_CONTRATO)?;
    }

    validar_recargo_y_gracia(input.recargo_porcentaje, input.dias_gracia)?;

    let existing = contrato::Entity::find_by_id(id)
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    let is_terminating = input
        .estado
        .as_ref()
        .is_some_and(|e| e == "terminado" || e == "vencido");

    let propiedad_id = existing.propiedad_id;
    let fecha_inicio = existing.fecha_inicio;

    let txn = db.begin().await?;

    let mut active: contrato::ActiveModel = existing.into();

    if let Some(fecha_fin) = input.fecha_fin {
        active.fecha_fin = Set(fecha_fin);
        if !is_terminating {
            validate_no_overlap(&txn, propiedad_id, fecha_inicio, fecha_fin, Some(id)).await?;
        }
    }
    if let Some(monto_mensual) = input.monto_mensual {
        active.monto_mensual = Set(monto_mensual);
    }
    if let Some(deposito) = input.deposito {
        active.deposito = Set(Some(deposito));
    }
    if let Some(ref estado) = input.estado {
        active.estado = Set(estado.clone());
    }
    if let Some(recargo_porcentaje) = input.recargo_porcentaje {
        active.recargo_porcentaje = Set(Some(recargo_porcentaje));
    }
    if let Some(dias_gracia) = input.dias_gracia {
        active.dias_gracia = Set(Some(dias_gracia));
    }

    active.updated_at = Set(Utc::now().into());

    let updated = active.update(&txn).await?;

    if is_terminating {
        let mut prop_active: propiedad::ActiveModel = propiedad::Entity::find_by_id(propiedad_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?
            .into();
        prop_active.estado = Set("disponible".to_string());
        prop_active.updated_at = Set(Utc::now().into());
        prop_active.update(&txn).await?;
    }

    auditoria::registrar_best_effort(
        &txn,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "contrato".to_string(),
            entity_id: id,
            accion: "actualizar".to_string(),
            cambios: serde_json::json!(ContratoResponse::from(updated.clone())),
        },
    )
    .await;

    txn.commit().await?;

    Ok(ContratoResponse::from(updated))
}

pub async fn delete(
    db: &DatabaseConnection,
    org_id: Uuid,
    id: Uuid,
    usuario_id: Uuid,
) -> Result<(), AppError> {
    let txn = db.begin().await?;

    let existing = contrato::Entity::find_by_id(id)
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    let active: contrato::ActiveModel = existing.into();
    active.delete(&txn).await?;

    auditoria::registrar_best_effort(
        &txn,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "contrato".to_string(),
            entity_id: id,
            accion: "eliminar".to_string(),
            cambios: serde_json::json!({ "id": id }),
        },
    )
    .await;

    txn.commit().await?;

    Ok(())
}

pub async fn renovar(
    db: &DatabaseConnection,
    org_id: Uuid,
    contrato_id: Uuid,
    input: RenovarContratoRequest,
    usuario_id: Uuid,
) -> Result<ContratoResponse, AppError> {
    let dia_vencimiento = input.dia_vencimiento.unwrap_or(1);
    if input.dia_vencimiento.is_some() {
        validar_dia_vencimiento(dia_vencimiento)?;
    }

    let txn = db.begin().await?;

    let original = contrato::Entity::find_by_id(contrato_id)
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    if original.estado != "activo" {
        return Err(AppError::Validation(
            "Solo se pueden renovar contratos activos".to_string(),
        ));
    }

    let new_fecha_inicio = original
        .fecha_fin
        .succ_opt()
        .ok_or_else(|| AppError::Validation("Fecha de fin inválida".to_string()))?;

    validate_no_overlap(
        &txn,
        original.propiedad_id,
        new_fecha_inicio,
        input.fecha_fin,
        None,
    )
    .await?;

    let now = Utc::now().into();
    let new_id = Uuid::new_v4();

    let new_contrato = contrato::ActiveModel {
        id: Set(new_id),
        propiedad_id: Set(original.propiedad_id),
        inquilino_id: Set(original.inquilino_id),
        fecha_inicio: Set(new_fecha_inicio),
        fecha_fin: Set(input.fecha_fin),
        monto_mensual: Set(input.monto_mensual),
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

    let mut original_active: contrato::ActiveModel = original.into();
    original_active.estado = Set("finalizado".to_string());
    original_active.updated_at = Set(Utc::now().into());
    original_active.update(&txn).await?;

    auditoria::registrar_best_effort(
        &txn,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "contrato".to_string(),
            entity_id: new_id,
            accion: "crear".to_string(),
            cambios: serde_json::json!({
                "accion": "renovacion",
                "contrato_original_id": contrato_id,
                "nuevo_contrato": ContratoResponse::from(new_record.clone()),
            }),
        },
    )
    .await;

    let pagos_count = generar_pagos_para_contrato(
        &txn,
        &PagoGeneracionParams {
            contrato_id: new_id,
            organizacion_id: new_record.organizacion_id,
            fecha_inicio: new_record.fecha_inicio,
            fecha_fin: new_record.fecha_fin,
            monto_mensual: new_record.monto_mensual,
            moneda: &new_record.moneda,
            dia_vencimiento,
            usuario_id,
        },
    )
    .await?;

    txn.commit().await?;

    let mut response = ContratoResponse::from(new_record);
    response.pagos_generados = Some(pagos_count);
    Ok(response)
}

pub async fn terminar(
    db: &DatabaseConnection,
    org_id: Uuid,
    contrato_id: Uuid,
    input: TerminarContratoRequest,
    usuario_id: Uuid,
) -> Result<ContratoResponse, AppError> {
    let txn = db.begin().await?;

    let existing = contrato::Entity::find_by_id(contrato_id)
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    if existing.estado != "activo" {
        return Err(AppError::Validation(
            "Solo se pueden terminar contratos activos".to_string(),
        ));
    }

    if input.fecha_terminacion < existing.fecha_inicio {
        return Err(AppError::Validation(
            "La fecha de terminación no puede ser anterior a la fecha de inicio".to_string(),
        ));
    }

    let propiedad_id = existing.propiedad_id;

    let mut active: contrato::ActiveModel = existing.into();
    active.estado = Set("terminado".to_string());
    active.fecha_fin = Set(input.fecha_terminacion);
    active.updated_at = Set(Utc::now().into());

    let updated = active.update(&txn).await?;

    let other_active = contrato::Entity::find()
        .filter(
            Condition::all()
                .add(contrato::Column::PropiedadId.eq(propiedad_id))
                .add(contrato::Column::Estado.eq("activo"))
                .add(contrato::Column::Id.ne(contrato_id)),
        )
        .one(&txn)
        .await?;

    if other_active.is_none() {
        let prop = propiedad::Entity::find_by_id(propiedad_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;
        let mut prop_active: propiedad::ActiveModel = prop.into();
        prop_active.estado = Set("disponible".to_string());
        prop_active.updated_at = Set(Utc::now().into());
        prop_active.update(&txn).await?;
    }

    auditoria::registrar_best_effort(
        &txn,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "contrato".to_string(),
            entity_id: contrato_id,
            accion: "actualizar".to_string(),
            cambios: serde_json::json!({
                "accion": "terminacion_anticipada",
                "fecha_terminacion": input.fecha_terminacion,
                "contrato": ContratoResponse::from(updated.clone()),
            }),
        },
    )
    .await;

    // Cancel future pending pagos
    let pagos_cancelados =
        cancelar_pagos_futuros(&txn, contrato_id, input.fecha_terminacion).await?;

    auditoria::registrar_best_effort(
        &txn,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "contrato".to_string(),
            entity_id: contrato_id,
            accion: "cancelar_pagos".to_string(),
            cambios: serde_json::json!({
                "pagos_cancelados": pagos_cancelados,
            }),
        },
    )
    .await;

    txn.commit().await?;

    Ok(ContratoResponse::from(updated))
}

/// Marks expired contracts across ALL organizations. Intentionally not
/// org-scoped because this runs as a global background job via the scheduler.
pub async fn marcar_vencidos(db: &DatabaseConnection) -> Result<u64, AppError> {
    let today = Utc::now().date_naive();

    let result = contrato::Entity::update_many()
        .col_expr(
            contrato::Column::Estado,
            sea_orm::sea_query::Expr::value("vencido"),
        )
        .col_expr(
            contrato::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(Utc::now().fixed_offset()),
        )
        .filter(contrato::Column::FechaFin.lt(today))
        .filter(contrato::Column::Estado.eq("activo"))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

pub async fn listar_por_vencer(
    db: &DatabaseConnection,
    org_id: Uuid,
    dias: Option<i64>,
) -> Result<Vec<ContratoResponse>, AppError> {
    let dias = dias.unwrap_or(90).min(365);
    let today = Utc::now().date_naive();
    let cutoff = today + Duration::days(dias);

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

    Ok(records.into_iter().map(ContratoResponse::from).collect())
}

pub async fn cambiar_estado_deposito(
    db: &DatabaseConnection,
    org_id: Uuid,
    contrato_id: Uuid,
    input: CambiarEstadoDepositoRequest,
    usuario_id: Uuid,
) -> Result<ContratoResponse, AppError> {
    // 1. Find contrato by ID, scoped to org
    let existing = contrato::Entity::find_by_id(contrato_id)
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    // 2. Validate contrato has deposito > 0
    let deposito = existing
        .deposito
        .filter(|d| *d > rust_decimal::Decimal::ZERO)
        .ok_or_else(|| {
            AppError::Validation("El contrato no tiene depósito de garantía".to_string())
        })?;

    // 3. Validate estado enum
    validate_enum("estado de depósito", &input.estado, ESTADOS_DEPOSITO)?;

    // 4. Get current estado_deposito
    let estado_actual = existing.estado_deposito.clone().ok_or_else(|| {
        AppError::Validation("El contrato no tiene depósito de garantía".to_string())
    })?;

    // 5. Validate transition
    validar_transicion_deposito(&estado_actual, &input.estado)?;

    // 6. For retenido: validate monto_retenido and motivo_retencion
    if input.estado == "retenido" {
        validar_retencion(
            input.monto_retenido,
            input.motivo_retencion.as_deref(),
            deposito,
        )?;
    }

    // 7. Open transaction and update fields based on new estado
    let txn = db.begin().await?;

    let mut active: contrato::ActiveModel = existing.into();
    active.estado_deposito = Set(Some(input.estado.clone()));
    active.updated_at = Set(Utc::now().into());

    match input.estado.as_str() {
        "cobrado" => {
            active.fecha_cobro_deposito = Set(Some(Utc::now().into()));
        }
        "devuelto" => {
            active.fecha_devolucion_deposito = Set(Some(Utc::now().into()));
        }
        "retenido" => {
            active.fecha_devolucion_deposito = Set(Some(Utc::now().into()));
            active.monto_retenido = Set(input.monto_retenido);
            active.motivo_retencion = Set(input.motivo_retencion.clone());
        }
        _ => {}
    }

    let updated = active.update(&txn).await?;

    // 8. Register auditoría
    auditoria::registrar_best_effort(
        &txn,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "contrato".to_string(),
            entity_id: contrato_id,
            accion: "cambio_deposito".to_string(),
            cambios: serde_json::json!({
                "estado_anterior": estado_actual,
                "estado_nuevo": input.estado,
                "contrato": ContratoResponse::from(updated.clone()),
            }),
        },
    )
    .await;

    // 9. Commit and return
    txn.commit().await?;

    Ok(ContratoResponse::from(updated))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Valid transitions ---

    #[test]
    fn transicion_pendiente_a_cobrado_ok() {
        assert!(validar_transicion_deposito("pendiente", "cobrado").is_ok());
    }

    #[test]
    fn transicion_cobrado_a_devuelto_ok() {
        assert!(validar_transicion_deposito("cobrado", "devuelto").is_ok());
    }

    #[test]
    fn transicion_cobrado_a_retenido_ok() {
        assert!(validar_transicion_deposito("cobrado", "retenido").is_ok());
    }

    // --- Invalid transitions ---

    #[test]
    fn transicion_pendiente_a_devuelto_err() {
        assert!(validar_transicion_deposito("pendiente", "devuelto").is_err());
    }

    #[test]
    fn transicion_pendiente_a_retenido_err() {
        assert!(validar_transicion_deposito("pendiente", "retenido").is_err());
    }

    #[test]
    fn transicion_cobrado_a_pendiente_err() {
        assert!(validar_transicion_deposito("cobrado", "pendiente").is_err());
    }

    #[test]
    fn transicion_devuelto_a_pendiente_err() {
        assert!(validar_transicion_deposito("devuelto", "pendiente").is_err());
    }

    #[test]
    fn transicion_devuelto_a_cobrado_err() {
        assert!(validar_transicion_deposito("devuelto", "cobrado").is_err());
    }

    #[test]
    fn transicion_devuelto_a_retenido_err() {
        assert!(validar_transicion_deposito("devuelto", "retenido").is_err());
    }

    #[test]
    fn transicion_retenido_a_pendiente_err() {
        assert!(validar_transicion_deposito("retenido", "pendiente").is_err());
    }

    #[test]
    fn transicion_retenido_a_cobrado_err() {
        assert!(validar_transicion_deposito("retenido", "cobrado").is_err());
    }

    #[test]
    fn transicion_retenido_a_devuelto_err() {
        assert!(validar_transicion_deposito("retenido", "devuelto").is_err());
    }
}
