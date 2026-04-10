use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::{contrato, pago};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::pago::{CreatePagoRequest, PagoListQuery, PagoResponse, UpdatePagoRequest};

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
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}

pub async fn create(
    db: &DatabaseConnection,
    input: CreatePagoRequest,
) -> Result<PagoResponse, AppError> {
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
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = model.insert(db).await?;
    Ok(PagoResponse::from(record))
}

pub async fn get_by_id(db: &DatabaseConnection, id: Uuid) -> Result<PagoResponse, AppError> {
    let record = pago::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Pago no encontrado".to_string()))?;
    Ok(PagoResponse::from(record))
}

pub async fn list(
    db: &DatabaseConnection,
    query: PagoListQuery,
) -> Result<PaginatedResponse<PagoResponse>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let mut select = pago::Entity::find();

    if let Some(ref contrato_id) = query.contrato_id {
        select = select.filter(pago::Column::ContratoId.eq(*contrato_id));
    }
    if let Some(ref estado) = query.estado {
        select = select.filter(pago::Column::Estado.eq(estado));
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

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdatePagoRequest,
) -> Result<PagoResponse, AppError> {
    let existing = pago::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Pago no encontrado".to_string()))?;

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

    let updated = active.update(db).await?;
    Ok(PagoResponse::from(updated))
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<(), AppError> {
    let result = pago::Entity::delete_by_id(id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(AppError::NotFound("Pago no encontrado".to_string()));
    }
    Ok(())
}

#[allow(dead_code)]
pub async fn mark_overdue(db: &DatabaseConnection) -> Result<u64, AppError> {
    let today = Utc::now().date_naive();

    let result = pago::Entity::update_many()
        .col_expr(
            pago::Column::Estado,
            sea_orm::sea_query::Expr::value("atrasado"),
        )
        .col_expr(
            pago::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(Utc::now().fixed_offset()),
        )
        .filter(pago::Column::FechaVencimiento.lt(today))
        .filter(pago::Column::Estado.eq("pendiente"))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}
