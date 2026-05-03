use std::collections::HashMap;

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::{contrato, documento, inquilino, notificacion, pago, propiedad, usuario};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::notificacion::{
    GenerarNotificacionesResponse, NotificacionListQuery, NotificacionResponse, PagoVencido,
};

pub const TIPOS_NOTIFICACION: &[&str] = &[
    "pago_vencido",
    "contrato_por_vencer",
    "documento_vencido",
    "mantenimiento_actualizado",
];

pub const DIAS_ANTICIPACION: i64 = 30;

impl From<notificacion::Model> for NotificacionResponse {
    fn from(m: notificacion::Model) -> Self {
        Self {
            id: m.id,
            tipo: m.tipo,
            titulo: m.titulo,
            mensaje: m.mensaje,
            leida: m.leida,
            entity_type: m.entity_type,
            entity_id: m.entity_id,
            usuario_id: m.usuario_id,
            created_at: m.created_at.into(),
        }
    }
}

pub async fn listar_pagos_vencidos(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<Vec<PagoVencido>, AppError> {
    let today = Utc::now().date_naive();

    let pagos = pago::Entity::find()
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .filter(pago::Column::Estado.eq("pendiente"))
        .filter(pago::Column::FechaVencimiento.lt(today))
        .all(db)
        .await?;

    if pagos.is_empty() {
        return Ok(vec![]);
    }

    let contrato_ids: Vec<uuid::Uuid> = pagos.iter().map(|p| p.contrato_id).collect();
    let contratos = contrato::Entity::find()
        .filter(contrato::Column::Id.is_in(contrato_ids))
        .all(db)
        .await?;

    let propiedad_ids: Vec<uuid::Uuid> = contratos.iter().map(|c| c.propiedad_id).collect();
    let inquilino_ids: Vec<uuid::Uuid> = contratos.iter().map(|c| c.inquilino_id).collect();

    let propiedades = propiedad::Entity::find()
        .filter(propiedad::Column::Id.is_in(propiedad_ids))
        .all(db)
        .await?;

    let inquilinos = inquilino::Entity::find()
        .filter(inquilino::Column::Id.is_in(inquilino_ids))
        .all(db)
        .await?;

    let mut results: Vec<PagoVencido> = pagos
        .iter()
        .filter_map(|p| {
            let contrato_model = contratos.iter().find(|c| c.id == p.contrato_id)?;

            let prop = propiedades
                .iter()
                .find(|pr| pr.id == contrato_model.propiedad_id);

            let inq = inquilinos
                .iter()
                .find(|i| i.id == contrato_model.inquilino_id);

            let dias_vencido = (today - p.fecha_vencimiento).num_days();

            Some(PagoVencido {
                pago_id: p.id,
                propiedad_titulo: prop.map(|pr| pr.titulo.clone()).unwrap_or_default(),
                inquilino_nombre: inq.map(|i| i.nombre.clone()).unwrap_or_default(),
                inquilino_apellido: inq.map(|i| i.apellido.clone()).unwrap_or_default(),
                monto: p.monto,
                moneda: p.moneda.clone(),
                dias_vencido,
            })
        })
        .collect();

    results.sort_by_key(|b| std::cmp::Reverse(b.dias_vencido));

    Ok(results)
}

pub async fn listar(
    db: &DatabaseConnection,
    usuario_id: Uuid,
    query: NotificacionListQuery,
) -> Result<PaginatedResponse<NotificacionResponse>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let mut select =
        notificacion::Entity::find().filter(notificacion::Column::UsuarioId.eq(usuario_id));

    if let Some(leida) = query.leida {
        select = select.filter(notificacion::Column::Leida.eq(leida));
    }
    if let Some(ref tipo) = query.tipo {
        select = select.filter(notificacion::Column::Tipo.eq(tipo));
    }

    let paginator = select
        .order_by_desc(notificacion::Column::CreatedAt)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records
            .into_iter()
            .map(NotificacionResponse::from)
            .collect(),
        total,
        page,
        per_page,
    })
}

pub async fn conteo_no_leidas(db: &DatabaseConnection, usuario_id: Uuid) -> Result<u64, AppError> {
    let count = notificacion::Entity::find()
        .filter(notificacion::Column::UsuarioId.eq(usuario_id))
        .filter(notificacion::Column::Leida.eq(false))
        .count(db)
        .await?;

    Ok(count)
}

pub async fn marcar_leida(
    db: &DatabaseConnection,
    id: Uuid,
    usuario_id: Uuid,
) -> Result<NotificacionResponse, AppError> {
    let record = notificacion::Entity::find_by_id(id)
        .filter(notificacion::Column::UsuarioId.eq(usuario_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Notificación no encontrada".to_string()))?;

    if record.leida {
        return Ok(NotificacionResponse::from(record));
    }

    let mut active: notificacion::ActiveModel = record.into();
    active.leida = Set(true);

    let updated = active.update(db).await?;
    Ok(NotificacionResponse::from(updated))
}

pub async fn marcar_todas_leidas(
    db: &DatabaseConnection,
    usuario_id: Uuid,
) -> Result<u64, AppError> {
    let result = notificacion::Entity::update_many()
        .col_expr(
            notificacion::Column::Leida,
            sea_orm::sea_query::Expr::value(true),
        )
        .filter(notificacion::Column::UsuarioId.eq(usuario_id))
        .filter(notificacion::Column::Leida.eq(false))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

pub async fn usuarios_activos_organizacion(
    db: &DatabaseConnection,
    organizacion_id: Uuid,
) -> Result<Vec<Uuid>, AppError> {
    let usuarios = usuario::Entity::find()
        .filter(usuario::Column::OrganizacionId.eq(organizacion_id))
        .filter(usuario::Column::Activo.eq(true))
        .all(db)
        .await?;

    Ok(usuarios.into_iter().map(|u| u.id).collect())
}

async fn usuarios_activos_organizacion_generic<C: sea_orm::ConnectionTrait>(
    db: &C,
    organizacion_id: Uuid,
) -> Result<Vec<Uuid>, AppError> {
    let usuarios = usuario::Entity::find()
        .filter(usuario::Column::OrganizacionId.eq(organizacion_id))
        .filter(usuario::Column::Activo.eq(true))
        .all(db)
        .await?;

    Ok(usuarios.into_iter().map(|u| u.id).collect())
}

pub async fn existe_notificacion(
    db: &DatabaseConnection,
    tipo: &str,
    entity_type: &str,
    entity_id: Uuid,
    usuario_id: Uuid,
) -> Result<bool, AppError> {
    let count = notificacion::Entity::find()
        .filter(notificacion::Column::Tipo.eq(tipo))
        .filter(notificacion::Column::EntityType.eq(entity_type))
        .filter(notificacion::Column::EntityId.eq(entity_id))
        .filter(notificacion::Column::UsuarioId.eq(usuario_id))
        .count(db)
        .await?;

    Ok(count > 0)
}

pub async fn generar_pagos_vencidos(
    db: &DatabaseConnection,
    organizacion_id: Uuid,
) -> Result<u64, AppError> {
    let today = Utc::now().date_naive();

    let pagos = pago::Entity::find()
        .filter(pago::Column::OrganizacionId.eq(organizacion_id))
        .filter(pago::Column::Estado.eq("pendiente"))
        .filter(pago::Column::FechaVencimiento.lt(today))
        .all(db)
        .await?;

    if pagos.is_empty() {
        return Ok(0);
    }

    // Batch-fetch contratos
    let contrato_ids: Vec<Uuid> = pagos.iter().map(|p| p.contrato_id).collect();
    let contratos = contrato::Entity::find()
        .filter(contrato::Column::Id.is_in(contrato_ids))
        .all(db)
        .await?;
    let contrato_map: HashMap<Uuid, &contrato::Model> =
        contratos.iter().map(|c| (c.id, c)).collect();

    // Batch-fetch propiedades
    let propiedad_ids: Vec<Uuid> = contratos.iter().map(|c| c.propiedad_id).collect();
    let propiedades = propiedad::Entity::find()
        .filter(propiedad::Column::Id.is_in(propiedad_ids))
        .all(db)
        .await?;
    let propiedad_map: HashMap<Uuid, &propiedad::Model> =
        propiedades.iter().map(|p| (p.id, p)).collect();

    let usuario_ids = usuarios_activos_organizacion(db, organizacion_id).await?;
    if usuario_ids.is_empty() {
        return Ok(0);
    }

    let now = Utc::now().into();
    let mut count: u64 = 0;

    for pago_model in &pagos {
        let propiedad_titulo = contrato_map
            .get(&pago_model.contrato_id)
            .and_then(|c| propiedad_map.get(&c.propiedad_id))
            .map_or("Propiedad", |p| p.titulo.as_str());

        let dias_vencido = (today - pago_model.fecha_vencimiento).num_days();

        for &uid in &usuario_ids {
            if existe_notificacion(db, "pago_vencido", "pago", pago_model.id, uid).await? {
                continue;
            }

            let active = notificacion::ActiveModel {
                id: Set(Uuid::new_v4()),
                tipo: Set("pago_vencido".to_string()),
                titulo: Set(format!("Pago vencido - {propiedad_titulo}")),
                mensaje: Set(format!(
                    "El pago de {} {} tiene {} días de vencido",
                    pago_model.monto, pago_model.moneda, dias_vencido
                )),
                leida: Set(false),
                entity_type: Set("pago".to_string()),
                entity_id: Set(pago_model.id),
                usuario_id: Set(uid),
                organizacion_id: Set(organizacion_id),
                created_at: Set(now),
            };
            active.insert(db).await?;
            count += 1;
        }
    }

    Ok(count)
}

pub async fn generar_contratos_por_vencer(
    db: &DatabaseConnection,
    organizacion_id: Uuid,
) -> Result<u64, AppError> {
    let today = Utc::now().date_naive();
    let limit_date = today + chrono::Duration::days(DIAS_ANTICIPACION);

    let contratos = contrato::Entity::find()
        .filter(contrato::Column::OrganizacionId.eq(organizacion_id))
        .filter(contrato::Column::Estado.eq("activo"))
        .filter(contrato::Column::FechaFin.gte(today))
        .filter(contrato::Column::FechaFin.lte(limit_date))
        .all(db)
        .await?;

    if contratos.is_empty() {
        return Ok(0);
    }

    // Batch-fetch propiedades
    let propiedad_ids: Vec<Uuid> = contratos.iter().map(|c| c.propiedad_id).collect();
    let propiedades = propiedad::Entity::find()
        .filter(propiedad::Column::Id.is_in(propiedad_ids))
        .all(db)
        .await?;
    let propiedad_map: HashMap<Uuid, &propiedad::Model> =
        propiedades.iter().map(|p| (p.id, p)).collect();

    let usuario_ids = usuarios_activos_organizacion(db, organizacion_id).await?;
    if usuario_ids.is_empty() {
        return Ok(0);
    }

    let now = Utc::now().into();
    let mut count: u64 = 0;

    for contrato_model in &contratos {
        let propiedad_titulo = propiedad_map
            .get(&contrato_model.propiedad_id)
            .map_or("Propiedad", |p| p.titulo.as_str());

        let dias_restantes = (contrato_model.fecha_fin - today).num_days();

        for &uid in &usuario_ids {
            if existe_notificacion(
                db,
                "contrato_por_vencer",
                "contrato",
                contrato_model.id,
                uid,
            )
            .await?
            {
                continue;
            }

            let active = notificacion::ActiveModel {
                id: Set(Uuid::new_v4()),
                tipo: Set("contrato_por_vencer".to_string()),
                titulo: Set(format!("Contrato por vencer - {propiedad_titulo}")),
                mensaje: Set(format!(
                    "El contrato vence el {} ({} días restantes)",
                    contrato_model.fecha_fin.format("%d/%m/%Y"),
                    dias_restantes
                )),
                leida: Set(false),
                entity_type: Set("contrato".to_string()),
                entity_id: Set(contrato_model.id),
                usuario_id: Set(uid),
                organizacion_id: Set(organizacion_id),
                created_at: Set(now),
            };
            active.insert(db).await?;
            count += 1;
        }
    }

    Ok(count)
}

pub async fn generar_documentos_vencidos(
    db: &DatabaseConnection,
    organizacion_id: Uuid,
) -> Result<u64, AppError> {
    let today = Utc::now().date_naive();
    let limit_date = today + chrono::Duration::days(DIAS_ANTICIPACION);

    // Documents are polymorphic (no organizacion_id). Fetch all expiring documents,
    // then resolve org membership through their parent entities.
    let documentos = documento::Entity::find()
        .filter(documento::Column::FechaVencimiento.is_not_null())
        .filter(documento::Column::FechaVencimiento.lte(limit_date))
        .all(db)
        .await?;

    if documentos.is_empty() {
        return Ok(0);
    }

    // Group document parent entity IDs by entity_type to batch-resolve org membership
    let mut propiedad_doc_ids: Vec<Uuid> = Vec::new();
    let mut contrato_doc_ids: Vec<Uuid> = Vec::new();

    for doc in &documentos {
        match doc.entity_type.as_str() {
            "propiedad" => propiedad_doc_ids.push(doc.entity_id),
            "contrato" => contrato_doc_ids.push(doc.entity_id),
            _ => {}
        }
    }

    // Batch-fetch parent entities and build org lookup sets
    let mut entity_org_map: HashMap<(String, Uuid), Uuid> = HashMap::new();

    if !propiedad_doc_ids.is_empty() {
        let props = propiedad::Entity::find()
            .filter(propiedad::Column::Id.is_in(propiedad_doc_ids))
            .all(db)
            .await?;
        for p in &props {
            entity_org_map.insert(("propiedad".to_string(), p.id), p.organizacion_id);
        }
    }

    if !contrato_doc_ids.is_empty() {
        let ctrs = contrato::Entity::find()
            .filter(contrato::Column::Id.is_in(contrato_doc_ids))
            .all(db)
            .await?;
        for c in &ctrs {
            entity_org_map.insert(("contrato".to_string(), c.id), c.organizacion_id);
        }
    }

    // Inquilinos don't have organizacion_id — skip org filtering for those
    // (they won't match any org and will be excluded)

    // Filter documents to those belonging to the given organization
    let org_docs: Vec<&documento::Model> = documentos
        .iter()
        .filter(|doc| {
            entity_org_map
                .get(&(doc.entity_type.clone(), doc.entity_id))
                .is_some_and(|&oid| oid == organizacion_id)
        })
        .collect();

    if org_docs.is_empty() {
        return Ok(0);
    }

    let usuario_ids = usuarios_activos_organizacion(db, organizacion_id).await?;
    if usuario_ids.is_empty() {
        return Ok(0);
    }

    let now = Utc::now().into();
    let mut count: u64 = 0;

    for doc in &org_docs {
        let Some(fecha_venc) = doc.fecha_vencimiento else {
            continue;
        };

        for &uid in &usuario_ids {
            if existe_notificacion(db, "documento_vencido", "documento", doc.id, uid).await? {
                continue;
            }

            let active = notificacion::ActiveModel {
                id: Set(Uuid::new_v4()),
                tipo: Set("documento_vencido".to_string()),
                titulo: Set(format!("Documento por vencer - {}", doc.filename)),
                mensaje: Set(format!(
                    "El documento vence el {}",
                    fecha_venc.format("%d/%m/%Y")
                )),
                leida: Set(false),
                entity_type: Set("documento".to_string()),
                entity_id: Set(doc.id),
                usuario_id: Set(uid),
                organizacion_id: Set(organizacion_id),
                created_at: Set(now),
            };
            active.insert(db).await?;
            count += 1;
        }
    }

    Ok(count)
}

pub async fn generar_notificaciones(
    db: &DatabaseConnection,
    organizacion_id: Uuid,
) -> Result<GenerarNotificacionesResponse, AppError> {
    let pago_vencido = generar_pagos_vencidos(db, organizacion_id).await?;
    let contrato_por_vencer = generar_contratos_por_vencer(db, organizacion_id).await?;
    let documento_vencido = generar_documentos_vencidos(db, organizacion_id).await?;

    Ok(GenerarNotificacionesResponse {
        pago_vencido,
        contrato_por_vencer,
        documento_vencido,
        total: pago_vencido + contrato_por_vencer + documento_vencido,
    })
}

pub async fn crear_notificacion_mantenimiento<C: sea_orm::ConnectionTrait>(
    db: &C,
    solicitud_id: Uuid,
    titulo_solicitud: &str,
    estado_anterior: &str,
    estado_nuevo: &str,
    organizacion_id: Uuid,
) -> Result<u64, AppError> {
    let usuario_ids = usuarios_activos_organizacion_generic(db, organizacion_id).await?;
    if usuario_ids.is_empty() {
        return Ok(0);
    }

    let now = Utc::now().into();
    let mut count: u64 = 0;

    for uid in usuario_ids {
        let active = notificacion::ActiveModel {
            id: Set(Uuid::new_v4()),
            tipo: Set("mantenimiento_actualizado".to_string()),
            titulo: Set(format!("Mantenimiento actualizado - {titulo_solicitud}")),
            mensaje: Set(format!(
                "El estado cambió de {estado_anterior} a {estado_nuevo}"
            )),
            leida: Set(false),
            entity_type: Set("solicitud_mantenimiento".to_string()),
            entity_id: Set(solicitud_id),
            usuario_id: Set(uid),
            organizacion_id: Set(organizacion_id),
            created_at: Set(now),
        };
        active.insert(db).await?;
        count += 1;
    }

    Ok(count)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::FixedOffset;
    use sea_orm::entity::prelude::DateTimeWithTimeZone;

    fn make_notificacion_model() -> notificacion::Model {
        let tz = FixedOffset::east_opt(0).unwrap();
        let now: DateTimeWithTimeZone = Utc::now().with_timezone(&tz);
        notificacion::Model {
            id: Uuid::new_v4(),
            tipo: "pago_vencido".to_string(),
            titulo: "Pago vencido - Apartamento Centro".to_string(),
            mensaje: "El pago de 25000 DOP tiene 15 días de vencido".to_string(),
            leida: false,
            entity_type: "pago".to_string(),
            entity_id: Uuid::new_v4(),
            usuario_id: Uuid::new_v4(),
            organizacion_id: Uuid::new_v4(),
            created_at: now,
        }
    }

    #[test]
    fn from_model_converts_all_fields() {
        let model = make_notificacion_model();
        let original_id = model.id;
        let original_entity_id = model.entity_id;
        let original_usuario_id = model.usuario_id;

        let resp = NotificacionResponse::from(model);
        assert_eq!(resp.id, original_id);
        assert_eq!(resp.tipo, "pago_vencido");
        assert_eq!(resp.titulo, "Pago vencido - Apartamento Centro");
        assert_eq!(
            resp.mensaje,
            "El pago de 25000 DOP tiene 15 días de vencido"
        );
        assert!(!resp.leida);
        assert_eq!(resp.entity_type, "pago");
        assert_eq!(resp.entity_id, original_entity_id);
        assert_eq!(resp.usuario_id, original_usuario_id);
    }

    #[test]
    fn from_model_converts_created_at_to_utc() {
        let model = make_notificacion_model();
        let resp = NotificacionResponse::from(model);
        assert_eq!(resp.created_at.timezone(), Utc);
    }

    #[test]
    fn from_model_with_leida_true() {
        let tz = FixedOffset::east_opt(0).unwrap();
        let now: DateTimeWithTimeZone = Utc::now().with_timezone(&tz);
        let model = notificacion::Model {
            id: Uuid::new_v4(),
            tipo: "contrato_por_vencer".to_string(),
            titulo: "Contrato por vencer - Casa Playa".to_string(),
            mensaje: "El contrato vence el 15/06/2025 (10 días restantes)".to_string(),
            leida: true,
            entity_type: "contrato".to_string(),
            entity_id: Uuid::new_v4(),
            usuario_id: Uuid::new_v4(),
            organizacion_id: Uuid::new_v4(),
            created_at: now,
        };

        let resp = NotificacionResponse::from(model);
        assert!(resp.leida);
        assert_eq!(resp.tipo, "contrato_por_vencer");
        assert_eq!(resp.entity_type, "contrato");
    }

    #[test]
    fn tipos_notificacion_contains_expected_values() {
        assert!(TIPOS_NOTIFICACION.contains(&"pago_vencido"));
        assert!(TIPOS_NOTIFICACION.contains(&"contrato_por_vencer"));
        assert!(TIPOS_NOTIFICACION.contains(&"documento_vencido"));
        assert!(TIPOS_NOTIFICACION.contains(&"mantenimiento_actualizado"));
        assert_eq!(TIPOS_NOTIFICACION.len(), 4);
    }

    #[test]
    fn tipos_notificacion_rejects_invalid_values() {
        assert!(!TIPOS_NOTIFICACION.contains(&"invalid_tipo"));
        assert!(!TIPOS_NOTIFICACION.contains(&""));
        assert!(!TIPOS_NOTIFICACION.contains(&"pago"));
    }

    #[test]
    fn dias_anticipacion_is_30() {
        assert_eq!(DIAS_ANTICIPACION, 30);
    }
}
