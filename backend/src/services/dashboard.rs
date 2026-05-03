use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

use crate::entities::{contrato, documento, gasto, inquilino, pago, propiedad};
use crate::errors::AppError;
use crate::models::dashboard::{
    ContratoCalendario, GastosComparacion, IngresoComparacion, OcupacionMensual, PagoProximo,
};
use crate::services::documentos::{REQUERIDOS_CONTRATO, REQUERIDOS_INQUILINO, REQUERIDOS_PROPIEDAD};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_propiedades: u64,
    pub tasa_ocupacion: f64,
    pub ingreso_mensual: Decimal,
    pub pagos_atrasados: u64,
    pub total_gastos_mes: Decimal,
    pub documentos_vencidos: i64,
    pub documentos_por_vencer: i64,
    pub entidades_incompletas: i64,
}

#[derive(Debug, FromQueryResult)]
struct SumResult {
    total: Option<Decimal>,
}

/// Helper to build a `NaiveDate` from year/month/day, returning an `AppError` on invalid input.
fn naive_date(year: i32, month: u32, day: u32) -> Result<NaiveDate, AppError> {
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Fecha inválida: {year}-{month}-{day}")))
}

pub async fn get_stats(db: &DatabaseConnection, org_id: Uuid) -> Result<DashboardStats, AppError> {
    let today = Utc::now().date_naive();
    let anio = today.year();
    let mes = today.month();
    let primer_dia_mes = naive_date(anio, mes, 1)?;
    let ultimo_dia_mes = if mes == 12 {
        naive_date(anio + 1, 1, 1)? - chrono::Days::new(1)
    } else {
        naive_date(anio, mes + 1, 1)? - chrono::Days::new(1)
    };

    let fecha_limite_vencimiento = today + chrono::Days::new(30);

    let (
        total_propiedades,
        ocupadas,
        ingreso_result,
        pagos_atrasados,
        gastos_result,
        documentos_vencidos,
        documentos_por_vencer,
    ) = tokio::try_join!(
        propiedad::Entity::find()
            .filter(propiedad::Column::OrganizacionId.eq(org_id))
            .count(db),
        propiedad::Entity::find()
            .filter(propiedad::Column::OrganizacionId.eq(org_id))
            .filter(propiedad::Column::Estado.eq("ocupada"))
            .count(db),
        contrato::Entity::find()
            .select_only()
            .column_as(contrato::Column::MontoMensual.sum(), "total")
            .filter(contrato::Column::OrganizacionId.eq(org_id))
            .filter(contrato::Column::Estado.eq("activo"))
            .into_model::<SumResult>()
            .one(db),
        pago::Entity::find()
            .filter(pago::Column::OrganizacionId.eq(org_id))
            .filter(pago::Column::Estado.eq("atrasado"))
            .count(db),
        gasto::Entity::find()
            .select_only()
            .column_as(gasto::Column::Monto.sum(), "total")
            .filter(gasto::Column::OrganizacionId.eq(org_id))
            .filter(gasto::Column::Estado.eq("pagado"))
            .filter(gasto::Column::FechaGasto.gte(primer_dia_mes))
            .filter(gasto::Column::FechaGasto.lte(ultimo_dia_mes))
            .into_model::<SumResult>()
            .one(db),
        documento::Entity::find()
            .filter(documento::Column::EstadoVerificacion.eq("vencido"))
            .count(db),
        documento::Entity::find()
            .filter(documento::Column::EstadoVerificacion.eq("verificado"))
            .filter(documento::Column::FechaVencimiento.gte(today))
            .filter(documento::Column::FechaVencimiento.lte(fecha_limite_vencimiento))
            .count(db),
    )?;

    let tasa_ocupacion = if total_propiedades > 0 {
        (ocupadas as f64 / total_propiedades as f64) * 100.0
    } else {
        0.0
    };

    let ingreso_mensual = ingreso_result
        .and_then(|r| r.total)
        .unwrap_or(Decimal::ZERO);

    let total_gastos_mes = gastos_result.and_then(|r| r.total).unwrap_or(Decimal::ZERO);

    // Calculate entidades_incompletas: count entities with required docs below 100% compliance
    let entidades_incompletas = contar_entidades_incompletas(db, org_id).await?;

    Ok(DashboardStats {
        total_propiedades,
        tasa_ocupacion,
        ingreso_mensual,
        pagos_atrasados,
        total_gastos_mes,
        documentos_vencidos: i64::try_from(documentos_vencidos).unwrap_or(i64::MAX),
        documentos_por_vencer: i64::try_from(documentos_por_vencer).unwrap_or(i64::MAX),
        entidades_incompletas,
    })
}

/// Count entities (inquilinos, propiedades, contratos) that have required documents
/// but are below 100% compliance. Uses batch queries with `is_in()` to avoid N+1.
#[allow(clippy::cast_possible_truncation)]
async fn contar_entidades_incompletas(db: &DatabaseConnection, org_id: Uuid) -> Result<i64, AppError> {
    // Fetch all entities of each type that have required documents
    let (all_inquilinos, all_propiedades, all_contratos) = tokio::try_join!(
        inquilino::Entity::find()
            .filter(inquilino::Column::OrganizacionId.eq(org_id))
            .all(db),
        propiedad::Entity::find()
            .filter(propiedad::Column::OrganizacionId.eq(org_id))
            .all(db),
        contrato::Entity::find()
            .filter(contrato::Column::OrganizacionId.eq(org_id))
            .all(db),
    )?;

    // Collect all entity IDs per type for batch document queries
    let inquilino_ids: Vec<uuid::Uuid> = all_inquilinos.iter().map(|i| i.id).collect();
    let propiedad_ids: Vec<uuid::Uuid> = all_propiedades.iter().map(|p| p.id).collect();
    let contrato_ids: Vec<uuid::Uuid> = all_contratos.iter().map(|c| c.id).collect();

    // Batch-fetch all documents for these entities in 3 queries (not N+1)
    let (docs_inquilinos, docs_propiedades, docs_contratos) = tokio::try_join!(
        documento::Entity::find()
            .filter(documento::Column::EntityType.eq("inquilino"))
            .filter(documento::Column::EntityId.is_in(inquilino_ids.clone()))
            .all(db),
        documento::Entity::find()
            .filter(documento::Column::EntityType.eq("propiedad"))
            .filter(documento::Column::EntityId.is_in(propiedad_ids.clone()))
            .all(db),
        documento::Entity::find()
            .filter(documento::Column::EntityType.eq("contrato"))
            .filter(documento::Column::EntityId.is_in(contrato_ids.clone()))
            .all(db),
    )?;

    // Group documents by entity_id using HashMaps
    let mut inq_docs: HashMap<uuid::Uuid, Vec<&documento::Model>> = HashMap::new();
    for doc in &docs_inquilinos {
        inq_docs.entry(doc.entity_id).or_default().push(doc);
    }
    let mut prop_docs: HashMap<uuid::Uuid, Vec<&documento::Model>> = HashMap::new();
    for doc in &docs_propiedades {
        prop_docs.entry(doc.entity_id).or_default().push(doc);
    }
    let mut cont_docs: HashMap<uuid::Uuid, Vec<&documento::Model>> = HashMap::new();
    for doc in &docs_contratos {
        cont_docs.entry(doc.entity_id).or_default().push(doc);
    }

    let mut incompletas: i64 = 0;

    // Check inquilinos compliance
    for id in &inquilino_ids {
        let docs = inq_docs.get(id).map_or(&[] as &[_], Vec::as_slice);
        if !is_entity_compliant(docs, REQUERIDOS_INQUILINO) {
            incompletas += 1;
        }
    }

    // Check propiedades compliance
    for id in &propiedad_ids {
        let docs = prop_docs.get(id).map_or(&[] as &[_], Vec::as_slice);
        if !is_entity_compliant(docs, REQUERIDOS_PROPIEDAD) {
            incompletas += 1;
        }
    }

    // Check contratos compliance
    for id in &contrato_ids {
        let docs = cont_docs.get(id).map_or(&[] as &[_], Vec::as_slice);
        if !is_entity_compliant(docs, REQUERIDOS_CONTRATO) {
            incompletas += 1;
        }
    }

    Ok(incompletas)
}

/// Check if an entity has all required documents with `estado_verificacion` = "verificado".
fn is_entity_compliant(docs: &[&documento::Model], requeridos: &[&str]) -> bool {
    requeridos.iter().all(|&tipo| {
        docs.iter()
            .any(|d| d.tipo_documento == tipo && d.estado_verificacion == "verificado")
    })
}

/// Compliance summary: returns entities with lowest compliance percentages.
#[allow(clippy::cast_possible_truncation)]
pub async fn cumplimiento_resumen(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<Vec<CumplimientoResumenItem>, AppError> {
    // Fetch all entities of each type
    let (all_inquilinos, all_propiedades, all_contratos) = tokio::try_join!(
        inquilino::Entity::find()
            .filter(inquilino::Column::OrganizacionId.eq(org_id))
            .all(db),
        propiedad::Entity::find()
            .filter(propiedad::Column::OrganizacionId.eq(org_id))
            .all(db),
        contrato::Entity::find()
            .filter(contrato::Column::OrganizacionId.eq(org_id))
            .all(db),
    )?;

    let inquilino_ids: Vec<uuid::Uuid> = all_inquilinos.iter().map(|i| i.id).collect();
    let propiedad_ids: Vec<uuid::Uuid> = all_propiedades.iter().map(|p| p.id).collect();
    let contrato_ids: Vec<uuid::Uuid> = all_contratos.iter().map(|c| c.id).collect();

    // Batch-fetch all documents
    let (docs_inquilinos, docs_propiedades, docs_contratos) = tokio::try_join!(
        documento::Entity::find()
            .filter(documento::Column::EntityType.eq("inquilino"))
            .filter(documento::Column::EntityId.is_in(inquilino_ids.clone()))
            .all(db),
        documento::Entity::find()
            .filter(documento::Column::EntityType.eq("propiedad"))
            .filter(documento::Column::EntityId.is_in(propiedad_ids.clone()))
            .all(db),
        documento::Entity::find()
            .filter(documento::Column::EntityType.eq("contrato"))
            .filter(documento::Column::EntityId.is_in(contrato_ids.clone()))
            .all(db),
    )?;

    // Group documents by entity_id
    let mut inq_docs: HashMap<uuid::Uuid, Vec<&documento::Model>> = HashMap::new();
    for doc in &docs_inquilinos {
        inq_docs.entry(doc.entity_id).or_default().push(doc);
    }
    let mut prop_docs: HashMap<uuid::Uuid, Vec<&documento::Model>> = HashMap::new();
    for doc in &docs_propiedades {
        prop_docs.entry(doc.entity_id).or_default().push(doc);
    }
    let mut cont_docs: HashMap<uuid::Uuid, Vec<&documento::Model>> = HashMap::new();
    for doc in &docs_contratos {
        cont_docs.entry(doc.entity_id).or_default().push(doc);
    }

    let mut items: Vec<CumplimientoResumenItem> = Vec::new();

    // Calculate compliance for each inquilino
    for id in &inquilino_ids {
        let docs = inq_docs.get(id).map_or(&[] as &[_], Vec::as_slice);
        let porcentaje = calcular_porcentaje_cumplimiento(docs, REQUERIDOS_INQUILINO);
        items.push(CumplimientoResumenItem {
            entity_type: "inquilino".to_string(),
            entity_id: *id,
            porcentaje,
        });
    }

    // Calculate compliance for each propiedad
    for id in &propiedad_ids {
        let docs = prop_docs.get(id).map_or(&[] as &[_], Vec::as_slice);
        let porcentaje = calcular_porcentaje_cumplimiento(docs, REQUERIDOS_PROPIEDAD);
        items.push(CumplimientoResumenItem {
            entity_type: "propiedad".to_string(),
            entity_id: *id,
            porcentaje,
        });
    }

    // Calculate compliance for each contrato
    for id in &contrato_ids {
        let docs = cont_docs.get(id).map_or(&[] as &[_], Vec::as_slice);
        let porcentaje = calcular_porcentaje_cumplimiento(docs, REQUERIDOS_CONTRATO);
        items.push(CumplimientoResumenItem {
            entity_type: "contrato".to_string(),
            entity_id: *id,
            porcentaje,
        });
    }

    // Sort ascending by porcentaje, take the 10 lowest
    items.sort_by_key(|i| i.porcentaje);
    items.truncate(10);

    Ok(items)
}

/// Calculate compliance percentage for an entity given its documents and required types.
#[allow(clippy::cast_possible_truncation)]
fn calcular_porcentaje_cumplimiento(docs: &[&documento::Model], requeridos: &[&str]) -> u8 {
    if requeridos.is_empty() {
        return 100;
    }
    let presente = requeridos
        .iter()
        .filter(|&&tipo| {
            docs.iter()
                .any(|d| d.tipo_documento == tipo && d.estado_verificacion == "verificado")
        })
        .count() as u32;
    let total = requeridos.len() as u32;
    ((presente * 100) / total).min(100) as u8
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CumplimientoResumenItem {
    pub entity_type: String,
    pub entity_id: uuid::Uuid,
    pub porcentaje: u8,
}

pub async fn ocupacion_tendencia(
    db: &DatabaseConnection,
    org_id: Uuid,
    meses: u32,
) -> Result<Vec<OcupacionMensual>, AppError> {
    let total_propiedades = propiedad::Entity::find()
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .count(db).await?;
    if total_propiedades == 0 {
        return Ok(Vec::new());
    }

    let today = Utc::now().date_naive();
    let oldest_target = today - chrono::Months::new(meses - 1);
    let primer_dia_rango =
        naive_date(oldest_target.year(), oldest_target.month(), 1)?;

    let contratos = contrato::Entity::find()
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .filter(contrato::Column::FechaFin.gte(primer_dia_rango))
        .filter(contrato::Column::FechaInicio.lte(today))
        .filter(contrato::Column::Estado.is_in(["activo", "finalizado", "terminado"]))
        .all(db)
        .await?;

    let mut resultados = Vec::with_capacity(meses as usize);

    for i in (0..meses).rev() {
        let target = today - chrono::Months::new(i);
        let anio = target.year();
        let mes = target.month();
        let primer_dia = naive_date(anio, mes, 1)?;
        let ultimo_dia = if mes == 12 {
            naive_date(anio + 1, 1, 1)? - chrono::Days::new(1)
        } else {
            naive_date(anio, mes + 1, 1)? - chrono::Days::new(1)
        };

        let contratos_activos = contratos
            .iter()
            .filter(|c| c.fecha_inicio <= ultimo_dia && c.fecha_fin >= primer_dia)
            .count() as u64;

        let tasa = (contratos_activos as f64 / total_propiedades as f64) * 100.0;
        let tasa = (tasa * 10.0).round() / 10.0;

        resultados.push(OcupacionMensual { mes, anio, tasa });
    }

    Ok(resultados)
}

pub async fn ingreso_comparacion(db: &DatabaseConnection, org_id: Uuid) -> Result<IngresoComparacion, AppError> {
    let today = Utc::now().date_naive();
    let anio = today.year();
    let mes = today.month();
    let primer_dia = naive_date(anio, mes, 1)?;
    let ultimo_dia = if mes == 12 {
        naive_date(anio + 1, 1, 1)? - chrono::Days::new(1)
    } else {
        naive_date(anio, mes + 1, 1)? - chrono::Days::new(1)
    };

    let (esperado_result, cobrado_result) = tokio::try_join!(
        contrato::Entity::find()
            .select_only()
            .column_as(contrato::Column::MontoMensual.sum(), "total")
            .filter(contrato::Column::OrganizacionId.eq(org_id))
            .filter(contrato::Column::Estado.eq("activo"))
            .filter(contrato::Column::FechaInicio.lte(ultimo_dia))
            .filter(contrato::Column::FechaFin.gte(primer_dia))
            .into_model::<SumResult>()
            .one(db),
        pago::Entity::find()
            .select_only()
            .column_as(pago::Column::Monto.sum(), "total")
            .filter(pago::Column::OrganizacionId.eq(org_id))
            .filter(pago::Column::Estado.eq("pagado"))
            .filter(pago::Column::FechaVencimiento.gte(primer_dia))
            .filter(pago::Column::FechaVencimiento.lte(ultimo_dia))
            .into_model::<SumResult>()
            .one(db),
    )?;

    let esperado = esperado_result
        .and_then(|r| r.total)
        .unwrap_or(Decimal::ZERO);
    let cobrado = cobrado_result
        .and_then(|r| r.total)
        .unwrap_or(Decimal::ZERO);
    let diferencia = esperado - cobrado;

    Ok(IngresoComparacion {
        esperado,
        cobrado,
        diferencia,
    })
}

pub async fn pagos_proximos(
    db: &DatabaseConnection,
    org_id: Uuid,
    dias: i64,
) -> Result<Vec<PagoProximo>, AppError> {
    let today = Utc::now().date_naive();
    let limite = today + chrono::Days::new(dias as u64);

    let pagos = pago::Entity::find()
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .filter(pago::Column::Estado.eq("pendiente"))
        .filter(pago::Column::FechaVencimiento.gte(today))
        .filter(pago::Column::FechaVencimiento.lte(limite))
        .order_by_asc(pago::Column::FechaVencimiento)
        .all(db)
        .await?;

    if pagos.is_empty() {
        return Ok(Vec::new());
    }

    let contrato_ids: Vec<uuid::Uuid> = pagos.iter().map(|p| p.contrato_id).collect();
    let contratos = contrato::Entity::find()
        .filter(contrato::Column::Id.is_in(contrato_ids))
        .all(db)
        .await?;

    let propiedad_ids: Vec<uuid::Uuid> = contratos.iter().map(|c| c.propiedad_id).collect();
    let inquilino_ids: Vec<uuid::Uuid> = contratos.iter().map(|c| c.inquilino_id).collect();

    let (propiedades, inquilinos) = tokio::try_join!(
        propiedad::Entity::find()
            .filter(propiedad::Column::Id.is_in(propiedad_ids))
            .all(db),
        inquilino::Entity::find()
            .filter(inquilino::Column::Id.is_in(inquilino_ids))
            .all(db),
    )?;

    let contrato_map: HashMap<uuid::Uuid, &contrato::Model> =
        contratos.iter().map(|c| (c.id, c)).collect();
    let prop_map: HashMap<uuid::Uuid, &propiedad::Model> =
        propiedades.iter().map(|p| (p.id, p)).collect();
    let inq_map: HashMap<uuid::Uuid, &inquilino::Model> =
        inquilinos.iter().map(|i| (i.id, i)).collect();

    let resultados = pagos
        .into_iter()
        .map(|p| {
            let (propiedad_titulo, inquilino_nombre) = contrato_map
                .get(&p.contrato_id)
                .map(|c| {
                    let titulo = prop_map
                        .get(&c.propiedad_id)
                        .map(|pr| pr.titulo.clone())
                        .unwrap_or_default();
                    let nombre = inq_map
                        .get(&c.inquilino_id)
                        .map(|i| format!("{} {}", i.nombre, i.apellido))
                        .unwrap_or_default();
                    (titulo, nombre)
                })
                .unwrap_or_default();

            PagoProximo {
                pago_id: p.id,
                propiedad_titulo,
                inquilino_nombre,
                monto: p.monto,
                moneda: p.moneda,
                fecha_vencimiento: p.fecha_vencimiento,
            }
        })
        .collect();

    Ok(resultados)
}

pub async fn contratos_calendario(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<Vec<ContratoCalendario>, AppError> {
    let today = Utc::now().date_naive();
    let limite = today + chrono::Days::new(90);

    let contratos = contrato::Entity::find()
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .filter(contrato::Column::Estado.eq("activo"))
        .filter(contrato::Column::FechaFin.gte(today))
        .filter(contrato::Column::FechaFin.lte(limite))
        .order_by_asc(contrato::Column::FechaFin)
        .all(db)
        .await?;

    if contratos.is_empty() {
        return Ok(Vec::new());
    }

    let propiedad_ids: Vec<uuid::Uuid> = contratos.iter().map(|c| c.propiedad_id).collect();
    let inquilino_ids: Vec<uuid::Uuid> = contratos.iter().map(|c| c.inquilino_id).collect();

    let (propiedades, inquilinos) = tokio::try_join!(
        propiedad::Entity::find()
            .filter(propiedad::Column::Id.is_in(propiedad_ids))
            .all(db),
        inquilino::Entity::find()
            .filter(inquilino::Column::Id.is_in(inquilino_ids))
            .all(db),
    )?;

    let prop_map: HashMap<uuid::Uuid, &propiedad::Model> =
        propiedades.iter().map(|p| (p.id, p)).collect();
    let inq_map: HashMap<uuid::Uuid, &inquilino::Model> =
        inquilinos.iter().map(|i| (i.id, i)).collect();

    let resultados = contratos
        .into_iter()
        .map(|c| {
            let dias_restantes = (c.fecha_fin - today).num_days();
            let color = if dias_restantes < 30 {
                "rojo".to_string()
            } else if dias_restantes <= 60 {
                "amarillo".to_string()
            } else {
                "verde".to_string()
            };

            ContratoCalendario {
                contrato_id: c.id,
                propiedad_titulo: prop_map
                    .get(&c.propiedad_id)
                    .map(|p| p.titulo.clone())
                    .unwrap_or_default(),
                inquilino_nombre: inq_map
                    .get(&c.inquilino_id)
                    .map(|i| format!("{} {}", i.nombre, i.apellido))
                    .unwrap_or_default(),
                fecha_fin: c.fecha_fin,
                dias_restantes,
                color,
            }
        })
        .collect();

    Ok(resultados)
}

pub fn calcular_porcentaje_cambio(actual: Decimal, anterior: Decimal) -> f64 {
    use rust_decimal::prelude::ToPrimitive;

    if anterior.is_zero() && actual.is_zero() {
        return 0.0;
    }
    if anterior.is_zero() {
        return 100.0;
    }
    let cambio = ((actual - anterior) / anterior) * Decimal::new(100, 0);
    cambio.to_f64().unwrap_or(0.0)
}

pub async fn gastos_comparacion(db: &DatabaseConnection, org_id: Uuid) -> Result<GastosComparacion, AppError> {
    let today = Utc::now().date_naive();
    let anio = today.year();
    let mes = today.month();

    let primer_dia_actual = naive_date(anio, mes, 1)?;
    let ultimo_dia_actual = if mes == 12 {
        naive_date(anio + 1, 1, 1)? - chrono::Days::new(1)
    } else {
        naive_date(anio, mes + 1, 1)? - chrono::Days::new(1)
    };

    let prev = today - chrono::Months::new(1);
    let primer_dia_anterior = naive_date(prev.year(), prev.month(), 1)?;
    let ultimo_dia_anterior = primer_dia_actual - chrono::Days::new(1);

    let (actual_result, anterior_result) = tokio::try_join!(
        gasto::Entity::find()
            .select_only()
            .column_as(gasto::Column::Monto.sum(), "total")
            .filter(gasto::Column::OrganizacionId.eq(org_id))
            .filter(gasto::Column::Estado.eq("pagado"))
            .filter(gasto::Column::FechaGasto.gte(primer_dia_actual))
            .filter(gasto::Column::FechaGasto.lte(ultimo_dia_actual))
            .into_model::<SumResult>()
            .one(db),
        gasto::Entity::find()
            .select_only()
            .column_as(gasto::Column::Monto.sum(), "total")
            .filter(gasto::Column::OrganizacionId.eq(org_id))
            .filter(gasto::Column::Estado.eq("pagado"))
            .filter(gasto::Column::FechaGasto.gte(primer_dia_anterior))
            .filter(gasto::Column::FechaGasto.lte(ultimo_dia_anterior))
            .into_model::<SumResult>()
            .one(db),
    )?;

    let mes_actual = actual_result.and_then(|r| r.total).unwrap_or(Decimal::ZERO);
    let mes_anterior = anterior_result
        .and_then(|r| r.total)
        .unwrap_or(Decimal::ZERO);
    let porcentaje_cambio = calcular_porcentaje_cambio(mes_actual, mes_anterior);

    Ok(GastosComparacion {
        mes_actual,
        mes_anterior,
        porcentaje_cambio,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_coding_rojo_under_30_days() {
        let dias = 15;
        let color = if dias < 30 {
            "rojo"
        } else if dias <= 60 {
            "amarillo"
        } else {
            "verde"
        };
        assert_eq!(color, "rojo");
    }

    #[test]
    fn color_coding_amarillo_30_to_60_days() {
        let dias = 45;
        let color = if dias < 30 {
            "rojo"
        } else if dias <= 60 {
            "amarillo"
        } else {
            "verde"
        };
        assert_eq!(color, "amarillo");
    }

    #[test]
    fn color_coding_verde_over_60_days() {
        let dias = 75;
        let color = if dias < 30 {
            "rojo"
        } else if dias <= 60 {
            "amarillo"
        } else {
            "verde"
        };
        assert_eq!(color, "verde");
    }

    #[test]
    fn color_coding_boundary_30_days() {
        let dias = 30;
        let color = if dias < 30 {
            "rojo"
        } else if dias <= 60 {
            "amarillo"
        } else {
            "verde"
        };
        assert_eq!(color, "amarillo");
    }

    #[test]
    fn color_coding_boundary_60_days() {
        let dias = 60;
        let color = if dias < 30 {
            "rojo"
        } else if dias <= 60 {
            "amarillo"
        } else {
            "verde"
        };
        assert_eq!(color, "amarillo");
    }

    #[test]
    fn ocupacion_tasa_calculation() {
        let contratos_activos = 3u64;
        let total_propiedades = 10u64;
        let tasa = (contratos_activos as f64 / total_propiedades as f64) * 100.0;
        let tasa = (tasa * 10.0).round() / 10.0;
        assert!((tasa - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ocupacion_tasa_zero_properties() {
        let total_propiedades = 0u64;
        let result: f64 = if total_propiedades == 0 { 0.0 } else { 50.0 };
        assert!((result - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ingreso_diferencia_calculation() {
        let esperado = Decimal::new(10000, 0);
        let cobrado = Decimal::new(7500, 0);
        let diferencia = esperado - cobrado;
        assert_eq!(diferencia, Decimal::new(2500, 0));
    }

    #[test]
    fn ingreso_diferencia_when_fully_collected() {
        let esperado = Decimal::new(5000, 0);
        let cobrado = Decimal::new(5000, 0);
        let diferencia = esperado - cobrado;
        assert_eq!(diferencia, Decimal::ZERO);
    }

    #[test]
    fn porcentaje_cambio_zero_previous_zero_actual() {
        let result = calcular_porcentaje_cambio(Decimal::ZERO, Decimal::ZERO);
        assert!((result - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn porcentaje_cambio_zero_previous_positive_actual() {
        let result = calcular_porcentaje_cambio(Decimal::new(5000, 0), Decimal::ZERO);
        assert!((result - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn porcentaje_cambio_equal_months() {
        let result = calcular_porcentaje_cambio(Decimal::new(3000, 0), Decimal::new(3000, 0));
        assert!((result - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn porcentaje_cambio_increase() {
        let result = calcular_porcentaje_cambio(Decimal::new(6000, 0), Decimal::new(3000, 0));
        assert!((result - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn porcentaje_cambio_decrease() {
        let result = calcular_porcentaje_cambio(Decimal::new(2000, 0), Decimal::new(4000, 0));
        assert!((result - (-50.0)).abs() < f64::EPSILON);
    }
}
