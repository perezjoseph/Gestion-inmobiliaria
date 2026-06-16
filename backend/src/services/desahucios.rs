use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::{contrato, desahucio};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::desahucio::{
    CreateDesahucioRequest, DesahucioListQuery, DesahucioResponse, UpdateDesahucioRequest,
};
use crate::services::auditoria::{self, CreateAuditoriaEntry};

pub const ESTADOS_DESAHUCIO: &[&str] = &["iniciado", "en_progreso", "completado"];

impl From<desahucio::Model> for DesahucioResponse {
    fn from(m: desahucio::Model) -> Self {
        Self {
            id: m.id,
            contrato_id: m.contrato_id,
            estado: m.estado,
            fecha_inicio: m.fecha_inicio,
            fecha_resolucion: m.fecha_resolucion,
            motivo: m.motivo,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}

pub async fn create(
    db: &DatabaseConnection,
    input: CreateDesahucioRequest,
    usuario_id: Uuid,
    organizacion_id: Uuid,
) -> Result<DesahucioResponse, AppError> {
    let contrato = contrato::Entity::find_by_id(input.contrato_id)
        .filter(contrato::Column::OrganizacionId.eq(organizacion_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    if contrato.estado != "activo" {
        return Err(AppError::Validation(
            "El contrato debe estar activo para iniciar un desahucio".to_string(),
        ));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();

    let model = desahucio::ActiveModel {
        id: Set(id),
        contrato_id: Set(input.contrato_id),
        estado: Set("iniciado".to_string()),
        fecha_inicio: Set(now.date_naive()),
        fecha_resolucion: Set(None),
        motivo: Set(input.motivo),
        organizacion_id: Set(organizacion_id),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
    };

    let inserted = model.insert(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "desahucio".to_string(),
            entity_id: id,
            accion: "crear".to_string(),
            cambios: serde_json::json!({
                "contrato_id": input.contrato_id,
                "estado": "iniciado"
            }),
        },
    )
    .await;

    Ok(DesahucioResponse::from(inserted))
}

pub async fn update(
    db: &DatabaseConnection,
    org_id: Uuid,
    id: Uuid,
    input: UpdateDesahucioRequest,
    usuario_id: Uuid,
) -> Result<DesahucioResponse, AppError> {
    let existing = desahucio::Entity::find_by_id(id)
        .filter(desahucio::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Desahucio no encontrado".to_string()))?;

    let mut active: desahucio::ActiveModel = existing.clone().into();
    let now = Utc::now();

    if let Some(ref new_estado) = input.estado {
        validate_estado_transition(&existing.estado, new_estado)?;
        validate_time_gap(&existing.estado, new_estado, existing.updated_at, now)?;

        if new_estado == "completado" && input.fecha_resolucion.is_none() {
            return Err(AppError::Validation(
                "Se requiere fecha_resolucion cuando el estado es completado".to_string(),
            ));
        }

        active.estado = Set(new_estado.clone());
    }

    if let Some(fecha) = input.fecha_resolucion {
        active.fecha_resolucion = Set(Some(fecha));
    }

    if let Some(ref motivo) = input.motivo {
        active.motivo = Set(motivo.clone());
    }

    active.updated_at = Set(now.into());

    let updated = active.update(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "desahucio".to_string(),
            entity_id: id,
            accion: "actualizar".to_string(),
            cambios: serde_json::json!({
                "estado": input.estado,
                "fecha_resolucion": input.fecha_resolucion,
                "motivo": input.motivo
            }),
        },
    )
    .await;

    Ok(DesahucioResponse::from(updated))
}

pub async fn list(
    db: &DatabaseConnection,
    org_id: Uuid,
    query: DesahucioListQuery,
) -> Result<PaginatedResponse<DesahucioResponse>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let mut select = desahucio::Entity::find().filter(desahucio::Column::OrganizacionId.eq(org_id));

    if let Some(contrato_id) = query.contrato_id {
        select = select.filter(desahucio::Column::ContratoId.eq(contrato_id));
    }

    if let Some(ref estado) = query.estado {
        select = select.filter(desahucio::Column::Estado.eq(estado));
    }

    let paginator = select
        .order_by_desc(desahucio::Column::CreatedAt)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records.into_iter().map(DesahucioResponse::from).collect(),
        total,
        page,
        per_page,
    })
}

pub fn validate_time_gap(
    from: &str,
    to: &str,
    updated_at: chrono::DateTime<chrono::FixedOffset>,
    now: chrono::DateTime<Utc>,
) -> Result<(), AppError> {
    let required_days: i64 = match (from, to) {
        ("iniciado", "en_progreso") => 30,
        ("iniciado", "completado") | ("en_progreso", "completado") => 90,
        _ => return Ok(()),
    };

    let elapsed_days = (now - updated_at.to_utc()).num_days();
    if elapsed_days < required_days {
        let remaining = required_days - elapsed_days;
        return Err(AppError::Validation(format!(
            "Deben transcurrir al menos {required_days} días para la transición '{from}' → '{to}'. Faltan {remaining} días."
        )));
    }

    Ok(())
}

pub fn validate_estado_transition(from: &str, to: &str) -> Result<(), AppError> {
    let valid = matches!(
        (from, to),
        ("iniciado", "en_progreso" | "completado") | ("en_progreso", "completado")
    );

    if !valid {
        return Err(AppError::Validation(format!(
            "Transición de estado inválida: '{from}' → '{to}'. Transiciones válidas: iniciado→en_progreso, en_progreso→completado, iniciado→completado"
        )));
    }

    Ok(())
}
