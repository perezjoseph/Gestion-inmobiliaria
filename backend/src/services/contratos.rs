use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use uuid::Uuid;

use crate::entities::{contrato, inquilino, propiedad};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::contrato::{
    ContratoResponse, CreateContratoRequest, RenovarContratoRequest, TerminarContratoRequest,
    UpdateContratoRequest,
};
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::validation::{validate_enum, MONEDAS};

const ESTADOS_CONTRATO: &[&str] = &["activo", "vencido", "cancelado"];

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

    txn.commit().await?;

    Ok(ContratoResponse::from(record))
}

pub async fn get_by_id(db: &DatabaseConnection, id: Uuid) -> Result<ContratoResponse, AppError> {
    let record = contrato::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;
    Ok(ContratoResponse::from(record))
}

pub async fn list(
    db: &DatabaseConnection,
    page: Option<u64>,
    per_page: Option<u64>,
) -> Result<PaginatedResponse<ContratoResponse>, AppError> {
    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(20).clamp(1, 100);

    let paginator = contrato::Entity::find()
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
    id: Uuid,
    input: UpdateContratoRequest,
    usuario_id: Uuid,
) -> Result<ContratoResponse, AppError> {
    if let Some(ref estado) = input.estado {
        validate_enum("estado", estado, ESTADOS_CONTRATO)?;
    }

    let existing = contrato::Entity::find_by_id(id)
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

pub async fn delete(db: &DatabaseConnection, id: Uuid, usuario_id: Uuid) -> Result<(), AppError> {
    let txn = db.begin().await?;

    let result = contrato::Entity::delete_by_id(id).exec(&txn).await?;
    if result.rows_affected == 0 {
        return Err(AppError::NotFound("Contrato no encontrado".to_string()));
    }

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
    contrato_id: Uuid,
    input: RenovarContratoRequest,
    usuario_id: Uuid,
) -> Result<ContratoResponse, AppError> {
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

    txn.commit().await?;

    Ok(ContratoResponse::from(new_record))
}

pub async fn terminar(
    db: &DatabaseConnection,
    contrato_id: Uuid,
    input: TerminarContratoRequest,
    usuario_id: Uuid,
) -> Result<ContratoResponse, AppError> {
    let txn = db.begin().await?;

    let existing = contrato::Entity::find_by_id(contrato_id)
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

    txn.commit().await?;

    Ok(ContratoResponse::from(updated))
}

pub async fn listar_por_vencer(
    db: &DatabaseConnection,
    dias: Option<i64>,
) -> Result<Vec<ContratoResponse>, AppError> {
    let dias = dias.unwrap_or(90).min(365);
    let today = Utc::now().date_naive();
    let cutoff = today + Duration::days(dias);

    let records = contrato::Entity::find()
        .filter(
            Condition::all()
                .add(contrato::Column::Estado.eq("activo"))
                .add(contrato::Column::FechaFin.gte(today))
                .add(contrato::Column::FechaFin.lte(cutoff)),
        )
        .order_by_asc(contrato::Column::FechaFin)
        .all(db)
        .await?;

    Ok(records.into_iter().map(ContratoResponse::from).collect())
}