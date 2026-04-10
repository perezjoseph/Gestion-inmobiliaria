use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use uuid::Uuid;

use crate::entities::{contrato, inquilino, propiedad};
use crate::errors::AppError;
use crate::models::contrato::{ContratoResponse, CreateContratoRequest, UpdateContratoRequest};

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
) -> Result<ContratoResponse, AppError> {
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
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = model.insert(&txn).await?;

    let mut prop_active: propiedad::ActiveModel = prop.into();
    prop_active.estado = Set("ocupada".to_string());
    prop_active.updated_at = Set(Utc::now().into());
    prop_active.update(&txn).await?;

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

pub async fn list(db: &DatabaseConnection) -> Result<Vec<ContratoResponse>, AppError> {
    let records = contrato::Entity::find()
        .order_by_desc(contrato::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(records.into_iter().map(ContratoResponse::from).collect())
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdateContratoRequest,
) -> Result<ContratoResponse, AppError> {
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

    let new_fecha_fin = input.fecha_fin.unwrap_or(fecha_inicio);
    if input.fecha_fin.is_some() {
        active.fecha_fin = Set(new_fecha_fin);
        // Re-validate overlap when fecha_fin changes on an active contract
        if !is_terminating {
            validate_no_overlap(&txn, propiedad_id, fecha_inicio, new_fecha_fin, Some(id)).await?;
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

    txn.commit().await?;

    Ok(ContratoResponse::from(updated))
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<(), AppError> {
    let existing = contrato::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    contrato::Entity::delete_by_id(existing.id).exec(db).await?;

    Ok(())
}
