use genpdf::Element as _;
use genpdf::{elements, style};
use sea_orm::{DatabaseConnection, EntityTrait};
use uuid::Uuid;

use crate::entities::{contrato, inquilino, pago, propiedad};
use crate::errors::AppError;

fn load_font_family() -> Result<genpdf::fonts::FontFamily<genpdf::fonts::FontData>, AppError> {
    let regular = genpdf::fonts::FontData::new(
        include_bytes!("../../fonts/Arial-Regular.ttf").to_vec(),
        None,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error cargando fuente regular: {e}")))?;
    let bold =
        genpdf::fonts::FontData::new(include_bytes!("../../fonts/Arial-Bold.ttf").to_vec(), None)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error cargando fuente bold: {e}")))?;
    let italic = genpdf::fonts::FontData::new(
        include_bytes!("../../fonts/Arial-Italic.ttf").to_vec(),
        None,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error cargando fuente italic: {e}")))?;
    let bold_italic = genpdf::fonts::FontData::new(
        include_bytes!("../../fonts/Arial-BoldItalic.ttf").to_vec(),
        None,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error cargando fuente bold-italic: {e}")))?;

    Ok(genpdf::fonts::FontFamily {
        regular,
        bold,
        italic,
        bold_italic,
    })
}

fn format_date_dr(date: chrono::NaiveDate) -> String {
    date.format("%d/%m/%Y").to_string()
}

fn format_currency_dr(monto: rust_decimal::Decimal, moneda: &str) -> String {
    let monto_str = format!("{monto:.2}");
    let parts: Vec<&str> = monto_str.split('.').collect();
    let integer_part = parts[0];
    let decimal_part = parts.get(1).unwrap_or(&"00");

    let negative = integer_part.starts_with('-');
    let digits: String = integer_part
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();

    let mut formatted = String::new();
    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            formatted.push('.');
        }
        formatted.push(ch);
    }
    let formatted: String = formatted.chars().rev().collect();

    let sign = if negative { "-" } else { "" };
    format!("{sign}{moneda} {formatted},{decimal_part}")
}

pub async fn generar_recibo(db: &DatabaseConnection, pago_id: Uuid) -> Result<Vec<u8>, AppError> {
    let pago_model = pago::Entity::find_by_id(pago_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Pago no encontrado".to_string()))?;

    if pago_model.estado != "pagado" {
        return Err(AppError::Validation(
            "Solo se pueden generar recibos para pagos completados".to_string(),
        ));
    }

    let contrato_model = contrato::Entity::find_by_id(pago_model.contrato_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    let propiedad_model = propiedad::Entity::find_by_id(contrato_model.propiedad_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    let inquilino_model = inquilino::Entity::find_by_id(contrato_model.inquilino_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Inquilino no encontrado".to_string()))?;

    let receipt_number = format!("REC-{}", Uuid::new_v4().to_string()[..8].to_uppercase());

    let font_family = load_font_family()?;
    let mut doc = genpdf::Document::new(font_family);
    doc.set_title("Recibo de Pago");
    doc.set_minimal_conformance();

    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    doc.push(
        elements::Paragraph::new("Gestión Inmobiliaria")
            .styled(style::Style::new().bold().with_font_size(20)),
    );
    doc.push(elements::Break::new(0.5));
    doc.push(
        elements::Paragraph::new("Recibo de Pago")
            .styled(style::Style::new().bold().with_font_size(16)),
    );
    doc.push(elements::Break::new(1.0));

    doc.push(elements::Paragraph::new(format!(
        "No. Recibo: {receipt_number}"
    )));

    let fecha_emision = chrono::Utc::now().date_naive();
    doc.push(elements::Paragraph::new(format!(
        "Fecha de Emisión: {}",
        format_date_dr(fecha_emision)
    )));
    doc.push(elements::Break::new(1.0));

    doc.push(
        elements::Paragraph::new("Datos del Pago")
            .styled(style::Style::new().bold().with_font_size(14)),
    );
    doc.push(elements::Break::new(0.5));

    doc.push(elements::Paragraph::new(format!(
        "Propiedad: {}",
        propiedad_model.direccion
    )));
    doc.push(elements::Paragraph::new(format!(
        "Inquilino: {} {} - Cédula: {}",
        inquilino_model.nombre, inquilino_model.apellido, inquilino_model.cedula
    )));
    doc.push(elements::Paragraph::new(format!(
        "Contrato: {}",
        contrato_model.id
    )));
    doc.push(elements::Paragraph::new(format!(
        "Monto: {}",
        format_currency_dr(pago_model.monto, &pago_model.moneda)
    )));

    if let Some(fecha_pago) = pago_model.fecha_pago {
        doc.push(elements::Paragraph::new(format!(
            "Fecha de Pago: {}",
            format_date_dr(fecha_pago)
        )));
    }

    if let Some(ref metodo) = pago_model.metodo_pago {
        doc.push(elements::Paragraph::new(format!(
            "Método de Pago: {metodo}"
        )));
    }

    doc.push(elements::Break::new(2.0));
    doc.push(
        elements::Paragraph::new("Este recibo es comprobante de pago recibido.")
            .styled(style::Style::new().italic().with_font_size(10)),
    );

    let mut buf: Vec<u8> = Vec::new();
    doc.render(&mut buf)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error renderizando PDF: {e}")))?;

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_date_dr_formats_correctly() {
        let date = chrono::NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        assert_eq!(format_date_dr(date), "15/01/2025");
    }

    #[test]
    fn format_date_dr_single_digit_day_and_month() {
        let date = chrono::NaiveDate::from_ymd_opt(2025, 3, 5).unwrap();
        assert_eq!(format_date_dr(date), "05/03/2025");
    }

    #[test]
    fn format_currency_dr_dop() {
        let monto = rust_decimal::Decimal::new(2500000, 2);
        assert_eq!(format_currency_dr(monto, "DOP"), "DOP 25.000,00");
    }

    #[test]
    fn format_currency_dr_usd() {
        let monto = rust_decimal::Decimal::new(150000, 2);
        assert_eq!(format_currency_dr(monto, "USD"), "USD 1.500,00");
    }

    #[test]
    fn format_currency_dr_small_amount() {
        let monto = rust_decimal::Decimal::new(500, 2);
        assert_eq!(format_currency_dr(monto, "DOP"), "DOP 5,00");
    }

    #[test]
    fn format_currency_dr_zero() {
        let monto = rust_decimal::Decimal::ZERO;
        assert_eq!(format_currency_dr(monto, "DOP"), "DOP 0,00");
    }

    #[test]
    fn format_currency_dr_large_amount() {
        let monto = rust_decimal::Decimal::new(123456789, 2);
        assert_eq!(format_currency_dr(monto, "DOP"), "DOP 1.234.567,89");
    }
}
