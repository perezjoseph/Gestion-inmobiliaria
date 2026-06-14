use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

use crate::entities::{contrato, gasto, notificacion, responsabilidad_servicio};
use crate::errors::AppError;
use crate::models::responsabilidad_servicio::{
    ResponsabilidadEfectivaResponse, UpdateResponsabilidadRequest,
};
use crate::services::validation::validate_enum;

pub const PROVEEDORES_SERVICIO: &[&str] = &["EDENORTE", "EDESUR", "EDEESTE", "CAASD"];
pub const RESPONSABLES: &[&str] = &["propietario", "inquilino"];

pub fn resolve_responsabilidad<'a>(
    unit_default: Option<&'a str>,
    contract_override: Option<&'a str>,
) -> &'a str {
    if let Some(override_val) = contract_override {
        return override_val;
    }
    unit_default.unwrap_or("propietario")
}

pub async fn obtener_responsabilidades(
    db: &DatabaseConnection,
    org_id: Uuid,
    unidad_id: Uuid,
) -> Result<Vec<ResponsabilidadEfectivaResponse>, AppError> {
    let records = responsabilidad_servicio::Entity::find()
        .filter(responsabilidad_servicio::Column::UnidadId.eq(unidad_id))
        .filter(responsabilidad_servicio::Column::OrganizacionId.eq(org_id))
        .all(db)
        .await?;

    let mut results: Vec<ResponsabilidadEfectivaResponse> = Vec::new();

    for proveedor in PROVEEDORES_SERVICIO {
        let unit_default = records
            .iter()
            .find(|r| r.proveedor_servicio == *proveedor && r.contrato_id.is_none());
        let contract_override = records
            .iter()
            .find(|r| r.proveedor_servicio == *proveedor && r.contrato_id.is_some());

        let (responsable, es_override) = if let Some(co) = contract_override {
            (co.responsable.as_str(), true)
        } else if let Some(ud) = unit_default {
            (ud.responsable.as_str(), false)
        } else {
            continue;
        };

        results.push(ResponsabilidadEfectivaResponse {
            proveedor_servicio: (*proveedor).to_string(),
            responsable: responsable.to_string(),
            es_override_contrato: es_override,
        });
    }

    Ok(results)
}

pub async fn actualizar_responsabilidad_unidad(
    db: &DatabaseConnection,
    org_id: Uuid,
    unidad_id: Uuid,
    input: UpdateResponsabilidadRequest,
    _usuario_id: Uuid,
) -> Result<Vec<ResponsabilidadEfectivaResponse>, AppError> {
    for item in &input.responsabilidades {
        validate_enum(
            "proveedor_servicio",
            &item.proveedor_servicio,
            PROVEEDORES_SERVICIO,
        )?;
        validate_enum("responsable", &item.responsable, RESPONSABLES)?;

        upsert_responsabilidad(
            db,
            org_id,
            unidad_id,
            &item.proveedor_servicio,
            &item.responsable,
            None,
        )
        .await?;
    }

    obtener_responsabilidades(db, org_id, unidad_id).await
}

pub async fn actualizar_responsabilidad_contrato(
    db: &DatabaseConnection,
    org_id: Uuid,
    contrato_id: Uuid,
    input: UpdateResponsabilidadRequest,
    _usuario_id: Uuid,
) -> Result<Vec<ResponsabilidadEfectivaResponse>, AppError> {
    let _contrato = contrato::Entity::find_by_id(contrato_id)
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    let unidad_id = input.unidad_id.ok_or_else(|| {
        AppError::Validation("unidad_id es requerido para overrides de contrato".to_string())
    })?;

    for item in &input.responsabilidades {
        validate_enum(
            "proveedor_servicio",
            &item.proveedor_servicio,
            PROVEEDORES_SERVICIO,
        )?;
        validate_enum("responsable", &item.responsable, RESPONSABLES)?;

        upsert_responsabilidad(
            db,
            org_id,
            unidad_id,
            &item.proveedor_servicio,
            &item.responsable,
            Some(contrato_id),
        )
        .await?;
    }

    obtener_responsabilidades(db, org_id, unidad_id).await
}

pub async fn verificar_consumo_anormal<C: ConnectionTrait>(
    db: &C,
    gasto: &gasto::Model,
    organizacion_id: Uuid,
) -> Result<(), AppError> {
    let Some(consumo) = gasto.consumo else {
        return Ok(());
    };
    let proveedor = match &gasto.proveedor_servicio {
        Some(p) => p.clone(),
        None => return Ok(()),
    };
    let Some(unidad_id) = gasto.unidad_id else {
        return Ok(());
    };

    let result =
        verificar_consumo_anormal_inner(db, gasto, organizacion_id, consumo, &proveedor, unidad_id)
            .await;
    if let Err(e) = result {
        tracing::warn!("Error en verificación de consumo anormal: {e}");
    }

    Ok(())
}

async fn verificar_consumo_anormal_inner<C: ConnectionTrait>(
    db: &C,
    gasto_actual: &gasto::Model,
    organizacion_id: Uuid,
    consumo: Decimal,
    proveedor: &str,
    unidad_id: Uuid,
) -> Result<(), AppError> {
    let historico = gasto::Entity::find()
        .filter(gasto::Column::UnidadId.eq(unidad_id))
        .filter(gasto::Column::ProveedorServicio.eq(proveedor))
        .filter(gasto::Column::Consumo.is_not_null())
        .filter(gasto::Column::Id.ne(gasto_actual.id))
        .order_by_desc(gasto::Column::FechaGasto)
        .limit(10)
        .all(db)
        .await?;

    if historico.len() < 3 {
        return Ok(());
    }

    let sum: Decimal = historico.iter().filter_map(|g| g.consumo).sum();
    let count = Decimal::from(historico.len() as u64);
    let promedio = sum / count;
    let umbral = promedio * Decimal::new(15, 1);

    if consumo > umbral {
        let porcentaje_desviacion = if promedio > Decimal::ZERO {
            ((consumo - promedio) / promedio * Decimal::new(100, 0)).round_dp(1)
        } else {
            Decimal::ZERO
        };

        let mensaje = format!(
            "Consumo anormal detectado en unidad {unidad_id}, proveedor {proveedor}: \
             consumo actual = {consumo}, promedio histórico = {promedio}, \
             desviación = {porcentaje_desviacion}%"
        );

        let now = Utc::now();
        let notif = notificacion::ActiveModel {
            id: Set(Uuid::new_v4()),
            tipo: Set("consumo_anormal".to_string()),
            titulo: Set("Consumo anormal detectado".to_string()),
            mensaje: Set(mensaje),
            leida: Set(false),
            entity_type: Set("gasto".to_string()),
            entity_id: Set(gasto_actual.id),
            usuario_id: Set(Uuid::nil()),
            organizacion_id: Set(organizacion_id),
            created_at: Set(now.into()),
        };

        notif.insert(db).await?;
    }

    Ok(())
}

pub fn es_consumo_anormal(historico: &[Decimal], consumo_nuevo: Decimal) -> Option<bool> {
    if historico.len() < 3 {
        return None;
    }

    let sum: Decimal = historico.iter().copied().sum();
    let count = Decimal::from(historico.len() as u64);
    let promedio = sum / count;
    let umbral = promedio * Decimal::new(15, 1);

    Some(consumo_nuevo > umbral)
}

async fn upsert_responsabilidad<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    unidad_id: Uuid,
    proveedor_servicio: &str,
    responsable: &str,
    contrato_id: Option<Uuid>,
) -> Result<(), AppError> {
    let now = Utc::now();

    let mut query = responsabilidad_servicio::Entity::find()
        .filter(responsabilidad_servicio::Column::UnidadId.eq(unidad_id))
        .filter(responsabilidad_servicio::Column::ProveedorServicio.eq(proveedor_servicio))
        .filter(responsabilidad_servicio::Column::OrganizacionId.eq(org_id));

    query = match contrato_id {
        Some(cid) => query.filter(responsabilidad_servicio::Column::ContratoId.eq(cid)),
        None => query.filter(responsabilidad_servicio::Column::ContratoId.is_null()),
    };

    let existing = query.one(db).await?;

    if let Some(record) = existing {
        let mut active: responsabilidad_servicio::ActiveModel = record.into();
        active.responsable = Set(responsable.to_string());
        active.updated_at = Set(now.into());
        active.update(db).await?;
    } else {
        let model = responsabilidad_servicio::ActiveModel {
            id: Set(Uuid::new_v4()),
            unidad_id: Set(unidad_id),
            proveedor_servicio: Set(proveedor_servicio.to_string()),
            responsable: Set(responsable.to_string()),
            contrato_id: Set(contrato_id),
            organizacion_id: Set(org_id),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };
        model.insert(db).await?;
    }

    Ok(())
}
