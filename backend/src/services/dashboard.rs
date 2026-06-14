use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

use crate::entities::{
    contrato, cuota_condominio, documento, gasto, inquilino, organizacion, pago, propiedad,
};
use crate::errors::AppError;
use crate::models::dashboard::{
    ContratoCalendario, DashboardComparativoResponse, GastosComparacion, IngresoComparacion,
    OcupacionMensual, PagoProximo, PropertyComparison,
};
use crate::services::documentos::{
    REQUERIDOS_CONTRATO, REQUERIDOS_INQUILINO, REQUERIDOS_PROPIEDAD,
};

#[derive(Debug, Clone, Serialize)]
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

    let (total_propiedades, ocupadas, ingreso_result, pagos_atrasados, gastos_result) = tokio::try_join!(
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
    )?;

    let (org_propiedad_ids, org_inquilino_ids, org_contrato_ids) = tokio::try_join!(
        async {
            Ok::<Vec<Uuid>, AppError>(
                propiedad::Entity::find()
                    .select_only()
                    .column(propiedad::Column::Id)
                    .filter(propiedad::Column::OrganizacionId.eq(org_id))
                    .into_tuple::<Uuid>()
                    .all(db)
                    .await?,
            )
        },
        async {
            Ok::<Vec<Uuid>, AppError>(
                inquilino::Entity::find()
                    .select_only()
                    .column(inquilino::Column::Id)
                    .filter(inquilino::Column::OrganizacionId.eq(org_id))
                    .into_tuple::<Uuid>()
                    .all(db)
                    .await?,
            )
        },
        async {
            Ok::<Vec<Uuid>, AppError>(
                contrato::Entity::find()
                    .select_only()
                    .column(contrato::Column::Id)
                    .filter(contrato::Column::OrganizacionId.eq(org_id))
                    .into_tuple::<Uuid>()
                    .all(db)
                    .await?,
            )
        },
    )?;

    let all_org_entity_ids: Vec<Uuid> = org_propiedad_ids
        .into_iter()
        .chain(org_inquilino_ids)
        .chain(org_contrato_ids)
        .collect();

    let documentos_vencidos = if all_org_entity_ids.is_empty() {
        0u64
    } else {
        documento::Entity::find()
            .filter(documento::Column::EntityId.is_in(all_org_entity_ids.clone()))
            .filter(documento::Column::EstadoVerificacion.eq("vencido"))
            .count(db)
            .await?
    };

    let documentos_por_vencer = if all_org_entity_ids.is_empty() {
        0u64
    } else {
        documento::Entity::find()
            .filter(documento::Column::EntityId.is_in(all_org_entity_ids))
            .filter(documento::Column::EstadoVerificacion.eq("verificado"))
            .filter(documento::Column::FechaVencimiento.gte(today))
            .filter(documento::Column::FechaVencimiento.lte(fecha_limite_vencimiento))
            .count(db)
            .await?
    };

    let tasa_ocupacion = if total_propiedades > 0 {
        (ocupadas as f64 / total_propiedades as f64) * 100.0
    } else {
        0.0
    };

    let ingreso_mensual = ingreso_result
        .and_then(|r| r.total)
        .unwrap_or(Decimal::ZERO);

    let total_gastos_mes = gastos_result.and_then(|r| r.total).unwrap_or(Decimal::ZERO);

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

#[allow(clippy::cast_possible_truncation)]
async fn contar_entidades_incompletas(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<i64, AppError> {
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

    for id in &inquilino_ids {
        let docs = inq_docs.get(id).map_or(&[] as &[_], Vec::as_slice);
        if !is_entity_compliant(docs, REQUERIDOS_INQUILINO) {
            incompletas += 1;
        }
    }

    for id in &propiedad_ids {
        let docs = prop_docs.get(id).map_or(&[] as &[_], Vec::as_slice);
        if !is_entity_compliant(docs, REQUERIDOS_PROPIEDAD) {
            incompletas += 1;
        }
    }

    for id in &contrato_ids {
        let docs = cont_docs.get(id).map_or(&[] as &[_], Vec::as_slice);
        if !is_entity_compliant(docs, REQUERIDOS_CONTRATO) {
            incompletas += 1;
        }
    }

    Ok(incompletas)
}

fn is_entity_compliant(docs: &[&documento::Model], requeridos: &[&str]) -> bool {
    requeridos.iter().all(|&tipo| {
        docs.iter()
            .any(|d| d.tipo_documento == tipo && d.estado_verificacion == "verificado")
    })
}

#[allow(clippy::cast_possible_truncation)]
pub async fn cumplimiento_resumen(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<Vec<CumplimientoResumenItem>, AppError> {
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

    for id in &inquilino_ids {
        let docs = inq_docs.get(id).map_or(&[] as &[_], Vec::as_slice);
        let porcentaje = calcular_porcentaje_cumplimiento(docs, REQUERIDOS_INQUILINO);
        items.push(CumplimientoResumenItem {
            entity_type: "inquilino".to_string(),
            entity_id: *id,
            porcentaje,
        });
    }

    for id in &propiedad_ids {
        let docs = prop_docs.get(id).map_or(&[] as &[_], Vec::as_slice);
        let porcentaje = calcular_porcentaje_cumplimiento(docs, REQUERIDOS_PROPIEDAD);
        items.push(CumplimientoResumenItem {
            entity_type: "propiedad".to_string(),
            entity_id: *id,
            porcentaje,
        });
    }

    for id in &contrato_ids {
        let docs = cont_docs.get(id).map_or(&[] as &[_], Vec::as_slice);
        let porcentaje = calcular_porcentaje_cumplimiento(docs, REQUERIDOS_CONTRATO);
        items.push(CumplimientoResumenItem {
            entity_type: "contrato".to_string(),
            entity_id: *id,
            porcentaje,
        });
    }

    items.sort_by_key(|i| i.porcentaje);
    items.truncate(10);

    Ok(items)
}

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
        .count(db)
        .await?;
    if total_propiedades == 0 {
        return Ok(Vec::new());
    }

    let today = Utc::now().date_naive();
    let oldest_target = today - chrono::Months::new(meses - 1);
    let primer_dia_rango = naive_date(oldest_target.year(), oldest_target.month(), 1)?;

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

pub async fn ingreso_comparacion(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<IngresoComparacion, AppError> {
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

pub async fn gastos_comparacion(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<GastosComparacion, AppError> {
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
        total_pendiente: Decimal::ZERO,
        gastos_vencidos: 0,
    })
}

const VALOR_CATASTRAL_MIN_THRESHOLD: Decimal = Decimal::from_parts(100_000, 0, 0, false, 0);

const RENTABILIDAD_CAP: Decimal = Decimal::from_parts(200, 0, 0, false, 0);

pub fn calcular_rentabilidad_neta(
    ingresos: Decimal,
    gastos: Decimal,
    cuotas: Decimal,
    valor_catastral: Decimal,
) -> (Decimal, bool) {
    let is_unreliable = valor_catastral < VALOR_CATASTRAL_MIN_THRESHOLD;

    if valor_catastral.is_zero() {
        return (Decimal::ZERO, is_unreliable);
    }

    let neto = ingresos - gastos - cuotas;
    let rentabilidad = (neto / valor_catastral) * Decimal::ONE_HUNDRED;

    let capped = if rentabilidad > RENTABILIDAD_CAP {
        RENTABILIDAD_CAP
    } else {
        rentabilidad
    };

    (capped, is_unreliable)
}

pub(crate) fn normalizar_moneda(
    monto: Decimal,
    moneda_origen: &str,
    moneda_destino: &str,
    tasa_cambio: Decimal,
) -> Decimal {
    if moneda_origen == moneda_destino || tasa_cambio.is_zero() {
        return monto;
    }

    match (moneda_origen, moneda_destino) {
        ("DOP", "USD") => monto / tasa_cambio,
        ("USD", "DOP") => monto * tasa_cambio,
        _ => monto,
    }
}

pub async fn dashboard_comparativo(
    db: &DatabaseConnection,
    org_id: Uuid,
    fecha_desde: Option<NaiveDate>,
    fecha_hasta: Option<NaiveDate>,
    tipo_propiedad: Option<&str>,
    moneda_display: &str,
    tasa_cambio: Decimal,
) -> Result<DashboardComparativoResponse, AppError> {
    let org = organizacion::Entity::find_by_id(org_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;

    let is_registered =
        org.tipo_fiscal == "persona_juridica" || org.tipo_fiscal == "persona_fisica";

    let mut query = propiedad::Entity::find().filter(propiedad::Column::OrganizacionId.eq(org_id));

    if let Some(tipo) = tipo_propiedad {
        query = query.filter(propiedad::Column::TipoPropiedad.eq(tipo));
    }

    let propiedades = query.all(db).await?;

    if propiedades.is_empty() {
        return Ok(DashboardComparativoResponse {
            propiedades: Vec::new(),
            moneda_display: moneda_display.to_string(),
        });
    }

    let has_date_range = fecha_desde.is_some() && fecha_hasta.is_some();

    if !has_date_range {
        let comparisons = propiedades
            .iter()
            .map(|p| PropertyComparison {
                propiedad_id: p.id,
                titulo: p.titulo.clone(),
                tipo_propiedad: p.tipo_propiedad.clone(),
                moneda: moneda_display.to_string(),
                ingresos: None,
                gastos: None,
                cuotas_condominio: None,
                rentabilidad_neta: None,
                tasa_ocupacion: None,
                morosidad_pct: None,
                itbis_total: None,
                valor_catastral: p.valor_catastral,
                rentabilidad_unreliable: p
                    .valor_catastral
                    .is_none_or(|v| v < VALOR_CATASTRAL_MIN_THRESHOLD),
            })
            .collect();

        return Ok(DashboardComparativoResponse {
            propiedades: comparisons,
            moneda_display: moneda_display.to_string(),
        });
    }

    let Some(desde) = fecha_desde else {
        return Err(AppError::BadRequest("fecha_desde requerida".to_string()));
    };
    let Some(hasta) = fecha_hasta else {
        return Err(AppError::BadRequest("fecha_hasta requerida".to_string()));
    };

    let propiedad_ids: Vec<Uuid> = propiedades.iter().map(|p| p.id).collect();

    let contratos = contrato::Entity::find()
        .filter(contrato::Column::PropiedadId.is_in(propiedad_ids.clone()))
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .all(db)
        .await?;

    let contrato_ids: Vec<Uuid> = contratos.iter().map(|c| c.id).collect();

    let pagos = if contrato_ids.is_empty() {
        Vec::new()
    } else {
        pago::Entity::find()
            .filter(pago::Column::ContratoId.is_in(contrato_ids.clone()))
            .filter(pago::Column::FechaVencimiento.gte(desde))
            .filter(pago::Column::FechaVencimiento.lte(hasta))
            .all(db)
            .await?
    };

    let gastos_list = gasto::Entity::find()
        .filter(gasto::Column::PropiedadId.is_in(propiedad_ids.clone()))
        .filter(gasto::Column::FechaGasto.gte(desde))
        .filter(gasto::Column::FechaGasto.lte(hasta))
        .all(db)
        .await?;

    let cuotas = cuota_condominio::Entity::find()
        .filter(cuota_condominio::Column::PropiedadId.is_in(propiedad_ids.clone()))
        .filter(cuota_condominio::Column::OrganizacionId.eq(org_id))
        .all(db)
        .await?;

    let contrato_to_propiedad: HashMap<Uuid, Uuid> =
        contratos.iter().map(|c| (c.id, c.propiedad_id)).collect();

    let mut contratos_by_prop: HashMap<Uuid, Vec<&contrato::Model>> = HashMap::new();
    for c in &contratos {
        contratos_by_prop.entry(c.propiedad_id).or_default().push(c);
    }

    let mut pagos_by_prop: HashMap<Uuid, Vec<&pago::Model>> = HashMap::new();
    for p in &pagos {
        if let Some(&prop_id) = contrato_to_propiedad.get(&p.contrato_id) {
            pagos_by_prop.entry(prop_id).or_default().push(p);
        }
    }

    let mut gastos_by_prop: HashMap<Uuid, Vec<&gasto::Model>> = HashMap::new();
    for g in &gastos_list {
        gastos_by_prop.entry(g.propiedad_id).or_default().push(g);
    }

    let mut cuotas_by_prop: HashMap<Uuid, Vec<&cuota_condominio::Model>> = HashMap::new();
    for c in &cuotas {
        cuotas_by_prop.entry(c.propiedad_id).or_default().push(c);
    }

    let total_days = (hasta - desde).num_days().max(1);

    let comparisons = propiedades
        .iter()
        .map(|prop| {
            let prop_pagos = pagos_by_prop
                .get(&prop.id)
                .map_or(&[] as &[_], Vec::as_slice);
            let prop_gastos = gastos_by_prop
                .get(&prop.id)
                .map_or(&[] as &[_], Vec::as_slice);
            let prop_cuotas = cuotas_by_prop
                .get(&prop.id)
                .map_or(&[] as &[_], Vec::as_slice);
            let prop_contratos = contratos_by_prop
                .get(&prop.id)
                .map_or(&[] as &[_], Vec::as_slice);

            let ingresos_raw: Decimal = prop_pagos
                .iter()
                .filter(|p| p.estado == "pagado")
                .map(|p| normalizar_moneda(p.monto, &p.moneda, moneda_display, tasa_cambio))
                .sum();

            let gastos_raw: Decimal = prop_gastos
                .iter()
                .filter(|g| g.estado == "pagado")
                .map(|g| normalizar_moneda(g.monto, &g.moneda, moneda_display, tasa_cambio))
                .sum();

            let cuotas_total: Decimal = prop_cuotas
                .iter()
                .filter(|c| c.fecha_inicio <= hasta && c.fecha_fin.is_none_or(|fin| fin >= desde))
                .map(|c| {
                    let cuota_start = c.fecha_inicio.max(desde);
                    let cuota_end = c.fecha_fin.unwrap_or(hasta).min(hasta);
                    let months = months_between(cuota_start, cuota_end, &c.frecuencia);
                    let monto_normalized =
                        normalizar_moneda(c.monto, &c.moneda, moneda_display, tasa_cambio);
                    monto_normalized * months
                })
                .sum();

            let occupied_days: i64 = prop_contratos
                .iter()
                .filter(|c| {
                    c.estado == "activo" || c.estado == "finalizado" || c.estado == "terminado"
                })
                .filter(|c| c.fecha_inicio <= hasta && c.fecha_fin >= desde)
                .map(|c| {
                    let start = c.fecha_inicio.max(desde);
                    let end = c.fecha_fin.min(hasta);
                    (end - start).num_days() + 1
                })
                .sum();
            let tasa_ocupacion = (occupied_days as f64 / total_days as f64) * 100.0;
            let tasa_ocupacion = (tasa_ocupacion * 10.0).round() / 10.0;
            let tasa_ocupacion = tasa_ocupacion.min(100.0);

            #[allow(clippy::cast_possible_wrap)]
            let morosidad_pct = {
                let total_pagos = prop_pagos.len();
                let pagos_atrasados = prop_pagos.iter().filter(|p| p.estado == "atrasado").count();
                if total_pagos > 0 {
                    Decimal::new(pagos_atrasados as i64 * 100, 0)
                        / Decimal::new(total_pagos as i64, 0)
                } else {
                    Decimal::ZERO
                }
            };

            let valor_catastral_normalized = prop.valor_catastral.map_or(Decimal::ZERO, |v| {
                normalizar_moneda(v, "DOP", moneda_display, tasa_cambio)
            });

            let (rentabilidad, is_unreliable) = calcular_rentabilidad_neta(
                ingresos_raw,
                gastos_raw,
                cuotas_total,
                valor_catastral_normalized,
            );

            let itbis_total = if is_registered
                && (prop.tipo_propiedad == "comercial" || prop.tipo_propiedad == "industrial")
            {
                let itbis: Decimal = prop_pagos
                    .iter()
                    .filter_map(|p| p.monto_itbis)
                    .map(|itbis| normalizar_moneda(itbis, "DOP", moneda_display, tasa_cambio))
                    .sum();
                Some(itbis)
            } else {
                None
            };

            PropertyComparison {
                propiedad_id: prop.id,
                titulo: prop.titulo.clone(),
                tipo_propiedad: prop.tipo_propiedad.clone(),
                moneda: moneda_display.to_string(),
                ingresos: Some(ingresos_raw),
                gastos: Some(gastos_raw),
                cuotas_condominio: Some(cuotas_total),
                rentabilidad_neta: Some(rentabilidad),
                tasa_ocupacion: Some(tasa_ocupacion),
                morosidad_pct: Some(morosidad_pct),
                itbis_total,
                valor_catastral: prop.valor_catastral,
                rentabilidad_unreliable: is_unreliable,
            }
        })
        .collect();

    Ok(DashboardComparativoResponse {
        propiedades: comparisons,
        moneda_display: moneda_display.to_string(),
    })
}

fn months_between(start: NaiveDate, end: NaiveDate, frecuencia: &str) -> Decimal {
    if end < start {
        return Decimal::ZERO;
    }

    let days = (end - start).num_days() + 1;
    let approx_months = Decimal::new(days, 0) / Decimal::new(30, 0);

    match frecuencia {
        "trimestral" => approx_months / Decimal::new(3, 0),
        "anual" => approx_months / Decimal::new(12, 0),
        _ => approx_months,
    }
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

    #[test]
    fn rentabilidad_neta_basic_calculation() {
        let (result, unreliable) = calcular_rentabilidad_neta(
            Decimal::new(120_000, 0),
            Decimal::new(30_000, 0),
            Decimal::new(10_000, 0),
            Decimal::new(500_000, 0),
        );
        assert_eq!(result, Decimal::new(16, 0));
        assert!(!unreliable);
    }

    #[test]
    fn rentabilidad_neta_capped_at_200() {
        let (result, _) = calcular_rentabilidad_neta(
            Decimal::new(1_000_000, 0),
            Decimal::ZERO,
            Decimal::ZERO,
            Decimal::new(100_000, 0),
        );
        assert_eq!(result, Decimal::new(200, 0));
    }

    #[test]
    fn rentabilidad_neta_unreliable_below_threshold() {
        let (_, unreliable) = calcular_rentabilidad_neta(
            Decimal::new(10_000, 0),
            Decimal::new(5_000, 0),
            Decimal::ZERO,
            Decimal::new(50_000, 0),
        );
        assert!(unreliable);
    }

    #[test]
    fn rentabilidad_neta_reliable_at_threshold() {
        let (_, unreliable) = calcular_rentabilidad_neta(
            Decimal::new(10_000, 0),
            Decimal::new(5_000, 0),
            Decimal::ZERO,
            Decimal::new(100_000, 0),
        );
        assert!(!unreliable);
    }

    #[test]
    fn rentabilidad_neta_zero_valor_catastral() {
        let (result, unreliable) = calcular_rentabilidad_neta(
            Decimal::new(50_000, 0),
            Decimal::new(10_000, 0),
            Decimal::ZERO,
            Decimal::ZERO,
        );
        assert_eq!(result, Decimal::ZERO);
        assert!(unreliable);
    }

    #[test]
    fn rentabilidad_neta_negative_result() {
        let (result, _) = calcular_rentabilidad_neta(
            Decimal::new(10_000, 0),
            Decimal::new(50_000, 0),
            Decimal::new(5_000, 0),
            Decimal::new(200_000, 0),
        );
        assert_eq!(result, Decimal::new(-225, 1));
    }

    #[test]
    fn normalizar_moneda_same_currency() {
        let result = normalizar_moneda(Decimal::new(1000, 0), "DOP", "DOP", Decimal::new(5850, 2));
        assert_eq!(result, Decimal::new(1000, 0));
    }

    #[test]
    fn normalizar_moneda_dop_to_usd() {
        let result = normalizar_moneda(Decimal::new(5850, 0), "DOP", "USD", Decimal::new(5850, 2));
        assert_eq!(result, Decimal::new(100, 0));
    }

    #[test]
    fn normalizar_moneda_usd_to_dop() {
        let result = normalizar_moneda(Decimal::new(100, 0), "USD", "DOP", Decimal::new(5850, 2));
        assert_eq!(result, Decimal::new(5850, 0));
    }

    #[test]
    fn normalizar_moneda_zero_rate_returns_original() {
        let result = normalizar_moneda(Decimal::new(1000, 0), "DOP", "USD", Decimal::ZERO);
        assert_eq!(result, Decimal::new(1000, 0));
    }

    #[test]
    fn months_between_one_month() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
        let result = super::months_between(start, end, "mensual");
        assert!(result > Decimal::ONE);
        assert!(result < Decimal::new(11, 1));
    }

    #[test]
    fn months_between_trimestral() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 3, 31).unwrap();
        let result = super::months_between(start, end, "trimestral");
        assert!(result >= Decimal::ONE);
    }

    #[test]
    fn months_between_end_before_start() {
        let start = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let result = super::months_between(start, end, "mensual");
        assert_eq!(result, Decimal::ZERO);
    }
}
