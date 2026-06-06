use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::entities::{contrato, gasto, inquilino, organizacion, pago, propiedad, reporte_dgii};
use crate::errors::AppError;
use crate::models::reportes_dgii::{
    ItbisNetoResult, Registro606, Registro607, RegistroExcluido, RegistroPreview, ReporteGenerado,
};
use crate::services::fiscal::verificar_acceso_fiscal;

/// Format a single 607 record as a pipe-delimited line per Norma 07-2018.
pub fn formatear_linea_607(record: &Registro607) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        record.rnc_cliente,
        record.tipo_ncf,
        record.ncf,
        record.fecha_comprobante.format("%Y%m%d"),
        record.fecha_pago.format("%Y%m%d"),
        record.monto_servicios,
        record.monto_bienes,
        record.itbis_facturado,
        record.itbis_retenido,
        record.forma_pago,
    )
}

/// Format a single 606 record as a pipe-delimited line per Norma 07-2018.
pub fn formatear_linea_606(record: &Registro606) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        record.rnc_proveedor,
        record.tipo_ncf,
        record.ncf_proveedor,
        record.fecha_comprobante.format("%Y%m%d"),
        record.fecha_pago.format("%Y%m%d"),
        record.monto_servicios,
        record.monto_bienes,
        record.itbis_facturado,
        record.itbis_retenido,
        record.itbis_al_costo,
        record.forma_pago,
    )
}

/// Generate the header line for a 606 or 607 report.
///
/// Format: `RNC|YYYYMM|count|total`
pub fn generar_header(rnc: &str, periodo: &str, cantidad: u32, total: Decimal) -> String {
    format!("{rnc}|{periodo}|{cantidad}|{total}")
}

/// Parse a periodo string (YYYYMM) into the first and last day of that month.
fn parse_periodo(periodo: &str) -> Result<(NaiveDate, NaiveDate), AppError> {
    if periodo.len() != 6 {
        return Err(AppError::Validation(
            "Periodo debe tener formato YYYYMM".to_string(),
        ));
    }
    let year: i32 = periodo[..4]
        .parse()
        .map_err(|_| AppError::Validation("Periodo: año inválido".to_string()))?;
    let month: u32 = periodo[4..]
        .parse()
        .map_err(|_| AppError::Validation("Periodo: mes inválido".to_string()))?;
    if !(1..=12).contains(&month) {
        return Err(AppError::Validation(
            "Periodo: mes debe estar entre 01 y 12".to_string(),
        ));
    }

    let first_day = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| AppError::Validation("Periodo: fecha inválida".to_string()))?;

    // Last day: go to next month day 1, subtract 1 day
    let last_day = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| AppError::Validation("Periodo: fecha inválida".to_string()))?
    .pred_opt()
    .ok_or_else(|| AppError::Validation("Periodo: fecha inválida".to_string()))?;

    Ok((first_day, last_day))
}

/// Map `metodo_pago` from the payment system to DGII forma de pago codes.
fn mapear_forma_pago(metodo_pago: Option<&str>) -> &'static str {
    match metodo_pago {
        Some("efectivo") => "efectivo",
        Some("transferencia" | "cheque") => "cheque-transferencia",
        Some("tarjeta") => "tarjeta",
        _ => "otro",
    }
}

/// Determine if a property is residential based on its `tipo_propiedad`.
fn is_residencial(tipo_propiedad: &str) -> bool {
    matches!(tipo_propiedad, "residencial" | "apartamento" | "casa")
}

/// Generate the 607 report (income/sales) for a given organization and period.
///
/// Filters payments by `fecha_pago` within the requested month. Excludes records
/// missing RNC or `fecha_comprobante`. Includes records with blank NCF if other
/// data is complete. Tracks report status as borrador; prevents double-submission.
pub async fn generar_607(
    db: &DatabaseConnection,
    org_id: Uuid,
    periodo: &str,
    user_id: Uuid,
) -> Result<ReporteGenerado, AppError> {
    // Verify fiscal access
    let org = organizacion::Entity::find_by_id(org_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;
    verificar_acceso_fiscal(&org)?;

    // Check for existing submitted report (prevent double-submission)
    let existing_enviado = reporte_dgii::Entity::find()
        .filter(reporte_dgii::Column::OrganizacionId.eq(org_id))
        .filter(reporte_dgii::Column::TipoReporte.eq("607"))
        .filter(reporte_dgii::Column::Periodo.eq(periodo))
        .filter(reporte_dgii::Column::Estado.eq("enviado"))
        .one(db)
        .await?;

    if existing_enviado.is_some() {
        return Err(AppError::Conflict(
            "Reporte ya fue enviado a DGII para este período".to_string(),
        ));
    }

    let (fecha_inicio, fecha_fin) = parse_periodo(periodo)?;

    let org_rnc = org.rnc.clone().unwrap_or_default();

    // Fetch payments with fecha_pago in the target month for this organization
    let pagos = pago::Entity::find()
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .filter(pago::Column::FechaPago.is_not_null())
        .filter(pago::Column::FechaPago.gte(fecha_inicio))
        .filter(pago::Column::FechaPago.lte(fecha_fin))
        .filter(pago::Column::Estado.eq("pagado"))
        .all(db)
        .await?;

    // Fetch all contratos for this org to get tenant info and property type
    let contratos = contrato::Entity::find()
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .all(db)
        .await?;

    // Fetch all inquilinos for this org
    let inquilinos = inquilino::Entity::find()
        .filter(inquilino::Column::OrganizacionId.eq(org_id))
        .all(db)
        .await?;

    // Fetch all properties for this org
    let propiedades = propiedad::Entity::find()
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .all(db)
        .await?;

    // Build lookup maps
    let contrato_map: std::collections::HashMap<Uuid, &contrato::Model> =
        contratos.iter().map(|c| (c.id, c)).collect();
    let inquilino_map: std::collections::HashMap<Uuid, &inquilino::Model> =
        inquilinos.iter().map(|i| (i.id, i)).collect();
    let propiedad_map: std::collections::HashMap<Uuid, &propiedad::Model> =
        propiedades.iter().map(|p| (p.id, p)).collect();

    let mut registros: Vec<Registro607> = Vec::new();
    let mut excluidos: Vec<RegistroExcluido> = Vec::new();
    let mut monto_total = Decimal::ZERO;
    let mut itbis_total = Decimal::ZERO;

    for p in &pagos {
        // Resolve contrato → inquilino → RNC/cédula
        let contrato = contrato_map.get(&p.contrato_id);
        let inquilino = contrato.and_then(|c| inquilino_map.get(&c.inquilino_id));
        let propiedad_model = contrato.and_then(|c| propiedad_map.get(&c.propiedad_id));

        // Get the tenant's RNC/cédula
        let rnc_cliente = inquilino.map(|i| i.cedula.clone());

        // Check exclusion: missing RNC
        if rnc_cliente.as_ref().is_none_or(String::is_empty) {
            excluidos.push(RegistroExcluido {
                razon: "Falta RNC/cédula del cliente".to_string(),
                referencia: p.id.to_string(),
            });
            continue;
        }

        // Check exclusion: missing fecha_comprobante
        if p.fecha_comprobante.is_none() {
            excluidos.push(RegistroExcluido {
                razon: "Falta fecha_comprobante".to_string(),
                referencia: p.id.to_string(),
            });
            continue;
        }

        let rnc_cliente = rnc_cliente.unwrap_or_default();
        let fecha_comprobante = p.fecha_comprobante.unwrap_or(fecha_inicio);
        let fecha_pago = p.fecha_pago.unwrap_or(fecha_inicio);
        let tipo_propiedad = propiedad_model.map_or("residencial", |pr| pr.tipo_propiedad.as_str());

        // Residential income has ITBIS = 0
        let itbis_facturado = if is_residencial(tipo_propiedad) {
            Decimal::ZERO
        } else {
            p.monto_itbis.unwrap_or(Decimal::ZERO)
        };

        let monto_servicios = p.monto_base.unwrap_or(p.monto);
        let itbis_retenido = p.monto_itbis_retenido.unwrap_or(Decimal::ZERO);
        let forma_pago = mapear_forma_pago(p.metodo_pago.as_deref());

        // NCF can be blank if other data is complete
        let ncf = p.ncf.clone().unwrap_or_default();
        let tipo_ncf = p.tipo_ncf.clone().unwrap_or_default();

        let registro = Registro607 {
            rnc_cliente,
            tipo_ncf,
            ncf,
            fecha_comprobante,
            fecha_pago,
            monto_servicios,
            monto_bienes: Decimal::ZERO, // Rental is a service, bienes = 0
            itbis_facturado,
            itbis_retenido,
            forma_pago: forma_pago.to_string(),
        };

        monto_total += monto_servicios;
        itbis_total += itbis_facturado;
        registros.push(registro);
    }

    // Build report content
    #[allow(clippy::cast_possible_wrap)]
    let cantidad = registros.len() as u32;
    let header = generar_header(&org_rnc, periodo, cantidad, monto_total);

    let mut lines: Vec<String> = Vec::with_capacity(registros.len() + 1);
    lines.push(header);
    for r in &registros {
        lines.push(formatear_linea_607(r));
    }
    let contenido = lines.join("\n");

    // Build preview
    let preview: Vec<RegistroPreview> = registros
        .iter()
        .map(|r| RegistroPreview {
            campos: vec![
                r.rnc_cliente.clone(),
                r.tipo_ncf.clone(),
                r.ncf.clone(),
                r.fecha_comprobante.format("%Y%m%d").to_string(),
                r.fecha_pago.format("%Y%m%d").to_string(),
                r.monto_servicios.to_string(),
                r.monto_bienes.to_string(),
                r.itbis_facturado.to_string(),
                r.itbis_retenido.to_string(),
                r.forma_pago.clone(),
            ],
        })
        .collect();

    // Persist as borrador (overwrite existing borrador if any)
    persist_borrador_607(
        db,
        org_id,
        periodo,
        user_id,
        &contenido,
        cantidad,
        monto_total,
        itbis_total,
        &excluidos,
    )
    .await?;

    Ok(ReporteGenerado {
        contenido,
        preview,
        excluidos,
        cantidad_registros: cantidad,
        monto_total,
        itbis_total,
    })
}

/// Persist the 607 report as a borrador, updating existing or creating new.
#[allow(clippy::too_many_arguments)]
async fn persist_borrador_607(
    db: &DatabaseConnection,
    org_id: Uuid,
    periodo: &str,
    user_id: Uuid,
    contenido: &str,
    cantidad: u32,
    monto_total: Decimal,
    itbis_total: Decimal,
    excluidos: &[RegistroExcluido],
) -> Result<(), AppError> {
    let existing_borrador = reporte_dgii::Entity::find()
        .filter(reporte_dgii::Column::OrganizacionId.eq(org_id))
        .filter(reporte_dgii::Column::TipoReporte.eq("607"))
        .filter(reporte_dgii::Column::Periodo.eq(periodo))
        .filter(reporte_dgii::Column::Estado.eq("borrador"))
        .one(db)
        .await?;

    let now = chrono::Utc::now().into();
    let excluidos_json = if excluidos.is_empty() {
        None
    } else {
        Some(serde_json::to_value(excluidos).unwrap_or_default())
    };

    #[allow(clippy::cast_possible_wrap)]
    let cantidad_i32 = cantidad as i32;

    if let Some(existing) = existing_borrador {
        let mut active: reporte_dgii::ActiveModel = existing.into();
        active.cantidad_registros = Set(cantidad_i32);
        active.monto_total = Set(monto_total);
        active.itbis_total = Set(itbis_total);
        active.contenido = Set(contenido.to_string());
        active.registros_excluidos = Set(excluidos_json);
        active.generated_by = Set(user_id);
        active.generated_at = Set(now);
        active.updated_at = Set(now);
        active.update(db).await?;
    } else {
        let new_report = reporte_dgii::ActiveModel {
            id: Set(Uuid::new_v4()),
            organizacion_id: Set(org_id),
            tipo_reporte: Set("607".to_string()),
            periodo: Set(periodo.to_string()),
            estado: Set("borrador".to_string()),
            cantidad_registros: Set(cantidad_i32),
            monto_total: Set(monto_total),
            itbis_total: Set(itbis_total),
            contenido: Set(contenido.to_string()),
            registros_excluidos: Set(excluidos_json),
            generated_by: Set(user_id),
            generated_at: Set(now),
            submitted_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        new_report.insert(db).await?;
    }

    Ok(())
}

/// Persist the 606 report as a borrador, updating existing or creating new.
#[allow(clippy::too_many_arguments)]
async fn persist_borrador_606(
    db: &DatabaseConnection,
    org_id: Uuid,
    periodo: &str,
    user_id: Uuid,
    contenido: &str,
    cantidad: u32,
    monto_total: Decimal,
    itbis_total: Decimal,
    excluidos: &[RegistroExcluido],
) -> Result<(), AppError> {
    let existing_borrador = reporte_dgii::Entity::find()
        .filter(reporte_dgii::Column::OrganizacionId.eq(org_id))
        .filter(reporte_dgii::Column::TipoReporte.eq("606"))
        .filter(reporte_dgii::Column::Periodo.eq(periodo))
        .filter(reporte_dgii::Column::Estado.eq("borrador"))
        .one(db)
        .await?;

    let now = chrono::Utc::now().into();
    let excluidos_json = if excluidos.is_empty() {
        None
    } else {
        Some(serde_json::to_value(excluidos).unwrap_or_default())
    };

    #[allow(clippy::cast_possible_wrap)]
    let cantidad_i32 = cantidad as i32;

    if let Some(existing) = existing_borrador {
        let mut active: reporte_dgii::ActiveModel = existing.into();
        active.cantidad_registros = Set(cantidad_i32);
        active.monto_total = Set(monto_total);
        active.itbis_total = Set(itbis_total);
        active.contenido = Set(contenido.to_string());
        active.registros_excluidos = Set(excluidos_json);
        active.generated_by = Set(user_id);
        active.generated_at = Set(now);
        active.updated_at = Set(now);
        active.update(db).await?;
    } else {
        let new_report = reporte_dgii::ActiveModel {
            id: Set(Uuid::new_v4()),
            organizacion_id: Set(org_id),
            tipo_reporte: Set("606".to_string()),
            periodo: Set(periodo.to_string()),
            estado: Set("borrador".to_string()),
            cantidad_registros: Set(cantidad_i32),
            monto_total: Set(monto_total),
            itbis_total: Set(itbis_total),
            contenido: Set(contenido.to_string()),
            registros_excluidos: Set(excluidos_json),
            generated_by: Set(user_id),
            generated_at: Set(now),
            submitted_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        new_report.insert(db).await?;
    }

    Ok(())
}

/// Generate the 606 report (purchases/expenses) for a given organization and period.
///
/// Filters expenses by `fecha_gasto` (used as `fecha_pago`) within the requested month.
/// Excludes records missing proveedor (RNC) or without a valid date.
/// Includes records with blank NCF if other fiscal data is present.
pub async fn generar_606(
    db: &DatabaseConnection,
    org_id: Uuid,
    periodo: &str,
    user_id: Uuid,
) -> Result<ReporteGenerado, AppError> {
    // Verify fiscal access
    let org = organizacion::Entity::find_by_id(org_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;
    verificar_acceso_fiscal(&org)?;

    // Check for existing submitted report (prevent double-submission)
    let existing_enviado = reporte_dgii::Entity::find()
        .filter(reporte_dgii::Column::OrganizacionId.eq(org_id))
        .filter(reporte_dgii::Column::TipoReporte.eq("606"))
        .filter(reporte_dgii::Column::Periodo.eq(periodo))
        .filter(reporte_dgii::Column::Estado.eq("enviado"))
        .one(db)
        .await?;

    if existing_enviado.is_some() {
        return Err(AppError::Conflict(
            "Reporte ya fue enviado a DGII para este período".to_string(),
        ));
    }

    let (fecha_inicio, fecha_fin) = parse_periodo(periodo)?;
    let org_rnc = org.rnc.clone().unwrap_or_default();

    // Fetch expenses with fecha_gasto in the target month
    let gastos = gasto::Entity::find()
        .filter(gasto::Column::OrganizacionId.eq(org_id))
        .filter(gasto::Column::FechaGasto.gte(fecha_inicio))
        .filter(gasto::Column::FechaGasto.lte(fecha_fin))
        .filter(gasto::Column::Estado.eq("pagado"))
        .all(db)
        .await?;

    let mut registros: Vec<Registro606> = Vec::new();
    let mut excluidos: Vec<RegistroExcluido> = Vec::new();
    let mut monto_total = Decimal::ZERO;
    let mut itbis_total = Decimal::ZERO;

    for g in &gastos {
        // For 606: proveedor serves as RNC (supplier identifier)
        let rnc_proveedor = g.proveedor.clone();

        // Exclude if missing RNC (proveedor)
        if rnc_proveedor.as_ref().is_none_or(String::is_empty) {
            excluidos.push(RegistroExcluido {
                razon: "Falta RNC/cédula del proveedor".to_string(),
                referencia: g.id.to_string(),
            });
            continue;
        }

        // fecha_gasto acts as both fecha_comprobante and fecha_pago for expenses
        // that don't have separate fiscal date fields
        let fecha_comprobante = g.fecha_gasto;
        let fecha_pago = g.fecha_gasto;

        let rnc_proveedor = rnc_proveedor.unwrap_or_default();
        // NCF from numero_factura if available; blank NCF is allowed
        let ncf_proveedor = g.numero_factura.clone().unwrap_or_default();
        let tipo_ncf = if ncf_proveedor.is_empty() {
            String::new()
        } else {
            // Infer tipo from NCF prefix (first 3 chars like B01, B02, etc.)
            ncf_proveedor.get(..3).unwrap_or("").to_string()
        };

        let monto_servicios = g.monto;

        let registro = Registro606 {
            rnc_proveedor,
            tipo_ncf,
            ncf_proveedor,
            fecha_comprobante,
            fecha_pago,
            monto_servicios,
            monto_bienes: Decimal::ZERO,
            itbis_facturado: Decimal::ZERO, // No ITBIS fields on gasto entity yet
            itbis_retenido: Decimal::ZERO,
            itbis_al_costo: Decimal::ZERO,
            forma_pago: "otro".to_string(),
        };

        monto_total += monto_servicios;
        itbis_total += registro.itbis_facturado;
        registros.push(registro);
    }

    // Build report content
    #[allow(clippy::cast_possible_wrap)]
    let cantidad = registros.len() as u32;
    let header = generar_header(&org_rnc, periodo, cantidad, monto_total);

    let mut lines: Vec<String> = Vec::with_capacity(registros.len() + 1);
    lines.push(header);
    for r in &registros {
        lines.push(formatear_linea_606(r));
    }
    let contenido = lines.join("\n");

    // Build preview
    let preview: Vec<RegistroPreview> = registros
        .iter()
        .map(|r| RegistroPreview {
            campos: vec![
                r.rnc_proveedor.clone(),
                r.tipo_ncf.clone(),
                r.ncf_proveedor.clone(),
                r.fecha_comprobante.format("%Y%m%d").to_string(),
                r.fecha_pago.format("%Y%m%d").to_string(),
                r.monto_servicios.to_string(),
                r.monto_bienes.to_string(),
                r.itbis_facturado.to_string(),
                r.itbis_retenido.to_string(),
                r.itbis_al_costo.to_string(),
                r.forma_pago.clone(),
            ],
        })
        .collect();

    // Persist as borrador
    persist_borrador_606(
        db,
        org_id,
        periodo,
        user_id,
        &contenido,
        cantidad,
        monto_total,
        itbis_total,
        &excluidos,
    )
    .await?;

    Ok(ReporteGenerado {
        contenido,
        preview,
        excluidos,
        cantidad_registros: cantidad,
        monto_total,
        itbis_total,
    })
}

/// Calculate the net ITBIS (ITBIS cobrado from 607 - ITBIS pagado from 606)
/// for a given organization and period.
pub async fn calcular_itbis_neto(
    db: &DatabaseConnection,
    org_id: Uuid,
    periodo: &str,
) -> Result<ItbisNetoResult, AppError> {
    // Verify fiscal access
    let org = organizacion::Entity::find_by_id(org_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;
    verificar_acceso_fiscal(&org)?;

    let (fecha_inicio, fecha_fin) = parse_periodo(periodo)?;

    // Sum ITBIS collected from payments (607) in this period
    let pagos = pago::Entity::find()
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .filter(pago::Column::FechaPago.is_not_null())
        .filter(pago::Column::FechaPago.gte(fecha_inicio))
        .filter(pago::Column::FechaPago.lte(fecha_fin))
        .filter(pago::Column::Estado.eq("pagado"))
        .all(db)
        .await?;

    // For ITBIS cobrado: check property type to ensure residential = 0
    let contratos = contrato::Entity::find()
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .all(db)
        .await?;
    let propiedades = propiedad::Entity::find()
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .all(db)
        .await?;

    let contrato_map: std::collections::HashMap<Uuid, &contrato::Model> =
        contratos.iter().map(|c| (c.id, c)).collect();
    let propiedad_map: std::collections::HashMap<Uuid, &propiedad::Model> =
        propiedades.iter().map(|p| (p.id, p)).collect();

    let mut itbis_cobrado = Decimal::ZERO;
    for p in &pagos {
        let contrato_ref = contrato_map.get(&p.contrato_id);
        let propiedad_ref = contrato_ref.and_then(|c| propiedad_map.get(&c.propiedad_id));
        let tipo_propiedad = propiedad_ref.map_or("residencial", |pr| pr.tipo_propiedad.as_str());

        // Residential income has ITBIS = 0
        if !is_residencial(tipo_propiedad) {
            itbis_cobrado += p.monto_itbis.unwrap_or(Decimal::ZERO);
        }
    }

    // Sum ITBIS paid from expenses (606) in this period
    // Currently gastos don't have ITBIS fields; this is zero until migration adds them
    let _gastos = gasto::Entity::find()
        .filter(gasto::Column::OrganizacionId.eq(org_id))
        .filter(gasto::Column::FechaGasto.gte(fecha_inicio))
        .filter(gasto::Column::FechaGasto.lte(fecha_fin))
        .filter(gasto::Column::Estado.eq("pagado"))
        .all(db)
        .await?;

    let itbis_pagado = Decimal::ZERO;
    let itbis_neto = itbis_cobrado - itbis_pagado;

    Ok(ItbisNetoResult {
        itbis_cobrado,
        itbis_pagado,
        itbis_neto,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ── formatear_linea_607 tests ─────────────────────────────

    #[test]
    fn formatear_linea_607_basic() {
        let record = Registro607 {
            rnc_cliente: "123456789".to_string(),
            tipo_ncf: "B01".to_string(),
            ncf: "B0100000001".to_string(),
            fecha_comprobante: NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
            fecha_pago: NaiveDate::from_ymd_opt(2026, 6, 20).unwrap(),
            monto_servicios: Decimal::new(50_000, 2),
            monto_bienes: Decimal::ZERO,
            itbis_facturado: Decimal::new(9_000, 2),
            itbis_retenido: Decimal::ZERO,
            forma_pago: "transferencia".to_string(),
        };
        let line = formatear_linea_607(&record);
        assert_eq!(
            line,
            "123456789|B01|B0100000001|20260615|20260620|500.00|0|90.00|0|transferencia"
        );
    }

    #[test]
    fn formatear_linea_607_blank_ncf() {
        let record = Registro607 {
            rnc_cliente: "999888777".to_string(),
            tipo_ncf: String::new(),
            ncf: String::new(),
            fecha_comprobante: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            fecha_pago: NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
            monto_servicios: Decimal::new(25_000, 2),
            monto_bienes: Decimal::ZERO,
            itbis_facturado: Decimal::ZERO,
            itbis_retenido: Decimal::ZERO,
            forma_pago: "efectivo".to_string(),
        };
        let line = formatear_linea_607(&record);
        assert!(line.contains("||")); // Blank NCF shows as empty between pipes
    }

    // ── formatear_linea_606 tests ─────────────────────────────

    #[test]
    fn formatear_linea_606_basic() {
        let record = Registro606 {
            rnc_proveedor: "987654321".to_string(),
            tipo_ncf: "B01".to_string(),
            ncf_proveedor: "B0100000005".to_string(),
            fecha_comprobante: NaiveDate::from_ymd_opt(2026, 3, 10).unwrap(),
            fecha_pago: NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
            monto_servicios: Decimal::new(30_000, 2),
            monto_bienes: Decimal::new(10_000, 2),
            itbis_facturado: Decimal::new(7_200, 2),
            itbis_retenido: Decimal::ZERO,
            itbis_al_costo: Decimal::ZERO,
            forma_pago: "cheque-transferencia".to_string(),
        };
        let line = formatear_linea_606(&record);
        assert_eq!(
            line,
            "987654321|B01|B0100000005|20260310|20260315|300.00|100.00|72.00|0|0|cheque-transferencia"
        );
    }

    // ── generar_header tests ──────────────────────────────────

    #[test]
    fn generar_header_formato_correcto() {
        let header = generar_header("123456789", "202606", 5, Decimal::new(250_000, 2));
        assert_eq!(header, "123456789|202606|5|2500.00");
    }

    #[test]
    fn generar_header_zero_records() {
        let header = generar_header("111222333", "202601", 0, Decimal::ZERO);
        assert_eq!(header, "111222333|202601|0|0");
    }

    // ── parse_periodo tests ───────────────────────────────────

    #[test]
    fn parse_periodo_valid() {
        let (start, end) = parse_periodo("202606").unwrap();
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 6, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 6, 30).unwrap());
    }

    #[test]
    fn parse_periodo_december() {
        let (start, end) = parse_periodo("202612").unwrap();
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 12, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 12, 31).unwrap());
    }

    #[test]
    fn parse_periodo_february_leap_year() {
        let (start, end) = parse_periodo("202402").unwrap();
        assert_eq!(start, NaiveDate::from_ymd_opt(2024, 2, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
    }

    #[test]
    fn parse_periodo_invalid_month() {
        assert!(parse_periodo("202613").is_err());
    }

    #[test]
    fn parse_periodo_invalid_length() {
        assert!(parse_periodo("20261").is_err());
        assert!(parse_periodo("2026012").is_err());
    }

    // ── mapear_forma_pago tests ───────────────────────────────

    #[test]
    fn mapear_forma_pago_values() {
        assert_eq!(mapear_forma_pago(Some("efectivo")), "efectivo");
        assert_eq!(
            mapear_forma_pago(Some("transferencia")),
            "cheque-transferencia"
        );
        assert_eq!(mapear_forma_pago(Some("cheque")), "cheque-transferencia");
        assert_eq!(mapear_forma_pago(Some("tarjeta")), "tarjeta");
        assert_eq!(mapear_forma_pago(Some("otro")), "otro");
        assert_eq!(mapear_forma_pago(None), "otro");
    }

    // ── is_residencial tests ──────────────────────────────────

    #[test]
    fn is_residencial_values() {
        assert!(is_residencial("residencial"));
        assert!(is_residencial("apartamento"));
        assert!(is_residencial("casa"));
        assert!(!is_residencial("comercial"));
        assert!(!is_residencial("industrial"));
        assert!(!is_residencial("terreno"));
    }
}
