use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use tracing::warn;
use uuid::Uuid;

use crate::entities::{contrato, cuota_condominio, inquilino, organizacion, pago, propiedad};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::fiscal::TipoFiscal;
use crate::models::ncf::TipoNCF;
use crate::models::pago::{CreatePagoRequest, PagoListQuery, PagoResponse, UpdatePagoRequest};
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::{
    itbis, ncf, recargos,
    validation::{METODOS_PAGO, METODOS_PAGO_DGII, MONEDAS, validate_enum},
};

fn parse_tipo_fiscal(s: &str) -> TipoFiscal {
    match s {
        "persona_juridica" => TipoFiscal::PersonaJuridica,
        "persona_fisica" => TipoFiscal::PersonaFisica,
        _ => TipoFiscal::Informal,
    }
}

fn infer_tenant_tipo_fiscal(cedula: &str) -> TipoFiscal {
    let digits: String = cedula.chars().filter(char::is_ascii_digit).collect();
    match digits.len() {
        9 => TipoFiscal::PersonaJuridica,
        11 => TipoFiscal::PersonaFisica,
        _ => TipoFiscal::Informal,
    }
}

const ESTADOS_PAGO: &[&str] = &["pendiente", "pagado", "atrasado", "cancelado"];

const VALID_TRANSITIONS: &[(&str, &[&str])] = &[
    ("pendiente", &["pagado", "atrasado", "cancelado"]),
    ("atrasado", &["pagado", "cancelado"]),
    ("pagado", &[]),
    ("cancelado", &[]),
];

fn validate_transition(old: &str, new: &str) -> Result<(), AppError> {
    if old == new {
        return Ok(());
    }
    for &(from, allowed) in VALID_TRANSITIONS {
        if from == old {
            if allowed.contains(&new) {
                return Ok(());
            }
            return Err(AppError::Validation(format!(
                "Transición de estado no válida: {old} → {new}"
            )));
        }
    }
    Err(AppError::Validation(format!(
        "Transición de estado no válida: {old} → {new}"
    )))
}

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

    let contrato_model = contrato::Entity::find_by_id(input.contrato_id)
        .filter(contrato::Column::OrganizacionId.eq(organizacion_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    let propiedad_model = propiedad::Entity::find_by_id(contrato_model.propiedad_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    let org_model = organizacion::Entity::find_by_id(organizacion_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;

    let tipo_fiscal = parse_tipo_fiscal(&org_model.tipo_fiscal);

    let itbis_result = itbis::calcular_itbis(
        input.monto,
        &propiedad_model.tipo_propiedad,
        &tipo_fiscal,
        None,
    );

    let monto_itbis_retenido = if itbis_result.monto_itbis > Decimal::ZERO {
        let inquilino_model = inquilino::Entity::find_by_id(contrato_model.inquilino_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Inquilino no encontrado".to_string()))?;

        let tenant_tipo_fiscal = infer_tenant_tipo_fiscal(&inquilino_model.cedula);
        let retencion = itbis::calcular_retencion(itbis_result.monto_itbis, &tenant_tipo_fiscal);
        if retencion.monto_retenido > Decimal::ZERO {
            Some(retencion.monto_retenido)
        } else {
            None
        }
    } else {
        None
    };

    let now = Utc::now().into();
    let id = Uuid::new_v4();

    let metric_metodo = input
        .metodo_pago
        .as_deref()
        .unwrap_or("sin_especificar")
        .to_string();
    let metric_moneda = input.moneda.as_deref().unwrap_or("DOP").to_string();

    let model = pago::ActiveModel {
        id: Set(id),
        contrato_id: Set(input.contrato_id),
        monto: Set(itbis_result.monto_total),
        moneda: Set(input.moneda.unwrap_or_else(|| "DOP".to_string())),
        fecha_pago: Set(input.fecha_pago),
        fecha_vencimiento: Set(input.fecha_vencimiento),
        metodo_pago: Set(input.metodo_pago),
        estado: Set("pendiente".to_string()),
        notas: Set(input.notas),
        recargo: Set(None),
        organizacion_id: Set(organizacion_id),
        monto_base: Set(Some(itbis_result.monto_base)),
        monto_itbis: Set(Some(itbis_result.monto_itbis)),
        monto_itbis_retenido: Set(monto_itbis_retenido),
        ncf: Set(None),
        fecha_comprobante: Set(None),
        tipo_ncf: Set(None),
        es_parcial: Set(false),
        saldo_pendiente: Set(None),
        tipo_linea: Set("renta".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = model.insert(db).await?;

    crate::metrics::PAGOS_PROCESADOS
        .with_label_values(&[&metric_metodo, &metric_moneda])
        .inc();

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

    let active_cuotas = cuota_condominio::Entity::find()
        .filter(cuota_condominio::Column::PropiedadId.eq(propiedad_model.id))
        .filter(cuota_condominio::Column::OrganizacionId.eq(organizacion_id))
        .filter(cuota_condominio::Column::EsPassthrough.eq(true))
        .filter(cuota_condominio::Column::FechaInicio.lte(input.fecha_vencimiento))
        .all(db)
        .await?;

    for cuota in &active_cuotas {
        if let Some(fecha_fin) = cuota.fecha_fin {
            if fecha_fin <= input.fecha_vencimiento {
                continue;
            }
        }

        let cuota_itbis_result = itbis::calcular_itbis(
            cuota.monto,
            &propiedad_model.tipo_propiedad,
            &tipo_fiscal,
            None,
        );

        let cuota_id = Uuid::new_v4();
        let cuota_pago = pago::ActiveModel {
            id: Set(cuota_id),
            contrato_id: Set(input.contrato_id),
            monto: Set(cuota_itbis_result.monto_total),
            moneda: Set(cuota.moneda.clone()),
            fecha_pago: Set(None),
            fecha_vencimiento: Set(input.fecha_vencimiento),
            metodo_pago: Set(None),
            estado: Set("pendiente".to_string()),
            notas: Set(None),
            recargo: Set(None),
            organizacion_id: Set(organizacion_id),
            monto_base: Set(Some(cuota_itbis_result.monto_base)),
            monto_itbis: Set(Some(cuota_itbis_result.monto_itbis)),
            monto_itbis_retenido: Set(None),
            ncf: Set(None),
            fecha_comprobante: Set(None),
            tipo_ncf: Set(None),
            es_parcial: Set(false),
            saldo_pendiente: Set(None),
            tipo_linea: Set("cuota_condominio".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        };

        cuota_pago.insert(db).await?;
    }

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

    if let Some(ref estado) = input.estado {
        validate_transition(&old_estado, estado)?;
    }

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

    let new_estado = &updated.estado;
    if new_estado == "atrasado" && old_estado != "atrasado" {
        if let Some(contrato_model) = contrato::Entity::find_by_id(contrato_id).one(db).await? {
            recargos::aplicar_recargo(db, pago_id, &contrato_model).await?;
            updated = pago::Entity::find_by_id(pago_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Pago no encontrado".to_string()))?;
        }
    } else if old_estado == "atrasado" && new_estado != "atrasado" {
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

pub async fn bulk_marcar_pagado<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    pago_ids: &[Uuid],
    fecha_pago: chrono::NaiveDate,
    metodo_pago: &str,
    usuario_id: Uuid,
) -> Result<u64, AppError> {
    use crate::services::validation::{METODOS_PAGO, validate_enum};

    validate_enum("metodo_pago", metodo_pago, METODOS_PAGO)?;

    if pago_ids.is_empty() {
        return Ok(0);
    }

    if pago_ids.len() > 100 {
        return Err(AppError::Validation(
            "Máximo 100 pagos por operación masiva".to_string(),
        ));
    }

    let pagos = pago::Entity::find()
        .filter(pago::Column::Id.is_in(pago_ids.to_vec()))
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .all(db)
        .await?;

    if pagos.len() != pago_ids.len() {
        return Err(AppError::Validation(
            "Uno o más pagos no fueron encontrados en la organización".to_string(),
        ));
    }

    let non_updatable: Vec<Uuid> = pagos
        .iter()
        .filter(|p| p.estado != "pendiente" && p.estado != "atrasado")
        .map(|p| p.id)
        .collect();

    if !non_updatable.is_empty() {
        return Err(AppError::Validation(format!(
            "Los siguientes pagos no están en estado pendiente/atrasado: {}",
            non_updatable
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let now = Utc::now().fixed_offset();
    let result = pago::Entity::update_many()
        .col_expr(
            pago::Column::Estado,
            sea_orm::sea_query::Expr::value("pagado"),
        )
        .col_expr(
            pago::Column::FechaPago,
            sea_orm::sea_query::Expr::value(fecha_pago),
        )
        .col_expr(
            pago::Column::MetodoPago,
            sea_orm::sea_query::Expr::value(metodo_pago),
        )
        .col_expr(
            pago::Column::Recargo,
            sea_orm::sea_query::Expr::value(Option::<rust_decimal::Decimal>::None),
        )
        .col_expr(
            pago::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(pago::Column::Id.is_in(pago_ids.to_vec()))
        .exec(db)
        .await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "pago".to_string(),
            entity_id: Uuid::nil(),
            accion: "bulk_marcar_pagado".to_string(),
            cambios: serde_json::json!({
                "pago_ids": pago_ids,
                "fecha_pago": fecha_pago,
                "metodo_pago": metodo_pago,
                "actualizados": result.rows_affected,
            }),
        },
    )
    .await;

    Ok(result.rows_affected)
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
    for chunk in recargo_candidates.chunks(20) {
        let futures: Vec<_> = chunk
            .iter()
            .map(|(pago_id, contrato_model)| {
                recargos::aplicar_recargo(db, *pago_id, contrato_model)
            })
            .collect();
        let results = futures_util::future::join_all(futures).await;
        for result in &results {
            match result {
                Ok(Some(_)) => recargos_calculated += 1,
                Ok(None) => {}
                Err(e) => {
                    warn!("Error aplicando recargo durante mark_overdue: {e}");
                }
            }
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

pub async fn registrar_pago_parcial<C: ConnectionTrait>(
    db: &C,
    contrato_id: Uuid,
    monto: Decimal,
    metodo_pago: &str,
    notas: Option<&str>,
    org_id: Uuid,
) -> Result<Vec<pago::Model>, AppError> {
    validate_enum("metodo_pago", metodo_pago, METODOS_PAGO_DGII)?;

    if monto <= Decimal::ZERO {
        return Err(AppError::Validation(
            "El monto debe ser mayor a cero".to_string(),
        ));
    }

    let contrato_model = contrato::Entity::find_by_id(contrato_id)
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    let unpaid_periods = pago::Entity::find()
        .filter(pago::Column::ContratoId.eq(contrato_id))
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .filter(pago::Column::Estado.is_in(["pendiente", "atrasado"]))
        .filter(pago::Column::EsParcial.eq(false))
        .order_by_asc(pago::Column::FechaVencimiento)
        .all(db)
        .await?;

    if unpaid_periods.is_empty() {
        return Err(AppError::Validation(
            "No hay períodos pendientes de pago para este contrato".to_string(),
        ));
    }

    let total_owed: Decimal = unpaid_periods
        .iter()
        .map(|p| balance_remaining(p, &contrato_model))
        .sum();

    if monto > total_owed {
        return Err(AppError::Validation(
            "Monto excede el total adeudado".to_string(),
        ));
    }

    let mut remaining = monto;
    let mut created_records = Vec::new();

    for period in &unpaid_periods {
        if remaining <= Decimal::ZERO {
            break;
        }

        let amount_due = balance_remaining(period, &contrato_model);

        if amount_due <= Decimal::ZERO {
            continue;
        }

        if created_records.is_empty() && remaining == amount_due && unpaid_periods.len() == 1 {
            return Err(AppError::Validation(
                "El monto es igual al saldo pendiente. Use el flujo de pago completo en vez de pago parcial".to_string(),
            ));
        }

        let applied = remaining.min(amount_due);
        remaining -= applied;

        let new_balance = amount_due - applied;
        let is_fully_paid = new_balance <= Decimal::ZERO;

        let now = Utc::now().into();
        let pago_id = Uuid::new_v4();
        let today = Utc::now().date_naive();

        let partial_record = pago::ActiveModel {
            id: Set(pago_id),
            contrato_id: Set(contrato_id),
            monto: Set(applied),
            moneda: Set(contrato_model.moneda.clone()),
            fecha_pago: Set(Some(today)),
            fecha_vencimiento: Set(period.fecha_vencimiento),
            metodo_pago: Set(Some(metodo_pago.to_string())),
            estado: Set("pagado".to_string()),
            notas: Set(notas.map(str::to_string)),
            recargo: Set(None),
            organizacion_id: Set(org_id),
            monto_base: Set(None),
            monto_itbis: Set(None),
            monto_itbis_retenido: Set(None),
            ncf: Set(None),
            fecha_comprobante: Set(None),
            tipo_ncf: Set(None),
            es_parcial: Set(true),
            saldo_pendiente: Set(Some(new_balance)),
            tipo_linea: Set("renta".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let record = partial_record.insert(db).await?;
        created_records.push(record);

        let mut active: pago::ActiveModel = period.clone().into();
        if is_fully_paid {
            active.estado = Set("pagado".to_string());
            active.saldo_pendiente = Set(Some(Decimal::ZERO));
        } else {
            active.saldo_pendiente = Set(Some(new_balance));
        }
        active.updated_at = Set(Utc::now().into());
        active.update(db).await?;
    }

    Ok(created_records)
}

#[allow(clippy::missing_const_for_fn)]
fn determinar_tipo_ncf_para_inquilino(_inquilino: &inquilino::Model) -> TipoNCF {
    TipoNCF::B02
}

pub async fn intentar_asignar_ncf(
    db: &DatabaseConnection,
    pago_id: Uuid,
    contrato_id: Uuid,
    org_id: Uuid,
) -> Result<(), AppError> {
    let org = organizacion::Entity::find_by_id(org_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;

    if org.tipo_fiscal == "informal" {
        return Ok(());
    }

    let contrato_model = contrato::Entity::find_by_id(contrato_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    let inquilino_model = inquilino::Entity::find_by_id(contrato_model.inquilino_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Inquilino no encontrado".to_string()))?;

    let tipo_ncf = determinar_tipo_ncf_para_inquilino(&inquilino_model);
    let fecha_comprobante = Utc::now().date_naive();

    match ncf::asignar_ncf(db, org_id, tipo_ncf.clone(), fecha_comprobante).await {
        Ok(ncf_string) => {
            let pago_model = pago::Entity::find_by_id(pago_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Pago no encontrado".to_string()))?;

            let mut active: pago::ActiveModel = pago_model.into();
            active.ncf = Set(Some(ncf_string));
            active.fecha_comprobante = Set(Some(fecha_comprobante));
            active.tipo_ncf = Set(Some(tipo_ncf.to_string()));
            active.updated_at = Set(Utc::now().into());
            active.update(db).await?;
        }
        Err(e) => {
            warn!(
                pago_id = %pago_id,
                org_id = %org_id,
                error = %e,
                "Error al asignar NCF al pago. El pago permanece pagado sin NCF para resolución manual"
            );
        }
    }

    Ok(())
}

fn balance_remaining(period: &pago::Model, _contrato: &contrato::Model) -> Decimal {
    if let Some(saldo) = period.saldo_pendiente {
        return saldo;
    }
    period.monto + period.recargo.unwrap_or(Decimal::ZERO)
}
