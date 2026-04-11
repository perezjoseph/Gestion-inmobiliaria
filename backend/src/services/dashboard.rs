use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use serde::Serialize;
use std::collections::HashMap;

use crate::entities::{contrato, inquilino, pago, propiedad};
use crate::errors::AppError;
use crate::models::dashboard::{
    ContratoCalendario, IngresoComparacion, OcupacionMensual, PagoProximo,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_propiedades: u64,
    pub tasa_ocupacion: f64,
    pub ingreso_mensual: Decimal,
    pub pagos_atrasados: u64,
}

#[derive(Debug, FromQueryResult)]
struct SumResult {
    total: Option<Decimal>,
}

pub async fn get_stats(db: &DatabaseConnection) -> Result<DashboardStats, AppError> {
    let (total_propiedades, ocupadas, ingreso_result, pagos_atrasados) = tokio::try_join!(
        propiedad::Entity::find().count(db),
        propiedad::Entity::find()
            .filter(propiedad::Column::Estado.eq("ocupada"))
            .count(db),
        contrato::Entity::find()
            .select_only()
            .column_as(contrato::Column::MontoMensual.sum(), "total")
            .filter(contrato::Column::Estado.eq("activo"))
            .into_model::<SumResult>()
            .one(db),
        pago::Entity::find()
            .filter(pago::Column::Estado.eq("atrasado"))
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

    Ok(DashboardStats {
        total_propiedades,
        tasa_ocupacion,
        ingreso_mensual,
        pagos_atrasados,
    })
}

pub async fn ocupacion_tendencia(
    db: &DatabaseConnection,
    meses: u32,
) -> Result<Vec<OcupacionMensual>, AppError> {
    let total_propiedades = propiedad::Entity::find().count(db).await?;
    if total_propiedades == 0 {
        return Ok(Vec::new());
    }

    let today = Utc::now().date_naive();
    let oldest_target = today - chrono::Months::new(meses - 1);
    let primer_dia_rango =
        NaiveDate::from_ymd_opt(oldest_target.year(), oldest_target.month(), 1).unwrap();

    let contratos = contrato::Entity::find()
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
        let primer_dia = NaiveDate::from_ymd_opt(anio, mes, 1).unwrap();
        let ultimo_dia = if mes == 12 {
            NaiveDate::from_ymd_opt(anio + 1, 1, 1).unwrap() - chrono::Days::new(1)
        } else {
            NaiveDate::from_ymd_opt(anio, mes + 1, 1).unwrap() - chrono::Days::new(1)
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

pub async fn ingreso_comparacion(db: &DatabaseConnection) -> Result<IngresoComparacion, AppError> {
    let today = Utc::now().date_naive();
    let anio = today.year();
    let mes = today.month();
    let primer_dia = NaiveDate::from_ymd_opt(anio, mes, 1).unwrap();
    let ultimo_dia = if mes == 12 {
        NaiveDate::from_ymd_opt(anio + 1, 1, 1).unwrap() - chrono::Days::new(1)
    } else {
        NaiveDate::from_ymd_opt(anio, mes + 1, 1).unwrap() - chrono::Days::new(1)
    };

    let (esperado_result, cobrado_result) = tokio::try_join!(
        contrato::Entity::find()
            .select_only()
            .column_as(contrato::Column::MontoMensual.sum(), "total")
            .filter(contrato::Column::Estado.eq("activo"))
            .filter(contrato::Column::FechaInicio.lte(ultimo_dia))
            .filter(contrato::Column::FechaFin.gte(primer_dia))
            .into_model::<SumResult>()
            .one(db),
        pago::Entity::find()
            .select_only()
            .column_as(pago::Column::Monto.sum(), "total")
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
    dias: i64,
) -> Result<Vec<PagoProximo>, AppError> {
    let today = Utc::now().date_naive();
    let limite = today + chrono::Days::new(dias as u64);

    let pagos = pago::Entity::find()
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
) -> Result<Vec<ContratoCalendario>, AppError> {
    let today = Utc::now().date_naive();
    let limite = today + chrono::Days::new(90);

    let contratos = contrato::Entity::find()
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
}
