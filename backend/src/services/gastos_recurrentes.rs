use chrono::{Datelike, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::{gasto, gasto_recurrente, propiedad, unidad};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::gasto_recurrente::{
    CreateGastoRecurrenteRequest, GastoRecurrenteListQuery, GastoRecurrenteResponse,
    UpdateGastoRecurrenteRequest,
};
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::gastos::CATEGORIAS_GASTO;
use crate::services::validation::{MONEDAS, validate_enum};

const FRECUENCIAS: &[&str] = &["mensual", "bimestral", "trimestral", "semestral", "anual"];

impl From<gasto_recurrente::Model> for GastoRecurrenteResponse {
    fn from(m: gasto_recurrente::Model) -> Self {
        Self {
            id: m.id,
            propiedad_id: m.propiedad_id,
            unidad_id: m.unidad_id,
            categoria: m.categoria,
            descripcion: m.descripcion,
            monto: m.monto,
            moneda: m.moneda,
            proveedor: m.proveedor,
            frecuencia: m.frecuencia,
            dia_del_mes: m.dia_del_mes,
            proxima_fecha: m.proxima_fecha,
            activo: m.activo,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    input: CreateGastoRecurrenteRequest,
    usuario_id: Uuid,
    organizacion_id: Uuid,
) -> Result<GastoRecurrenteResponse, AppError> {
    if !CATEGORIAS_GASTO.contains(&input.categoria.as_str()) {
        return Err(AppError::Validation(
            "Categoría de gasto no válida".to_string(),
        ));
    }
    validate_enum("moneda", &input.moneda, MONEDAS)?;
    validate_enum("frecuencia", &input.frecuencia, FRECUENCIAS)?;

    if input.monto <= Decimal::ZERO {
        return Err(AppError::Validation(
            "El monto debe ser mayor que cero".to_string(),
        ));
    }

    if let Some(dia) = input.dia_del_mes {
        if !(1..=28).contains(&dia) {
            return Err(AppError::Validation(
                "El día del mes debe estar entre 1 y 28".to_string(),
            ));
        }
    }

    propiedad::Entity::find_by_id(input.propiedad_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    if let Some(unidad_id) = input.unidad_id {
        let u = unidad::Entity::find_by_id(unidad_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Unidad no encontrada".to_string()))?;
        if u.propiedad_id != input.propiedad_id {
            return Err(AppError::Validation(
                "La unidad no pertenece a la propiedad especificada".to_string(),
            ));
        }
    }

    let now = Utc::now().into();
    let id = Uuid::new_v4();

    let model = gasto_recurrente::ActiveModel {
        id: Set(id),
        propiedad_id: Set(input.propiedad_id),
        unidad_id: Set(input.unidad_id),
        categoria: Set(input.categoria),
        descripcion: Set(input.descripcion),
        monto: Set(input.monto),
        moneda: Set(input.moneda),
        proveedor: Set(input.proveedor),
        frecuencia: Set(input.frecuencia),
        dia_del_mes: Set(input.dia_del_mes),
        proxima_fecha: Set(input.proxima_fecha),
        activo: Set(true),
        organizacion_id: Set(organizacion_id),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = model.insert(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "gasto_recurrente".to_string(),
            entity_id: id,
            accion: "crear".to_string(),
            cambios: serde_json::json!(GastoRecurrenteResponse::from(record.clone())),
        },
    )
    .await;

    Ok(GastoRecurrenteResponse::from(record))
}

pub async fn list(
    db: &DatabaseConnection,
    org_id: Uuid,
    query: GastoRecurrenteListQuery,
) -> Result<PaginatedResponse<GastoRecurrenteResponse>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let mut select = gasto_recurrente::Entity::find()
        .filter(gasto_recurrente::Column::OrganizacionId.eq(org_id));

    if let Some(propiedad_id) = query.propiedad_id {
        select = select.filter(gasto_recurrente::Column::PropiedadId.eq(propiedad_id));
    }
    if let Some(activo) = query.activo {
        select = select.filter(gasto_recurrente::Column::Activo.eq(activo));
    }

    let paginator = select
        .order_by_desc(gasto_recurrente::Column::ProximaFecha)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records
            .into_iter()
            .map(GastoRecurrenteResponse::from)
            .collect(),
        total,
        page,
        per_page,
    })
}

pub async fn get_by_id(
    db: &DatabaseConnection,
    org_id: Uuid,
    id: Uuid,
) -> Result<GastoRecurrenteResponse, AppError> {
    let record = gasto_recurrente::Entity::find_by_id(id)
        .filter(gasto_recurrente::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Gasto recurrente no encontrado".to_string()))?;
    Ok(GastoRecurrenteResponse::from(record))
}

pub async fn update<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    id: Uuid,
    input: UpdateGastoRecurrenteRequest,
    usuario_id: Uuid,
) -> Result<GastoRecurrenteResponse, AppError> {
    if let Some(ref categoria) = input.categoria {
        if !CATEGORIAS_GASTO.contains(&categoria.as_str()) {
            return Err(AppError::Validation(
                "Categoría de gasto no válida".to_string(),
            ));
        }
    }
    if let Some(ref moneda) = input.moneda {
        validate_enum("moneda", moneda, MONEDAS)?;
    }
    if let Some(ref frecuencia) = input.frecuencia {
        validate_enum("frecuencia", frecuencia, FRECUENCIAS)?;
    }
    if let Some(ref monto) = input.monto {
        if *monto <= Decimal::ZERO {
            return Err(AppError::Validation(
                "El monto debe ser mayor que cero".to_string(),
            ));
        }
    }
    if let Some(dia) = input.dia_del_mes {
        if !(1..=28).contains(&dia) {
            return Err(AppError::Validation(
                "El día del mes debe estar entre 1 y 28".to_string(),
            ));
        }
    }

    let existing = gasto_recurrente::Entity::find_by_id(id)
        .filter(gasto_recurrente::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Gasto recurrente no encontrado".to_string()))?;

    let mut active: gasto_recurrente::ActiveModel = existing.into();

    if let Some(categoria) = input.categoria {
        active.categoria = Set(categoria);
    }
    if let Some(descripcion) = input.descripcion {
        active.descripcion = Set(descripcion);
    }
    if let Some(monto) = input.monto {
        active.monto = Set(monto);
    }
    if let Some(moneda) = input.moneda {
        active.moneda = Set(moneda);
    }
    if let Some(proveedor) = input.proveedor {
        active.proveedor = Set(Some(proveedor));
    }
    if let Some(frecuencia) = input.frecuencia {
        active.frecuencia = Set(frecuencia);
    }
    if let Some(dia_del_mes) = input.dia_del_mes {
        active.dia_del_mes = Set(Some(dia_del_mes));
    }
    if let Some(proxima_fecha) = input.proxima_fecha {
        active.proxima_fecha = Set(proxima_fecha);
    }
    if let Some(activo) = input.activo {
        active.activo = Set(activo);
    }

    active.updated_at = Set(Utc::now().into());

    let updated = active.update(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "gasto_recurrente".to_string(),
            entity_id: id,
            accion: "actualizar".to_string(),
            cambios: serde_json::json!(GastoRecurrenteResponse::from(updated.clone())),
        },
    )
    .await;

    Ok(GastoRecurrenteResponse::from(updated))
}

pub async fn delete<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    id: Uuid,
    usuario_id: Uuid,
) -> Result<(), AppError> {
    let existing = gasto_recurrente::Entity::find_by_id(id)
        .filter(gasto_recurrente::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Gasto recurrente no encontrado".to_string()))?;

    let active: gasto_recurrente::ActiveModel = existing.into();
    active.delete(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "gasto_recurrente".to_string(),
            entity_id: id,
            accion: "eliminar".to_string(),
            cambios: serde_json::json!({ "id": id }),
        },
    )
    .await;

    Ok(())
}

/// Generates gastos from active recurring templates whose proxima_fecha <= today.
/// Advances proxima_fecha after generating each gasto.
/// Called by the background scheduler.
pub async fn generar_gastos_pendientes(db: &DatabaseConnection) -> Result<i64, AppError> {
    let today = Utc::now().date_naive();

    let pendientes = gasto_recurrente::Entity::find()
        .filter(gasto_recurrente::Column::Activo.eq(true))
        .filter(gasto_recurrente::Column::ProximaFecha.lte(today))
        .all(db)
        .await?;

    let mut count: i64 = 0;

    for template in pendientes {
        let now = Utc::now().into();
        let gasto_id = Uuid::new_v4();

        let nuevo_gasto = gasto::ActiveModel {
            id: Set(gasto_id),
            propiedad_id: Set(template.propiedad_id),
            unidad_id: Set(template.unidad_id),
            categoria: Set(template.categoria.clone()),
            descripcion: Set(template.descripcion.clone()),
            monto: Set(template.monto),
            moneda: Set(template.moneda.clone()),
            fecha_gasto: Set(template.proxima_fecha),
            estado: Set("pendiente".to_string()),
            proveedor: Set(template.proveedor.clone()),
            numero_factura: Set(None),
            notas: Set(Some(
                "Generado automáticamente desde gasto recurrente".to_string(),
            )),
            organizacion_id: Set(template.organizacion_id),
            nic_contrato: Set(None),
            proveedor_servicio: Set(None),
            consumo: Set(None),
            unidad_consumo: Set(None),
            periodo_desde: Set(None),
            periodo_hasta: Set(None),
            numero_cuenta: Set(None),
            periodo_inicio: Set(None),
            periodo_fin: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        nuevo_gasto.insert(db).await?;

        // Advance proxima_fecha
        let next = calcular_proxima_fecha(template.proxima_fecha, &template.frecuencia);
        let mut active: gasto_recurrente::ActiveModel = template.into();
        active.proxima_fecha = Set(next);
        active.updated_at = Set(now);
        active.update(db).await?;

        count += 1;
    }

    Ok(count)
}

/// Calculates the next date based on frequency.
fn calcular_proxima_fecha(current: NaiveDate, frecuencia: &str) -> NaiveDate {
    match frecuencia {
        "mensual" => advance_months(current, 1),
        "bimestral" => advance_months(current, 2),
        "trimestral" => advance_months(current, 3),
        "semestral" => advance_months(current, 6),
        "anual" => advance_months(current, 12),
        _ => current + Duration::days(30),
    }
}

fn advance_months(date: NaiveDate, months: u32) -> NaiveDate {
    let mut year = date.year();
    let mut month = date.month() + months;

    while month > 12 {
        month -= 12;
        year += 1;
    }

    let day = date.day().min(last_day_of_month(year, month));
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or(date)
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    if month == 12 {
        31
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .and_then(|d| d.pred_opt())
            .map_or(28, |d| d.day())
    }
}
