use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::{inquilino, nota_mantenimiento, propiedad, solicitud_mantenimiento, unidad};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::mantenimiento::{
    CreateSolicitudRequest, NotaResponse, SolicitudListQuery, SolicitudResponse,
    UpdateSolicitudRequest,
};
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::validation::{validate_enum, MONEDAS};

const ESTADOS_SOLICITUD: &[&str] = &["pendiente", "en_progreso", "completado"];
const PRIORIDADES: &[&str] = &["baja", "media", "alta", "urgente"];

impl From<solicitud_mantenimiento::Model> for SolicitudResponse {
    fn from(m: solicitud_mantenimiento::Model) -> Self {
        Self {
            id: m.id,
            propiedad_id: m.propiedad_id,
            unidad_id: m.unidad_id,
            inquilino_id: m.inquilino_id,
            titulo: m.titulo,
            descripcion: m.descripcion,
            estado: m.estado,
            prioridad: m.prioridad,
            nombre_proveedor: m.nombre_proveedor,
            telefono_proveedor: m.telefono_proveedor,
            email_proveedor: m.email_proveedor,
            costo_monto: m.costo_monto,
            costo_moneda: m.costo_moneda,
            fecha_inicio: m.fecha_inicio.map(Into::into),
            fecha_fin: m.fecha_fin.map(Into::into),
            notas: None,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}

impl From<nota_mantenimiento::Model> for NotaResponse {
    fn from(m: nota_mantenimiento::Model) -> Self {
        Self {
            id: m.id,
            solicitud_id: m.solicitud_id,
            autor_id: m.autor_id,
            contenido: m.contenido,
            created_at: m.created_at.into(),
        }
    }
}

pub fn validar_transicion(estado_actual: &str, nuevo_estado: &str) -> Result<(), AppError> {
    match (estado_actual, nuevo_estado) {
        ("pendiente", "en_progreso") | ("en_progreso", "completado") => Ok(()),
        ("pendiente", "completado") => Err(AppError::Validation(
            "La solicitud debe pasar por 'en_progreso' antes de completarse".to_string(),
        )),
        ("completado", _) => Err(AppError::Validation(
            "Las solicitudes completadas no pueden revertirse".to_string(),
        )),
        ("en_progreso", "pendiente") => Err(AppError::Validation(
            "No se puede revertir una solicitud en progreso a pendiente".to_string(),
        )),
        _ => Err(AppError::Validation(format!(
            "Transición de estado no válida: '{estado_actual}' → '{nuevo_estado}'"
        ))),
    }
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    input: CreateSolicitudRequest,
    usuario_id: Uuid,
) -> Result<SolicitudResponse, AppError> {
    if input.titulo.trim().is_empty() {
        return Err(AppError::Validation("El título es requerido".to_string()));
    }

    propiedad::Entity::find_by_id(input.propiedad_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    if let Some(unidad_id) = input.unidad_id {
        let unidad_record = unidad::Entity::find_by_id(unidad_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Unidad no encontrada".to_string()))?;
        if unidad_record.propiedad_id != input.propiedad_id {
            return Err(AppError::Validation(
                "La unidad no pertenece a la propiedad indicada".to_string(),
            ));
        }
    }

    if let Some(inquilino_id) = input.inquilino_id {
        inquilino::Entity::find_by_id(inquilino_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Inquilino no encontrado".to_string()))?;
    }

    let prioridad = input.prioridad.unwrap_or_else(|| "media".to_string());
    validate_enum("prioridad", &prioridad, PRIORIDADES)?;

    if let Some(ref moneda) = input.costo_moneda {
        validate_enum("moneda", moneda, MONEDAS)?;
    }
    if let Some(monto) = input.costo_monto
        && monto < Decimal::ZERO
    {
        return Err(AppError::Validation(
            "El monto debe ser mayor o igual a cero".to_string(),
        ));
    }

    let now = Utc::now().into();
    let id = Uuid::new_v4();

    let model = solicitud_mantenimiento::ActiveModel {
        id: Set(id),
        propiedad_id: Set(input.propiedad_id),
        unidad_id: Set(input.unidad_id),
        inquilino_id: Set(input.inquilino_id),
        titulo: Set(input.titulo),
        descripcion: Set(input.descripcion),
        estado: Set("pendiente".to_string()),
        prioridad: Set(prioridad),
        nombre_proveedor: Set(input.nombre_proveedor),
        telefono_proveedor: Set(input.telefono_proveedor),
        email_proveedor: Set(input.email_proveedor),
        costo_monto: Set(input.costo_monto),
        costo_moneda: Set(input.costo_moneda),
        fecha_inicio: Set(None),
        fecha_fin: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = model.insert(db).await?;

    auditoria::registrar_best_effort(db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "solicitud_mantenimiento".to_string(),
            entity_id: id,
            accion: "crear".to_string(),
            cambios: serde_json::json!(SolicitudResponse::from(record.clone())),
        },
    )
    .await;

    Ok(SolicitudResponse::from(record))
}

pub async fn get_by_id(db: &DatabaseConnection, id: Uuid) -> Result<SolicitudResponse, AppError> {
    let record = solicitud_mantenimiento::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound("Solicitud de mantenimiento no encontrada".to_string())
        })?;

    let notas = nota_mantenimiento::Entity::find()
        .filter(nota_mantenimiento::Column::SolicitudId.eq(id))
        .order_by_desc(nota_mantenimiento::Column::CreatedAt)
        .all(db)
        .await?;

    let mut response = SolicitudResponse::from(record);
    response.notas = Some(notas.into_iter().map(NotaResponse::from).collect());
    Ok(response)
}

pub async fn list(
    db: &DatabaseConnection,
    query: SolicitudListQuery,
) -> Result<PaginatedResponse<SolicitudResponse>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let mut select = solicitud_mantenimiento::Entity::find();

    if let Some(ref estado) = query.estado {
        select = select.filter(solicitud_mantenimiento::Column::Estado.eq(estado));
    }
    if let Some(ref prioridad) = query.prioridad {
        select = select.filter(solicitud_mantenimiento::Column::Prioridad.eq(prioridad));
    }
    if let Some(propiedad_id) = query.propiedad_id {
        select = select.filter(solicitud_mantenimiento::Column::PropiedadId.eq(propiedad_id));
    }

    let paginator = select
        .order_by_desc(solicitud_mantenimiento::Column::CreatedAt)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records.into_iter().map(SolicitudResponse::from).collect(),
        total,
        page,
        per_page,
    })
}

pub async fn update<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
    input: UpdateSolicitudRequest,
    usuario_id: Uuid,
) -> Result<SolicitudResponse, AppError> {
    if let Some(ref prioridad) = input.prioridad {
        validate_enum("prioridad", prioridad, PRIORIDADES)?;
    }
    if let Some(ref moneda) = input.costo_moneda {
        validate_enum("moneda", moneda, MONEDAS)?;
    }
    if let Some(monto) = input.costo_monto
        && monto < Decimal::ZERO
    {
        return Err(AppError::Validation(
            "El monto debe ser mayor o igual a cero".to_string(),
        ));
    }

    let existing = solicitud_mantenimiento::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound("Solicitud de mantenimiento no encontrada".to_string())
        })?;

    let mut active: solicitud_mantenimiento::ActiveModel = existing.into();

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
    if let Some(costo_monto) = input.costo_monto {
        active.costo_monto = Set(Some(costo_monto));
    }
    if let Some(costo_moneda) = input.costo_moneda {
        active.costo_moneda = Set(Some(costo_moneda));
    }
    if let Some(unidad_id) = input.unidad_id {
        active.unidad_id = Set(Some(unidad_id));
    }
    if let Some(inquilino_id) = input.inquilino_id {
        active.inquilino_id = Set(Some(inquilino_id));
    }

    active.updated_at = Set(Utc::now().into());

    let updated = active.update(db).await?;

    auditoria::registrar_best_effort(db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "solicitud_mantenimiento".to_string(),
            entity_id: id,
            accion: "actualizar".to_string(),
            cambios: serde_json::json!(SolicitudResponse::from(updated.clone())),
        },
    )
    .await;

    Ok(SolicitudResponse::from(updated))
}

pub async fn cambiar_estado<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
    nuevo_estado: &str,
    usuario_id: Uuid,
) -> Result<SolicitudResponse, AppError> {
    validate_enum("estado", nuevo_estado, ESTADOS_SOLICITUD)?;

    let existing = solicitud_mantenimiento::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound("Solicitud de mantenimiento no encontrada".to_string())
        })?;

    validar_transicion(&existing.estado, nuevo_estado)?;

    let mut active: solicitud_mantenimiento::ActiveModel = existing.into();
    active.estado = Set(nuevo_estado.to_string());

    let now = Utc::now().into();
    if nuevo_estado == "en_progreso" {
        active.fecha_inicio = Set(Some(now));
    }
    if nuevo_estado == "completado" {
        active.fecha_fin = Set(Some(now));
    }

    active.updated_at = Set(Utc::now().into());

    let updated = active.update(db).await?;

    auditoria::registrar_best_effort(db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "solicitud_mantenimiento".to_string(),
            entity_id: id,
            accion: "cambiar_estado".to_string(),
            cambios: serde_json::json!({
                "nuevo_estado": nuevo_estado,
            }),
        },
    )
    .await;

    Ok(SolicitudResponse::from(updated))
}

pub async fn delete<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
    usuario_id: Uuid,
) -> Result<(), AppError> {
    let result = solicitud_mantenimiento::Entity::delete_by_id(id)
        .exec(db)
        .await?;
    if result.rows_affected == 0 {
        return Err(AppError::NotFound(
            "Solicitud de mantenimiento no encontrada".to_string(),
        ));
    }

    auditoria::registrar_best_effort(db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "solicitud_mantenimiento".to_string(),
            entity_id: id,
            accion: "eliminar".to_string(),
            cambios: serde_json::json!({ "id": id }),
        },
    )
    .await;

    Ok(())
}

pub async fn agregar_nota<C: ConnectionTrait>(
    db: &C,
    solicitud_id: Uuid,
    contenido: String,
    usuario_id: Uuid,
) -> Result<NotaResponse, AppError> {
    solicitud_mantenimiento::Entity::find_by_id(solicitud_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound("Solicitud de mantenimiento no encontrada".to_string())
        })?;

    if contenido.trim().is_empty() {
        return Err(AppError::Validation(
            "El contenido de la nota es requerido".to_string(),
        ));
    }

    let now = Utc::now().into();
    let id = Uuid::new_v4();

    let model = nota_mantenimiento::ActiveModel {
        id: Set(id),
        solicitud_id: Set(solicitud_id),
        autor_id: Set(usuario_id),
        contenido: Set(contenido),
        created_at: Set(now),
    };

    let record = model.insert(db).await?;

    auditoria::registrar_best_effort(db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "nota_mantenimiento".to_string(),
            entity_id: id,
            accion: "crear".to_string(),
            cambios: serde_json::json!(NotaResponse::from(record.clone())),
        },
    )
    .await;

    Ok(NotaResponse::from(record))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use actix_web::error::ResponseError;
    use actix_web::http::StatusCode;
    use chrono::FixedOffset;
    use sea_orm::entity::prelude::DateTimeWithTimeZone;

    fn make_solicitud_model() -> solicitud_mantenimiento::Model {
        let tz = FixedOffset::east_opt(0).unwrap();
        let now = Utc::now().with_timezone(&tz);
        solicitud_mantenimiento::Model {
            id: Uuid::new_v4(),
            propiedad_id: Uuid::new_v4(),
            unidad_id: Some(Uuid::new_v4()),
            inquilino_id: Some(Uuid::new_v4()),
            titulo: "Reparar tubería".to_string(),
            descripcion: Some("Fuga en el baño principal".to_string()),
            estado: "pendiente".to_string(),
            prioridad: "alta".to_string(),
            nombre_proveedor: Some("Plomería Express".to_string()),
            telefono_proveedor: Some("809-555-1234".to_string()),
            email_proveedor: Some("plomeria@example.com".to_string()),
            costo_monto: Some(Decimal::new(150_050, 2)),
            costo_moneda: Some("DOP".to_string()),
            fecha_inicio: Some(now),
            fecha_fin: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_nota_model() -> nota_mantenimiento::Model {
        let tz = FixedOffset::east_opt(0).unwrap();
        let now: DateTimeWithTimeZone = Utc::now().with_timezone(&tz);
        nota_mantenimiento::Model {
            id: Uuid::new_v4(),
            solicitud_id: Uuid::new_v4(),
            autor_id: Uuid::new_v4(),
            contenido: "Se contactó al proveedor".to_string(),
            created_at: now,
        }
    }

    #[test]
    fn validar_transicion_pendiente_a_en_progreso_ok() {
        assert!(validar_transicion("pendiente", "en_progreso").is_ok());
    }

    #[test]
    fn validar_transicion_en_progreso_a_completado_ok() {
        assert!(validar_transicion("en_progreso", "completado").is_ok());
    }

    #[test]
    fn validar_transicion_pendiente_a_completado_err() {
        let result = validar_transicion("pendiente", "completado");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(
            err.to_string()
                .contains("debe pasar por 'en_progreso' antes de completarse")
        );
    }

    #[test]
    fn validar_transicion_completado_a_pendiente_err() {
        let result = validar_transicion("completado", "pendiente");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err.to_string().contains("completadas no pueden revertirse"));
    }

    #[test]
    fn validar_transicion_completado_a_en_progreso_err() {
        let result = validar_transicion("completado", "en_progreso");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("completadas no pueden revertirse"));
    }

    #[test]
    fn validar_transicion_completado_a_completado_err() {
        let result = validar_transicion("completado", "completado");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("completadas no pueden revertirse"));
    }

    #[test]
    fn validar_transicion_en_progreso_a_pendiente_err() {
        let result = validar_transicion("en_progreso", "pendiente");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(
            err.to_string()
                .contains("No se puede revertir una solicitud en progreso a pendiente")
        );
    }

    #[test]
    fn from_solicitud_model_converts_all_fields() {
        let model = make_solicitud_model();
        let original_id = model.id;
        let original_propiedad_id = model.propiedad_id;
        let original_unidad_id = model.unidad_id;
        let original_inquilino_id = model.inquilino_id;

        let resp = SolicitudResponse::from(model);
        assert_eq!(resp.id, original_id);
        assert_eq!(resp.propiedad_id, original_propiedad_id);
        assert_eq!(resp.unidad_id, original_unidad_id);
        assert_eq!(resp.inquilino_id, original_inquilino_id);
        assert_eq!(resp.titulo, "Reparar tubería");
        assert_eq!(
            resp.descripcion.as_deref(),
            Some("Fuga en el baño principal")
        );
        assert_eq!(resp.estado, "pendiente");
        assert_eq!(resp.prioridad, "alta");
        assert_eq!(resp.nombre_proveedor.as_deref(), Some("Plomería Express"));
        assert_eq!(resp.telefono_proveedor.as_deref(), Some("809-555-1234"));
        assert_eq!(
            resp.email_proveedor.as_deref(),
            Some("plomeria@example.com")
        );
        assert_eq!(resp.costo_monto, Some(Decimal::new(150_050, 2)));
        assert_eq!(resp.costo_moneda.as_deref(), Some("DOP"));
        assert!(resp.fecha_inicio.is_some());
        assert!(resp.fecha_fin.is_none());
        assert!(resp.notas.is_none());
    }

    #[test]
    fn from_solicitud_model_with_none_optional_fields() {
        let tz = FixedOffset::east_opt(0).unwrap();
        let now = Utc::now().with_timezone(&tz);
        let model = solicitud_mantenimiento::Model {
            id: Uuid::new_v4(),
            propiedad_id: Uuid::new_v4(),
            unidad_id: None,
            inquilino_id: None,
            titulo: "Pintar paredes".to_string(),
            descripcion: None,
            estado: "pendiente".to_string(),
            prioridad: "media".to_string(),
            nombre_proveedor: None,
            telefono_proveedor: None,
            email_proveedor: None,
            costo_monto: None,
            costo_moneda: None,
            fecha_inicio: None,
            fecha_fin: None,
            created_at: now,
            updated_at: now,
        };

        let resp = SolicitudResponse::from(model);
        assert!(resp.unidad_id.is_none());
        assert!(resp.inquilino_id.is_none());
        assert!(resp.descripcion.is_none());
        assert!(resp.nombre_proveedor.is_none());
        assert!(resp.costo_monto.is_none());
        assert!(resp.costo_moneda.is_none());
        assert!(resp.fecha_inicio.is_none());
        assert!(resp.fecha_fin.is_none());
    }

    #[test]
    fn from_nota_model_converts_all_fields() {
        let model = make_nota_model();
        let original_id = model.id;
        let original_solicitud_id = model.solicitud_id;
        let original_autor_id = model.autor_id;

        let resp = NotaResponse::from(model);
        assert_eq!(resp.id, original_id);
        assert_eq!(resp.solicitud_id, original_solicitud_id);
        assert_eq!(resp.autor_id, original_autor_id);
        assert_eq!(resp.contenido, "Se contactó al proveedor");
    }

    #[test]
    fn from_nota_model_converts_created_at_to_utc() {
        let model = make_nota_model();
        let resp = NotaResponse::from(model);
        assert_eq!(resp.created_at.timezone(), Utc);
    }
}