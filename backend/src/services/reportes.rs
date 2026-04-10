use std::collections::HashMap;

use chrono::{NaiveDate, Utc};
use numaelis_rckive_genpdf::Element as _;
use numaelis_rckive_genpdf::elements;
use numaelis_rckive_genpdf::style;
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use uuid::Uuid;

use crate::entities::{contrato, gasto, inquilino, pago, propiedad};
use crate::errors::AppError;
use crate::models::reporte::{
    HistorialPagoEntry, IngresoReportQuery, IngresoReportRow, IngresoReportSummary,
    RentabilidadReportQuery, RentabilidadReportRow, RentabilidadReportSummary,
};

pub async fn generar_reporte_ingresos(
    db: &DatabaseConnection,
    org_id: Uuid,
    query: IngresoReportQuery,
    generated_by: String,
) -> Result<IngresoReportSummary, AppError> {
    if query.mes < 1 || query.mes > 12 {
        return Err(AppError::Validation("Mes debe estar entre 1 y 12".into()));
    }

    let first_day = NaiveDate::from_ymd_opt(query.anio, query.mes, 1)
        .ok_or_else(|| AppError::Validation("Fecha inválida".into()))?;
    let last_day = if query.mes == 12 {
        NaiveDate::from_ymd_opt(query.anio + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(query.anio, query.mes + 1, 1)
    }
    .ok_or_else(|| AppError::Validation("Fecha inválida".into()))?
    .pred_opt()
    .ok_or_else(|| AppError::Validation("Fecha inválida".into()))?;

    let mut pago_select = pago::Entity::find()
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .filter(pago::Column::FechaVencimiento.gte(first_day))
        .filter(pago::Column::FechaVencimiento.lte(last_day));

    if let Some(propiedad_id) = query.propiedad_id {
        let contrato_ids: Vec<uuid::Uuid> = contrato::Entity::find()
            .filter(contrato::Column::PropiedadId.eq(propiedad_id))
            .all(db)
            .await?
            .into_iter()
            .map(|c| c.id)
            .collect();

        if contrato_ids.is_empty() {
            return Ok(empty_summary(generated_by));
        }
        pago_select = pago_select.filter(pago::Column::ContratoId.is_in(contrato_ids));
    }

    if let Some(inquilino_id) = query.inquilino_id {
        let contrato_ids: Vec<uuid::Uuid> = contrato::Entity::find()
            .filter(contrato::Column::InquilinoId.eq(inquilino_id))
            .all(db)
            .await?
            .into_iter()
            .map(|c| c.id)
            .collect();

        if contrato_ids.is_empty() {
            return Ok(empty_summary(generated_by));
        }
        pago_select = pago_select.filter(pago::Column::ContratoId.is_in(contrato_ids));
    }

    let pagos = pago_select.all(db).await?;

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

    let mut rows = Vec::with_capacity(pagos.len());
    let mut total_pagado = Decimal::ZERO;
    let mut total_pendiente = Decimal::ZERO;
    let mut total_atrasado = Decimal::ZERO;

    let contrato_map: HashMap<uuid::Uuid, &contrato::Model> =
        contratos.iter().map(|c| (c.id, c)).collect();
    let prop_map: HashMap<uuid::Uuid, &propiedad::Model> =
        propiedades.iter().map(|p| (p.id, p)).collect();
    let inq_map: HashMap<uuid::Uuid, &inquilino::Model> =
        inquilinos.iter().map(|i| (i.id, i)).collect();

    for p in &pagos {
        let contrato_model = contrato_map.get(&p.contrato_id);
        let (prop_titulo, inq_nombre) = contrato_model.map_or_else(
            || (String::new(), String::new()),
            |c| {
                let prop = prop_map
                    .get(&c.propiedad_id)
                    .map(|pr| pr.titulo.clone())
                    .unwrap_or_default();
                let inq = inq_map
                    .get(&c.inquilino_id)
                    .map(|i| format!("{} {}", i.nombre, i.apellido))
                    .unwrap_or_default();
                (prop, inq)
            },
        );

        match p.estado.as_str() {
            "pagado" => total_pagado += p.monto,
            "pendiente" => total_pendiente += p.monto,
            "atrasado" => total_atrasado += p.monto,
            _ => {}
        }

        rows.push(IngresoReportRow {
            propiedad_titulo: prop_titulo,
            inquilino_nombre: inq_nombre,
            monto: p.monto,
            moneda: p.moneda.clone(),
            estado: p.estado.clone(),
        });
    }

    let tasa_ocupacion = calcular_tasa_ocupacion(db, org_id).await?;

    Ok(IngresoReportSummary {
        rows,
        total_pagado,
        total_pendiente,
        total_atrasado,
        tasa_ocupacion,
        generated_at: Utc::now(),
        generated_by,
    })
}

pub async fn historial_pagos(
    db: &DatabaseConnection,
    org_id: Uuid,
    fecha_desde: NaiveDate,
    fecha_hasta: NaiveDate,
) -> Result<Vec<HistorialPagoEntry>, AppError> {
    if fecha_desde > fecha_hasta {
        return Err(AppError::Validation(
            "fecha_desde no puede ser posterior a fecha_hasta".into(),
        ));
    }

    let pagos = pago::Entity::find()
        .filter(pago::Column::OrganizacionId.eq(org_id))
        .filter(pago::Column::FechaVencimiento.gte(fecha_desde))
        .filter(pago::Column::FechaVencimiento.lte(fecha_hasta))
        .order_by_asc(pago::Column::FechaVencimiento)
        .all(db)
        .await?;

    let entries = pagos
        .into_iter()
        .map(|p| HistorialPagoEntry {
            contrato_id: p.contrato_id,
            monto: p.monto,
            moneda: p.moneda,
            fecha_vencimiento: p.fecha_vencimiento,
            fecha_pago: p.fecha_pago,
            estado: p.estado,
        })
        .collect();

    Ok(entries)
}

pub async fn calcular_tasa_ocupacion(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<f64, AppError> {
    let total = propiedad::Entity::find()
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .count(db)
        .await?;
    if total == 0 {
        return Ok(0.0);
    }

    let ocupadas = propiedad::Entity::find()
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .filter(propiedad::Column::Estado.eq("ocupada"))
        .count(db)
        .await?;

    let rate = (ocupadas as f64 / total as f64) * 100.0;
    Ok((rate * 10.0).round() / 10.0)
}

fn empty_summary(generated_by: String) -> IngresoReportSummary {
    IngresoReportSummary {
        rows: vec![],
        total_pagado: Decimal::ZERO,
        total_pendiente: Decimal::ZERO,
        total_atrasado: Decimal::ZERO,
        tasa_ocupacion: 0.0,
        generated_at: Utc::now(),
        generated_by,
    }
}

pub async fn generar_reporte_rentabilidad(
    db: &DatabaseConnection,
    org_id: Uuid,
    query: RentabilidadReportQuery,
    generated_by: String,
) -> Result<RentabilidadReportSummary, AppError> {
    if query.mes < 1 || query.mes > 12 {
        return Err(AppError::Validation("Mes debe estar entre 1 y 12".into()));
    }

    let first_day = NaiveDate::from_ymd_opt(query.anio, query.mes, 1)
        .ok_or_else(|| AppError::Validation("Fecha inválida".into()))?;
    let last_day = if query.mes == 12 {
        NaiveDate::from_ymd_opt(query.anio + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(query.anio, query.mes + 1, 1)
    }
    .ok_or_else(|| AppError::Validation("Fecha inválida".into()))?
    .pred_opt()
    .ok_or_else(|| AppError::Validation("Fecha inválida".into()))?;

    let propiedades = if let Some(pid) = query.propiedad_id {
        propiedad::Entity::find()
            .filter(propiedad::Column::OrganizacionId.eq(org_id))
            .filter(propiedad::Column::Id.eq(pid))
            .all(db)
            .await?
    } else {
        propiedad::Entity::find()
            .filter(propiedad::Column::OrganizacionId.eq(org_id))
            .all(db)
            .await?
    };

    let prop_ids: Vec<uuid::Uuid> = propiedades.iter().map(|p| p.id).collect();

    // Batch-load all contratos for the target propiedades
    let all_contratos = contrato::Entity::find()
        .filter(contrato::Column::PropiedadId.is_in(prop_ids.clone()))
        .all(db)
        .await?;

    let all_contrato_ids: Vec<uuid::Uuid> = all_contratos.iter().map(|c| c.id).collect();

    // Batch-load all paid pagos for those contratos in the date range
    let all_pagos = if all_contrato_ids.is_empty() {
        vec![]
    } else {
        pago::Entity::find()
            .filter(pago::Column::ContratoId.is_in(all_contrato_ids))
            .filter(pago::Column::Estado.eq("pagado"))
            .filter(pago::Column::FechaVencimiento.gte(first_day))
            .filter(pago::Column::FechaVencimiento.lte(last_day))
            .all(db)
            .await?
    };

    // Batch-load all paid gastos for those propiedades in the date range
    let all_gastos = gasto::Entity::find()
        .filter(gasto::Column::PropiedadId.is_in(prop_ids))
        .filter(gasto::Column::Estado.eq("pagado"))
        .filter(gasto::Column::FechaGasto.gte(first_day))
        .filter(gasto::Column::FechaGasto.lte(last_day))
        .all(db)
        .await?;

    // Build lookup maps: contrato -> propiedad, pago -> contrato
    let contrato_prop_map: HashMap<uuid::Uuid, uuid::Uuid> = all_contratos
        .iter()
        .map(|c| (c.id, c.propiedad_id))
        .collect();

    // Aggregate income per propiedad via contrato linkage
    let mut income_by_prop: HashMap<uuid::Uuid, Decimal> = HashMap::new();
    for p in &all_pagos {
        if let Some(&prop_id) = contrato_prop_map.get(&p.contrato_id) {
            *income_by_prop.entry(prop_id).or_insert(Decimal::ZERO) += p.monto;
        }
    }

    // Aggregate expenses per propiedad
    let mut expense_by_prop: HashMap<uuid::Uuid, Decimal> = HashMap::new();
    for g in &all_gastos {
        *expense_by_prop
            .entry(g.propiedad_id)
            .or_insert(Decimal::ZERO) += g.monto;
    }

    let mut rows = Vec::with_capacity(propiedades.len());
    let mut grand_ingresos = Decimal::ZERO;
    let mut grand_gastos = Decimal::ZERO;

    for prop in &propiedades {
        let total_ingresos = income_by_prop
            .get(&prop.id)
            .copied()
            .unwrap_or(Decimal::ZERO);
        let total_gastos = expense_by_prop
            .get(&prop.id)
            .copied()
            .unwrap_or(Decimal::ZERO);
        let ingreso_neto = total_ingresos - total_gastos;

        grand_ingresos += total_ingresos;
        grand_gastos += total_gastos;

        rows.push(RentabilidadReportRow {
            propiedad_id: prop.id,
            propiedad_titulo: prop.titulo.clone(),
            total_ingresos,
            total_gastos,
            ingreso_neto,
            moneda: prop.moneda.clone(),
        });
    }

    Ok(RentabilidadReportSummary {
        rows,
        total_ingresos: grand_ingresos,
        total_gastos: grand_gastos,
        total_neto: grand_ingresos - grand_gastos,
        mes: query.mes,
        anio: query.anio,
        generated_at: Utc::now(),
        generated_by,
    })
}

fn load_font_family() -> Result<
    numaelis_rckive_genpdf::fonts::FontFamily<numaelis_rckive_genpdf::fonts::FontData>,
    AppError,
> {
    let regular = numaelis_rckive_genpdf::fonts::FontData::new(
        include_bytes!("../../fonts/Arial-Regular.ttf").to_vec(),
        None,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error cargando fuente regular: {e}")))?;
    let bold = numaelis_rckive_genpdf::fonts::FontData::new(
        include_bytes!("../../fonts/Arial-Bold.ttf").to_vec(),
        None,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error cargando fuente bold: {e}")))?;
    let italic = numaelis_rckive_genpdf::fonts::FontData::new(
        include_bytes!("../../fonts/Arial-Italic.ttf").to_vec(),
        None,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error cargando fuente italic: {e}")))?;
    let bold_italic = numaelis_rckive_genpdf::fonts::FontData::new(
        include_bytes!("../../fonts/Arial-BoldItalic.ttf").to_vec(),
        None,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error cargando fuente bold-italic: {e}")))?;

    Ok(numaelis_rckive_genpdf::fonts::FontFamily {
        regular,
        bold,
        italic,
        bold_italic,
    })
}

pub fn exportar_pdf(summary: &IngresoReportSummary) -> Result<Vec<u8>, AppError> {
    let font_family = load_font_family().map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    let mut doc = numaelis_rckive_genpdf::Document::new(font_family);
    doc.set_title("Reporte de Ingresos");
    doc.set_minimal_conformance();

    let mut decorator = numaelis_rckive_genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    doc.push(
        elements::Paragraph::new("Reporte de Ingresos")
            .styled(style::Style::new().bold().with_font_size(18)),
    );
    doc.push(elements::Break::new(1.0));

    let timestamp = summary.generated_at.format("%d/%m/%Y %H:%M:%S").to_string();
    doc.push(elements::Paragraph::new(format!(
        "Generado: {} | Usuario: {}",
        timestamp, summary.generated_by
    )));
    doc.push(elements::Break::new(1.5));

    if summary.rows.is_empty() {
        doc.push(
            elements::Paragraph::new("Sin registros para el periodo")
                .styled(style::Style::new().italic().with_font_size(12)),
        );
    } else {
        let mut table = elements::TableLayout::new(vec![3, 3, 2, 1, 2]);
        table.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));

        let header_row = table.row();
        header_row
            .element(
                elements::Paragraph::new("Propiedad")
                    .styled(style::Style::new().bold().with_font_size(10)),
            )
            .element(
                elements::Paragraph::new("Inquilino")
                    .styled(style::Style::new().bold().with_font_size(10)),
            )
            .element(
                elements::Paragraph::new("Monto")
                    .styled(style::Style::new().bold().with_font_size(10)),
            )
            .element(
                elements::Paragraph::new("Moneda")
                    .styled(style::Style::new().bold().with_font_size(10)),
            )
            .element(
                elements::Paragraph::new("Estado")
                    .styled(style::Style::new().bold().with_font_size(10)),
            )
            .push()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error en tabla PDF: {e}")))?;

        for row in &summary.rows {
            let data_row = table.row();
            data_row
                .element(elements::Paragraph::new(&row.propiedad_titulo))
                .element(elements::Paragraph::new(&row.inquilino_nombre))
                .element(elements::Paragraph::new(row.monto.to_string()))
                .element(elements::Paragraph::new(&row.moneda))
                .element(elements::Paragraph::new(&row.estado))
                .push()
                .map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("Error en fila de tabla PDF: {e}"))
                })?;
        }

        doc.push(table);
    }

    doc.push(elements::Break::new(1.5));
    doc.push(
        elements::Paragraph::new("Resumen").styled(style::Style::new().bold().with_font_size(14)),
    );
    doc.push(elements::Break::new(0.5));
    doc.push(elements::Paragraph::new(format!(
        "Total Pagado: {}",
        summary.total_pagado
    )));
    doc.push(elements::Paragraph::new(format!(
        "Total Pendiente: {}",
        summary.total_pendiente
    )));
    doc.push(elements::Paragraph::new(format!(
        "Total Atrasado: {}",
        summary.total_atrasado
    )));
    doc.push(elements::Paragraph::new(format!(
        "Tasa de Ocupacion: {:.1}%",
        summary.tasa_ocupacion
    )));

    let mut buf: Vec<u8> = Vec::new();
    doc.render(&mut buf)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error renderizando PDF: {e}")))?;

    Ok(buf)
}

pub fn exportar_xlsx(summary: &IngresoReportSummary) -> Result<Vec<u8>, AppError> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    let title_format = rust_xlsxwriter::Format::new().set_bold().set_font_size(16);
    let header_format = rust_xlsxwriter::Format::new()
        .set_bold()
        .set_border(rust_xlsxwriter::FormatBorder::Thin);
    let bold_format = rust_xlsxwriter::Format::new().set_bold();

    worksheet
        .write_string_with_format(0, 0, "Reporte de Ingresos", &title_format)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;

    let timestamp = summary.generated_at.format("%d/%m/%Y %H:%M:%S").to_string();
    worksheet
        .write_string(1, 0, format!("Generado: {timestamp}"))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .write_string(2, 0, format!("Usuario: {}", &summary.generated_by))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;

    let header_row: u32 = 4;
    let headers = ["Propiedad", "Inquilino", "Monto", "Moneda", "Estado"];
    for (col, header) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(header_row, col as u16, *header, &header_format)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX header: {e}")))?;
    }

    worksheet
        .set_column_width(0, 25)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .set_column_width(1, 25)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .set_column_width(2, 15)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .set_column_width(3, 10)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .set_column_width(4, 15)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;

    if summary.rows.is_empty() {
        worksheet
            .write_string(header_row + 1, 0, "Sin registros para el periodo")
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    } else {
        for (idx, row) in summary.rows.iter().enumerate() {
            let r = header_row + 1 + idx as u32;
            worksheet
                .write_string(r, 0, &row.propiedad_titulo)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
            worksheet
                .write_string(r, 1, &row.inquilino_nombre)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
            let monto_f64 = row.monto.to_string().parse::<f64>().unwrap_or_default();
            worksheet
                .write_number(r, 2, monto_f64)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
            worksheet
                .write_string(r, 3, &row.moneda)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
            worksheet
                .write_string(r, 4, &row.estado)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
        }
    }

    let summary_row = header_row + 2 + summary.rows.len() as u32;
    let pagado_f64 = summary
        .total_pagado
        .to_string()
        .parse::<f64>()
        .unwrap_or_default();
    let pendiente_f64 = summary
        .total_pendiente
        .to_string()
        .parse::<f64>()
        .unwrap_or_default();
    let atrasado_f64 = summary
        .total_atrasado
        .to_string()
        .parse::<f64>()
        .unwrap_or_default();

    worksheet
        .write_string_with_format(summary_row, 0, "Total Pagado", &bold_format)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .write_number(summary_row, 2, pagado_f64)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;

    worksheet
        .write_string_with_format(summary_row + 1, 0, "Total Pendiente", &bold_format)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .write_number(summary_row + 1, 2, pendiente_f64)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;

    worksheet
        .write_string_with_format(summary_row + 2, 0, "Total Atrasado", &bold_format)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .write_number(summary_row + 2, 2, atrasado_f64)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;

    worksheet
        .write_string_with_format(summary_row + 3, 0, "Tasa de Ocupacion", &bold_format)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .write_string(
            summary_row + 3,
            2,
            format!("{:.1}%", summary.tasa_ocupacion),
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;

    let buf = workbook
        .save_to_buffer()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error guardando XLSX: {e}")))?;

    Ok(buf)
}

pub fn exportar_rentabilidad_pdf(summary: &RentabilidadReportSummary) -> Result<Vec<u8>, AppError> {
    let font_family = load_font_family().map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    let mut doc = numaelis_rckive_genpdf::Document::new(font_family);
    doc.set_title("Reporte de Rentabilidad");
    doc.set_minimal_conformance();

    let mut decorator = numaelis_rckive_genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    doc.push(
        elements::Paragraph::new("Reporte de Rentabilidad")
            .styled(style::Style::new().bold().with_font_size(18)),
    );
    doc.push(elements::Break::new(1.0));

    let timestamp = summary.generated_at.format("%d/%m/%Y %H:%M:%S").to_string();
    doc.push(elements::Paragraph::new(format!(
        "Periodo: {}/{} | Generado: {} | Usuario: {}",
        summary.mes, summary.anio, timestamp, summary.generated_by
    )));
    doc.push(elements::Break::new(1.5));

    if summary.rows.is_empty() {
        doc.push(
            elements::Paragraph::new("Sin registros para el periodo")
                .styled(style::Style::new().italic().with_font_size(12)),
        );
    } else {
        let mut table = elements::TableLayout::new(vec![4, 2, 2, 2, 1]);
        table.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));

        let header_row = table.row();
        header_row
            .element(
                elements::Paragraph::new("Propiedad")
                    .styled(style::Style::new().bold().with_font_size(10)),
            )
            .element(
                elements::Paragraph::new("Ingresos")
                    .styled(style::Style::new().bold().with_font_size(10)),
            )
            .element(
                elements::Paragraph::new("Gastos")
                    .styled(style::Style::new().bold().with_font_size(10)),
            )
            .element(
                elements::Paragraph::new("Neto")
                    .styled(style::Style::new().bold().with_font_size(10)),
            )
            .element(
                elements::Paragraph::new("Moneda")
                    .styled(style::Style::new().bold().with_font_size(10)),
            )
            .push()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error en tabla PDF: {e}")))?;

        for row in &summary.rows {
            let data_row = table.row();
            data_row
                .element(elements::Paragraph::new(&row.propiedad_titulo))
                .element(elements::Paragraph::new(row.total_ingresos.to_string()))
                .element(elements::Paragraph::new(row.total_gastos.to_string()))
                .element(elements::Paragraph::new(row.ingreso_neto.to_string()))
                .element(elements::Paragraph::new(&row.moneda))
                .push()
                .map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("Error en fila de tabla PDF: {e}"))
                })?;
        }

        doc.push(table);
    }

    doc.push(elements::Break::new(1.5));
    doc.push(
        elements::Paragraph::new("Resumen").styled(style::Style::new().bold().with_font_size(14)),
    );
    doc.push(elements::Break::new(0.5));
    doc.push(elements::Paragraph::new(format!(
        "Total Ingresos: {}",
        summary.total_ingresos
    )));
    doc.push(elements::Paragraph::new(format!(
        "Total Gastos: {}",
        summary.total_gastos
    )));
    doc.push(elements::Paragraph::new(format!(
        "Total Neto: {}",
        summary.total_neto
    )));

    let mut buf: Vec<u8> = Vec::new();
    doc.render(&mut buf)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error renderizando PDF: {e}")))?;

    Ok(buf)
}

pub fn exportar_rentabilidad_xlsx(
    summary: &RentabilidadReportSummary,
) -> Result<Vec<u8>, AppError> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    let title_format = rust_xlsxwriter::Format::new().set_bold().set_font_size(16);
    let header_format = rust_xlsxwriter::Format::new()
        .set_bold()
        .set_border(rust_xlsxwriter::FormatBorder::Thin);
    let bold_format = rust_xlsxwriter::Format::new().set_bold();

    worksheet
        .write_string_with_format(0, 0, "Reporte de Rentabilidad", &title_format)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;

    let timestamp = summary.generated_at.format("%d/%m/%Y %H:%M:%S").to_string();
    worksheet
        .write_string(1, 0, format!("Periodo: {}/{}", summary.mes, summary.anio))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .write_string(2, 0, format!("Generado: {timestamp}"))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .write_string(3, 0, format!("Usuario: {}", &summary.generated_by))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;

    let header_row: u32 = 5;
    let headers = ["Propiedad", "Ingresos", "Gastos", "Neto", "Moneda"];
    for (col, header) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(header_row, col as u16, *header, &header_format)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX header: {e}")))?;
    }

    worksheet
        .set_column_width(0, 30)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .set_column_width(1, 15)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .set_column_width(2, 15)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .set_column_width(3, 15)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .set_column_width(4, 10)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;

    if summary.rows.is_empty() {
        worksheet
            .write_string(header_row + 1, 0, "Sin registros para el periodo")
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    } else {
        for (idx, row) in summary.rows.iter().enumerate() {
            let r = header_row + 1 + idx as u32;
            worksheet
                .write_string(r, 0, &row.propiedad_titulo)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
            let ingresos_f64 = row
                .total_ingresos
                .to_string()
                .parse::<f64>()
                .unwrap_or_default();
            worksheet
                .write_number(r, 1, ingresos_f64)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
            let gastos_f64 = row
                .total_gastos
                .to_string()
                .parse::<f64>()
                .unwrap_or_default();
            worksheet
                .write_number(r, 2, gastos_f64)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
            let neto_f64 = row
                .ingreso_neto
                .to_string()
                .parse::<f64>()
                .unwrap_or_default();
            worksheet
                .write_number(r, 3, neto_f64)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
            worksheet
                .write_string(r, 4, &row.moneda)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
        }
    }

    let summary_row = header_row + 2 + summary.rows.len() as u32;
    let total_ingresos_f64 = summary
        .total_ingresos
        .to_string()
        .parse::<f64>()
        .unwrap_or_default();
    let total_gastos_f64 = summary
        .total_gastos
        .to_string()
        .parse::<f64>()
        .unwrap_or_default();
    let total_neto_f64 = summary
        .total_neto
        .to_string()
        .parse::<f64>()
        .unwrap_or_default();

    worksheet
        .write_string_with_format(summary_row, 0, "Total Ingresos", &bold_format)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .write_number(summary_row, 1, total_ingresos_f64)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;

    worksheet
        .write_string_with_format(summary_row + 1, 0, "Total Gastos", &bold_format)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .write_number(summary_row + 1, 2, total_gastos_f64)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;

    worksheet
        .write_string_with_format(summary_row + 2, 0, "Total Neto", &bold_format)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;
    worksheet
        .write_number(summary_row + 2, 3, total_neto_f64)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error XLSX: {e}")))?;

    let buf = workbook
        .save_to_buffer()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error guardando XLSX: {e}")))?;

    Ok(buf)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample_summary_with_rows() -> IngresoReportSummary {
        IngresoReportSummary {
            rows: vec![
                IngresoReportRow {
                    propiedad_titulo: "Apartamento Centro".into(),
                    inquilino_nombre: "Juan Perez".into(),
                    monto: Decimal::new(25000, 0),
                    moneda: "DOP".into(),
                    estado: "pagado".into(),
                },
                IngresoReportRow {
                    propiedad_titulo: "Casa Naco".into(),
                    inquilino_nombre: "Maria Lopez".into(),
                    monto: Decimal::new(1500, 0),
                    moneda: "USD".into(),
                    estado: "pendiente".into(),
                },
            ],
            total_pagado: Decimal::new(25000, 0),
            total_pendiente: Decimal::new(1500, 0),
            total_atrasado: Decimal::ZERO,
            tasa_ocupacion: 66.7,
            generated_at: Utc.with_ymd_and_hms(2025, 4, 10, 12, 0, 0).unwrap(),
            generated_by: "Admin Test".into(),
        }
    }

    fn sample_empty_summary() -> IngresoReportSummary {
        IngresoReportSummary {
            rows: vec![],
            total_pagado: Decimal::ZERO,
            total_pendiente: Decimal::ZERO,
            total_atrasado: Decimal::ZERO,
            tasa_ocupacion: 0.0,
            generated_at: Utc.with_ymd_and_hms(2025, 4, 10, 12, 0, 0).unwrap(),
            generated_by: "Admin Test".into(),
        }
    }

    #[test]
    fn exportar_pdf_with_data_returns_valid_bytes() {
        let summary = sample_summary_with_rows();
        let result = exportar_pdf(&summary);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn exportar_pdf_empty_report_returns_valid_pdf() {
        let summary = sample_empty_summary();
        let result = exportar_pdf(&summary);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn exportar_xlsx_with_data_returns_valid_bytes() {
        let summary = sample_summary_with_rows();
        let result = exportar_xlsx(&summary);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[0..2], b"PK");
    }

    #[test]
    fn exportar_xlsx_empty_report_returns_valid_xlsx() {
        let summary = sample_empty_summary();
        let result = exportar_xlsx(&summary);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[0..2], b"PK");
    }

    #[test]
    fn empty_summary_returns_correct_defaults() {
        let summary = empty_summary("test@example.com".into());
        assert!(summary.rows.is_empty());
        assert_eq!(summary.total_pagado, Decimal::ZERO);
        assert_eq!(summary.total_pendiente, Decimal::ZERO);
        assert_eq!(summary.total_atrasado, Decimal::ZERO);
        assert_eq!(summary.tasa_ocupacion, 0.0);
        assert_eq!(summary.generated_by, "test@example.com");
    }

    #[test]
    fn empty_summary_preserves_generated_by() {
        let summary = empty_summary("admin@inmobiliaria.do".into());
        assert_eq!(summary.generated_by, "admin@inmobiliaria.do");
    }

    #[test]
    fn sample_summary_mixed_statuses_has_correct_totals() {
        let summary = sample_summary_with_rows();
        assert_eq!(summary.total_pagado, Decimal::new(25000, 0));
        assert_eq!(summary.total_pendiente, Decimal::new(1500, 0));
        assert_eq!(summary.total_atrasado, Decimal::ZERO);
        assert_eq!(summary.rows.len(), 2);
    }

    #[test]
    fn exportar_pdf_with_mixed_statuses_includes_all_rows() {
        let mut summary = sample_summary_with_rows();
        summary.rows.push(IngresoReportRow {
            propiedad_titulo: "Local Comercial".into(),
            inquilino_nombre: "Pedro Garcia".into(),
            monto: Decimal::new(8000, 0),
            moneda: "DOP".into(),
            estado: "atrasado".into(),
        });
        summary.total_atrasado = Decimal::new(8000, 0);
        let result = exportar_pdf(&summary);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn exportar_xlsx_with_mixed_statuses_produces_valid_file() {
        let mut summary = sample_summary_with_rows();
        summary.rows.push(IngresoReportRow {
            propiedad_titulo: "Oficina Piantini".into(),
            inquilino_nombre: "Ana Martinez".into(),
            monto: Decimal::new(3500, 0),
            moneda: "USD".into(),
            estado: "atrasado".into(),
        });
        summary.total_atrasado = Decimal::new(3500, 0);
        let result = exportar_xlsx(&summary);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..2], b"PK");
    }

    fn sample_rentabilidad_with_rows() -> RentabilidadReportSummary {
        RentabilidadReportSummary {
            rows: vec![
                RentabilidadReportRow {
                    propiedad_id: uuid::Uuid::new_v4(),
                    propiedad_titulo: "Apartamento Centro".into(),
                    total_ingresos: Decimal::new(50000, 0),
                    total_gastos: Decimal::new(15000, 0),
                    ingreso_neto: Decimal::new(35000, 0),
                    moneda: "DOP".into(),
                },
                RentabilidadReportRow {
                    propiedad_id: uuid::Uuid::new_v4(),
                    propiedad_titulo: "Casa Naco".into(),
                    total_ingresos: Decimal::new(2000, 0),
                    total_gastos: Decimal::new(500, 0),
                    ingreso_neto: Decimal::new(1500, 0),
                    moneda: "USD".into(),
                },
            ],
            total_ingresos: Decimal::new(52000, 0),
            total_gastos: Decimal::new(15500, 0),
            total_neto: Decimal::new(36500, 0),
            mes: 4,
            anio: 2025,
            generated_at: Utc.with_ymd_and_hms(2025, 4, 10, 12, 0, 0).unwrap(),
            generated_by: "Admin Test".into(),
        }
    }

    fn sample_empty_rentabilidad() -> RentabilidadReportSummary {
        RentabilidadReportSummary {
            rows: vec![],
            total_ingresos: Decimal::ZERO,
            total_gastos: Decimal::ZERO,
            total_neto: Decimal::ZERO,
            mes: 4,
            anio: 2025,
            generated_at: Utc.with_ymd_and_hms(2025, 4, 10, 12, 0, 0).unwrap(),
            generated_by: "Admin Test".into(),
        }
    }

    #[test]
    fn exportar_rentabilidad_pdf_with_data_returns_valid_bytes() {
        let summary = sample_rentabilidad_with_rows();
        let result = exportar_rentabilidad_pdf(&summary);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn exportar_rentabilidad_pdf_empty_report_returns_valid_pdf() {
        let summary = sample_empty_rentabilidad();
        let result = exportar_rentabilidad_pdf(&summary);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn exportar_rentabilidad_xlsx_with_data_returns_valid_bytes() {
        let summary = sample_rentabilidad_with_rows();
        let result = exportar_rentabilidad_xlsx(&summary);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[0..2], b"PK");
    }

    #[test]
    fn exportar_rentabilidad_xlsx_empty_report_returns_valid_xlsx() {
        let summary = sample_empty_rentabilidad();
        let result = exportar_rentabilidad_xlsx(&summary);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[0..2], b"PK");
    }
}
