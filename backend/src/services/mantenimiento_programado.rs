use chrono::{Datelike, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::{mantenimiento_programado, propiedad, solicitud_mantenimiento, unidad};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::mantenimiento_programado::{
    CreateMantenimientoProgramadoRequest, MantenimientoProgramadoListQuery,
    MantenimientoProgramadoResponse, UpdateMantenimientoProgramadoRequest,
};
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::validation::{MONEDAS, validate_enum};

const FRECUENCIAS: &[&str] = &["mensual", "bimestral", "trimestral", "semestral", "anual"];
const PRIORIDADES: &[&str] = &["baja", "media", "alta", "urgente"];

impl From<mantenimiento_programado::Model> for MantenimientoProgramadoResponse {
    fn from(m: mantenimiento_programado::Model) -> Self {
        Self {
            id: m.id,
            propiedad_id: m.propiedad_id,
            unidad_id: m.unidad_id,
            titulo: m.titulo,
            descripcion: m.descripcion,
            prioridad: m.prioridad,
            nombre_proveedor: m.nombre_proveedor,
            telefono_proveedor: m.telefono_proveedor,
            email_proveedor: m.email_proveedor,
            costo_estimado: m.costo_estimado,
            costo_moneda: m.costo_moneda,
            frecuencia: m.frecuencia,
            proxima_fecha: m.proxima_fecha,
            activo: m.activo,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    input: CreateMantenimientoProgramadoRequest,
    usuario_id: Uuid,
    organizacion_id: Uuid,
) -> Result<MantenimientoProgramadoResponse, AppError> {
    if input.titulo.trim().is_empty() {
        return Err(AppError::Validation("El título es requerido".to_string()));
    }

    validate_enum("frecuencia", &input.frecuencia, FRECUENCIAS)?;

    let prioridad = input.prioridad.unwrap_or_else(|| "media".to_string());
    validate_enum("prioridad", &prioridad, PRIORIDADES)?;

    if let Some(ref moneda) = input.costo_moneda {
        validate_enum("moneda", moneda, MONEDAS)?;
    }
    if let Some(monto) = input.costo_estimado {
        if monto < Decimal::ZERO {
            return Err(AppError::Validation(
                "El costo estimado debe ser mayor o igual a cero".to_string(),
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
                "La unidad no pertenece a la propiedad indicada".to_string(),
            ));
        }
    }

    let now = Utc::now().into();
    let id = Uuid::new_v4();

    let model = mantenimiento_programado::ActiveModel {
        id: Set(id),
        propiedad_id: Set(input.propiedad_id),
        unidad_id: Set(input.unidad_id),
        titulo: Set(input.titulo),
        descripcion: Set(input.descripcion),
        prioridad: Set(prioridad),
        nombre_proveedor: Set(input.nombre_proveedor),
        telefono_proveedor: Set(input.telefono_proveedor),
        email_proveedor: Set(input.email_proveedor),
        costo_estimado: Set(input.costo_estimado),
        costo_moneda: Set(input.costo_moneda),
        frecuencia: Set(input.frecuencia),
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
            entity_type: "mantenimiento_programado".to_string(),
            entity_id: id,
            accion: "crear".to_string(),
            cambios: serde_json::json!(MantenimientoProgramadoResponse::from(record.clone())),
        },
    )
    .await;

    Ok(MantenimientoProgramadoResponse::from(record))
}

pub async fn list(
    db: &DatabaseConnection,
    org_id: Uuid,
    query: MantenimientoProgramadoListQuery,
) -> Result<PaginatedResponse<MantenimientoProgramadoResponse>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let mut select = mantenimiento_programado::Entity::find()
        .filter(mantenimiento_programado::Column::OrganizacionId.eq(org_id));

    if let Some(propiedad_id) = query.propiedad_id {
        select = select.filter(mantenimiento_programado::Column::PropiedadId.eq(propiedad_id));
    }
    if let Some(activo) = query.activo {
        select = select.filter(mantenimiento_programado::Column::Activo.eq(activo));
    }

    let paginator = select
        .order_by_asc(mantenimiento_programado::Column::ProximaFecha)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records
            .into_iter()
            .map(MantenimientoProgramadoResponse::from)
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
) -> Result<MantenimientoProgramadoResponse, AppError> {
    let record = mantenimiento_programado::Entity::find_by_id(id)
        .filter(mantenimiento_programado::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Mantenimiento programado no encontrado".to_string()))?;
    Ok(MantenimientoProgramadoResponse::from(record))
}

pub async fn update<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    id: Uuid,
    input: UpdateMantenimientoProgramadoRequest,
    usuario_id: Uuid,
) -> Result<MantenimientoProgramadoResponse, AppError> {
    if let Some(ref prioridad) = input.prioridad {
        validate_enum("prioridad", prioridad, PRIORIDADES)?;
    }
    if let Some(ref frecuencia) = input.frecuencia {
        validate_enum("frecuencia", frecuencia, FRECUENCIAS)?;
    }
    if let Some(ref moneda) = input.costo_moneda {
        validate_enum("moneda", moneda, MONEDAS)?;
    }
    if let Some(monto) = input.costo_estimado {
        if monto < Decimal::ZERO {
            return Err(AppError::Validation(
                "El costo estimado debe ser mayor o igual a cero".to_string(),
            ));
        }
    }

    let existing = mantenimiento_programado::Entity::find_by_id(id)
        .filter(mantenimiento_programado::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Mantenimiento programado no encontrado".to_string()))?;

    let mut active: mantenimiento_programado::ActiveModel = existing.into();

    if let Some(titulo) = input.titulo {
        active.titulo = Set(titulo);
    }
    if let Some(descripcion) = input.descripcion {
        active.descripcion = Set(Some(descripcion));
    }
    if let Some(prioridad) = input.prioridad {
        active.prioridad = Set(prioridad);
    }
    if let Some(nombre_proveedor) = input.nombre_proveedor {
        active.nombre_proveedor = Set(Some(nombre_proveedor));
    }
    if let Some(telefono_proveedor) = input.telefono_proveedor {
        active.telefono_proveedor = Set(Some(telefono_proveedor));
    }
    if let Some(email_proveedor) = input.email_proveedor {
        active.email_proveedor = Set(Some(email_proveedor));
    }
    if let Some(costo_estimado) = input.costo_estimado {
        active.costo_estimado = Set(Some(costo_estimado));
    }
    if let Some(costo_moneda) = input.costo_moneda {
        active.costo_moneda = Set(Some(costo_moneda));
    }
    if let Some(frecuencia) = input.frecuencia {
        active.frecuencia = Set(frecuencia);
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
            entity_type: "mantenimiento_programado".to_string(),
            entity_id: id,
            accion: "actualizar".to_string(),
            cambios: serde_json::json!(MantenimientoProgramadoResponse::from(updated.clone())),
        },
    )
    .await;

    Ok(MantenimientoProgramadoResponse::from(updated))
}

pub async fn delete<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    id: Uuid,
    usuario_id: Uuid,
) -> Result<(), AppError> {
    let existing = mantenimiento_programado::Entity::find_by_id(id)
        .filter(mantenimiento_programado::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Mantenimiento programado no encontrado".to_string()))?;

    let active: mantenimiento_programado::ActiveModel = existing.into();
    active.delete(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "mantenimiento_programado".to_string(),
            entity_id: id,
            accion: "eliminar".to_string(),
            cambios: serde_json::json!({ "id": id }),
        },
    )
    .await;

    Ok(())
}

/// Generates solicitudes de mantenimiento from active scheduled templates
/// whose proxima_fecha <= today. Advances proxima_fecha after generating each solicitud.
/// Called by the background scheduler.
pub async fn generar_solicitudes_pendientes(db: &DatabaseConnection) -> Result<i64, AppError> {
    let today = Utc::now().date_naive();

    let pendientes = mantenimiento_programado::Entity::find()
        .filter(mantenimiento_programado::Column::Activo.eq(true))
        .filter(mantenimiento_programado::Column::ProximaFecha.lte(today))
        .all(db)
        .await?;

    let mut count: i64 = 0;

    for template in pendientes {
        let now = Utc::now().into();
        let solicitud_id = Uuid::new_v4();

        let nueva_solicitud = solicitud_mantenimiento::ActiveModel {
            id: Set(solicitud_id),
            propiedad_id: Set(template.propiedad_id),
            unidad_id: Set(template.unidad_id),
            inquilino_id: Set(None),
            titulo: Set(template.titulo.clone()),
            descripcion: Set(template.descripcion.clone()),
            estado: Set("pendiente".to_string()),
            prioridad: Set(template.prioridad.clone()),
            nombre_proveedor: Set(template.nombre_proveedor.clone()),
            telefono_proveedor: Set(template.telefono_proveedor.clone()),
            email_proveedor: Set(template.email_proveedor.clone()),
            costo_monto: Set(template.costo_estimado),
            costo_moneda: Set(template.costo_moneda.clone()),
            fecha_inicio: Set(None),
            fecha_fin: Set(None),
            organizacion_id: Set(template.organizacion_id),
            created_at: Set(now),
            updated_at: Set(now),
        };

        nueva_solicitud.insert(db).await?;

        // Advance proxima_fecha
        let next = calcular_proxima_fecha(template.proxima_fecha, &template.frecuencia);
        let mut active: mantenimiento_programado::ActiveModel = template.into();
        active.proxima_fecha = Set(next);
        active.updated_at = Set(now);
        active.update(db).await?;

        count += 1;
    }

    Ok(count)
}

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
